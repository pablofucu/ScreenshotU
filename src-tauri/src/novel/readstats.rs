//! Estadisticas de lectura de la partida de novela en curso: registra lineas, caracteres y tiempo activo en la base
//! (`visor_core::jp::readstats`), fechas de palabras conocidas y frases anadidas al glosario, y las reparte a la
//! pagina de lectura y a la app principal.
use crate::state::{log_db, log_db_err, AppState};
use serde_json::json;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use visor_core::db::Db;
use visor_core::jp::readstats::{self as rs, Stats, Tracker};

struct Run {
    game_id: String,
    session_id: Option<i64>,
    tracker: Tracker,
}

static RUN: Mutex<Option<Run>> = Mutex::new(None);

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn db(app: &AppHandle) -> Arc<Db> {
    app.state::<AppState>().db.clone()
}

fn publish(live: Option<rs::Live>) {
    let v = serde_json::to_value(live).unwrap_or(serde_json::Value::Null);
    crate::reader::server::set_extra("readLive", v.clone());
    crate::reader::server::send(json!({ "type": "readLive", "live": v }));
}

/// Empieza a registrar la sesion de un juego de novela (se llama al lanzarlo).
pub fn start(app: &AppHandle, game_id: &str, game_name: &str) {
    finish(app);
    let t0 = now_ms();
    let session_id = log_db("estadisticas de lectura", db(app).with(|c| rs::session_start(c, game_id, game_name, t0)).map(Some));
    let tracker = Tracker::new(t0);
    let live = tracker.live(t0);
    *RUN.lock().unwrap() = Some(Run { game_id: game_id.to_string(), session_id, tracker });
    publish(Some(live));
}

/// Cierra la sesion en curso (al cerrar el juego). Si no se leyo nada, se borra.
pub fn finish(app: &AppHandle) {
    let Some(run) = RUN.lock().unwrap().take() else { return };
    if let Some(id) = run.session_id {
        let end = if run.tracker.lines > 0 { run.tracker.live(now_ms()).last_ms.max(run.tracker.start_ms) } else { now_ms() };
        log_db_err("estadisticas de lectura", db(app).with(|c| rs::session_finish(c, id, end, &run.tracker)));
    }
    publish(None);
}

/// Una linea final de la partida (ya deduplicada, sin nombre): suma caracteres y tiempo activo.
pub fn on_line(app: &AppHandle, text: &str) {
    let at = now_ms();
    let (delta, live, game_id, session_id, snapshot) = {
        let mut g = RUN.lock().unwrap();
        let Some(run) = g.as_mut() else { return };
        let d = run.tracker.push(text, at);
        (d, run.tracker.live(at), run.game_id.clone(), run.session_id, run.tracker.clone())
    };
    if !delta.counted {
        return;
    }
    let day = rs::day_of(at, rs::local_offset_min(at));
    let d = db(app);
    log_db_err("estadisticas de lectura", d.with(|c| rs::day_add(c, &game_id, day, 1, delta.chars, delta.active_ms, 0)));
    if let Some(id) = session_id {
        log_db_err("estadisticas de lectura", d.with(|c| rs::session_update(c, id, at, &snapshot)));
    }
    publish(Some(live));
}

/// Una palabra pasa a conocida (o deja de serlo).
pub fn on_known(app: &AppHandle, word: &str, on: bool) {
    log_db_err("estadisticas de lectura", db(app).with(|c| rs::known_dated(c, word, on, now_ms())));
}

/// Una frase nueva entra al glosario.
pub fn on_phrase_added(app: &AppHandle, game_id: &str) {
    let at = now_ms();
    let day = rs::day_of(at, rs::local_offset_min(at));
    log_db_err("estadisticas de lectura", db(app).with(|c| rs::day_add(c, game_id, day, 0, 0, 0, 1)));
}

pub fn stats(app: &AppHandle, game_id: Option<&str>) -> Stats {
    log_db("estadisticas de lectura", db(app).with(|c| rs::load_stats(c, game_id, now_ms())))
}

/// La pagina pide las estadisticas (`{"type":"statsGet"}`): se le responde con `{"type":"stats","stats":...}`.
pub fn push_stats(app: &AppHandle) {
    crate::reader::server::send(json!({ "type": "stats", "stats": stats(app, None) }));
}

/// Estadisticas globales y por juego (la app principal pasa `game_id` para el resumen de un juego).
#[tauri::command]
pub fn readstats_get(app: AppHandle, game_id: Option<String>) -> Stats {
    stats(&app, game_id.as_deref())
}

/// Sesion de lectura en curso, si la hay.
#[tauri::command]
pub fn readstats_live() -> Option<rs::Live> {
    RUN.lock().unwrap().as_ref().map(|r| r.tracker.live(now_ms()))
}
