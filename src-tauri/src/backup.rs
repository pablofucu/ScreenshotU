//! Comandos de copia de seguridad y restauracion, y avisos de arranque. La logica esta en `visor_core::backup`.
use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, State};
use visor_core::backup as core;

/// Aviso para la interfaz nacido al arrancar. `kind`: "error" | "info".
#[derive(Debug, Clone, Serialize)]
pub struct Notice {
    pub kind: &'static str,
    pub message: String,
}

pub fn db_failed_notice(e: &impl std::fmt::Display) -> Notice {
    Notice {
        kind: "error",
        message: format!(
            "No se pudo abrir la base de datos (visor.db): {e}. Esta sesión funciona en memoria y NO se guardará nada (favoritas, horas, estados...). Cierra el visor y revisa el archivo, o restaura una copia de seguridad."
        ),
    }
}

pub fn restore_notice<E: std::fmt::Display>(r: Result<core::Restored, E>) -> Notice {
    match r {
        Ok(_) => Notice { kind: "info", message: "Copia de seguridad restaurada. La base anterior está en visor.db.antes-de-restaurar.".into() },
        Err(e) => Notice { kind: "error", message: format!("No se pudo restaurar la copia: {e}. Se mantienen los datos anteriores.") },
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub path: String,
    pub size: u64,
    pub media_files: usize,
}

/// Crea el .zip de copia de seguridad en `dest` (visor.db + config.json + opcionalmente el glosario).
#[tauri::command]
pub async fn backup_create(state: State<'_, AppState>, dest: String, include_media: bool) -> Result<BackupResult, String> {
    let (db, dir, media) = (state.db.clone(), state.paths.data.clone(), state.paths.glossary.clone());
    let path = PathBuf::from(&dest);
    tauri::async_runtime::spawn_blocking(move || {
        core::create_backup(&db, &dir, &media, &path, include_media).map(|i| BackupResult { path: dest, size: i.size, media_files: i.media_files })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub db_version: i64,
    pub size: u64,
    pub has_config: bool,
    pub media_files: usize,
}

/// Extrae y valida un .zip de copia; no cambia nada hasta `restore_apply`.
#[tauri::command]
pub async fn restore_prepare(state: State<'_, AppState>, zip: String) -> Result<RestorePreview, String> {
    let dir = state.paths.data.clone();
    tauri::async_runtime::spawn_blocking(move || core::stage_restore(&PathBuf::from(zip), &dir))
        .await
        .map_err(|e| e.to_string())?
        .map(|i| RestorePreview { db_version: i.db.version, size: i.db.size, has_config: i.has_config, media_files: i.media_files })
        .map_err(|e| format!("{e:#}"))
}

/// El usuario no confirmo: se descarta lo extraido.
#[tauri::command]
pub fn restore_cancel(state: State<'_, AppState>) {
    core::cancel_restore(&state.paths.data);
}

/// Confirmada: reinicia el visor; al arrancar se sustituye la base (ver `visor_core::backup`).
#[tauri::command]
pub fn restore_apply(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if !state.paths.data.join(core::PENDING_DIR).join(core::READY_FILE).is_file() {
        return Err("No hay ninguna copia preparada para restaurar".into());
    }
    app.restart()
}

/// Avisos de arranque pendientes de mostrar (se entregan una sola vez).
#[tauri::command]
pub fn startup_notices(state: State<'_, AppState>) -> Vec<Notice> {
    std::mem::take(&mut *state.notices.lock().unwrap())
}
