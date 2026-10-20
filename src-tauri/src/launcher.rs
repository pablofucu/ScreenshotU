//! Lanzamiento de juegos y vigilancia del proceso (tiempo de juego y juego «activo» para las capturas).
use crate::state::{Active, AppState};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::{AppHandle, Emitter, Manager};
use visor_core::games::{self, Game};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEvent {
    pub id: String,
    pub name: String,
    pub seconds: u64,
}

/// Abre una carpeta en el explorador de archivos.
pub fn open_folder(path: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    let prog = "explorer";
    #[cfg(not(windows))]
    let prog = "xdg-open";
    Command::new(prog).arg(path).spawn().map(|_| ()).map_err(|e| format!("No se pudo abrir la carpeta: {e}"))
}

/// Abre el explorador con un archivo seleccionado.
pub fn reveal(path: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("No se pudo abrir el explorador: {e}"))
    }
    #[cfg(not(windows))]
    {
        let dir = path.parent().unwrap_or(path);
        open_folder(dir)
    }
}

fn spawn_exe(exe: &str, args: &[String]) -> Result<Child, String> {
    let mut c = Command::new(exe);
    c.args(args);
    if let Some(dir) = std::path::Path::new(exe).parent().filter(|d| d.is_dir()) {
        c.current_dir(dir);
    }
    c.spawn().map_err(|e| format!("No se pudo iniciar {exe}: {e}"))
}

/// HWND de la ventana principal del visor (0 si no hay).
pub fn main_hwnd(app: &AppHandle) -> isize {
    #[cfg(windows)]
    {
        app.get_webview_window("main").and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize).unwrap_or(0)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        0
    }
}

/// Deja que el juego que se va a lanzar se ponga delante aunque tarde en abrir su ventana.
fn allow_foreground() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Lanza el juego en el monitor del visor, deja el visor detras y empieza a vigilar el proceso.
pub fn start(app: &AppHandle, game: Game) -> Result<(), String> {
    let st = app.state::<AppState>();
    if let Some(a) = st.active.lock().unwrap().as_ref() {
        return Err(format!("Ya hay un juego en marcha: {}", a.name));
    }
    let (exe, args) = game.launch.command().ok_or("Este juego no tiene ejecutable")?;
    if !std::path::Path::new(&exe).is_file() {
        return Err(format!("No existe el ejecutable: {exe}"));
    }
    // El juego se abre en el monitor del visor: si no es el principal, pasa a serlo mientras dure la partida
    // (los juegos a pantalla completa siempre salen en el principal).
    let ours = main_hwnd(app);
    let target = crate::display::of_window(ours);
    if let Some(t) = &target {
        crate::display::save_window_monitor(&st.config_dir, &t.device);
        if let Err(e) = crate::display::make_primary(&st.config_dir, &t.device) {
            eprintln!("{e}");
        }
    }
    allow_foreground();
    let child: Option<Child> = match spawn_exe(&exe, &args) {
        Ok(c) => Some(c),
        Err(e) => {
            crate::display::restore_primary(&st.config_dir);
            return Err(e);
        }
    };
    {
        let mut stats = st.stats.lock().unwrap();
        let now = now_ms();
        games::mark_started(&mut stats, &game.id, now);
        crate::state::log_db_err("inicio de partida", st.db.with(|c| visor_core::db::data::stat_mark_started(c, &game.id, now)));
    }
    let pids = Arc::new(Mutex::new(child.as_ref().map(|c| vec![c.id()]).unwrap_or_default()));
    *st.active.lock().unwrap() = Some(Active {
        id: game.id.clone(),
        name: game.name.clone(),
        platform: game.platform.clone(),
        folder: game.folder.clone(),
        since: now_ms(),
        pids: pids.clone(),
    });
    let _ = app.emit("game-started", GameEvent { id: game.id.clone(), name: game.name.clone(), seconds: 0 });
    // El visor se queda abierto justo detras del juego (sin minimizar).
    {
        let (app2, id2, pids2) = (app.clone(), game.id.clone(), pids.clone());
        let device = target.as_ref().map(|t| t.device.clone());
        std::thread::spawn(move || {
            let alive = || app2.state::<AppState>().active.lock().unwrap().as_ref().map(|a| a.id == id2).unwrap_or(false);
            crate::display::follow_game(ours, device, pids2, alive)
        });
    }
    let monitor = target.map(|t| t.size());
    let (app2, game2, pids2) = (app.clone(), game.clone(), pids.clone());
    std::thread::spawn(move || crate::magpie::on_launch(app2, game2, pids2, monitor));
    if game.platform == visor_core::config::NOVELS_PLATFORM {
        let (app2, game2, pids2) = (app.clone(), game.clone(), pids.clone());
        std::thread::spawn(move || crate::novel::on_launch(app2, game2, pids2));
    }
    let app2 = app.clone();
    std::thread::spawn(move || watch(app2, game, child, pids));
    Ok(())
}

/// Vigila el juego cada pocos segundos: cuenta el tiempo, mantiene los PIDs y avisa al terminar.
fn watch(app: AppHandle, game: Game, mut child: Option<Child>, pids: Arc<Mutex<Vec<u32>>>) {
    let own = std::process::id();
    let dir = game.install_dir.clone();
    let mut sys = System::new();
    let launched = Instant::now();
    let mut first_seen: Option<Instant> = None;
    let mut first_seen_ms: i64 = 0;
    let mut misses = 0u32;
    loop {
        std::thread::sleep(Duration::from_millis(500));
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );
        let mut found: Vec<u32> = Vec::new();
        if let Some(d) = dir.as_deref() {
            for (pid, p) in sys.processes() {
                if pid.as_u32() == own {
                    continue;
                }
                if p.exe().map(|e| games::path_inside(&e.to_string_lossy(), d)).unwrap_or(false) {
                    found.push(pid.as_u32());
                }
            }
        }
        if let Some(c) = child.as_mut() {
            if matches!(c.try_wait(), Ok(None)) && !found.contains(&c.id()) {
                found.push(c.id());
            }
        }
        if found.is_empty() {
            if first_seen.is_some() {
                misses += 1;
                if misses >= 4 {
                    break; // ~2 s sin ningun proceso: la partida termino
                }
            } else if launched.elapsed() > Duration::from_secs(180) || (dir.is_none() && child.is_none() && launched.elapsed() > Duration::from_secs(60)) {
                break; // nunca llego a verse el proceso
            }
        } else {
            misses = 0;
            if first_seen.is_none() {
                first_seen = Some(Instant::now());
                first_seen_ms = now_ms();
            }
            *pids.lock().unwrap() = found;
        }
    }
    let seconds = first_seen.map(|t| t.elapsed().as_secs().saturating_sub(2)).unwrap_or(0);
    let st = app.state::<AppState>();
    if seconds > 0 {
        let mut stats = st.stats.lock().unwrap();
        let end = first_seen_ms + seconds as i64 * 1000;
        games::add_session(&mut stats, &game.id, first_seen_ms, end);
        crate::state::log_db_err("partida", st.db.with(|c| visor_core::db::data::stat_add_session(c, &game.id, first_seen_ms, end)));
    }
    {
        let mut a = st.active.lock().unwrap();
        if a.as_ref().map(|a| a.id == game.id).unwrap_or(false) {
            *a = None;
        }
    }
    crate::display::restore_primary(&st.config_dir);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    if game.platform == visor_core::config::NOVELS_PLATFORM {
        crate::novel::on_exit(&app, &game.id);
    }
    if seconds > 0 {
        crate::saves::after_session(&app, &game);
    }
    let _ = app.emit("game-exited", GameEvent { id: game.id, name: game.name, seconds });
}
