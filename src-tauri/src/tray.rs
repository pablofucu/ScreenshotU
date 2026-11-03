//! Icono en la bandeja del sistema: permite que el visor siga en segundo plano (atajos de
//! captura activos) cuando se cierra la ventana, si el usuario lo activa en Ajustes
//! (`Config::keep_in_tray`). Sin esto, cerrar "main" sale siempre del todo (`exit_app`).
use crate::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

const ID: &str = "visor";
static HIDDEN: AtomicBool = AtomicBool::new(false);
static WAS_FULL: AtomicBool = AtomicBool::new(true);

pub fn is_hidden() -> bool {
    HIDDEN.load(Ordering::SeqCst)
}

/// Crea el icono de la bandeja si no existe. `false` = no se pudo crear (no se debe ocultar el visor).
pub fn ensure(app: &AppHandle) -> bool {
    if app.tray_by_id(ID).is_some() {
        return true;
    }
    let Some(icon) = app.default_window_icon().cloned() else { return false };
    let build = || -> tauri::Result<()> {
        let show = MenuItem::with_id(app, "tray-show", "Mostrar visor", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "tray-quit", "Salir", true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&show, &quit])?;
        TrayIconBuilder::with_id(ID)
            .icon(icon)
            .tooltip("Visor de capturas")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|a, e| match e.id.as_ref() {
                "tray-show" => show_main(a),
                "tray-quit" => a.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|t, e| match e {
                TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }
                | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => show_main(t.app_handle()),
                _ => {}
            })
            .build(app)?;
        Ok(())
    };
    build().map_err(|e| eprintln!("No se pudo crear el icono de la bandeja: {e}")).is_ok()
}

/// Pone o quita el icono segun el ajuste `keep_in_tray`. Si se desactiva con el visor oculto, lo
/// vuelve a mostrar antes de quitar el icono: nunca se deja el visor oculto sin icono.
pub fn sync(app: &AppHandle, on: bool) {
    if on {
        ensure(app);
    } else {
        if is_hidden() {
            show_main(app);
        }
        let _ = app.remove_tray_by_id(ID);
    }
}

/// Oculta "main" en vez de cerrarla. Si no hay icono de bandeja (o no se pudo crear), sale del todo.
pub fn hide_main(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else { return app.exit(0) };
    if !ensure(app) {
        return app.exit(0);
    }
    WAS_FULL.store(w.is_fullscreen().unwrap_or(true), Ordering::SeqCst);
    HIDDEN.store(true, Ordering::SeqCst);
    let _ = w.hide();
}

/// Vuelve a mostrar "main" tal y como estaba (mismo monitor, pantalla completa si la tenia).
pub fn show_main(app: &AppHandle) {
    HIDDEN.store(false, Ordering::SeqCst);
    let Some(w) = app.get_webview_window("main") else { return app.exit(0) };
    let _ = w.show();
    let _ = w.unminimize();
    if WAS_FULL.load(Ordering::SeqCst) && !w.is_fullscreen().unwrap_or(false) {
        let _ = w.set_fullscreen(true);
    }
    // Truco de always-on-top: Windows suele negar el foco a una ventana oculta (ver launcher.rs).
    let _ = w.set_always_on_top(true);
    let _ = w.set_focus();
    let _ = w.set_always_on_top(false);
}

/// Cerrar el visor: a la bandeja si `keep_in_tray` esta activado, si no, sale del todo.
pub fn close_main(app: &AppHandle) {
    let keep = app.try_state::<AppState>().map(|s| s.config.lock().unwrap().keep_in_tray).unwrap_or(false);
    if keep {
        hide_main(app);
    } else {
        app.exit(0);
    }
}
