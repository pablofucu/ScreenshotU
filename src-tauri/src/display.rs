//! Monitores y orden de las ventanas al jugar: el juego se abre en el monitor donde esta el visor (cambiando
//! el monitor principal de Windows mientras dura la partida si hace falta) y el visor queda justo detras.
use std::path::Path;

/// Un monitor activo del escritorio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monitor {
    /// Nombre de dispositivo de Windows (`\\.\DISPLAY1`).
    pub device: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub primary: bool,
}

impl Monitor {
    pub fn size(&self) -> (u32, u32) {
        (self.w, self.h)
    }
    /// «2560×1440 · principal» para Ajustes.
    pub fn label(&self, index: usize, total: usize) -> String {
        let p = if self.primary { " · principal" } else { "" };
        format!("{} de {} · {}×{}{p}", index + 1, total, self.w, self.h)
    }
}

/// Archivo donde se apunta el monitor principal original mientras se cambia (por si el visor se cierra a
/// mitad de partida: se restaura al volver a abrirlo).
const RESTORE_FILE: &str = "primary-restore.txt";
/// Monitor donde estaba el visor la ultima vez (para volver a abrirlo ahi).
const WINDOW_FILE: &str = "window-monitor.txt";

pub fn saved_window_monitor(config_dir: &Path) -> Option<String> {
    std::fs::read_to_string(config_dir.join(WINDOW_FILE)).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn save_window_monitor(config_dir: &Path, device: &str) {
    let _ = std::fs::write(config_dir.join(WINDOW_FILE), device);
}

/// Pone `device` como monitor principal si no lo es ya. Devuelve el principal anterior (para restaurarlo) y lo
/// deja apuntado en disco.
pub fn make_primary(config_dir: &Path, device: &str) -> Result<Option<String>, String> {
    let Some(prev) = monitors().into_iter().find(|m| m.primary).map(|m| m.device) else { return Ok(None) };
    if prev.eq_ignore_ascii_case(device) {
        return Ok(None);
    }
    imp::set_primary(device)?;
    let _ = std::fs::write(config_dir.join(RESTORE_FILE), &prev);
    Ok(Some(prev))
}

/// Vuelve a dejar como principal el monitor apuntado (si lo hay).
pub fn restore_primary(config_dir: &Path) {
    let f = config_dir.join(RESTORE_FILE);
    let Ok(dev) = std::fs::read_to_string(&f) else { return };
    let dev = dev.trim();
    if !dev.is_empty() && monitors().iter().any(|m| m.device.eq_ignore_ascii_case(dev) && !m.primary) {
        if let Err(e) = imp::set_primary(dev) {
            eprintln!("{e}");
            return;
        }
    }
    let _ = std::fs::remove_file(f);
}

pub fn monitors() -> Vec<Monitor> {
    imp::monitors()
}

/// Monitor donde esta una ventana (por su HWND).
pub fn of_window(hwnd: isize) -> Option<Monitor> {
    imp::of_window(hwnd)
}

/// Vigila la ventana principal del juego mientras aparece: la lleva al monitor `target` si salio en otro (y
/// cabe), la pone delante y deja el visor (`ours`) justo detras. Se repite con cada ventana principal nueva
/// (pantallas de carga, lanzadores) durante los primeros minutos.
pub fn follow_game(ours: isize, target: Option<String>, pids: std::sync::Arc<std::sync::Mutex<Vec<u32>>>, alive: impl Fn() -> bool) {
    let start = std::time::Instant::now();
    let mut last: Option<isize> = None;
    while alive() && start.elapsed() < std::time::Duration::from_secs(180) {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let list = pids.lock().unwrap().clone();
        let Some(game) = imp::main_window(&list) else { continue };
        if last == Some(game) {
            continue;
        }
        last = Some(game);
        if let Some(t) = target.as_deref() {
            imp::move_to_monitor(game, t);
        }
        imp::front_and_behind(game, ours);
    }
}

#[cfg(windows)]
mod imp {
    use super::Monitor;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{BOOL, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{
        ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW, MonitorFromWindow, CDS_NORESET,
        CDS_SET_PRIMARY, CDS_UPDATEREGISTRY, DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISP_CHANGE_SUCCESSFUL, DM_POSITION,
        ENUM_CURRENT_SETTINGS, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
        SetForegroundWindow, SetWindowPos, GWL_EXSTYLE, GW_OWNER, MONITORINFOF_PRIMARY, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        WS_EX_TOPMOST,
    };

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    fn from_wide(b: &[u16]) -> String {
        let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        String::from_utf16_lossy(&b[..n])
    }

    fn info(m: HMONITOR) -> Option<Monitor> {
        let mut mi = MONITORINFOEXW::default();
        mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        let ok = unsafe { GetMonitorInfoW(m, &mut mi as *mut MONITORINFOEXW as *mut MONITORINFO) }.as_bool();
        let r = mi.monitorInfo.rcMonitor;
        ok.then(|| Monitor {
            device: from_wide(&mi.szDevice),
            x: r.left,
            y: r.top,
            w: (r.right - r.left).max(0) as u32,
            h: (r.bottom - r.top).max(0) as u32,
            primary: mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        })
    }

    pub fn monitors() -> Vec<Monitor> {
        unsafe extern "system" fn cb(m: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
            (*(lp.0 as *mut Vec<HMONITOR>)).push(m);
            BOOL(1)
        }
        let mut hs: Vec<HMONITOR> = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(None, None, Some(cb), LPARAM(&mut hs as *mut Vec<HMONITOR> as isize));
        }
        let mut v: Vec<Monitor> = hs.into_iter().filter_map(info).collect();
        v.sort_by_key(|m| (m.x, m.y));
        v
    }

    pub fn of_window(hwnd: isize) -> Option<Monitor> {
        info(unsafe { MonitorFromWindow(HWND(hwnd as _), MONITOR_DEFAULTTONEAREST) })
    }

    /// Cambia el monitor principal: el nuevo pasa a (0,0) y los demas se desplazan lo mismo para conservar la
    /// disposicion del escritorio.
    pub fn set_primary(device: &str) -> Result<(), String> {
        let mut displays: Vec<(String, DEVMODEW)> = Vec::new();
        for i in 0.. {
            let mut dd = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
            if !unsafe { EnumDisplayDevicesW(PCWSTR::null(), i, &mut dd, 0) }.as_bool() {
                break;
            }
            if dd.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP != DISPLAY_DEVICE_ATTACHED_TO_DESKTOP {
                continue;
            }
            let name = from_wide(&dd.DeviceName);
            let mut dm = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
            if unsafe { EnumDisplaySettingsW(PCWSTR(wide(&name).as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm) }.as_bool() {
                displays.push((name, dm));
            }
        }
        let Some((_, target)) = displays.iter().find(|(n, _)| n.eq_ignore_ascii_case(device)) else {
            return Err(format!("No se encuentra el monitor {device}"));
        };
        let off = unsafe { target.Anonymous1.Anonymous2.dmPosition };
        for (name, dm) in displays.iter_mut() {
            let primary = name.eq_ignore_ascii_case(device);
            unsafe {
                let p = &mut dm.Anonymous1.Anonymous2.dmPosition;
                p.x -= off.x;
                p.y -= off.y;
            }
            dm.dmFields = DM_POSITION;
            let flags = if primary { CDS_SET_PRIMARY | CDS_UPDATEREGISTRY | CDS_NORESET } else { CDS_UPDATEREGISTRY | CDS_NORESET };
            let r = unsafe { ChangeDisplaySettingsExW(PCWSTR(wide(name).as_ptr()), Some(dm), None, flags, None) };
            if r != DISP_CHANGE_SUCCESSFUL {
                return Err(format!("Windows no dejo cambiar el monitor principal (codigo {})", r.0));
            }
        }
        // Aplica de golpe todos los cambios pendientes.
        let r = unsafe { ChangeDisplaySettingsExW(PCWSTR::null(), None, None, Default::default(), None) };
        if r != DISP_CHANGE_SUCCESSFUL {
            return Err(format!("Windows no dejo cambiar el monitor principal (codigo {})", r.0));
        }
        // Dar tiempo a que el escritorio se recoloque antes de lanzar nada.
        std::thread::sleep(std::time::Duration::from_millis(800));
        Ok(())
    }

    fn pid_of(hwnd: HWND) -> u32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        pid
    }

    /// Ventana principal visible mas grande de esos procesos.
    pub fn main_window(pids: &[u32]) -> Option<isize> {
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            (*(lp.0 as *mut Vec<HWND>)).push(hwnd);
            BOOL(1)
        }
        if pids.is_empty() {
            return None;
        }
        let mut all: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(cb), LPARAM(&mut all as *mut Vec<HWND> as isize));
        }
        let mut best: Option<(i64, HWND)> = None;
        for h in all {
            if !pids.contains(&pid_of(h)) {
                continue;
            }
            let visible = unsafe { IsWindowVisible(h).as_bool() && !IsIconic(h).as_bool() };
            let owned = unsafe { GetWindow(h, GW_OWNER) }.map(|o| !o.is_invalid()).unwrap_or(false);
            if !visible || owned {
                continue;
            }
            let mut r = RECT::default();
            if unsafe { GetWindowRect(h, &mut r) }.is_err() {
                continue;
            }
            let (w, hh) = ((r.right - r.left).max(0) as i64, (r.bottom - r.top).max(0) as i64);
            if w < 320 || hh < 200 {
                continue; // iconos de bandeja, ventanas auxiliares
            }
            if best.map_or(true, |(a, _)| w * hh > a) {
                best = Some((w * hh, h));
            }
        }
        best.map(|(_, h)| h.0 as isize)
    }

    /// Si la ventana salio en otro monitor y cabe en el bueno, la centra en el bueno.
    pub fn move_to_monitor(hwnd: isize, device: &str) {
        let h = HWND(hwnd as _);
        let Some(cur) = of_window(hwnd) else { return };
        if cur.device.eq_ignore_ascii_case(device) {
            return;
        }
        let Some(t) = monitors().into_iter().find(|m| m.device.eq_ignore_ascii_case(device)) else { return };
        let mut r = RECT::default();
        if unsafe { GetWindowRect(h, &mut r) }.is_err() {
            return;
        }
        let (w, hh) = (r.right - r.left, r.bottom - r.top);
        if w > t.w as i32 || hh > t.h as i32 {
            return; // no cabe: mejor no tocarla
        }
        let (x, y) = (t.x + (t.w as i32 - w) / 2, t.y + (t.h as i32 - hh) / 2);
        unsafe {
            let _ = SetWindowPos(h, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }

    /// Pone el juego en primer plano y el visor justo detras de el.
    pub fn front_and_behind(game: isize, ours: isize) {
        let (g, o) = (HWND(game as _), HWND(ours as _));
        unsafe {
            let fg = GetForegroundWindow();
            if fg != g {
                let _ = SetForegroundWindow(g);
            }
            // Detras de una ventana «siempre encima» el visor tambien lo seria: entonces no se toca.
            let topmost = GetWindowLongPtrW(g, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0;
            if ours != 0 && !topmost {
                let _ = SetWindowPos(o, Some(g), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Monitor;
    pub fn monitors() -> Vec<Monitor> {
        Vec::new()
    }
    pub fn of_window(_: isize) -> Option<Monitor> {
        None
    }
    pub fn set_primary(_: &str) -> Result<(), String> {
        Err("Solo en Windows".into())
    }
    pub fn main_window(_: &[u32]) -> Option<isize> {
        None
    }
    pub fn move_to_monitor(_: isize, _: &str) {}
    pub fn front_and_behind(_: isize, _: isize) {}
}
