//! Diagnostico del tactil en streaming (fase B1 de docs/plan-magpie-tablet.md).
//! Solo observa: un gancho de raton de bajo nivel copia cada evento a un registro, sin bloquear nada.
//! Solo se activa si existe el archivo `stream_touch_diag` en `%APPDATA%\app.visorcapturas.desktop`.
//! Registro: `stream_touch.log` en esa misma carpeta.

#[cfg(windows)]
mod imp {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::Mutex;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
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
    }

    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
    static TX: Mutex<Option<Sender<Ev>>> = Mutex::new(None);

    fn dir() -> Option<std::path::PathBuf> {
        std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("app.visorcapturas.desktop"))
    }

    unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let m = &*(lp.0 as *const MSLLHOOKSTRUCT);
            if let Ok(g) = TX.try_lock() {
                if let Some(tx) = g.as_ref() {
                    let _ = tx.send(Ev { at: Instant::now(), msg: wp.0 as u32, x: m.pt.x, y: m.pt.y, data: m.mouseData, flags: m.flags, extra: m.dwExtraInfo });
                }
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
        if !dir.join("stream_touch_diag").exists() || ACTIVE.swap(true, Ordering::SeqCst) {
            return;
        }
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
                            "[{:>8}ms] {:<6} pos=({},{}) data={:#x} flags={:#x}{} extra={:#x}{} cursor=({},{})",
                            ev.at.duration_since(t0).as_millis(),
                            name(ev.msg),
                            ev.x,
                            ev.y,
                            ev.data,
                            ev.flags,
                            if ev.flags & 1 != 0 { " INJECTED" } else { "" },
                            ev.extra,
                            if (ev.extra as u64) & 0xFFFF_FF00 == 0xFF51_5700 { " TOUCH/PEN-SIG" } else { "" },
                            cx,
                            cy
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
        std::thread::spawn(|| unsafe {
            HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
            let module = GetModuleHandleW(None).ok();
            let h = match SetWindowsHookExW(WH_MOUSE_LL, Some(hook), module.map(|m| m.into()), 0) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("stream_touch: no se pudo instalar el gancho: {e}");
                    ACTIVE.store(false, Ordering::SeqCst);
                    *TX.lock().unwrap() = None;
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
        *TX.lock().unwrap() = None;
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
