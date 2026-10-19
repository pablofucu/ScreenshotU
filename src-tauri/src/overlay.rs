//! Indicador «● REC 0:03» sobre la pantalla del juego: ventana Tauri pequena, transparente, siempre encima, sin foco y
//! que no captura clics (`rec.html`), en la esquina superior derecha del monitor del juego. Se crea la primera vez que
//! hace falta y despues solo se muestra, se oculta y se actualiza (`window.__rec(estado, segundos)`).
//! No se ve sobre juegos en pantalla completa exclusiva; en ventana o sin bordes si.
//! Mientras se ve, un hilo lo vuelve a poner encima cada 250 ms: al hacer clic en el juego (sobre todo si va sin bordes a
//! pantalla completa o se pone el mismo «siempre encima») Windows lo sube por encima del indicador y este desaparecia.
use crate::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "recindicator";
const W: f64 = 190.0;
const H: f64 = 44.0;
const MARGIN: i32 = 18;

static APP: std::sync::Mutex<Option<AppHandle>> = std::sync::Mutex::new(None);
/// El indicador esta visible (el hilo que lo mantiene encima sigue mientras sea `true`).
static VISIBLE: AtomicBool = AtomicBool::new(false);
static KEEPER: AtomicBool = AtomicBool::new(false);

/// Guarda el `AppHandle` para poder mostrar el indicador desde cualquier hilo.
pub fn init(app: &AppHandle) {
    *APP.lock().unwrap() = Some(app.clone());
}

/// Muestra el indicador con un estado: "recording" (REC y segundos), "done" (✓ y segundos) o "error".
pub fn show(state: &'static str, secs: u64) {
    let Some(app) = APP.lock().unwrap().clone() else { return };
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        let app = app2;
        let win = match app.get_webview_window(LABEL) {
            Some(w) => w,
            None => match build(&app, state, secs) {
                Some(w) => w,
                None => return,
            },
        };
        place(&app, &win);
        show_quiet(&win);
        let _ = win.eval(format!("window.__rec&&window.__rec('{state}',{secs})"));
    });
    VISIBLE.store(true, Ordering::SeqCst);
    keep_on_top(app);
}

/// Hilo que, mientras el indicador se ve, lo devuelve encima de todo (sin activarlo).
fn keep_on_top(app: AppHandle) {
    if KEEPER.swap(true, Ordering::SeqCst) {
        return;
    }
    let spawned = std::thread::Builder::new().name("rec-top".into()).spawn(move || {
        while VISIBLE.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(250));
            let a = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = a.get_webview_window(LABEL) {
                    to_top(&w);
                }
            });
        }
        KEEPER.store(false, Ordering::SeqCst);
    });
    if spawned.is_err() {
        KEEPER.store(false, Ordering::SeqCst);
    }
}

/// El indicador esta en pantalla (la miniatura de captura se coloca debajo).
pub fn is_visible() -> bool {
    VISIBLE.load(Ordering::SeqCst)
}

/// Alto fisico ocupado por el indicador en la esquina (margen incluido), con la escala de `scale`.
pub fn corner_height(scale: f64) -> i32 {
    MARGIN + (H * scale) as i32
}

pub fn hide(app: &AppHandle) {
    VISIBLE.store(false, Ordering::SeqCst);
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = app2.get_webview_window(LABEL) {
            hide_quiet(&w);
        }
    });
}

fn build(app: &AppHandle, state: &str, secs: u64) -> Option<tauri::WebviewWindow> {
    let url = format!("rec.html?s={state}&t={secs}");
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(url.into()))
        .title("REC")
        .inner_size(W, H)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|e| eprintln!("No se pudo crear el indicador de grabacion: {e}"))
        .ok()?;
    let _ = w.set_ignore_cursor_events(true);
    no_activate(&w);
    Some(w)
}

/// Estilos de Windows para que nunca robe el foco ni salga en Alt+Tab, y que no salga en las capturas (las de
/// monitor completo lo incluirian).
#[cfg(windows)]
pub(crate) fn no_activate(w: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowDisplayAffinity, SetWindowLongPtrW, GWL_EXSTYLE, WDA_EXCLUDEFROMCAPTURE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT};
    if let Ok(h) = w.hwnd() {
        let hwnd = HWND(h.0);
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TRANSPARENT.0) as isize);
            let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn no_activate(_w: &tauri::WebviewWindow) {}

/// Lo pone el primero de las ventanas «siempre encima», sin activarlo ni moverlo.
#[cfg(windows)]
pub(crate) fn to_top(w: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOOWNERZORDER};
    if let Ok(h) = w.hwnd() {
        unsafe {
            let _ = SetWindowPos(HWND(h.0), Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER);
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn to_top(_w: &tauri::WebviewWindow) {}

/// Muestra la ventana encima de todo sin activarla. `WebviewWindow::show` usa `SW_SHOW`, que la activa: como el
/// atajo de captura da permiso de primer plano, le quitaba el foco al juego y uno a pantalla completa (emuladores)
/// salia y volvia a entrar en ella, con un segundo de pantalla en negro.
/// Se oculta con [`hide_quiet`]: Tauri no se entera de este `show` y su `hide` no haria nada.
#[cfg(windows)]
pub(crate) fn show_quiet(w: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_SHOWWINDOW};
    if let Ok(h) = w.hwnd() {
        unsafe {
            let _ = SetWindowPos(HWND(h.0), Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW);
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn show_quiet(w: &tauri::WebviewWindow) {
    let _ = w.show();
}

#[cfg(windows)]
pub(crate) fn hide_quiet(w: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    if let Ok(h) = w.hwnd() {
        unsafe {
            let _ = ShowWindow(HWND(h.0), SW_HIDE);
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn hide_quiet(w: &tauri::WebviewWindow) {
    let _ = w.hide();
}

/// Esquina superior derecha del monitor donde esta la ventana del juego.
fn place(app: &AppHandle, win: &tauri::WebviewWindow) {
    let pid = {
        let st = app.state::<AppState>();
        let p = st.active.lock().unwrap().as_ref().and_then(|a| a.pids.lock().unwrap().first().copied());
        p.or_else(|| crate::texthook::status().proc.map(|p| p.pid))
    };
    let m = crate::reader::monitor::game_monitor(pid);
    // Primero en el monitor, para que el factor de escala sea el suyo; luego en la esquina.
    let _ = win.set_position(tauri::PhysicalPosition::new(m.x + 10, m.y + 10));
    let scale = win.scale_factor().unwrap_or(1.0);
    let _ = win.set_size(tauri::LogicalSize::new(W, H));
    let wpx = (W * scale) as i32;
    let _ = win.set_position(tauri::PhysicalPosition::new(m.x + m.w as i32 - wpx - MARGIN, m.y + MARGIN));
}
