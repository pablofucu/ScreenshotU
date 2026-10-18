//! Comandos Tauri de la ventana de hook (`th_*`). Se registran en `lib.rs` como `texthook::commands::<nombre>`.
use crate::novel::ThStatus;
use tauri::AppHandle;

#[tauri::command]
pub fn th_status() -> ThStatus {
    super::status()
}

#[tauri::command]
pub fn th_select_thread(key: String) {
    super::select_thread(key);
}

#[tauri::command]
pub fn th_select_name_thread(key: Option<String>) {
    super::select_name_thread(key);
}

#[tauri::command]
pub fn th_insert_code(code: String) -> Result<(), String> {
    super::insert_code(&code)
}

#[tauri::command]
pub fn th_remove_hook(key: String) -> Result<(), String> {
    super::remove_hook(&key)
}

#[tauri::command]
pub fn th_search_text(app: AppHandle, text: String, codepage: u32, deep: bool) -> Result<(), String> {
    super::search_text(&app, &text, codepage, deep)
}

#[tauri::command]
pub fn th_search_cancel(app: AppHandle) {
    super::search_cancel(&app);
}

/// Bloquea unos instantes (desconecta, espera y vuelve a inyectar): se ejecuta en un hilo aparte.
#[tauri::command]
pub async fn th_reinject(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || super::reinject(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn th_detach(app: AppHandle) {
    super::detach_all(&app);
}
