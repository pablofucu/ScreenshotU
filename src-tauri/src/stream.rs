//! Sesion de streaming a la tablet: estado de la partida (para `GET /session`), resumen al acabar
//! (`session-summary`) y cierre del juego desde la tablet (`POST /close`). No toca `launcher::watch`: se
//! construye escuchando `game-started`, `capture-saved` y `game-exited`.
use crate::state::AppState;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Listener, Manager};
use visor_core::games::Game;

/// Segundos en los que una segunda peticion de cierre fuerza el cierre.
const CLOSE_AGAIN: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    /// La tablet pidio jugar y aun no ha conectado: el juego no se ha abierto.
    Pending,
    /// Ya conecto: se esta abriendo el juego en la pantalla virtual.
    Starting,
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
    /// Juego que espera a la tablet (fase `Pending`).
    pending: Option<Game>,
    /// Cambia con cada pendiente nuevo o cancelado: los hilos de espera comprueban que siguen siendo suyos.
    epoch: u64,
    /// App de Apollo con la que se lanzó el stream (la tablet la necesita para volver a la partida).
    stream_app: Option<String>,
}

static SESSION: Mutex<Session> = Mutex::new(Session { phase: Phase::Idle, streamed: false, game: None, started_ms: 0, shots: 0, last: None, close_asked: None, pending: None, epoch: 0, stream_app: None });

fn session() -> std::sync::MutexGuard<'static, Session> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner())
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Escucha los eventos de la partida. Se llama una vez, antes de `remote::forward_events`.
pub fn init(app: &AppHandle) {
    let h = app.clone();
    app.listen_any("game-started", move |ev| {
        let p: Value = serde_json::from_str(ev.payload()).unwrap_or(Value::Null);
        let mut s = session();
        s.game = Some((p["id"].as_str().unwrap_or_default().to_string(), p["name"].as_str().unwrap_or_default().to_string()));
        s.phase = Phase::Playing;
        s.started_ms = now_ms();
        s.shots = 0;
        s.close_asked = None;
        let hide = s.streamed;
        drop(s);
        if hide {
            hide_cursor_if_enabled(&h);
        }
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
            s.stream_app = None;
            s.shots = 0;
            s.close_asked = None;
            sum
        };
        crate::stream_touch::stop();
        restore_cursor(&cursor_dir(&a));
        let _ = a.emit("session-summary", summary);
    });
}

fn cursor_dir(app: &AppHandle) -> PathBuf {
    app.state::<AppState>().config_dir.clone()
}

static CURSOR_HIDDEN: AtomicBool = AtomicBool::new(false);
static WATCHING: AtomicBool = AtomicBool::new(false);

fn marker(dir: &Path) -> PathBuf {
    dir.join("cursor-oculto")
}

fn hide_cursor_if_enabled(app: &AppHandle) {
    if !app.state::<AppState>().config.lock().unwrap().stream_hide_cursor {
        return;
    }
    hide_cursor(&cursor_dir(app));
    if !WATCHING.swap(true, Ordering::SeqCst) {
        let a = app.clone();
        std::thread::spawn(move || {
            watch_cursor(&a);
            WATCHING.store(false, Ordering::SeqCst);
        });
    }
}

/// Vigila la pantalla virtual mientras el cursor esta oculto: si desaparece mas de 5 s (Artemis se desconecto a
/// mitad) lo restaura; si vuelve, lo oculta otra vez. Sin pantalla virtual (Sunshine) no hace nada.
fn watch_cursor(app: &AppHandle) {
    let dir = cursor_dir(app);
    let (mut seen, mut missing) = (false, 0u32);
    loop {
        std::thread::sleep(Duration::from_secs(1));
        if session().phase != Phase::Playing {
            return;
        }
        if crate::display::virtual_monitor().is_some() {
            seen = true;
            missing = 0;
            if !CURSOR_HIDDEN.load(Ordering::SeqCst) {
                hide_cursor(&dir);
            }
        } else if seen {
            missing += 1;
            if missing >= 5 && CURSOR_HIDDEN.load(Ordering::SeqCst) {
                restore_cursor(&dir);
            }
        }
    }
}

/// Cursores del sistema transparentes. Deja un marcador para restaurarlos si el visor muere antes de acabar.
#[cfg(windows)]
pub fn hide_cursor(dir: &Path) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    let _ = std::fs::write(marker(dir), b"1");
    let ids = [OCR_NORMAL, OCR_IBEAM, OCR_WAIT, OCR_CROSS, OCR_UP, OCR_SIZENWSE, OCR_SIZENESW, OCR_SIZEWE, OCR_SIZENS, OCR_SIZEALL, OCR_NO, OCR_HAND, OCR_APPSTARTING];
    let and = [0xFFu8; 128];
    let xor = [0u8; 128];
    for id in ids {
        unsafe {
            if let Ok(c) = CreateCursor(None, 0, 0, 32, 32, and.as_ptr().cast(), xor.as_ptr().cast()) {
                let _ = SetSystemCursor(c, id);
            }
        }
    }
    CURSOR_HIDDEN.store(true, Ordering::SeqCst);
}

#[cfg(windows)]
pub fn restore_cursor(dir: &Path) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let _ = SystemParametersInfoW(SPI_SETCURSORS, 0, None, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    }
    let _ = std::fs::remove_file(marker(dir));
    CURSOR_HIDDEN.store(false, Ordering::SeqCst);
}

#[cfg(not(windows))]
pub fn hide_cursor(_: &Path) {}
#[cfg(not(windows))]
pub fn restore_cursor(dir: &Path) {
    let _ = std::fs::remove_file(marker(dir));
}

/// Al arrancar: si el visor murio con el cursor oculto, lo devuelve.
pub fn recover_cursor(dir: &Path) {
    if marker(dir).exists() {
        restore_cursor(dir);
    }
}

/// Caducidad del juego pendiente cuando lanza el ayudante de Apollo.
const PENDING_TTL: Duration = Duration::from_secs(120);
/// Espera de la pantalla virtual en el respaldo sin ayudante.
const FALLBACK_WAIT: Duration = Duration::from_secs(30);

fn notice(app: &AppHandle, id: &str, message: &str, error: bool) {
    let _ = app.emit("magpie-status", crate::magpie::MagpieEvent { id: id.to_string(), message: message.to_string(), error });
}

/// Si hay una app «Juego actual» al dia en Apollo: con ella lanza el ayudante al conectar la tablet.
fn app_installed(app: &AppHandle) -> bool {
    let port = app.state::<AppState>().config.lock().unwrap().remote_port;
    crate::apollo::app_installed(port)
}

/// «Jugar» desde la tablet. Con el ayudante el juego queda pendiente hasta que la tablet conecte; sin el
/// (app «Desktop» o Sunshine) se sondea la pantalla virtual y, si no aparece, se abre en el monitor del PC.
/// Devuelve `{app}`: el nombre de la app de Apollo que debe abrir la tablet, o `null` para usar su ajuste.
pub fn launch(app: &AppHandle, game: Game) -> Result<Value, String> {
    if let Some(a) = app.state::<AppState>().active.lock().unwrap().as_ref() {
        return Err(format!("Ya hay un juego en marcha: {}", a.name));
    }
    let epoch = {
        let mut s = session();
        if s.phase != Phase::Idle {
            return Err("Ya se está abriendo un juego".to_string());
        }
        s.epoch += 1;
        s.phase = Phase::Pending;
        s.streamed = true;
        s.game = Some((game.id.clone(), game.name.clone()));
        s.pending = Some(game.clone());
        s.epoch
    };
    let app = app.clone();
    if app_installed(&app) {
        std::thread::spawn(move || {
            let t0 = Instant::now();
            while t0.elapsed() < PENDING_TTL {
                std::thread::sleep(Duration::from_secs(1));
                if session().epoch != epoch || session().phase != Phase::Pending {
                    return;
                }
            }
            if let Some(g) = take_pending(epoch) {
                reset_idle();
                notice(&app, &g.id, "La tablet no llegó a conectar; no se abre el juego", false);
            }
        });
        session().stream_app = Some("Juego actual".to_string());
        return Ok(json!({ "app": "Juego actual" }));
    }
    std::thread::spawn(move || {
        let t0 = Instant::now();
        while crate::display::virtual_monitor().is_none() && t0.elapsed() < FALLBACK_WAIT {
            std::thread::sleep(Duration::from_millis(500));
            if session().epoch != epoch {
                return;
            }
        }
        if let Some(g) = take_pending(epoch) {
            launch_now(&app, g, FALLBACK_WAIT);
        }
    });
    Ok(json!({ "app": null }))
}

/// Quita el pendiente si sigue siendo el de `epoch` y pasa la fase a `Starting`.
fn take_pending(epoch: u64) -> Option<Game> {
    let mut s = session();
    if s.epoch != epoch || s.phase != Phase::Pending {
        return None;
    }
    s.phase = Phase::Starting;
    s.pending.take()
}

fn reset_idle() {
    let mut s = session();
    s.phase = Phase::Idle;
    s.streamed = false;
    s.game = None;
    s.stream_app = None;
    s.pending = None;
    s.epoch += 1;
    drop(s);
    crate::stream_touch::stop();
}

/// Espera la pantalla virtual (hasta `wait`), la deja asentar y abre el juego en ella. Bloquea: va en un hilo.
fn launch_now(app: &AppHandle, game: Game, wait: Duration) {
    let t0 = Instant::now();
    let mut screen = crate::display::virtual_monitor();
    while screen.is_none() && t0.elapsed() < wait {
        std::thread::sleep(Duration::from_millis(500));
        screen = crate::display::virtual_monitor();
    }
    if screen.is_some() {
        // Apollo aun ajusta la resolucion nada mas crearla.
        std::thread::sleep(Duration::from_millis(1500));
        screen = crate::display::virtual_monitor();
    } else {
        notice(app, &game.id, "No apareció la pantalla virtual: el juego se abre en el monitor del PC", false);
    }
    let (app2, id) = (app.clone(), game.id.clone());
    // El lanzador toca la ventana y la pantalla principal: va en el hilo principal.
    let (tx, rx) = std::sync::mpsc::channel();
    let sent = app.run_on_main_thread(move || {
        let _ = tx.send(crate::launcher::start_at(&app2, game, screen));
    });
    let r = sent.map_err(|e| e.to_string()).and_then(|_| rx.recv_timeout(Duration::from_secs(30)).map_err(|_| "El visor no respondió a tiempo".to_string()));
    if let Err(e) = r.and_then(|r| r) {
        reset_idle();
        notice(app, &id, &e, true);
    }
}

/// La tablet conecto (llama el ayudante de Apollo): abre el juego pendiente o reengancha el que ya esta en marcha.
pub fn session_start(app: &AppHandle, _w: u32, _h: u32, _fps: u32) -> Result<Value, String> {
    let (phase, epoch) = {
        let s = session();
        (s.phase, s.epoch)
    };
    match phase {
        Phase::Pending => {
            if let Some(g) = take_pending(epoch) {
                let a = app.clone();
                std::thread::spawn(move || launch_now(&a, g, Duration::from_secs(10)));
            }
        }
        Phase::Starting | Phase::Playing => {}
        Phase::Idle => {
            if app.state::<AppState>().active.lock().unwrap().is_none() {
                return Err("No hay ningún juego pendiente ni en marcha".to_string());
            }
            // Artemis se reconecto con el juego aun abierto en el PC.
            let mut s = session();
            s.phase = Phase::Playing;
            s.streamed = true;
            drop(s);
            hide_cursor_if_enabled(app);
        }
    }
    crate::stream_touch::start();
    Ok(json!({ "ok": true, "state": session_json()["state"] }))
}

/// Modo auxiliar `visor.exe --stream-session --port N`: es la app «Juego actual» de Apollo. Avisa al visor de que la
/// tablet conecto y no termina hasta que acaba la partida, para que Apollo cierre la sesion de streaming.
/// Sin Tauri. Salida: 0 = fin normal, 1 = no habia juego / caduco, 2 = el visor no responde.
pub fn helper_main() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--stream-session") {
        return None;
    }
    let port: u16 = args.iter().position(|a| a == "--port").and_then(|i| args.get(i + 1)).and_then(|p| p.parse().ok()).unwrap_or(visor_core::remote::DEFAULT_PORT);
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.trim().parse::<u32>().ok()).unwrap_or(0);
    let body = json!({ "width": env("SUNSHINE_CLIENT_WIDTH"), "height": env("SUNSHINE_CLIENT_HEIGHT"), "fps": env("SUNSHINE_CLIENT_FPS") });
    let log = |m: &str| {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(std::env::temp_dir().join("visor-stream.log")) {
            let _ = writeln!(f, "{} {m}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
        }
    };
    let base = format!("http://127.0.0.1:{port}/api/v1/session");
    log(&format!("ayudante en marcha, puerto {port}, {body}"));
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(5)).build();
    // Arranque: el visor puede estar aun abriendose.
    let t0 = Instant::now();
    loop {
        match agent.post(&format!("{base}/start")).send_json(body.clone()) {
            Ok(_) => break,
            Err(ureq::Error::Status(c, _)) => {
                log(&format!("start rechazado: {c}"));
                return Some(1);
            }
            Err(e) => {
                if t0.elapsed() > Duration::from_secs(10) {
                    log(&format!("el visor no responde: {e}"));
                    return Some(2);
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
    let started = Instant::now();
    let mut seen_active = false;
    let mut down_since: Option<Instant> = None;
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let state = match agent.get(&base).call().ok().and_then(|r| r.into_json::<Value>().ok()) {
            Some(v) => {
                down_since = None;
                v["state"].as_str().unwrap_or("idle").to_string()
            }
            None => {
                if down_since.get_or_insert_with(Instant::now).elapsed() > Duration::from_secs(10) {
                    log("el visor dejo de responder");
                    return Some(2);
                }
                continue;
            }
        };
        match state.as_str() {
            "playing" => seen_active = true,
            "starting" => {
                seen_active = true;
                if started.elapsed() > PENDING_TTL {
                    log("el juego no llego a abrirse");
                    return Some(1);
                }
            }
            "idle" => {
                if seen_active {
                    log("partida terminada");
                    return Some(0);
                }
                if started.elapsed() > Duration::from_secs(10) {
                    log("sin partida");
                    return Some(1);
                }
            }
            _ => {
                if started.elapsed() > PENDING_TTL {
                    log("el juego sigue pendiente");
                    return Some(1);
                }
            }
        }
    }
}

/// La tablet no llego a abrir Artemis: se descarta el juego pendiente.
pub fn cancel() -> Value {
    let was = session().phase == Phase::Pending;
    if was {
        reset_idle();
    }
    json!({ "cancelled": was })
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
        Phase::Pending => "pending",
        Phase::Starting => "starting",
        Phase::Playing => "playing",
    };
    let elapsed = if s.started_ms > 0 && matches!(s.phase, Phase::Starting | Phase::Playing) { ((now_ms() - s.started_ms) / 1000).max(0) } else { 0 };
    json!({ "state": state, "game": s.game.as_ref().map(|(id, name)| json!({ "id": id, "name": name })), "streamed": s.streamed, "last": s.last, "app": s.stream_app, "elapsedSeconds": elapsed })
}
