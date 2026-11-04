//! `POST /api/v1/call`: ejecuta desde la tablet los mismos comandos que la interfaz llama con `invoke`.
//!
//! `dispatch` recibe el nombre del comando y los argumentos tal como los manda el JS (claves en camelCase, que es
//! como Tauri convierte los parametros snake_case de Rust) y llama a la funcion del comando. Lo que no esta aqui
//! da `CALL_NOT_ALLOWED` (403): ventanas y atajos del PC (`window_monitor`, `hotkey_suspend`, `exit_app`...),
//! copias de seguridad y biblioteca, abrir carpetas o elegir rutas del PC, texthook, Anki, el lector de novelas y
//! los datos del propio servidor remoto.
use crate::state::AppState;
use crate::{achievements, glossary, novel, saves, sgdb};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager};
use visor_core::remote::{CALL_BAD_ARGS, CALL_NOT_ALLOWED};

type Res = Result<Value, String>;

/// Argumento `k` (si falta se trata como `null`, asi que los `Option` quedan en `None`).
fn arg<T: DeserializeOwned>(args: &Value, k: &str) -> Result<T, String> {
    let v = args.get(k).cloned().unwrap_or(Value::Null);
    serde_json::from_value(v).map_err(|e| format!("{CALL_BAD_ARGS} ({k}): {e}"))
}

/// Resultado de un comando como JSON (`()` -> `null`).
fn out<T: Serialize>(r: Result<T, String>) -> Res {
    r.and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string()))
}

/// Valor que no puede fallar.
fn val<T: Serialize>(v: T) -> Res {
    out(Ok::<T, String>(v))
}

/// Para los comandos `async` de Tauri: se esperan en el hilo de la peticion.
fn block<F: std::future::Future>(f: F) -> F::Output {
    tauri::async_runtime::block_on(f)
}

/// Ejecuta `f` en el hilo principal (lo que toca la ventana o la pantalla principal) y espera su resultado.
fn on_main<T: Send + 'static>(app: &AppHandle, f: impl FnOnce(AppHandle) -> T + Send + 'static) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let app2 = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(f(app2));
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(std::time::Duration::from_secs(30)).map_err(|_| "El visor no respondió a tiempo".to_string())
}

pub fn dispatch(app: &AppHandle, cmd: &str, args: Value) -> Res {
    let st = || app.state::<AppState>();
    let a = &args;
    match cmd {
        /* --- biblioteca y ajustes --- */
        "get_config" => val(crate::get_config(st())),
        "save_config" => {
            // La tablet no decide la biblioteca ni el acceso remoto (podría desconectarse sola o pisar el PC con
            // una copia vieja): esos campos se quedan como estén en el PC.
            let mut c: visor_core::config::Config = arg(a, "config")?;
            {
                let state = st();
                let cur = state.config.lock().unwrap();
                c.root = cur.root.clone();
                c.remote_enabled = cur.remote_enabled;
                c.remote_port = cur.remote_port;
                c.remote_token = cur.remote_token.clone();
            }
            out(crate::save_config(app.clone(), st(), c))
        }
        "scan_library" => out(block(crate::scan_library(app.clone(), st()))),
        "cached_library" => out(block(crate::cached_library(st()))),
        "get_thumb" => out(block(crate::get_thumb(st(), arg(a, "path")?))),
        "prefetch_thumbs" => val(crate::prefetch_thumbs(app.clone(), st(), arg(a, "paths")?)),
        "get_favorites" => val(crate::get_favorites(st())),
        "set_favorite" => out(crate::set_favorite(st(), arg(a, "path")?, arg(a, "on")?)),
        "launch_stream" => {
            let id: String = arg(a, "id")?;
            let state = st();
            let mut game = state.games.lock().unwrap().iter().find(|g| g.id == id).cloned();
            if game.is_none() {
                // Aun no hay escaneo (o el juego es nuevo).
                let (_, found, _) = crate::scan_full(&state);
                game = found.iter().find(|g| g.id == id).cloned();
                *state.games.lock().unwrap() = found;
            }
            out(crate::launcher::start_stream(app, game.ok_or("Juego no encontrado")?))
        }
        "close_game" => out(crate::stream::close(app, arg(a, "force").unwrap_or(false))),
        "launch_game" => {
            let id: String = arg(a, "id")?;
            // El lanzador toca la ventana y la pantalla principal: va en el hilo principal.
            out(on_main(app, move |app| {
                let st = app.state::<AppState>();
                crate::launch_game(app.clone(), st, id)
            })?)
        }
        "goal_notice" => val(crate::goal_notice(app.clone(), arg(a, "id")?, arg(a, "at")?, arg(a, "text")?, arg(a, "days")?, arg(a, "key")?)),
        "set_hidden" => out(crate::set_hidden(st(), arg(a, "id")?, arg(a, "name")?, arg(a, "platform")?, arg(a, "hidden")?)),
        "set_accent_override" => out(crate::set_accent_override(st(), arg(a, "id")?, arg(a, "color")?)),
        "set_sort_pref" => out(crate::set_sort_pref(st(), arg(a, "id")?, arg(a, "mode")?)),
        "set_magpie_mode" => out(crate::set_magpie_mode(st(), arg(a, "id")?, arg(a, "mode")?)),
        "set_title_status" => out(crate::set_title_status(st(), arg(a, "id")?, arg(a, "status")?, arg(a, "finishedMs")?, arg(a, "rating")?)),
        "set_release_date" => out(crate::set_release_date(st(), arg(a, "id")?, arg(a, "year")?, arg(a, "month")?, arg(a, "day")?)),
        "game_stats" => out(crate::game_stats(st(), arg(a, "id")?)),
        "set_pending_order" => out(crate::set_pending_order(st(), arg(a, "ids")?)),
        "snooze_stalled" => out(crate::snooze_stalled(st(), arg(a, "ids")?)),
        "magpie_status" => out(block(crate::magpie_status(st()))),
        "get_collections" => val(crate::get_collections(st())),
        "save_collections" => out(crate::save_collections(st(), arg(a, "collections")?)),
        "convert_collections" => out(crate::convert_collections(st(), arg(a, "platforms")?)),
        "move_shots" => out(crate::move_shots(st(), arg(a, "paths")?, arg(a, "targetDir")?)),
        "trash_shots" => out(block(crate::trash_shots(st(), arg(a, "paths")?))),
        "remove_game" => out(crate::remove_game(st(), arg(a, "id")?)),
        "move_game_platform" => out(crate::move_game_platform(st(), arg(a, "id")?, arg(a, "platform")?)),
        "capture_now" => val(crate::capture_now(app.clone())),
        "audio_outputs" => out(block(crate::audio_outputs())),
        "set_audio_output" => out(block(crate::set_audio_output(app.clone(), arg(a, "id")?))),

        /* --- partidas guardadas --- */
        "saves_info" => out(block(saves::saves_info(st(), arg(a, "id")?))),
        "saves_set_enabled" => out(saves::saves_set_enabled(st(), arg(a, "id")?, arg(a, "path")?, arg(a, "kind")?, arg(a, "enabled")?)),
        "saves_remove" => out(saves::saves_remove(st(), arg(a, "id")?, arg(a, "path")?)),
        "saves_backup" => out(block(saves::saves_backup(st(), arg(a, "id")?))),
        "saves_restore" => out(block(saves::saves_restore(st(), arg(a, "id")?, arg(a, "file")?))),
        "saves_restore_file" => out(block(saves::saves_restore_file(st(), arg(a, "id")?, arg(a, "file")?, arg(a, "entry")?))),
        "saves_copy_files" => out(block(saves::saves_copy_files(st(), arg(a, "id")?, arg(a, "file")?))),
        "saves_set_note" => out(saves::saves_set_note(st(), arg(a, "id")?, arg(a, "file")?, arg(a, "note")?)),
        "saves_set_pinned" => out(saves::saves_set_pinned(st(), arg(a, "id")?, arg(a, "file")?, arg(a, "pinned")?)),
        "saves_delete_copy" => out(saves::saves_delete_copy(st(), arg(a, "id")?, arg(a, "file")?)),
        "saves_status" => val(saves::saves_status(st())),
        "saves_update_manifest" => out(block(saves::saves_update_manifest(st()))),

        /* --- SteamGridDB --- */
        "sgdb_auto" => out(sgdb::sgdb_auto(app.clone(), st(), arg(a, "titles")?, arg(a, "force")?)),
        "sgdb_search" => out(block(sgdb::sgdb_search(st(), arg(a, "term")?))),
        "sgdb_images" => out(block(sgdb::sgdb_images(st(), arg(a, "gameId")?, arg(a, "kind")?, arg(a, "page")?))),
        "sgdb_info" => val(sgdb::sgdb_info(st(), arg(a, "titleId")?)),
        "sgdb_link" => val(sgdb::sgdb_link(st(), arg(a, "titleId")?, arg(a, "gameId")?, arg(a, "gameName")?, arg(a, "year")?)),
        "sgdb_apply" => out(block(sgdb::sgdb_apply(st(), arg(a, "titleId")?, arg(a, "kind")?, arg(a, "image")?))),
        "sgdb_local" => out(block(sgdb::sgdb_local(st(), arg(a, "titleId")?, arg(a, "kind")?, arg(a, "path")?))),
        "sgdb_clear" => out(sgdb::sgdb_clear(st(), arg(a, "titleId")?, arg(a, "kind")?)),

        /* --- frases, lectura y logros --- */
        "phrases_list" => val(glossary::phrases_list(app.clone(), arg(a, "gameId")?, arg(a, "filter")?)),
        "phrases_get" => val(glossary::phrases_get(app.clone(), arg(a, "gameId")?, arg(a, "id")?)),
        "phrases_update" => out(glossary::phrases_update(app.clone(), arg(a, "gameId")?, arg(a, "id")?, arg(a, "patch")?)),
        "phrases_grade" => out(glossary::phrases_grade(app.clone(), arg(a, "gameId")?, arg(a, "id")?, arg(a, "gradeN")?)),
        "phrases_delete" => val(glossary::phrases_delete(app.clone(), arg(a, "gameId")?, arg(a, "id")?)),
        "phrases_due_count" => val(glossary::phrases_due_count(app.clone(), arg(a, "gameId")?)),
        "phrases_media_dir" => val(glossary::phrases_media_dir(app.clone())),
        "novel_get_prefs" => val(novel::novel_get_prefs(app.clone(), arg(a, "gameId")?)),
        "novel_save_prefs" => out(novel::novel_save_prefs(app.clone(), arg(a, "gameId")?, arg(a, "prefs")?)),
        "readstats_get" => val(novel::readstats::readstats_get(app.clone(), arg(a, "gameId")?)),
        "readstats_live" => val(novel::readstats::readstats_live()),
        "get_game_achievements" => out(block(achievements::get_game_achievements(app.clone(), arg(a, "gameId")?, arg(a, "appId")?))),
        "get_achievements_summary" => out(block(achievements::get_achievements_summary(app.clone(), arg(a, "gameId")?, arg(a, "appId")?))),
        "set_achievements_card_variant" => out(achievements::set_achievements_card_variant(app.clone(), arg(a, "gameId")?, arg(a, "variant")?)),
        "get_achievements_card_variant" => out(achievements::get_achievements_card_variant(app.clone(), arg(a, "gameId")?)),

        _ => Err(CALL_NOT_ALLOWED.into()),
    }
}
