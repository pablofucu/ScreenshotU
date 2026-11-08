//! Tactil en streaming con Magpie (fases B1 y B2 de docs/plan-magpie-tablet.md).
//! Un gancho de raton de bajo nivel, activo solo durante la partida. Se enciende con archivos en
//! `%APPDATA%\app.visorcapturas.desktop`:
//! - `stream_touch_diag`: copia cada evento a `stream_touch.log`, sin bloquear nada.
//! - `stream_touch_fix`: mientras Magpie escala, los eventos que vienen de un toque (firma 0xFF5157xx) llegan con
//!   coordenadas de la imagen escalada; se bloquean y se repiten en el punto equivalente de la ventana real del juego.
//!   El raton fisico y el modo trackpad de Artemis (sin firma tactil) pasan sin tocar.

#[cfg(windows)]
mod imp {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::Mutex;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN,
        MOUSEEVENTF_RIGHTUP, MOUSEINPUT, MOUSE_EVENT_FLAGS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, EnumWindows, GetCursorPos, GetMessageW, GetPropW, PostThreadMessageW, SetCursorPos,
        SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_QUIT,
    };

    struct Ev {
        at: Instant,
        msg: u32,
        x: i32,
        y: i32,
        data: u32,
        flags: u32,
        extra: usize,
        /// Punto al que se ha llevado el evento, si la correccion lo ha bloqueado.
        fixed: Option<(i32, i32)>,
    }

    /// Orden para el hilo que repite los toques corregidos.
    struct Cmd {
        msg: u32,
        x: i32,
        y: i32,
    }

    /// Marca de los clics que repite el propio visor (no lleva la firma tactil, asi que el gancho los deja pasar).
    const OWN_MARK: usize = 0x5649_5300;

    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
    static TX: Mutex<Option<Sender<Ev>>> = Mutex::new(None);
    static FIX: AtomicBool = AtomicBool::new(false);
    static FIX_TX: Mutex<Option<Sender<Cmd>>> = Mutex::new(None);
    /// Copia de los rectangulos de Magpie (src y dest), refrescada por un hilo: el gancho no puede enumerar ventanas.
    static RECTS_OK: AtomicBool = AtomicBool::new(false);
    static RECTS: [AtomicIsize; 8] = [const { AtomicIsize::new(0) }; 8];

    fn is_touch(extra: usize) -> bool {
        (extra as u64) & 0xFFFF_FF00 == 0xFF51_5700
    }

    /// Imagen escalada -> ventana real del juego, la inversa de `SrcToScaling` de Magpie.
    /// Los toques en las bandas negras se llevan al borde mas cercano de la imagen.
    pub(super) fn dest_to_src(x: i32, y: i32, r: [isize; 8]) -> (i32, i32) {
        let axis = |v: i32, s0: isize, s1: isize, d0: isize, d1: isize| -> i32 {
            let (sw, dw) = (s1 - s0, d1 - d0);
            if sw <= 1 || dw <= 1 {
                return s0 as i32;
            }
            let v = (v as isize).clamp(d0, d1 - 1);
            let num = (v - d0) * (sw - 1);
            let den = dw - 1;
            (s0 + (num + den / 2) / den) as i32
        };
        (axis(x, r[0], r[2], r[4], r[6]), axis(y, r[1], r[3], r[5], r[7]))
    }

    fn cached_rects() -> Option<[isize; 8]> {
        if !RECTS_OK.load(Ordering::Acquire) {
            return None;
        }
        Some(std::array::from_fn(|i| RECTS[i].load(Ordering::Relaxed)))
    }

    fn store_rects(r: Option<[isize; 8]>) {
        match r {
            Some(v) => {
                if cached_rects() == Some(v) {
                    return;
                }
                RECTS_OK.store(false, Ordering::Release);
                for (a, b) in RECTS.iter().zip(v) {
                    a.store(b, Ordering::Relaxed);
                }
                RECTS_OK.store(true, Ordering::Release);
            }
            None => RECTS_OK.store(false, Ordering::Release),
        }
    }

    fn send_button(flags: MOUSE_EVENT_FLAGS) {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: OWN_MARK } },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn dir() -> Option<std::path::PathBuf> {
        std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("app.visorcapturas.desktop"))
    }

    unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let m = &*(lp.0 as *const MSLLHOOKSTRUCT);
            let msg = wp.0 as u32;
            let mut fixed = None;
            if FIX.load(Ordering::Relaxed) && is_touch(m.dwExtraInfo) && matches!(msg, 0x200 | 0x201 | 0x202 | 0x204 | 0x205) {
                if let Some(r) = cached_rects() {
                    let (x, y) = dest_to_src(m.pt.x, m.pt.y, r);
                    if let Ok(g) = FIX_TX.try_lock() {
                        if let Some(tx) = g.as_ref() {
                            if tx.send(Cmd { msg, x, y }).is_ok() {
                                fixed = Some((x, y));
                            }
                        }
                    }
                }
            }
            if let Ok(g) = TX.try_lock() {
                if let Some(tx) = g.as_ref() {
                    let _ = tx.send(Ev { at: Instant::now(), msg, x: m.pt.x, y: m.pt.y, data: m.mouseData, flags: m.flags, extra: m.dwExtraInfo, fixed });
                }
            }
            if fixed.is_some() {
                return LRESULT(1);
            }
        }
        CallNextHookEx(None, code, wp, lp)
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lp: LPARAM) -> BOOL {
        let prop = |n: &str| {
            let w: Vec<u16> = n.encode_utf16().chain(std::iter::once(0)).collect();
            GetPropW(hwnd, windows::core::PCWSTR(w.as_ptr())).0 as isize
        };
        // El valor 0 es indistinguible de «no existe»: basta con que algun extremo derecho/inferior no sea 0.
        let r = [
            prop("Magpie.SrcLeft"), prop("Magpie.SrcTop"), prop("Magpie.SrcRight"), prop("Magpie.SrcBottom"),
            prop("Magpie.DestLeft"), prop("Magpie.DestTop"), prop("Magpie.DestRight"), prop("Magpie.DestBottom"),
        ];
        if r[2] != 0 && r[3] != 0 && r[6] != 0 && r[7] != 0 {
            *(lp.0 as *mut Option<[isize; 8]>) = Some(r);
            return BOOL(0);
        }
        BOOL(1)
    }

    fn magpie_rects() -> Option<[isize; 8]> {
        let mut out: Option<[isize; 8]> = None;
        unsafe {
            let _ = EnumWindows(Some(enum_cb), LPARAM(&mut out as *mut _ as isize));
        }
        out
    }

    fn rects_text(r: Option<[isize; 8]>) -> String {
        match r {
            Some(v) => format!("src=({},{})-({},{}) dest=({},{})-({},{})", v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]),
            None => "magpie=no".to_string(),
        }
    }

    fn cursor() -> (i32, i32) {
        let mut p = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut p);
        }
        (p.x, p.y)
    }

    fn name(msg: u32) -> &'static str {
        match msg {
            0x200 => "MOVE",
            0x201 => "LDOWN",
            0x202 => "LUP",
            0x204 => "RDOWN",
            0x205 => "RUP",
            0x207 => "MDOWN",
            0x208 => "MUP",
            0x20A => "WHEEL",
            0x20B => "XDOWN",
            0x20C => "XUP",
            0x20E => "HWHEEL",
            _ => "?",
        }
    }

    pub fn start() {
        let Some(dir) = dir() else { return };
        let diag = dir.join("stream_touch_diag").exists();
        let fix = dir.join("stream_touch_fix").exists();
        if !(diag || fix) || ACTIVE.swap(true, Ordering::SeqCst) {
            return;
        }
        if fix {
            let (ctx, crx) = channel::<Cmd>();
            *FIX_TX.lock().unwrap() = Some(ctx);
            FIX.store(true, Ordering::SeqCst);
            // Refresca los rectangulos de Magpie mientras dure la sesion.
            std::thread::spawn(|| {
                while ACTIVE.load(Ordering::SeqCst) {
                    store_rects(magpie_rects());
                    std::thread::sleep(Duration::from_millis(100));
                }
                store_rects(None);
            });
            // Repite cada evento bloqueado en su sitio corregido, en el mismo orden en que llego.
            std::thread::spawn(move || {
                while let Ok(c) = crx.recv() {
                    unsafe {
                        let _ = SetCursorPos(c.x, c.y);
                    }
                    match c.msg {
                        0x201 => send_button(MOUSEEVENTF_LEFTDOWN),
                        0x202 => send_button(MOUSEEVENTF_LEFTUP),
                        0x204 => send_button(MOUSEEVENTF_RIGHTDOWN),
                        0x205 => send_button(MOUSEEVENTF_RIGHTUP),
                        _ => {}
                    }
                }
            });
        }
        if diag {
        let (tx, rx) = channel::<Ev>();
        *TX.lock().unwrap() = Some(tx);
        let log_path = dir.join("stream_touch.log");
        std::thread::spawn(move || {
            let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) else { return };
            let t0 = Instant::now();
            let wall = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let _ = writeln!(f, "=== inicio sesion (unix {wall}) ===");
            let mut last_rects: Option<Option<[isize; 8]>> = None;
            let mut last_move = Instant::now() - Duration::from_secs(1);
            let mut test_done = false;
            loop {
                match rx.recv_timeout(Duration::from_millis(250)) {
                    Ok(ev) => {
                        // Los movimientos llegan a rafagas: se deja uno cada 15 ms.
                        if ev.msg == 0x200 && ev.at.duration_since(last_move) < Duration::from_millis(15) {
                            continue;
                        }
                        if ev.msg == 0x200 {
                            last_move = ev.at;
                        }
                        let r = magpie_rects();
                        if last_rects != Some(r) {
                            let _ = writeln!(f, "[{:>8}ms] MAGPIE {}", ev.at.duration_since(t0).as_millis(), rects_text(r));
                            last_rects = Some(r);
                        }
                        let (cx, cy) = cursor();
                        let _ = writeln!(
                            f,
                            "[{:>8}ms] {:<6} pos=({},{}) data={:#x} flags={:#x}{} extra={:#x}{} cursor=({},{}){}",
                            ev.at.duration_since(t0).as_millis(),
                            name(ev.msg),
                            ev.x,
                            ev.y,
                            ev.data,
                            ev.flags,
                            if ev.flags & 1 != 0 { " INJECTED" } else { "" },
                            ev.extra,
                            if is_touch(ev.extra) { " TOUCH/PEN-SIG" } else { "" },
                            cx,
                            cy,
                            ev.fixed.map(|(x, y)| format!(" FIX->({x},{y})")).unwrap_or_default()
                        );
                        let _ = f.flush();
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Una vez, con la sesion asentada, se mueve el cursor 1 px para ver como aparece un movimiento propio.
                        if !test_done && t0.elapsed() > Duration::from_secs(8) {
                            test_done = true;
                            let (cx, cy) = cursor();
                            let _ = writeln!(f, "[{:>8}ms] TEST SetCursorPos desde ({cx},{cy}) a ({},{})", t0.elapsed().as_millis(), cx + 1, cy);
                            let _ = f.flush();
                            unsafe {
                                let _ = SetCursorPos(cx + 1, cy);
                                std::thread::sleep(Duration::from_millis(200));
                                let _ = SetCursorPos(cx, cy);
                            }
                            let _ = writeln!(f, "[{:>8}ms] TEST fin", t0.elapsed().as_millis());
                            let _ = f.flush();
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            let _ = writeln!(f, "=== fin sesion ===");
        });
        }
        std::thread::spawn(|| unsafe {
            HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
            let module = GetModuleHandleW(None).ok();
            let h = match SetWindowsHookExW(WH_MOUSE_LL, Some(hook), module.map(|m| m.into()), 0) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("stream_touch: no se pudo instalar el gancho: {e}");
                    ACTIVE.store(false, Ordering::SeqCst);
                    FIX.store(false, Ordering::SeqCst);
                    *TX.lock().unwrap() = None;
                    *FIX_TX.lock().unwrap() = None;
                    return;
                }
            };
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = UnhookWindowsHookEx(h);
        });
    }

    pub fn stop() {
        if !ACTIVE.swap(false, Ordering::SeqCst) {
            return;
        }
        let tid = HOOK_THREAD.swap(0, Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        FIX.store(false, Ordering::SeqCst);
        *TX.lock().unwrap() = None;
        *FIX_TX.lock().unwrap() = None;
    }

    #[cfg(test)]
    mod tests {
        use super::dest_to_src;
        // Tsukihime en la pantalla virtual de 2560x1600 (datos reales de docs/plan-magpie-tablet.md).
        const R: [isize; 8] = [142, 171, 2417, 1451, 0, 80, 2560, 1520];

        #[test]
        fn esquinas_y_centro() {
            assert_eq!(dest_to_src(0, 80, R), (142, 171));
            assert_eq!(dest_to_src(2559, 1519, R), (2416, 1450));
            assert_eq!(dest_to_src(1000, 800, R), (1031, 811));
        }

        #[test]
        fn bandas_al_borde() {
            assert_eq!(dest_to_src(46, 10, R), dest_to_src(46, 80, R));
            assert_eq!(dest_to_src(1280, 1599, R).1, 1450);
        }
    }
}

/// Empieza el registro si existe `stream_touch_diag`. Se puede llamar varias veces.
pub fn start() {
    #[cfg(windows)]
    imp::start();
}

/// Para el registro y quita el gancho.
pub fn stop() {
    #[cfg(windows)]
    imp::stop();
}
