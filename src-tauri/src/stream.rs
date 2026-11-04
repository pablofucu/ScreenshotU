//! Sesion de streaming a la tablet: estado de la partida (para `GET /session`), resumen al acabar
//! (`session-summary`) y cierre del juego desde la tablet (`POST /close`). No toca `launcher::watch`: se
//! construye escuchando `game-started`, `capture-saved` y `game-exited`.
use crate::state::AppState;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Listener, Manager};

/// Segundos en los que una segunda peticion de cierre fuerza el cierre.
const CLOSE_AGAIN: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Playing,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    id: String,
    name: String,
    platform: String,
    started_ms: i64,
    ended_ms: i64,
    seconds: u64,
    shots: u32,
    streamed: bool,
}

struct Session {
    phase: Phase,
    streamed: bool,
    game: Option<(String, String)>,
    started_ms: i64,
    shots: u32,
    last: Option<Summary>,
    close_asked: Option<Instant>,
}

static SESSION: Mutex<Session> = Mutex::new(Session { phase: Phase::Idle, streamed: false, game: None, started_ms: 0, shots: 0, last: None, close_asked: None });

fn session() -> std::sync::MutexGuard<'static, Session> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner())
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Escucha los eventos de la partida. Se llama una vez, antes de `remote::forward_events`.
pub fn init(app: &AppHandle) {
    app.listen_any("game-started", |ev| {
        let p: Value = serde_json::from_str(ev.payload()).unwrap_or(Value::Null);
        let mut s = session();
        s.game = Some((p["id"].as_str().unwrap_or_default().to_string(), p["name"].as_str().unwrap_or_default().to_string()));
        s.phase = Phase::Playing;
        s.started_ms = now_ms();
        s.shots = 0;
        s.close_asked = None;
    });
    app.listen_any("capture-saved", |_| {
        let mut s = session();
        if s.phase == Phase::Playing {
            s.shots += 1;
        }
    });
    let a = app.clone();
    app.listen_any("game-exited", move |ev| {
        let p: Value = serde_json::from_str(ev.payload()).unwrap_or(Value::Null);
        let id = p["id"].as_str().unwrap_or_default().to_string();
        let name = p["name"].as_str().unwrap_or_default().to_string();
        let seconds = p["seconds"].as_u64().unwrap_or(0);
        let platform = a.state::<AppState>().games.lock().unwrap().iter().find(|g| g.id == id).map(|g| g.platform.clone()).unwrap_or_default();
        let summary = {
            let mut s = session();
            let ended_ms = now_ms();
            let sum = Summary { id, name, platform, started_ms: if s.started_ms > 0 { s.started_ms } else { ended_ms - seconds as i64 * 1000 }, ended_ms, seconds, shots: s.shots, streamed: s.streamed };
            s.last = Some(sum.clone());
            s.phase = Phase::Idle;
            s.game = None;
            s.streamed = false;
            s.shots = 0;
            s.close_asked = None;
            sum
        };
        let _ = a.emit("session-summary", summary);
    });
}

/// Cierra el juego en marcha. `none` = no hay juego; `asked` = se le pidio cerrar; `forced` = se mato.
pub fn close(app: &AppHandle, force: bool) -> Result<Value, String> {
    let pids = {
        let st = app.state::<AppState>();
        let a = st.active.lock().unwrap();
        match a.as_ref() {
            None => return Ok(json!({ "closed": "none" })),
            Some(a) => a.pids.lock().unwrap().clone(),
        }
    };
    let again = session().close_asked.is_some_and(|t| t.elapsed() < CLOSE_AGAIN);
    if force || again {
        return Ok(forced(app));
    }
    let Some(hwnd) = crate::display::game_window(&pids) else {
        return Ok(forced(app));
    };
    if !ask_close(hwnd) {
        return Ok(forced(app));
    }
    session().close_asked = Some(Instant::now());
    Ok(json!({ "closed": "asked" }))
}

fn forced(app: &AppHandle) -> Value {
    session().close_asked = None;
    let a = app.clone();
    std::thread::spawn(move || crate::quickmenu::force_close_game(&a));
    json!({ "closed": "forced" })
}

/// `WM_CLOSE` con limite de 3 s. `false` si la ventana esta colgada o ya no existe.
#[cfg(windows)]
fn ask_close(hwnd: isize) -> bool {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_CLOSE};
    let mut res = 0usize;
    let r = unsafe { SendMessageTimeoutW(HWND(hwnd as *mut _), WM_CLOSE, WPARAM(0), LPARAM(0), SMTO_ABORTIFHUNG, 3000, Some(&mut res)) };
    r.0 != 0
}

#[cfg(not(windows))]
fn ask_close(_: isize) -> bool {
    false
}

/// Estado ligero de la sesion (`GET /api/v1/session`).
pub fn session_json() -> Value {
    let s = session();
    let state = match s.phase {
        Phase::Idle => "idle",
        Phase::Playing => "playing",
    };
    json!({ "state": state, "game": s.game.as_ref().map(|(id, name)| json!({ "id": id, "name": name })), "streamed": s.streamed, "last": s.last })
}
