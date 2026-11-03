//! Boton del raton que captura, con un gancho global de bajo nivel (funciona aunque el juego tenga el foco).
//! El clic no se bloquea: el juego lo sigue recibiendo. El mismo gancho sirve para el menu sobre el juego, que tambien
//! necesita saber cuando se suelta el boton (mantenerlo fuerza el cierre).
use tauri::AppHandle;

#[cfg(windows)]
pub fn spawn(app: AppHandle) {
    use std::sync::atomic::Ordering;
    use std::sync::mpsc::{channel, Sender};
    use std::sync::{Mutex, OnceLock};
    use tauri::Manager;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, HC_ACTION, MSG, MSLLHOOKSTRUCT,
        WH_MOUSE_LL, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
    };

    /// Bit que marca que el boton se ha soltado.
    const RELEASED: u8 = 0x80;
    // El gancho debe volver enseguida (si tarda, Windows lo quita): solo avisa a otro hilo.
    static TX: OnceLock<Mutex<Sender<u8>>> = OnceLock::new();

    unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let x = || match (*(lp.0 as *const MSLLHOOKSTRUCT)).mouseData >> 16 {
                1 => 2,
                2 => 3,
                _ => 0,
            };
            let b = match wp.0 as u32 {
                WM_MBUTTONDOWN => 1,
                WM_XBUTTONDOWN => x(),
                WM_MBUTTONUP => 1 | RELEASED,
                WM_XBUTTONUP => match x() {
                    0 => 0,
                    n => n | RELEASED,
                },
                _ => 0,
            };
            if b != 0 {
                if let Some(tx) = TX.get() {
                    if let Ok(tx) = tx.try_lock() {
                        let _ = tx.send(b);
                    }
                }
            }
        }
        CallNextHookEx(None, code, wp, lp)
    }

    let (tx, rx) = channel::<u8>();
    let _ = TX.set(Mutex::new(tx));
    std::thread::spawn(move || {
        for b in rx {
            let st = app.state::<crate::state::AppState>();
            let (want, menu) = (st.mouse_button.load(Ordering::Relaxed), st.overlay_mouse.load(Ordering::Relaxed));
            let (button, released) = (b & !RELEASED, b & RELEASED != 0);
            if menu != 0 && button == menu {
                if released {
                    crate::quickmenu::trigger_up(&app, crate::quickmenu::Src::Mouse);
                } else if !crate::platform::own_window_focused() || crate::quickmenu::is_open() {
                    crate::quickmenu::trigger_down(&app, crate::quickmenu::Src::Mouse, 0);
                }
                continue;
            }
            // Con el visor delante el raton es para los menus.
            if !released && want != 0 && button == want && !crate::platform::own_window_focused() && !crate::quickmenu::is_open() {
                crate::snap::trigger(&app);
            }
        }
    });
    std::thread::spawn(move || unsafe {
        let module = GetModuleHandleW(None).ok();
        let h = match SetWindowsHookExW(WH_MOUSE_LL, Some(hook), module.map(|m| m.into()), 0) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("no se pudo instalar el gancho del raton: {e}");
                return;
            }
        };
        let _ = h;
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

#[cfg(not(windows))]
pub fn spawn(_app: AppHandle) {}
