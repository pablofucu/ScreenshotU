//! Magpie en Windows: localizar `Magpie.exe`, medir la ventana del juego y reiniciar Magpie para que lea los
//! perfiles nuevos. Magpie suele correr como administrador, asi que el reinicio lo hace el propio visor en un
//! modo auxiliar elevado (`--magpie-restart`), con un aviso de UAC.
use crate::state::AppState;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use visor_core::games::Game;
use visor_core::magpie;
use visor_core::userdata::MagpiePref;

/// Argumento del modo auxiliar elevado.
pub const RESTART_ARG: &str = "--magpie-restart";

/// Lo medido de la ventana de un juego.
#[derive(Debug, Clone)]
pub struct GameWindow {
    pub exe: String,
    pub class: String,
    pub native: (u32, u32),
    pub monitor: (u32, u32),
}

/// `Magpie.exe` a usar: el configurado, el que esta abierto o el primero que se encuentre en las carpetas
/// habituales (si hay varios, el usado mas recientemente segun sus registros).
pub fn find_exe(configured: &str) -> Option<PathBuf> {
    let c = configured.trim();
    if !c.is_empty() {
        return Some(PathBuf::from(c)).filter(|p| p.is_file());
    }
    if let Some(p) = imp::running_exe() {
        return Some(p);
    }
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(h) = env("USERPROFILE") {
        roots.extend(["Desktop", "Escritorio", "OneDrive\\Escritorio", "OneDrive\\Desktop", "scoop\\apps", "Downloads"].map(|d| h.join(d)));
    }
    if let Some(o) = env("OneDrive") {
        roots.extend([o.join("Escritorio"), o.join("Desktop")]);
    }
    if let Some(l) = env("LOCALAPPDATA") {
        roots.push(l.join("Programs"));
    }
    for k in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(p) = env(k) {
            roots.push(p);
        }
    }
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for r in roots {
        let Ok(rd) = std::fs::read_dir(&r) else { continue };
        for e in rd.flatten() {
            if !e.file_name().to_string_lossy().to_lowercase().contains("magpie") {
                continue;
            }
            // `scoop\apps\magpie\current\Magpie.exe` o `<carpeta>\Magpie.exe`.
            for exe in [e.path().join("Magpie.exe"), e.path().join("current").join("Magpie.exe")] {
                if exe.is_file() {
                    let used = std::fs::metadata(exe.with_file_name("logs").join("magpie.log"))
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    found.push((used, exe));
                }
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, p)| p).next()
}

/// `config.json` de esa instalacion de Magpie.
pub fn config_path(exe: &Path) -> Option<PathBuf> {
    let la = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
    magpie::config_path(exe, &la)
}

pub fn is_running() -> bool {
    imp::magpie_pids().next().is_some()
}

/// Busca la ventana principal del juego entre sus procesos y la mide.
pub fn find_game_window(pids: &[u32]) -> Option<GameWindow> {
    imp::find_game_window(pids)
}

/// Cierra Magpie (si esta abierto) y lo vuelve a abrir en la bandeja, desde un proceso elevado. Bloquea hasta
/// que termina; devuelve error si el usuario rechaza el permiso.
pub fn restart(exe: &Path) -> Result<(), String> {
    imp::restart_elevated(exe)
}

/// Modo auxiliar: si el visor se lanzo con `--magpie-restart <exe>`, reinicia Magpie y devuelve el codigo de
/// salida. Si no, `None` y se abre la aplicacion normal.
pub fn helper_main() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some(RESTART_ARG) {
        return None;
    }
    let Some(exe) = args.get(2).map(PathBuf::from).filter(|p| p.is_file()) else { return Some(2) };
    imp::kill_magpie();
    let started = std::process::Command::new(&exe).arg("-t").current_dir(exe.parent().unwrap_or(Path::new("."))).spawn();
    Some(if started.is_ok() { 0 } else { 3 })
}

/* ------------------------------------------------------------------ */
/* Al lanzar un juego                                                   */
/* ------------------------------------------------------------------ */

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MagpieEvent {
    pub id: String,
    pub message: String,
    pub error: bool,
}

enum Outcome {
    /// Magpie ya estaba al dia (o el juego no lo usa).
    Unchanged,
    /// Se escribio el perfil y se reinicio Magpie.
    Applied(String),
}

fn pref_of(st: &AppState, id: &str) -> Option<MagpiePref> {
    st.prefs.lock().unwrap().get(id).and_then(|p| p.magpie.clone())
}

fn store_pref(st: &AppState, id: &str, f: impl FnOnce(&mut MagpiePref)) {
    let mut p = st.prefs.lock().unwrap();
    if let Some(m) = p.get_mut(id).and_then(|t| t.magpie.as_mut()) {
        f(m);
        crate::state::log_db_err("preferencias de Magpie", st.db.with(|c| visor_core::db::data::pref_save(c, id, p.get(id))));
    }
}

/// Lleva el perfil de Magpie del juego a lo que pide su preferencia y reinicia Magpie si hubo cambios. Con
/// `ensure_running`, tambien lo abre si estaba cerrado.
fn sync(st: &AppState, game: &Game, ensure_running: bool) -> Result<Outcome, String> {
    let Some(pref) = pref_of(st, &game.id) else { return Ok(Outcome::Unchanged) };
    if !pref.enabled() && !pref.managed {
        return Ok(Outcome::Unchanged);
    }
    let configured = st.config.lock().unwrap().magpie_exe.clone();
    let exe = find_exe(&configured).ok_or("No se encuentra Magpie.exe: indica su ruta en Ajustes")?;
    let cfg_path = config_path(&exe).ok_or("No se encuentra la configuracion de Magpie: abrelo una vez a mano")?;
    let mut cfg = magpie::load(&cfg_path).map_err(|e| e.to_string())?;
    let enabled = pref.enabled();
    let (changed, summary) = if enabled {
        let (Some(native), Some(monitor), Some(class), Some(wexe)) = (pref.native, pref.monitor, pref.class.clone(), pref.exe.clone()) else {
            return Ok(Outcome::Unchanged); // aun sin medir: se mide cuando aparezca la ventana
        };
        let plan = magpie::choose(native, monitor, magpie::forced_scale(&pref.mode), magpie::has_cunny(&exe));
        (magpie::apply(&mut cfg, &wexe, &class, &game.name, &plan), plan.summary(native))
    } else {
        let changed = pref.exe.as_deref().map(|e| magpie::disable(&mut cfg, e)).unwrap_or(false);
        (changed, "escalado desactivado".to_string())
    };
    if changed {
        magpie::save(&cfg_path, &cfg).map_err(|e| e.to_string())?;
    }
    let s2 = summary.clone();
    store_pref(st, &game.id, |m| {
        m.summary = enabled.then_some(s2);
        m.managed = enabled;
    });
    if changed {
        restart(&exe)?;
        return Ok(Outcome::Applied(summary));
    }
    if enabled && ensure_running && !is_running() {
        restart(&exe)?;
    }
    Ok(Outcome::Unchanged)
}

/// Espera a que la ventana del juego este en primer plano y estable (unos segundos con el mismo tamano) y la mide.
fn measure(pids: &Arc<Mutex<Vec<u32>>>, timeout: Duration) -> Option<GameWindow> {
    let t = Instant::now();
    let mut last: Option<(GameWindow, Instant)> = None;
    while t.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(700));
        let list = pids.lock().unwrap().clone();
        if list.is_empty() || !imp::foreground_in(&list) {
            continue;
        }
        let Some(w) = find_game_window(&list) else { continue };
        match &last {
            Some((p, since)) if p.native == w.native && p.class == w.class => {
                if since.elapsed() >= Duration::from_secs(3) {
                    return Some(w);
                }
            }
            _ => last = Some((w, Instant::now())),
        }
    }
    None
}

/// Hilo de Magpie de una partida: aplica lo que ya se sabe del juego y, cuando aparece su ventana, la mide y
/// corrige el perfil si la resolucion o la clase cambiaron. Nunca bloquea el juego: los errores solo se avisan.
/// `monitor`: tamano del monitor donde se abrira el juego (el del visor); el perfil se calcula para ese
/// monitor desde el principio, aunque la ultima vez se jugara en otro.
pub fn on_launch(app: AppHandle, game: Game, pids: Arc<Mutex<Vec<u32>>>, monitor: Option<(u32, u32)>) {
    let st = app.state::<AppState>();
    let Some(mut pref) = pref_of(&st, &game.id) else { return };
    if !pref.enabled() && !pref.managed {
        return;
    }
    if let Some(m) = monitor.filter(|m| pref.enabled() && pref.monitor != Some(*m)) {
        store_pref(&st, &game.id, |p| p.monitor = Some(m));
        pref.monitor = Some(m);
    }
    let emit = |message: String, error: bool| {
        let _ = app.emit("magpie-status", MagpieEvent { id: game.id.clone(), message, error });
    };
    let report = |r: Result<Outcome, String>| match r {
        Ok(Outcome::Applied(s)) => emit(format!("Magpie: {s}"), false),
        Ok(Outcome::Unchanged) => {}
        Err(e) => emit(e, true),
    };
    report(sync(&st, &game, true));
    if !pref.enabled() {
        return;
    }
    let Some(w) = measure(&pids, Duration::from_secs(120)) else { return };
    let same = pref.native == Some(w.native)
        && pref.monitor == Some(w.monitor)
        && pref.class.as_deref() == Some(w.class.as_str())
        && pref.exe.as_deref().map(|e| e.eq_ignore_ascii_case(&w.exe)).unwrap_or(false);
    if same {
        return;
    }
    store_pref(&st, &game.id, |m| {
        m.native = Some(w.native);
        m.monitor = Some(w.monitor);
        m.class = Some(w.class.clone());
        m.exe = Some(w.exe.clone());
    });
    report(sync(&st, &game, true));
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MagpieStatus {
    /// Ruta de `Magpie.exe` encontrada ("" si no).
    pub exe: String,
    pub running: bool,
    /// Estan los modelos CuNNy2 (los mejores para novelas visuales).
    pub cunny: bool,
}

pub fn status(configured: &str) -> MagpieStatus {
    let exe = find_exe(configured);
    MagpieStatus {
        cunny: exe.as_deref().map(magpie::has_cunny).unwrap_or(false),
        exe: exe.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
        running: is_running(),
    }
}

#[cfg(windows)]
mod imp {
    use super::GameWindow;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    use windows::core::{BOOL, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HWND, LPARAM, RECT, WAIT_OBJECT_0};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };
    use windows::Win32::UI::HiDpi::{GetWindowDpiAwarenessContext, SetThreadDpiAwarenessContext};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetClientRect, GetForegroundWindow, GetWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible, GW_OWNER, SW_HIDE,
    };

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    pub fn magpie_pids() -> impl Iterator<Item = u32> {
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        sys.processes()
            .iter()
            .filter(|(_, p)| p.name().to_string_lossy().eq_ignore_ascii_case("Magpie.exe"))
            .map(|(pid, _)| pid.as_u32())
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Ruta completa del ejecutable de un proceso (funciona tambien con procesos elevados).
    pub fn exe_path(pid: u32) -> Option<String> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
            let _ = CloseHandle(h);
            r.ok()?;
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    pub fn running_exe() -> Option<PathBuf> {
        magpie_pids().find_map(exe_path).map(PathBuf::from).filter(|p| p.is_file())
    }

    fn top_windows() -> Vec<HWND> {
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            (*(lp.0 as *mut Vec<HWND>)).push(hwnd);
            BOOL(1)
        }
        let mut v: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(cb), LPARAM(&mut v as *mut Vec<HWND> as isize));
        }
        v
    }

    fn pid_of(hwnd: HWND) -> u32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        pid
    }

    /// Zona cliente tal y como la ve el propio juego (su resolucion real de render, aunque Windows la estire
    /// por no ser compatible con PPP altos).
    fn native_client(hwnd: HWND) -> Option<(u32, u32)> {
        unsafe {
            let prev = SetThreadDpiAwarenessContext(GetWindowDpiAwarenessContext(hwnd));
            let mut r = RECT::default();
            let ok = GetClientRect(hwnd, &mut r).is_ok();
            if !prev.is_invalid() {
                SetThreadDpiAwarenessContext(prev);
            }
            (ok && r.right > 0 && r.bottom > 0).then_some((r.right as u32, r.bottom as u32))
        }
    }

    fn monitor_size(hwnd: HWND) -> Option<(u32, u32)> {
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        unsafe { GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut mi) }.as_bool().then(|| {
            let r = mi.rcMonitor;
            ((r.right - r.left) as u32, (r.bottom - r.top) as u32)
        })
    }

    pub fn find_game_window(pids: &[u32]) -> Option<GameWindow> {
        let mut best: Option<(u64, HWND)> = None;
        for h in top_windows() {
            let visible = unsafe { IsWindowVisible(h).as_bool() && !IsIconic(h).as_bool() };
            // Solo ventanas principales (sin propietaria) de los procesos del juego.
            let owned = unsafe { GetWindow(h, GW_OWNER) }.map(|o| !o.is_invalid()).unwrap_or(false);
            if !visible || owned || !pids.contains(&pid_of(h)) {
                continue;
            }
            let Some((w, hh)) = native_client(h) else { continue };
            if w < 320 || hh < 200 {
                continue;
            }
            let a = w as u64 * hh as u64;
            if best.map(|(b, _)| a > b).unwrap_or(true) {
                best = Some((a, h));
            }
        }
        let (_, h) = best?;
        let mut buf = [0u16; 256];
        let n = unsafe { GetClassNameW(h, &mut buf) };
        Some(GameWindow {
            exe: exe_path(pid_of(h))?,
            class: String::from_utf16_lossy(&buf[..n.max(0) as usize]),
            native: native_client(h)?,
            monitor: monitor_size(h)?,
        })
    }

    /// La ventana en primer plano es de uno de esos procesos.
    pub fn foreground_in(pids: &[u32]) -> bool {
        let h = unsafe { GetForegroundWindow() };
        !h.is_invalid() && pids.contains(&pid_of(h))
    }

    pub fn kill_magpie() {
        for pid in magpie_pids() {
            unsafe {
                if let Ok(h) = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, pid) {
                    let _ = TerminateProcess(h, 0);
                    let _ = WaitForSingleObject(h, 5000);
                    let _ = CloseHandle(h);
                }
            }
        }
        let t = Instant::now();
        while magpie_pids().next().is_some() && t.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn restart_elevated(exe: &Path) -> Result<(), String> {
        let me = std::env::current_exe().map_err(|e| e.to_string())?;
        let file = wide(&me.to_string_lossy());
        let params = wide(&format!("{} \"{}\"", super::RESTART_ARG, exe.display()));
        let verb = wide("runas");
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(params.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };
        unsafe {
            if let Err(e) = ShellExecuteExW(&mut info) {
                return Err(if e.code() == ERROR_CANCELLED.to_hresult() {
                    "No se dio permiso para reiniciar Magpie".into()
                } else {
                    format!("No se pudo reiniciar Magpie: {e}")
                });
            }
            if info.hProcess.is_invalid() {
                return Ok(());
            }
            let waited = WaitForSingleObject(info.hProcess, 20_000);
            let mut code = 0u32;
            let _ = GetExitCodeProcess(info.hProcess, &mut code);
            let _ = CloseHandle(info.hProcess);
            if waited != WAIT_OBJECT_0 {
                return Err("Magpie tarda demasiado en reiniciarse".into());
            }
            match code {
                0 => Ok(()),
                _ => Err(format!("No se pudo abrir Magpie (codigo {code})")),
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::GameWindow;
    use std::path::{Path, PathBuf};
    pub fn magpie_pids() -> impl Iterator<Item = u32> {
        std::iter::empty()
    }
    pub fn running_exe() -> Option<PathBuf> {
        None
    }
    pub fn find_game_window(_: &[u32]) -> Option<GameWindow> {
        None
    }
    pub fn foreground_in(_: &[u32]) -> bool {
        false
    }
    pub fn kill_magpie() {}
    pub fn restart_elevated(_: &Path) -> Result<(), String> {
        Err("Magpie solo existe en Windows".into())
    }
}
