//! Boton del raton que captura, con un gancho global de bajo nivel (funciona aunque el juego tenga el foco).
//! El clic no se bloquea: el juego lo sigue recibiendo.
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
        WH_MOUSE_LL, WM_MBUTTONDOWN, WM_XBUTTONDOWN,
    };

    // El gancho debe volver enseguida (si tarda, Windows lo quita): solo avisa a otro hilo.
    static TX: OnceLock<Mutex<Sender<u8>>> = OnceLock::new();

    unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let b = match wp.0 as u32 {
                WM_MBUTTONDOWN => 1,
                WM_XBUTTONDOWN => match (*(lp.0 as *const MSLLHOOKSTRUCT)).mouseData >> 16 {
                    1 => 2,
                    2 => 3,
                    _ => 0,
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
            let want = app.state::<crate::state::AppState>().mouse_button.load(Ordering::Relaxed);
            // Con el visor delante el raton es para los menus.
            if want != 0 && b == want && !crate::platform::own_window_focused() {
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
