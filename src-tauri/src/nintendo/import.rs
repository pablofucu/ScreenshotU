//! Comandos de la importacion de la lista de juegos de Nintendo (SW-17). La logica esta en
//! `visor_core::nintendo::import`; aqui solo se reunen los datos del sistema (base, carpetas de la categoria Switch).

use std::path::Path;

use serde::Serialize;
use tauri::State;
use visor_core::db::data as dbd;
use visor_core::games;
use visor_core::nintendo::import::{self, Choice, Preview, Report};
use visor_core::nintendo::link::{self, Candidate};
use visor_core::nintendo::store;
use visor_core::nintendo::SWITCH_PLATFORM;
use visor_core::scan::find_dir_ci;

use crate::state::AppState;

/// Fichas y carpetas sueltas de la categoria Switch. Las carpetas se leen un nivel (solo nombres): barato en el HDD.
pub(crate) fn switch_candidates(state: &AppState) -> Result<Vec<Candidate>, String> {
    let manual = state.db.with(dbd::manual_load).map_err(|e| e.to_string())?;
    let all = games::all_games(&manual);
    let cfg = state.config.lock().unwrap().clone();
    let mut folders: Vec<String> = Vec::new();
    if let (Some(root), Some(p)) = (cfg.root.as_deref(), cfg.platforms.iter().find(|p| p.id == SWITCH_PLATFORM)) {
        if let Some(dir) = find_dir_ci(Path::new(root), &p.folder) {
            if let Ok(rd) = std::fs::read_dir(dir) {
                folders = rd
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect();
                folders.sort_by_key(|f| f.to_lowercase());
            }
        }
    }
    Ok(link::candidates(&all, &folders))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReport {
    pub preview: Preview,
    /// Error al refrescar el historial; la vista previa se hace igual con lo guardado.
    pub error: Option<String>,
}

fn new_id() -> String {
    games::new_manual_id()
}

/// Vista previa de lo que se importaria. Con `refresh` baja antes el historial (un error no la impide).
#[tauri::command]
pub async fn nintendo_import_preview(
    state: State<'_, AppState>,
    min_minutes: Option<i64>,
    refresh: bool,
) -> Result<PreviewReport, String> {
    let min = min_minutes.unwrap_or_else(|| state.config.lock().unwrap().nintendo_min_minutes as i64).max(0);
    let cands = switch_candidates(&state)?;
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let error = if refresh { super::refresh(&db).err().map(|e| e.to_string()) } else { None };
        let titles = db.with(store::titles_load).map_err(|e| e.to_string())?;
        Ok(PreviewReport { preview: import::preview(&titles, &cands, min), error })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Aplica las decisiones del usuario. El frontend vuelve a escanear una vez despues.
#[tauri::command]
pub async fn nintendo_import_apply(state: State<'_, AppState>, choices: Vec<Choice>) -> Result<Report, String> {
    let cands = switch_candidates(&state)?;
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || db.with(|c| import::apply(c, &choices, &cands, new_id)).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

/// Enlaza (o desenlaza con `(None, None)`) un juego de Nintendo con una ficha o carpeta de la categoria Switch.
#[tauri::command]
pub async fn nintendo_link(
    state: State<'_, AppState>,
    nx_id: String,
    game_id: Option<String>,
    folder: Option<String>,
) -> Result<(), String> {
    if game_id.is_some() || folder.is_some() {
        let ok = switch_candidates(&state)?.iter().any(|c| match (&game_id, &folder) {
            (Some(g), _) => c.game_id.as_deref() == Some(g.as_str()),
            (None, Some(f)) => c.game_id.is_none() && &c.folder == f,
            _ => false,
        });
        if !ok {
            return Err("Esa ficha no es de la categoria Switch".into());
        }
    }
    let found = state
        .db
        .with(|c| store::title_set_link(c, &nx_id, game_id.as_deref(), folder.as_deref()))
        .map_err(|e| e.to_string())?;
    if found { Ok(()) } else { Err("Juego de Nintendo no encontrado".into()) }
}

#[tauri::command]
pub fn nintendo_set_ignored(state: State<'_, AppState>, nx_id: String, ignored: bool) -> Result<(), String> {
    let found = state.db.with(|c| store::title_set_ignored(c, &nx_id, ignored)).map_err(|e| e.to_string())?;
    if found { Ok(()) } else { Err("Juego de Nintendo no encontrado".into()) }
}
