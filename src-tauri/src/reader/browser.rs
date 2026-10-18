//! Abre la pagina de lectura en Edge o Chrome en modo aplicacion (`--app`) con un perfil propio,
//! donde el usuario instala Yomitan.
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

/// Ruta de Microsoft Edge o, si no esta, de Google Chrome.
pub fn browser_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let mut cands: Vec<PathBuf> = Vec::new();
        // Edge primero, luego Chrome.
        for (exe, rels) in [
            ("msedge.exe", ["Microsoft/Edge/Application/msedge.exe", ""]),
            ("chrome.exe", ["Google/Chrome/Application/chrome.exe", ""]),
        ] {
            for var in ["ProgramFiles(x86)", "ProgramFiles", "LocalAppData"] {
                if let Some(base) = std::env::var_os(var) {
                    if !rels[0].is_empty() {
                        cands.push(Path::new(&base).join(rels[0]));
                    }
                }
            }
            if let Some(p) = app_path_from_registry(exe) {
                cands.push(p);
            }
            if let Some(p) = cands.iter().find(|p| p.is_file()) {
                return Some(p.clone());
            }
            cands.clear();
        }
        None
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Lee `App Paths\<exe>` (valor por defecto) de HKLM y HKCU usando `reg.exe`.
#[cfg(windows)]
fn app_path_from_registry(exe: &str) -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    for hive in ["HKLM", "HKCU"] {
        let key = format!(r"{hive}\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{exe}");
        let out = Command::new("reg").args(["query", &key, "/ve"]).creation_flags(0x0800_0000).output().ok()?;
        if !out.status.success() {
            continue;
        }
        // Linea: "    (Predeterminado)    REG_SZ    C:\ruta\msedge.exe"
        let text = String::from_utf8_lossy(&out.stdout);
        if let Some(line) = text.lines().find(|l| l.contains("REG_SZ")) {
            if let Some(v) = line.split("REG_SZ").nth(1) {
                let p = PathBuf::from(v.trim().trim_matches('"'));
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Lanza el navegador en modo app. `rect` = (x, y, ancho, alto) en pixeles fisicos.
pub fn open(url: &str, profile_dir: &Path, rect: (i32, i32, u32, u32)) -> Result<Child, String> {
    let exe = browser_path().ok_or("No se encontro Microsoft Edge ni Google Chrome")?;
    std::fs::create_dir_all(profile_dir).map_err(|e| format!("No se pudo crear el perfil del navegador: {e}"))?;
    // Pantalla completa: ocupa el monitor entero, no solo su area de trabajo.
    let full = crate::reader::monitor::full_monitor(crate::reader::monitor::Rect { x: rect.0, y: rect.1, w: rect.2, h: rect.3 });
    let rect = (full.x, full.y, full.w, full.h);
    let (x, y, w, h) = rect;
    let mut cmd = Command::new(&exe);
    cmd.arg(format!("--app={url}"))
        .arg(format!("--user-data-dir={}", profile_dir.display()))
        .arg(format!("--window-position={x},{y}"))
        .arg(format!("--window-size={w},{h}"))
        .arg("--start-fullscreen")
        .arg("--no-first-run")
        .arg("--no-default-browser-check");
    let child = cmd.spawn().map_err(|e| format!("No se pudo abrir {}: {e}", exe.display()))?;
    #[cfg(windows)]
    {
        let pid = child.id();
        std::thread::spawn(move || imp::place_window(pid, rect));
    }
    Ok(child)
}

#[cfg(windows)]
mod imp {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, POINT};
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindow, GetWindowThreadProcessId, IsWindowVisible, SetWindowPos, GW_OWNER, SWP_NOACTIVATE,
        SWP_NOZORDER,
    };

    struct Search {
        pid: u32,
        found: Option<HWND>,
    }

    fn find_window(pid: u32) -> Option<HWND> {
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            let s = &mut *(lp.0 as *mut Search);
            let mut p = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut p));
            if p == s.pid && IsWindowVisible(hwnd).as_bool() {
                let owned = GetWindow(hwnd, GW_OWNER).map(|o| !o.is_invalid()).unwrap_or(false);
                let mut buf = [0u16; 64];
                let n = GetClassNameW(hwnd, &mut buf) as usize;
                // Edge y Chrome (Chromium): ventana principal de clase Chrome_WidgetWin_1.
                if !owned && String::from_utf16_lossy(&buf[..n.min(64)]) == "Chrome_WidgetWin_1" {
                    s.found = Some(hwnd);
                    return BOOL(0);
                }
            }
            BOOL(1)
        }
        let mut s = Search { pid, found: None };
        unsafe {
            let _ = EnumWindows(Some(cb), LPARAM(&mut s as *mut Search as isize));
        }
        s.found
    }

    /// Espera a que aparezca la ventana del navegador y, si salio en otro monitor (`--window-position` a veces se
    /// ignora), la lleva al de lectura ocupandolo entero (sigue a pantalla completa).
    pub fn place_window(pid: u32, (x, y, w, h): (i32, i32, u32, u32)) {
        let target = unsafe { MonitorFromPoint(POINT { x: x + w as i32 / 2, y: y + h as i32 / 2 }, MONITOR_DEFAULTTONEAREST) };
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            if let Some(hwnd) = find_window(pid) {
                std::thread::sleep(std::time::Duration::from_millis(400));
                if unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) } == target {
                    return;
                }
                for _ in 0..2 {
                    unsafe {
                        let _ = SetWindowPos(hwnd, None, x, y, w as i32, h as i32, SWP_NOZORDER | SWP_NOACTIVATE);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(600));
                }
                return;
            }
        }
    }
}
