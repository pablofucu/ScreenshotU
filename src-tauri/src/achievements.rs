//! Comandos Tauri y vigilancia en vivo para el sistema de logros.
use crate::snap;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use visor_core::achievements::{
    self as core, calculate_summary, classify_game, find_app_id, find_runtime_files,
    find_schema_file, normalize_variant, parse_runtime_json, parse_schema_json,
    Achievement, AchievementSummary, GameDetection,
};
use visor_core::games::Game;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockEvent {
    pub app_id: String,
    pub achievement: Achievement,
    pub has_capture: bool,
}

/// Resuelve el AppID y directorio de instalacion de un juego.
fn resolve_game_info(state: &AppState, game_id: &str, app_id_hint: Option<&str>) -> (Option<String>, Option<PathBuf>) {
    if let Some(hint) = app_id_hint {
        if !hint.trim().is_empty() {
            return (Some(hint.trim().to_string()), None);
        }
    }

    let games = state.games.lock().unwrap();
    let game = games.iter().find(|g| g.id == game_id);

    if let Some(g) = game {
        let exe_path = g.launch.command().map(|(exe, _)| PathBuf::from(exe));
        let game_dir = g
            .install_dir
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| exe_path.as_ref().and_then(|e| e.parent().map(|p| p.to_path_buf())));

        if let Some(ref dir) = game_dir {
            if let Some(id) = find_app_id(dir, exe_path.as_deref()) {
                return (Some(id), game_dir);
            }
        }

        if let Some(rest) = g.id.strip_prefix("steam:") {
            return (Some(rest.to_string()), game_dir);
        }

        return (Some(game_id.to_string()), game_dir);
    }

    (Some(game_id.to_string()), None)
}

/// Sincroniza schema y runtime desde disco hacia la base SQLite y devuelve la lista de logros.
pub fn sync_achievements_from_disk(
    state: &AppState,
    game_id: &str,
    app_id_opt: Option<&str>,
) -> Result<(String, Vec<Achievement>), String> {
    let (app_id, game_dir) = resolve_game_info(state, game_id, app_id_opt);
    let app_id = app_id.ok_or_else(|| "No se pudo determinar el AppID del juego".to_string())?;

    // 1. Intentar cargar schema de disco si existe directorio
    if let Some(ref dir) = game_dir {
        if let Some(schema_path) = find_schema_file(dir) {
            if let Ok(content) = std::fs::read_to_string(&schema_path) {
                let base = schema_path.parent();
                if let Ok(schema_list) = parse_schema_json(&app_id, &content, base) {
                    if !schema_list.is_empty() {
                        let _ = state.db.with(|c| core::achievements_save(c, &app_id, &schema_list));
                    }
                }
            }
        }
    }

    // 2. Leer runtime files conocidos
    let dir_ref = game_dir.as_deref().unwrap_or_else(|| Path::new("."));
    let runtimes = find_runtime_files(&app_id, dir_ref);

    for rf in &runtimes {
        if let Ok(content) = std::fs::read_to_string(rf) {
            if let Ok(map) = parse_runtime_json(&content) {
                let ev_str = rf.to_string_lossy();
                for (api_name, (earned, time_opt)) in map {
                    if earned {
                        let ts = time_opt.unwrap_or_else(|| {
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs() as i64)
                                .unwrap_or(0)
                        });
                        let _ = state.db.with(|c| {
                            core::unlock_record(c, &app_id, &api_name, ts, "gse", Some(&ev_str))
                        });
                    }
                }
            }
        }
    }

    // 3. Cargar lista actualizada de la base SQLite
    let list = state
        .db
        .with(|c| core::achievements_load(c, &app_id))
        .map_err(|e| e.to_string())?;

    Ok((app_id, list))
}

#[tauri::command]
pub fn get_game_achievements(
    app: AppHandle,
    game_id: String,
    app_id: Option<String>,
) -> Result<Vec<Achievement>, String> {
    let state = app.state::<AppState>();
    let (_, list) = sync_achievements_from_disk(&state, &game_id, app_id.as_deref())?;
    Ok(list)
}

#[tauri::command]
pub fn get_achievements_summary(
    app: AppHandle,
    game_id: String,
    app_id: Option<String>,
) -> Result<Option<AchievementSummary>, String> {
    let state = app.state::<AppState>();
    let (resolved_id, list) = match sync_achievements_from_disk(&state, &game_id, app_id.as_deref()) {
        Ok(res) => res,
        Err(_) => return Ok(None),
    };

    if list.is_empty() {
        return Ok(None);
    }

    let variant = state
        .db
        .with(|c| core::card_pref_get(c, &game_id))
        .unwrap_or(1);

    let summary = calculate_summary(&resolved_id, &list, variant, chrono::Local::now());
    Ok(Some(summary))
}

#[tauri::command]
pub fn set_achievements_card_variant(
    app: AppHandle,
    game_id: String,
    variant: u8,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    state
        .db
        .with(|c| core::card_pref_set(c, &game_id, variant))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_achievements_card_variant(
    app: AppHandle,
    game_id: String,
) -> Result<u8, String> {
    let state = app.state::<AppState>();
    let v = state
        .db
        .with(|c| core::card_pref_get(c, &game_id))
        .unwrap_or(1);
    Ok(normalize_variant(v))
}

#[tauri::command]
pub fn detect_game_steamworks(
    game_dir: String,
    exe_path: Option<String>,
) -> Result<GameDetection, String> {
    let gdir = Path::new(&game_dir);
    let epath = exe_path.as_deref().map(Path::new);
    Ok(classify_game(gdir, epath))
}

/// Hilo de vigilancia de logros mientras un juego está en ejecucion.
pub fn on_launch(app: AppHandle, game: Game) {
    let st = app.state::<AppState>();
    let (app_id, game_dir) = resolve_game_info(&st, &game.id, None);
    let app_id = match app_id {
        Some(id) => id,
        None => return,
    };

    let dir_owned = game_dir.unwrap_or_else(|| PathBuf::from("."));
    let mut known_unlocks: HashMap<String, i64> = HashMap::new();

    // Cargar logros ya desbloqueados previamente
    if let Ok(existing) = st.db.with(|c| core::achievements_load(c, &app_id)) {
        for a in existing {
            if a.earned {
                known_unlocks.insert(a.api_name, a.unlocked_at.unwrap_or(0));
            }
        }
    }

    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();
    let (app_clone, game_id_clone) = (app.clone(), game.id.clone());

    // Loop de comprobacion cada 2 segundos mientras el juego este activo
    std::thread::spawn(move || {
        while r_clone.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(2000));
            let is_alive = app_clone
                .state::<AppState>()
                .active
                .lock()
                .unwrap()
                .as_ref()
                .map(|a| a.id == game_id_clone)
                .unwrap_or(false);

            if !is_alive {
                break;
            }

            // Comprobar runtime files
            let runtimes = find_runtime_files(&app_id, &dir_owned);
            for rf in runtimes {
                if let Ok(content) = std::fs::read_to_string(&rf) {
                    if let Ok(map) = parse_runtime_json(&content) {
                        for (api_name, (earned, time_opt)) in map {
                            if earned && !known_unlocks.contains_key(&api_name) {
                                let ts = time_opt.unwrap_or_else(|| {
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_secs() as i64)
                                        .unwrap_or(0)
                                });
                                known_unlocks.insert(api_name.clone(), ts);

                                // 1. Registrar desbloqueo en la base de datos
                                let ev_str = rf.to_string_lossy();
                                let _ = app_clone.state::<AppState>().db.with(|c| {
                                    core::unlock_record(c, &app_id, &api_name, ts, "gse", Some(&ev_str))
                                });

                                // 2. Captura automatica segun el contrato de integracion
                                snap::trigger(&app_clone);

                                // 3. Notificar a la interfaz mediante evento
                                let ach = Achievement {
                                    app_id: app_id.clone(),
                                    api_name: api_name.clone(),
                                    display_name: api_name.clone(),
                                    description: String::new(),
                                    hidden: false,
                                    icon: None,
                                    icon_gray: None,
                                    earned: true,
                                    unlocked_at: Some(ts),
                                    source: "gse".into(),
                                    stat_current: None,
                                    stat_max: None,
                                    rarity_tier: 0,
                                    rarity_pct: None,
                                };

                                let _ = app_clone.emit(
                                    "achievement-unlocked",
                                    UnlockEvent {
                                        app_id: app_id.clone(),
                                        achievement: ach,
                                        has_capture: true,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    });
}
