//! Aprender un atajo F1-F24 desde Rust, sin depender de la webview: una tecla inyectada por software (DS4Windows, por
//! ejemplo) puede llegar a la pagina sin codigo de escaneo o no llegar. Un gancho de bajo nivel mira las F1-F24 y avisa
//! con `learn-key`; nunca se traga la tecla. Solo esta encendido mientras Ajustes espera un atajo.
use tauri::AppHandle;

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::{Mutex, OnceLock};
    use tauri::{AppHandle, Emitter};
    use visor_core::capture::fkey_combo;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
    };

    static TX: OnceLock<Mutex<Sender<String>>> = OnceLock::new();
    /// Hilo del gancho (0 = no hay).
    static THREAD: AtomicU32 = AtomicU32::new(0);
    /// Se esta aprendiendo: si se apaga antes de que el hilo termine de arrancar, el hilo se quita solo.
    static ON: AtomicBool = AtomicBool::new(false);

    unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
            let k = &*(lp.0 as *const KBDLLHOOKSTRUCT);
            let held = |vk: u16| GetAsyncKeyState(vk as i32) < 0;
            let text = fkey_combo(k.vkCode, held(VK_CONTROL.0), held(VK_MENU.0), held(VK_SHIFT.0), held(VK_LWIN.0) || held(VK_RWIN.0));
            if let (Some(text), Some(tx)) = (text, TX.get()) {
                // El gancho debe volver enseguida: solo avisa a otro hilo.
                if let Ok(tx) = tx.try_lock() {
                    let _ = tx.send(text);
                }
            }
        }
        CallNextHookEx(None, code, wp, lp)
    }

    pub fn set(app: &AppHandle, on: bool) {
        stop();
        if !on {
            return;
        }
        if TX.get().is_none() {
            let (tx, rx) = channel::<String>();
            let _ = TX.set(Mutex::new(tx));
            let app = app.clone();
            std::thread::spawn(move || {
                for key in rx {
                    // Lo que llegue cuando ya no se aprende lo ignora la interfaz (`app.learn` es `null`).
                    if ON.load(Ordering::SeqCst) {
                        let _ = app.emit("learn-key", key);
                    }
                }
            });
        }
        ON.store(true, Ordering::SeqCst);
        let spawned = std::thread::Builder::new().name("learn-key".into()).spawn(|| unsafe {
            let module = GetModuleHandleW(None).ok();
            let h = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), module.map(|m| m.into()), 0) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("no se pudo instalar el gancho para aprender teclas: {e}");
                    return;
                }
            };
            THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
            if ON.load(Ordering::SeqCst) {
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            let _ = UnhookWindowsHookEx(h);
        });
        if let Err(e) = spawned {
            eprintln!("no se pudo arrancar el hilo para aprender teclas: {e}");
        }
    }

    fn stop() {
        ON.store(false, Ordering::SeqCst);
        let t = THREAD.swap(0, Ordering::SeqCst);
        if t != 0 {
            unsafe {
                let _ = PostThreadMessageW(t, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use tauri::AppHandle;
    pub fn set(_app: &AppHandle, _on: bool) {}
}

/// Enciende o apaga el aprendizaje de teclas F1-F24 desde Rust.
pub fn set(app: &AppHandle, on: bool) {
    imp::set(app, on);
}
