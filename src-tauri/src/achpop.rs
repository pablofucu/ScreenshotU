//! Aviso de logro conseguido sobre el juego (`ach.html`): una tarjeta con el icono y el nombre del logro, abajo a la
//! derecha del monitor del juego. Misma clase de ventana que la miniatura de captura y el indicador REC: transparente,
//! siempre encima, sin foco, sin clics y fuera de las capturas. Se va sola a los `SHOW_MS`.
use crate::overlay;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use visor_core::achievements::Achievement;

const LABEL: &str = "achpop";
const W: f64 = 400.0;
const H: f64 = 96.0;
const MARGIN: i32 = 12;
/// Separacion del borde de abajo, para no quedar tapado por la barra de tareas si el juego va en ventana.
const BOTTOM: i32 = 56;
const SHOW_MS: u64 = 5000;

/// Cada aparicion tiene su numero: solo la ultima la oculta.
static GEN: AtomicU64 = AtomicU64::new(0);

/// Muestra el logro `ach` (con `icon` ya como `data:`) sobre el monitor del juego en marcha.
pub fn show(app: &AppHandle, ach: &Achievement) {
    let data = serde_json::json!({
        "name": ach.display_name,
        "desc": ach.description,
        "icon": ach.icon,
        "tier": ach.rarity_tier,
        "pct": ach.rarity_pct,
    });
    let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    let _ = std::thread::Builder::new().name("ach-popup".into()).spawn(move || {
        let pid = overlay::active_pid(&app);
        let a = app.clone();
        let _ = app.run_on_main_thread(move || {
            let win = match a.get_webview_window(LABEL) {
                Some(w) => {
                    let _ = w.eval(format!("window.__ach&&window.__ach({data})"));
                    w
                }
                // La primera vez los datos van en el fragmento: la pagina aun no ha cargado para recibir `eval`.
                None => match build(&a, &data) {
                    Some(w) => w,
                    None => return,
                },
            };
            place(&win, pid);
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

fn build(app: &AppHandle, data: &serde_json::Value) -> Option<tauri::WebviewWindow> {
    let b64 = visor_core::capture::base64(data.to_string().as_bytes());
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(format!("ach.html#{b64}").into()))
        .title("Logro")
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
        .map_err(|e| eprintln!("No se pudo crear el aviso de logro: {e}"))
        .ok()?;
    let _ = w.set_ignore_cursor_events(true);
    overlay::no_activate(&w);
    Some(w)
}

/// Esquina inferior derecha del monitor del juego.
fn place(win: &tauri::WebviewWindow, pid: Option<u32>) {
    let m = crate::reader::monitor::game_monitor(pid);
    // Primero en el monitor, para que el factor de escala sea el suyo; luego en la esquina.
    let _ = win.set_position(tauri::PhysicalPosition::new(m.x + 10, m.y + 10));
    let scale = win.scale_factor().unwrap_or(1.0);
    let _ = win.set_size(tauri::LogicalSize::new(W, H));
    let (wpx, hpx) = ((W * scale) as i32, (H * scale) as i32);
    let x = m.x + m.w as i32 - wpx - (MARGIN as f64 * scale) as i32;
    let y = m.y + m.h as i32 - hpx - (BOTTOM as f64 * scale) as i32;
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
}
