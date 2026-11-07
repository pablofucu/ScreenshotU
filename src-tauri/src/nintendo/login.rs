//! Ventana de inicio de sesion de Nintendo. Abre la pagina de login en una ventana propia (sin
//! permisos de Tauri), intercepta la redireccion `npf…://auth` y canjea el codigo. Si la ventana no
//! sirve (captcha, bloqueo del webview), `nintendo_login_paste` acepta la URL copiada a mano.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use visor_core::nintendo::pkce::Pkce;
use visor_core::nintendo::HISTORY_REDIRECT_URI;

use super::auth;

const LABEL: &str = "nintendo-login";
/// Evento con el resultado del login (`LoginResult`).
pub const EVENT: &str = "nintendo-login-result";

static PENDING: Mutex<Option<Pkce>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize)]
pub struct LoginResult {
    pub ok: bool,
    pub message: String,
}

fn pending() -> std::sync::MutexGuard<'static, Option<Pkce>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

fn log(msg: &str) {
    eprintln!("[nintendo-login] {msg}");
}

/// Canjea la URL de vuelta, avisa a la interfaz y cierra la ventana.
fn finish(app: &AppHandle, redirect: String) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("nintendo-login-finish".into()).spawn(move || {
        let pkce = pending().clone();
        let result = match pkce {
            None => LoginResult { ok: false, message: "No hay un inicio de sesion en curso".into() },
            Some(p) => match auth::connect(&p, &redirect) {
                Ok(_) => {
                    *pending() = None;
                    LoginResult { ok: true, message: "Cuenta conectada".into() }
                }
                Err(e) => LoginResult { ok: false, message: e.to_string() },
            },
        };
        log(&format!("resultado: ok={} {}", result.ok, result.message));
        if let Some(w) = app.get_webview_window(LABEL) {
            let _ = w.close();
        }
        let _ = app.emit(EVENT, result);
    });
}

/// Abre la ventana de login de Nintendo.
#[tauri::command]
pub fn nintendo_login_start(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_focus();
        return Ok(());
    }
    let pkce = Pkce::new();
    let url = pkce.login_url();
    *pending() = Some(pkce);
    let parsed: tauri::Url = url.parse().map_err(|e| format!("URL de login invalida: {e}"))?;
    // `build` debe llamarse desde un hilo secundario (ver quickmenu::prewarm).
    let a = app.clone();
    std::thread::Builder::new()
        .name("nintendo-login".into())
        .spawn(move || {
            let nav = a.clone();
            let built = WebviewWindowBuilder::new(&a, LABEL, WebviewUrl::External(parsed))
                .title("Conectar cuenta de Nintendo")
                .inner_size(520.0, 760.0)
                .center()
                .on_navigation(move |u| {
                    let s = u.as_str();
                    if s.starts_with(HISTORY_REDIRECT_URI) {
                        log("redireccion capturada");
                        finish(&nav, s.to_string());
                        return false;
                    }
                    log(&format!("navega a {}", u.host_str().unwrap_or(u.scheme())));
                    true
                })
                .build();
            match built {
                Ok(_) => log("ventana abierta"),
                Err(e) => {
                    log(&format!("no se pudo abrir la ventana: {e}"));
                    let _ = a.emit(EVENT, LoginResult { ok: false, message: format!("No se pudo abrir la ventana: {e}") });
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Alternativa: el usuario pega la URL `npf…://auth#session_token_code=…` copiada del boton «Seleccionar esta cuenta».
#[tauri::command]
pub fn nintendo_login_paste(app: AppHandle, url: String) -> Result<(), String> {
    if pending().is_none() {
        return Err("Empieza primero el inicio de sesion".into());
    }
    finish(&app, url.trim().to_string());
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub connected: bool,
}

#[tauri::command]
pub fn nintendo_status() -> Status {
    Status { connected: auth::is_connected() }
}

#[tauri::command]
pub fn nintendo_logout() -> Result<(), String> {
    auth::disconnect().map_err(|e| e.to_string())
}
