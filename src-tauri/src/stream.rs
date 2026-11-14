//! Sesion de streaming a la tablet: estado de la partida (para `GET /session`), resumen al acabar
//! (`session-summary`) y cierre del juego desde la tablet (`POST /close`). No toca `launcher::watch`: se
//! construye escuchando `game-started`, `capture-saved` y `game-exited`.
use crate::state::AppState;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Listener, Manager};
use visor_core::games::Game;

/// Segundos en los que una segunda peticion de cierre fuerza el cierre.
const CLOSE_AGAIN: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Dispositivo que pidió el juego (id del cliente; `''` = la tablet antigua o el propio equipo).
    device: String,
    /// Tipo del dispositivo que esta en streaming (`"tablet"`, `"pc"` o `""` si no se conoce).
    device_kind: String,
}

impl Session {
    const fn new() -> Self {
        Self { phase: Phase::Idle, streamed: false, game: None, started_ms: 0, shots: 0, last: None, close_asked: None, pending: None, epoch: 0, stream_app: None, device: String::new(), device_kind: String::new() }
    }

    fn reset_idle(&mut self) {
        self.phase = Phase::Idle;
        self.streamed = false;
        self.game = None;
        self.stream_app = None;
        self.pending = None;
        self.device.clear();
        self.device_kind.clear();
        self.epoch += 1;
    }

    /// Solo se oculta el cursor del sistema cuando quien recibe el streaming es una tablet.
    fn should_hide_cursor(&self) -> bool {
        self.streamed && self.device_kind == "tablet"
    }

    fn on_started(&mut self, id: String, name: String, started_ms: i64) -> bool {
        self.game = Some((id, name));
        self.phase = Phase::Playing;
        self.started_ms = started_ms;
        self.shots = 0;
        self.close_asked = None;
        self.should_hide_cursor()
    }

    fn on_exited(&mut self, id: String, name: String, platform: String, seconds: u64, ended_ms: i64) -> (Summary, Option<String>, bool) {
        let sum = Summary {
            id,
            name,
            platform,
            started_ms: if self.started_ms > 0 { self.started_ms } else { ended_ms - seconds as i64 * 1000 },
            ended_ms,
            seconds,
            shots: self.shots,
            streamed: self.streamed,
        };
        let was_streamed = self.streamed || self.stream_app.is_some();
        let was_app = self.stream_app.take();
        self.last = Some(sum.clone());
        self.phase = Phase::Idle;
        self.game = None;
        self.streamed = false;
        self.device.clear();
        self.device_kind.clear();
        self.shots = 0;
        self.close_asked = None;
        self.epoch += 1;
        (sum, was_app, was_streamed)
    }

    fn cancel_pending(&mut self) -> bool {
        if self.phase == Phase::Pending {
            self.reset_idle();
            true
        } else {
            false
        }
    }

    /// Limpia sesión pendiente o en memoria cuando el juego ya cerró antes de recibir `close`.
    fn close_without_active(&mut self) -> (bool, Option<String>) {
        let app_name = self.stream_app.clone();
        let pending = self.cancel_pending();
        if !pending {
            self.reset_idle();
        }
        (pending, app_name)
    }
}

static SESSION: Mutex<Session> = Mutex::new(Session::new());

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
        let hide = session().on_started(
            p["id"].as_str().unwrap_or_default().to_string(),
            p["name"].as_str().unwrap_or_default().to_string(),
            now_ms(),
        );
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
        let (summary, was_app, was_streamed) = session().on_exited(id, name, platform, seconds, now_ms());
        crate::stream_touch::stop();
        restore_cursor(&cursor_dir(&a));
        if was_streamed {
            spawn_apollo_close(was_app);
        }
        let _ = a.emit("session-summary", summary);
    });
}

fn cursor_dir(app: &AppHandle) -> PathBuf {
    app.state::<AppState>().config_dir.clone()
}

static CURSOR_HIDDEN: AtomicBool = AtomicBool::new(false);
static WATCH_SEQ: AtomicU64 = AtomicU64::new(0);

fn marker(dir: &Path) -> PathBuf {
    dir.join("cursor-oculto")
}

/// Tipo del cliente conectado segun la lista de Apollo y los dispositivos emparejados.
/// Si hay un PC conectado a la vez que una tablet, gana `"pc"` para no ocultar el cursor al PC.
fn connected_kind_from_apollo(
    apollo_clients: &[crate::apollo::web::Client],
    paired: &[visor_core::devices::Device],
) -> Option<&'static str> {
    let mut saw_tablet = false;
    for c in apollo_clients.iter().filter(|c| c.connected == Some(true)) {
        let matched = paired
            .iter()
            .find(|d| !d.revoked && d.apollo_id.as_deref() == Some(c.uuid.as_str()))
            .or_else(|| paired.iter().find(|d| !d.revoked && d.apollo_id.is_none() && d.name == c.name));
        if let Some(d) = matched {
            if d.kind == "pc" {
                return Some("pc");
            }
            if d.kind == "tablet" {
                saw_tablet = true;
            }
        }
    }
    saw_tablet.then_some("tablet")
}

fn has_active_pc(paired: &[visor_core::devices::Device]) -> bool {
    paired.iter().any(|d| !d.revoked && d.kind == "pc")
}

fn resolve_connected_kind(app: &AppHandle) -> Option<&'static str> {
    if !crate::apollo::web::creds_present() {
        return None;
    }
    let clients = crate::apollo::web::clients().ok()?;
    let paired = app.state::<AppState>().db.with(visor_core::devices::list).unwrap_or_default();
    connected_kind_from_apollo(&clients, &paired)
}

fn resolve_fallback_tablet(app: &AppHandle) -> bool {
    let paired = app.state::<AppState>().db.with(visor_core::devices::list).unwrap_or_default();
    !has_active_pc(&paired)
}

fn apply_reconciled_kind(app: &AppHandle, expected_epoch: u64, kind: &str) {
    {
        let mut s = session();
        if s.epoch != expected_epoch || s.phase != Phase::Playing {
            return;
        }
        s.device_kind = kind.to_string();
    }
    if kind == "tablet" {
        hide_cursor_if_enabled(app);
    } else {
        restore_cursor(&cursor_dir(app));
    }
}

fn spawn_cursor_reconcile(app: AppHandle, expected_epoch: u64) {
    std::thread::spawn(move || {
        let delays = [Duration::ZERO, Duration::from_millis(400), Duration::from_millis(1000)];
        for d in delays {
            if !d.is_zero() {
                std::thread::sleep(d);
            }
            {
                let s = session();
                if s.epoch != expected_epoch || s.phase != Phase::Playing {
                    return;
                }
            }
            if let Some(kind) = resolve_connected_kind(&app) {
                apply_reconciled_kind(&app, expected_epoch, kind);
                return;
            }
            if !crate::apollo::web::creds_present() {
                break;
            }
        }
        let kind = if resolve_fallback_tablet(&app) { "tablet" } else { "pc" };
        apply_reconciled_kind(&app, expected_epoch, kind);
    });
}

fn hide_cursor_if_enabled(app: &AppHandle) {
    if !app.state::<AppState>().config.lock().unwrap().stream_hide_cursor || !session().should_hide_cursor() {
        return;
    }
    hide_cursor(&cursor_dir(app));
    let seq = WATCH_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let a = app.clone();
    std::thread::spawn(move || watch_cursor(&a, seq));
}

/// Vigila la pantalla virtual mientras el cursor esta oculto: si desaparece mas de 5 s (Artemis se desconecto a
/// mitad) lo restaura; si vuelve, lo oculta otra vez. Sin pantalla virtual (Sunshine) no hace nada.
fn watch_cursor(app: &AppHandle, seq: u64) {
    let dir = cursor_dir(app);
    let (mut seen, mut missing) = (false, 0u32);
    loop {
        std::thread::sleep(Duration::from_millis(250));
        if WATCH_SEQ.load(Ordering::SeqCst) != seq {
            return;
        }
        let allow = {
            let s = session();
            s.phase == Phase::Playing && s.should_hide_cursor()
        } && app.state::<AppState>().config.lock().unwrap().stream_hide_cursor;
        if !allow {
            if CURSOR_HIDDEN.load(Ordering::SeqCst) {
                restore_cursor(&dir);
            }
            return;
        }
        if crate::display::virtual_monitor().is_some() {
            seen = true;
            missing = 0;
            if !CURSOR_HIDDEN.load(Ordering::SeqCst) {
                if resolve_connected_kind(app) == Some("pc") {
                    session().device_kind = "pc".into();
                    return;
                }
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
/// `device`: id del dispositivo que juega (se guarda en la partida). `device_kind`: `"tablet"` o `"pc"`.
pub fn launch(app: &AppHandle, game: Game, device: String, device_kind: String) -> Result<Value, String> {
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
        s.device = device;
        s.device_kind = device_kind;
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
    session().reset_idle();
    crate::stream_touch::stop();
}

/// Marca la sesión actual como remota para cerrar Apollo si hace falta al terminar.
pub fn mark_remote_close() {
    session().streamed = true;
}

fn spawn_apollo_close(was_app: Option<String>) {
    let wait_helper = was_app.as_deref() == Some("Juego actual");
    std::thread::spawn(move || {
        if wait_helper {
            std::thread::sleep(Duration::from_millis(500));
        }
        if let Err(e) = crate::apollo::web::close_session(None) {
            eprintln!("[stream] apollo close_session: {e}");
        }
    });
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
    let device = session().device.clone();
    // El lanzador toca la ventana y la pantalla principal: va en el hilo principal.
    let (tx, rx) = std::sync::mpsc::channel();
    let sent = app.run_on_main_thread(move || {
        let _ = tx.send(crate::launcher::start_at(&app2, game, screen, device));
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
        Phase::Starting => {}
        Phase::Playing => {
            if crate::apollo::web::creds_present() {
                let new_epoch = {
                    let mut s = session();
                    s.epoch += 1;
                    s.device_kind.clear();
                    s.epoch
                };
                restore_cursor(&cursor_dir(app));
                spawn_cursor_reconcile(app.clone(), new_epoch);
            } else {
                let kind = session().device_kind.clone();
                let should_hide = match kind.as_str() {
                    "tablet" => true,
                    "" => resolve_fallback_tablet(app),
                    _ => false,
                };
                if should_hide {
                    session().device_kind = "tablet".into();
                    hide_cursor_if_enabled(app);
                } else {
                    restore_cursor(&cursor_dir(app));
                }
            }
        }
        Phase::Idle => {
            if app.state::<AppState>().active.lock().unwrap().is_none() {
                return Err("No hay ningún juego pendiente ni en marcha".to_string());
            }
            // Un cliente se conecto con el juego aun abierto en el PC.
            let new_epoch = {
                let mut s = session();
                s.epoch += 1;
                s.phase = Phase::Playing;
                s.streamed = true;
                s.device_kind.clear();
                s.epoch
            };
            spawn_cursor_reconcile(app.clone(), new_epoch);
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
    let was = session().cancel_pending();
    if was {
        crate::stream_touch::stop();
    }
    json!({ "cancelled": was })
}

fn should_force_close(force: bool, close_asked: Option<Instant>) -> bool {
    force || close_asked.is_some_and(|t| t.elapsed() < CLOSE_AGAIN)
}

/// Cierra el juego en marcha. `none` = no hay juego; `asked` = se le pidio cerrar; `forced` = se mato.
pub fn close(app: &AppHandle, force: bool) -> Result<Value, String> {
    let pids = {
        let st = app.state::<AppState>();
        let a = st.active.lock().unwrap();
        match a.as_ref() {
            None => None,
            Some(a) => Some(a.pids.lock().unwrap().clone()),
        }
    };
    let Some(pids) = pids else {
        let (was_pending, was_app) = session().close_without_active();
        if was_pending {
            crate::stream_touch::stop();
        }
        spawn_apollo_close(was_app);
        return Ok(json!({ "closed": "none" }));
    };
    let asked = {
        let mut s = session();
        s.streamed = true;
        s.close_asked
    };
    if should_force_close(force, asked) {
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

/// Hay una sesion de streaming activa (lanzada desde un cliente o reenganchada por Apollo/Artemis).
pub fn is_streaming() -> bool {
    let s = session();
    s.streamed && s.phase != Phase::Idle
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ciclo_sesion_streaming_y_local() {
        let mut s = Session::new();
        s.streamed = true;
        s.stream_app = Some("Juego actual".into());
        s.device = "tablet-1".into();
        s.device_kind = "tablet".into();
        assert!(s.on_started("g1".into(), "Juego 1".into(), 1000));
        let (sum, was_app, was_streamed) = s.on_exited("g1".into(), "Juego 1".into(), "PC".into(), 12, 13000);
        assert!(sum.streamed);
        assert_eq!(was_app.as_deref(), Some("Juego actual"));
        assert!(was_streamed);
        assert_eq!(s.phase, Phase::Idle);
        assert!(!s.streamed);
        assert!(s.stream_app.is_none());
        assert!(s.device.is_empty());
        assert!(s.device_kind.is_empty());

        // Otro PC en streaming no debe ocultar el cursor.
        s.streamed = true;
        s.device = "pc-2".into();
        s.device_kind = "pc".into();
        assert!(!s.on_started("g3".into(), "Juego 3".into(), 15000));
        s.on_exited("g3".into(), "Juego 3".into(), "PC".into(), 2, 17000);

        assert!(!s.on_started("g2".into(), "Juego 2".into(), 20000));
        let (sum2, app2, streamed2) = s.on_exited("g2".into(), "Juego 2".into(), "PC".into(), 5, 25000);
        assert!(!sum2.streamed);
        assert!(app2.is_none());
        assert!(!streamed2);
    }

    fn dev(id: &str, name: &str, kind: &str, apollo_id: Option<&str>, revoked: bool) -> visor_core::devices::Device {
        visor_core::devices::Device {
            id: id.into(),
            name: name.into(),
            kind: kind.into(),
            created_ms: 0,
            last_seen_ms: 0,
            app_version: String::new(),
            api: 3,
            revoked,
            apollo_id: apollo_id.map(str::to_string),
        }
    }

    fn apollo_client(uuid: &str, name: &str, connected: Option<bool>) -> crate::apollo::web::Client {
        crate::apollo::web::Client { uuid: uuid.into(), name: name.into(), perm: None, connected }
    }

    #[test]
    fn detecta_tipo_conectado_desde_apollo() {
        let devs = vec![
            dev("d1", "Mi Tablet", "tablet", Some("u-tab"), false),
            dev("d2", "Portátil", "pc", Some("u-pc"), false),
            dev("d3", "Tablet vieja", "tablet", None, false),
            dev("d4", "PC revocado", "pc", Some("u-rev"), true),
        ];
        // Solo tablet conectada -> tablet.
        assert_eq!(connected_kind_from_apollo(&[apollo_client("u-tab", "Mi Tablet", Some(true))], &devs), Some("tablet"));
        // Solo PC conectado -> pc.
        assert_eq!(connected_kind_from_apollo(&[apollo_client("u-pc", "Portátil", Some(true))], &devs), Some("pc"));
        // Si hay PC y tablet conectados a la vez, gana pc para no dejarle sin cursor.
        assert_eq!(
            connected_kind_from_apollo(&[apollo_client("u-tab", "Mi Tablet", Some(true)), apollo_client("u-pc", "Portátil", Some(true))], &devs),
            Some("pc")
        );
        // Emparejado sin apollo_id: coincide por nombre si sigue activo.
        assert_eq!(connected_kind_from_apollo(&[apollo_client("otra", "Tablet vieja", Some(true))], &devs), Some("tablet"));
        // Desconectados o revocados no cuentan.
        assert_eq!(connected_kind_from_apollo(&[apollo_client("u-tab", "Mi Tablet", Some(false))], &devs), None);
        assert_eq!(connected_kind_from_apollo(&[apollo_client("u-rev", "PC revocado", Some(true))], &devs), None);
        assert!(!has_active_pc(&[dev("d1", "Mi Tablet", "tablet", None, false), dev("d4", "PC revocado", "pc", None, true)]));
        assert!(has_active_pc(&devs));
    }

    #[test]
    fn cancelar_pendiente_limpia_estado() {
        let mut s = Session::new();
        s.phase = Phase::Pending;
        s.streamed = true;
        s.stream_app = Some("Desktop".into());
        s.device = "pc-salon".into();
        assert!(s.cancel_pending());
        assert_eq!(s.phase, Phase::Idle);
        assert!(!s.streamed);
        assert!(s.stream_app.is_none());
        assert!(s.device.is_empty());
        assert_eq!(s.epoch, 1);
        assert!(!s.cancel_pending());
    }

    #[test]
    fn cierre_con_juego_ya_cerrado_y_forzado() {
        let mut s = Session::new();
        s.phase = Phase::Playing;
        s.streamed = true;
        s.stream_app = Some("Juego actual".into());
        let (was_pending, was_app) = s.close_without_active();
        assert!(!was_pending);
        assert_eq!(was_app.as_deref(), Some("Juego actual"));
        assert_eq!(s.phase, Phase::Idle);
        assert!(s.stream_app.is_none());

        assert!(!should_force_close(false, None));
        assert!(should_force_close(true, None));
        assert!(should_force_close(false, Some(Instant::now())));
        assert!(!should_force_close(false, Some(Instant::now() - CLOSE_AGAIN - Duration::from_secs(1))));
    }
}
