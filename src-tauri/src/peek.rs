//! Miniatura de la captura recien hecha, muy pequena, en la esquina superior derecha del monitor del juego (debajo del
//! indicador REC si esta a la vista). Misma ventana que el indicador: transparente, siempre encima, sin foco, sin clics
//! y fuera de las capturas (`peek.html`). Se va sola a los `SHOW_MS`.
//! No se muestra sobre juegos que cubren todo el monitor (pantalla completa o sin bordes): parpadeaban en negro.
use crate::overlay;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "shotpeek";
/// Ancho logico de la miniatura; el alto sale de la proporcion de la captura.
const W: f64 = 112.0;
const MARGIN: i32 = 18;
const GAP: i32 = 8;
const SHOW_MS: u64 = 1700;
/// Ancho del JPEG que se manda (algo mas que `W` para pantallas con escalado).
pub const THUMB_W: u32 = 240;

/// Cada aparicion tiene su numero: solo la ultima la oculta.
static GEN: AtomicU64 = AtomicU64::new(0);

fn height_for(width: u32, height: u32) -> f64 {
    if width == 0 {
        return W * 9.0 / 16.0;
    }
    (W * height as f64 / width as f64).clamp(40.0, W)
}

/// La ventana en primer plano cubre su monitor entero (pantalla completa o sin bordes). Poner otra ventana «siempre
/// encima» sobre ella hace que Windows saque al juego del modo de pantalla completa optimizado y la pantalla parpadea
/// en negro al aparecer y al desaparecer la miniatura.
#[cfg(windows)]
pub fn foreground_fullscreen() -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        let mut r = RECT::default();
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetWindowRect(fg, &mut r).is_err() || !GetMonitorInfoW(MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST), &mut mi).as_bool() {
            return false;
        }
        let m = mi.rcMonitor;
        r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
    }
}

#[cfg(not(windows))]
pub fn foreground_fullscreen() -> bool {
    false
}

/// Muestra `jpeg` (miniatura de una captura de `width`x`height`) sobre el monitor del proceso `pid`.
pub fn show(app: &AppHandle, jpeg: &[u8], width: u32, height: u32, pid: Option<u32>) {
    if foreground_fullscreen() {
        return;
    }
    let url = format!("data:image/jpeg;base64,{}", visor_core::capture::base64(jpeg));
    let h = height_for(width, height);
    let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    let _ = std::thread::Builder::new().name("shot-peek".into()).spawn(move || {
        let a = app.clone();
        let _ = app.run_on_main_thread(move || {
            let win = match a.get_webview_window(LABEL) {
                Some(w) => {
                    let _ = w.eval(format!("window.__peek&&window.__peek('{url}')"));
                    w
                }
                // La primera vez la imagen va en el fragmento: la pagina aun no ha cargado para recibir `eval`.
                None => match build(&a, &url) {
                    Some(w) => w,
                    None => return,
                },
            };
            place(&win, h, pid);
            overlay::show_quiet(&win);
        });

        // Se mantiene encima (al hacer clic en el juego Windows lo bajaria) y se oculta al acabar.
        let mut left = SHOW_MS;
        while left > 0 {
            let step = left.min(250);
            std::thread::sleep(Duration::from_millis(step));
            left -= step;
            if GEN.load(Ordering::SeqCst) != gen {
                return;
            }
            let a = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = a.get_webview_window(LABEL) {
                    if left == 0 {
                        overlay::hide_quiet(&w);
                    } else {
                        overlay::to_top(&w);
                    }
                }
            });
        }
    });
}

fn build(app: &AppHandle, data_url: &str) -> Option<tauri::WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(format!("peek.html#{data_url}").into()))
        .title("Captura")
        .inner_size(W, W * 9.0 / 16.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|e| eprintln!("No se pudo crear la miniatura de captura: {e}"))
        .ok()?;
    let _ = w.set_ignore_cursor_events(true);
    overlay::no_activate(&w);
    Some(w)
}

/// Esquina superior derecha del monitor del juego, debajo del indicador REC si se ve.
fn place(win: &tauri::WebviewWindow, h: f64, pid: Option<u32>) {
    let m = crate::reader::monitor::game_monitor(pid);
    // Primero en el monitor, para que el factor de escala sea el suyo; luego en la esquina.
    let _ = win.set_position(tauri::PhysicalPosition::new(m.x + 10, m.y + 10));
    let scale = win.scale_factor().unwrap_or(1.0);
    let _ = win.set_size(tauri::LogicalSize::new(W, h));
    let wpx = (W * scale) as i32;
    let top = if overlay::is_visible() { overlay::corner_height(scale) + GAP } else { MARGIN };
    let _ = win.set_position(tauri::PhysicalPosition::new(m.x + m.w as i32 - wpx - MARGIN, m.y + top));
}
