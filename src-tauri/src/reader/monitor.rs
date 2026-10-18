//! Eleccion del monitor de lectura: el que queda a la izquierda del monitor donde esta la ventana del juego.
use serde::{Deserialize, Serialize};

/// Rectangulo en pixeles fisicos del escritorio virtual.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

/// Area de trabajo del monitor donde se muestra la pagina de lectura.
/// - Monitor del juego: el de su ventana principal visible mas grande (o el principal si aun no hay ventana).
/// - Resultado: el monitor de la izquierda (borde derecho mas cercano por la izquierda y con solape vertical);
///   si no hay, otro monitor cualquiera; si solo hay uno, ese mismo.
pub fn reading_monitor(game_pid: Option<u32>) -> Rect {
    #[cfg(windows)]
    {
        imp::reading_monitor(game_pid)
    }
    #[cfg(not(windows))]
    {
        let _ = game_pid;
        Rect { x: 0, y: 0, w: 1920, h: 1080 }
    }
}

/// Monitor completo (pixeles fisicos) que contiene el centro de `r` (p. ej. el area de trabajo de `reading_monitor`).
pub fn full_monitor(r: Rect) -> Rect {
    #[cfg(windows)]
    {
        imp::full_monitor(r)
    }
    #[cfg(not(windows))]
    {
        r
    }
}

/// Monitor completo (pixeles fisicos) donde esta la ventana del juego (o el principal si aun no hay ventana).
pub fn game_monitor(game_pid: Option<u32>) -> Rect {
    #[cfg(windows)]
    {
        imp::game_monitor(game_pid)
    }
    #[cfg(not(windows))]
    {
        let _ = game_pid;
        Rect { x: 0, y: 0, w: 1920, h: 1080 }
    }
}

/// Elige el monitor de lectura entre `mons` = (monitor completo, area de trabajo) dado el indice del monitor del juego.
/// Separado de la API de Windows para poder razonarlo/probarlo.
#[allow(dead_code)]
fn pick_left(mons: &[(Rect, Rect)], game: usize) -> usize {
    let g = mons[game].0;
    let overlaps_v = |a: Rect, b: Rect| a.y < b.y + b.h as i32 && b.y < a.y + a.h as i32;
    let right = |r: Rect| r.x + r.w as i32;
    // A la izquierda: borde derecho <= borde izquierdo del juego (con pequena tolerancia), el mas cercano.
    let left = mons
        .iter()
        .enumerate()
        .filter(|(i, (m, _))| *i != game && right(*m) <= g.x + 2 && overlaps_v(*m, g))
        .max_by_key(|(_, (m, _))| right(*m))
        .map(|(i, _)| i);
    if let Some(i) = left {
        return i;
    }
    // Otro monitor cualquiera: el mas cercano en horizontal.
    let other = mons
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != game)
        .min_by_key(|(_, (m, _))| ((m.x - g.x).abs(), (m.y - g.y).abs()))
        .map(|(i, _)| i);
    other.unwrap_or(game)
}

#[cfg(windows)]
mod imp {
    use super::{pick_left, Rect};
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, HDC, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        MONITOR_DEFAULTTOPRIMARY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible, GW_OWNER,
    };

    fn rect(r: RECT) -> Rect {
        Rect { x: r.left, y: r.top, w: (r.right - r.left).max(0) as u32, h: (r.bottom - r.top).max(0) as u32 }
    }

    fn info(m: HMONITOR) -> Option<(Rect, Rect)> {
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        unsafe { GetMonitorInfoW(m, &mut mi) }.as_bool().then(|| (rect(mi.rcMonitor), rect(mi.rcWork)))
    }

    pub fn full_monitor(r: Rect) -> Rect {
        let p = windows::Win32::Foundation::POINT { x: r.x + r.w as i32 / 2, y: r.y + r.h as i32 / 2 };
        info(unsafe { MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST) }).map(|(full, _)| full).unwrap_or(r)
    }

    /// Todos los monitores: (handle, area completa, area de trabajo).
    fn monitors() -> Vec<(HMONITOR, Rect, Rect)> {
        unsafe extern "system" fn cb(m: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
            (*(lp.0 as *mut Vec<HMONITOR>)).push(m);
            BOOL(1)
        }
        let mut hs: Vec<HMONITOR> = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(None, None, Some(cb), LPARAM(&mut hs as *mut Vec<HMONITOR> as isize));
        }
        hs.into_iter().filter_map(|h| info(h).map(|(f, w)| (h, f, w))).collect()
    }

    /// Ventana principal visible mas grande de un proceso.
    fn game_window(pid: u32) -> Option<HWND> {
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            (*(lp.0 as *mut Vec<HWND>)).push(hwnd);
            BOOL(1)
        }
        let mut all: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(cb), LPARAM(&mut all as *mut Vec<HWND> as isize));
        }
        let mut best: Option<(i64, HWND)> = None;
        for h in all {
            let mut p = 0u32;
            unsafe { GetWindowThreadProcessId(h, Some(&mut p)) };
            if p != pid {
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
            let area = (r.right - r.left).max(0) as i64 * (r.bottom - r.top).max(0) as i64;
            if area > 0 && best.map_or(true, |(a, _)| area > a) {
                best = Some((area, h));
            }
        }
        best.map(|(_, h)| h)
    }

    pub fn game_monitor(game_pid: Option<u32>) -> Rect {
        let h = game_pid
            .and_then(game_window)
            .map(|h| unsafe { MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST) })
            .unwrap_or_else(|| unsafe { MonitorFromPoint(Default::default(), MONITOR_DEFAULTTOPRIMARY) });
        info(h).map(|(full, _)| full).unwrap_or(Rect { x: 0, y: 0, w: 1920, h: 1080 })
    }

    pub fn reading_monitor(game_pid: Option<u32>) -> Rect {
        let mons = monitors();
        let game_h = game_pid
            .and_then(game_window)
            .map(|h| unsafe { MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST) })
            .unwrap_or_else(|| unsafe { MonitorFromPoint(Default::default(), MONITOR_DEFAULTTOPRIMARY) });
        if mons.is_empty() {
            return Rect { x: 0, y: 0, w: 1920, h: 1080 };
        }
        let game = mons.iter().position(|(h, ..)| *h == game_h).unwrap_or(0);
        let pairs: Vec<(Rect, Rect)> = mons.iter().map(|(_, f, w)| (*f, *w)).collect();
        pairs[pick_left(&pairs, game)].1
    }
}
