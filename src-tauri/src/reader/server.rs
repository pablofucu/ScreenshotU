//! Servidor local de la pagina de lectura (127.0.0.1:6677, o el primer puerto libre hasta 6690).
//!
//! Un unico puerto atiende HTTP y WebSocket: se espia (`peek`) la cabecera de cada conexion y, si es una
//! peticion de actualizacion a WebSocket en `/ws`, se entrega el socket a `tungstenite`; si no, se responde HTTP.
//! - HTTP: `reader.html` y `/assets/...` se sirven desde los assets embebidos de Tauri
//!   (`AssetResolver::get`). En compilacion de depuracion se intenta antes el servidor de Vite
//!   (`http://127.0.0.1:1420`) haciendo de proxy, de modo que el WebSocket sigue en este mismo puerto.
//! - Seguridad: solo se aceptan `Host` locales y, en el WebSocket, `Origin` local (evita que una web
//!   cualquiera lea el texto del juego o use DNS rebinding).
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::{Error as WsError, Message};
use visor_core::jp::{NovelPrefs, ReaderLine};

const PORT_FIRST: u16 = 6677;
const PORT_LAST: u16 = 6690;
const HISTORY_MAX: usize = 200;
#[cfg(debug_assertions)]
const VITE_DEV: &str = "http://127.0.0.1:1420";

type PageCb = Arc<dyn Fn(Value) + Send + Sync>;

struct State {
    port: Option<u16>,
    clients: Vec<Sender<String>>,
    history: VecDeque<ReaderLine>,
    game: Option<String>,
    prefs: Option<NovelPrefs>,
    on_page_prefs: Option<PageCb>,
    /// Otros mensajes de la pagina (`openDicts`, `reloadDicts`...): reciben el mensaje entero.
    on_page_cmd: Option<PageCb>,
    /// Estado de los diccionarios: `{names, loading, dir}`.
    dicts: Value,
    /// Campos extra del `hello` (`known`, `anki`, `shots`...): lo que una pagina recien conectada necesita de entrada.
    extra: serde_json::Map<String, Value>,
}

fn state() -> MutexGuard<'static, State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(State {
            port: None,
            clients: Vec::new(),
            history: VecDeque::new(),
            game: None,
            prefs: None,
            on_page_prefs: None,
            on_page_cmd: None,
            dicts: json!({ "names": [], "loading": false, "dir": "" }),
            extra: serde_json::Map::new(),
        })
    })
    .lock()
    .unwrap_or_else(|e| e.into_inner())
}

fn send_all(st: &mut State, msg: &Value) {
    let text = msg.to_string();
    st.clients.retain(|tx| tx.send(text.clone()).is_ok());
}

fn hello(st: &State) -> Value {
    let mut v = json!({ "type": "hello", "game": st.game, "prefs": st.prefs, "history": st.history, "dicts": st.dicts });
    if let Some(o) = v.as_object_mut() {
        for (k, x) in &st.extra {
            o.insert(k.clone(), x.clone());
        }
    }
    v
}

// ---------------------------------------------------------------- API publica

/// Arranca el servidor (una sola vez; las siguientes llamadas devuelven el puerto ya abierto).
pub fn start(app: &tauri::AppHandle) -> Result<u16, String> {
    let mut st = state();
    if let Some(p) = st.port {
        return Ok(p);
    }
    let mut last_err = String::new();
    let mut bound = None;
    for port in PORT_FIRST..=PORT_LAST {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(l) => {
                bound = Some((l, port));
                break;
            }
            Err(e) => last_err = e.to_string(),
        }
    }
    let (listener, port) = bound.ok_or_else(|| format!("No hay puertos libres en {PORT_FIRST}-{PORT_LAST}: {last_err}"))?;
    st.port = Some(port);
    drop(st);
    let app = app.clone();
    std::thread::Builder::new()
        .name("reader-server".into())
        .spawn(move || {
            for conn in listener.incoming().flatten() {
                let app = app.clone();
                let _ = std::thread::Builder::new().name("reader-conn".into()).spawn(move || {
                    let _ = handle(conn, &app);
                });
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(port)
}

/// URL de la pagina de lectura (con el puerto real si el servidor ya esta en marcha).
pub fn url() -> String {
    format!("http://127.0.0.1:{}/", state().port.unwrap_or(PORT_FIRST))
}

/// Anade la linea al historial (ultimas ~200) y la envia a todas las paginas conectadas.
pub fn broadcast(line: &ReaderLine) {
    let mut st = state();
    st.history.push_back(line.clone());
    while st.history.len() > HISTORY_MAX {
        st.history.pop_front();
    }
    send_all(&mut st, &json!({ "type": "line", "line": line }));
}

/// Fija el juego en curso y sus preferencias. Si cambia el juego se reinicia el historial y las paginas
/// reciben un `hello` nuevo (vacio); si no, solo un mensaje `prefs`.
pub fn set_session(game: Option<String>, prefs: Option<NovelPrefs>) {
    let mut st = state();
    let changed = st.game != game;
    st.game = game;
    st.prefs = prefs;
    if changed {
        st.history.clear();
        let h = hello(&st);
        send_all(&mut st, &h);
    } else if let Some(p) = st.prefs.clone() {
        send_all(&mut st, &json!({ "type": "prefs", "prefs": p }));
    }
}

/// Envia nuevas preferencias (colores de personajes, furigana...) a las paginas.
pub fn send_prefs(prefs: &NovelPrefs) {
    let mut st = state();
    st.prefs = Some(prefs.clone());
    send_all(&mut st, &json!({ "type": "prefs", "prefs": prefs }));
}

/// Avisa de que el juego se cerro y olvida la sesion.
pub fn bye() {
    let mut st = state();
    send_all(&mut st, &json!({ "type": "bye" }));
    st.game = None;
    st.prefs = None;
    st.history.clear();
}

/// Callback para los cambios de lectura hechos desde la pagina (`{"type":"prefs","reader":{...}}`):
/// recibe el objeto `reader` (`{furigana, posColors, fontSize}`). Tambien se actualiza `prefs.reader` guardado.
pub fn on_page_prefs(f: impl Fn(Value) + Send + Sync + 'static) {
    state().on_page_prefs = Some(Arc::new(f));
}

/// Callback para el resto de mensajes de la pagina (`{"type":"openDicts"}`...).
pub fn on_page_cmd(f: impl Fn(Value) + Send + Sync + 'static) {
    state().on_page_cmd = Some(Arc::new(f));
}

/// Guarda un campo del `hello` (`known`, `anki`, `shots`...) para las paginas que se conecten despues.
pub fn set_extra(key: &str, v: Value) {
    state().extra.insert(key.to_string(), v);
}

/// Envia un mensaje cualquiera a todas las paginas conectadas.
pub fn send(msg: Value) {
    let mut st = state();
    send_all(&mut st, &msg);
}

/// Linea del historial por id.
pub fn line(id: u64) -> Option<ReaderLine> {
    state().history.iter().find(|l| l.id == id).cloned()
}

/// Id de la ultima linea del historial.
pub fn last_line_id() -> Option<u64> {
    state().history.back().map(|l| l.id)
}

/// Las `n` lineas anteriores a `id` en el historial (de mas antigua a mas reciente).
pub fn lines_before(id: u64, n: usize) -> Vec<ReaderLine> {
    let st = state();
    let pos = st.history.iter().position(|l| l.id == id).unwrap_or(st.history.len());
    st.history.iter().skip(pos.saturating_sub(n)).take(pos.min(n)).cloned().collect()
}

/// Publica el estado de los diccionarios (`{names, loading, dir}`) a las paginas conectadas.
pub fn set_dicts(v: Value) {
    let mut st = state();
    st.dicts = v.clone();
    send_all(&mut st, &json!({ "type": "dicts", "dicts": v }));
}

// ---------------------------------------------------------------- conexiones

struct Head {
    method: String,
    target: String,
    headers: HashMap<String, String>,
    len: usize, // bytes de la cabecera incluyendo \r\n\r\n
}

/// Lee (sin consumir) la cabecera de la peticion.
fn peek_head(s: &TcpStream) -> Option<Head> {
    let mut buf = vec![0u8; 16 * 1024];
    for _ in 0..1000 {
        let n = s.peek(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        if let Some(end) = buf[..n].windows(4).position(|w| w == b"\r\n\r\n") {
            let text = String::from_utf8_lossy(&buf[..end]).to_string();
            let mut lines = text.split("\r\n");
            let mut first = lines.next()?.split_whitespace();
            let method = first.next()?.to_string();
            let target = first.next()?.to_string();
            let headers = lines
                .filter_map(|l| l.split_once(':'))
                .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
                .collect();
            return Some(Head { method, target, headers, len: end + 4 });
        }
        if n == buf.len() {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    None
}

/// `host[:puerto]` o URL de origen -> es un nombre local?
fn is_local_host(h: &str) -> bool {
    let h = h.trim();
    let h = h.split("://").last().unwrap_or(h);
    let h = h.split('/').next().unwrap_or(h);
    let name = if let Some(rest) = h.strip_prefix('[') { rest.split(']').next().unwrap_or("") } else { h.split(':').next().unwrap_or("") };
    matches!(name, "127.0.0.1" | "localhost" | "::1")
}

fn handle(mut s: TcpStream, app: &tauri::AppHandle) -> std::io::Result<()> {
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    let _ = s.set_nodelay(true);
    let Some(head) = peek_head(&s) else { return Ok(()) };
    if !head.headers.get("host").map_or(false, |h| is_local_host(h)) {
        return respond(&mut s, 403, "text/plain; charset=utf-8", b"Forbidden", false);
    }
    let path = head.target.split('?').next().unwrap_or("/");
    let is_ws = head.headers.get("upgrade").map_or(false, |u| u.eq_ignore_ascii_case("websocket"));
    if is_ws && path == "/ws" {
        ws_session(s);
        return Ok(());
    }
    // Consume la cabecera y responde HTTP.
    let mut skip = vec![0u8; head.len];
    s.read_exact(&mut skip)?;
    let head_only = head.method == "HEAD";
    if head.method != "GET" && !head_only {
        return respond(&mut s, 405, "text/plain; charset=utf-8", b"Method Not Allowed", false);
    }
    if let Some((mime, bytes)) = crate::shots::http(app, path) {
        return respond(&mut s, 200, &mime, &bytes, head_only);
    }
    match fetch_asset(app, &head.target) {
        Some((mime, bytes)) => respond(&mut s, 200, &mime, &bytes, head_only),
        None => respond(&mut s, 404, "text/plain; charset=utf-8", b"Not Found", head_only),
    }
}

fn respond(s: &mut TcpStream, code: u16, mime: &str, body: &[u8], head_only: bool) -> std::io::Result<()> {
    let reason = match code {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    let head = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes())?;
    if !head_only {
        s.write_all(body)?;
    }
    s.flush()
}

/// Busca el recurso: (tipo MIME, bytes).
fn fetch_asset(app: &tauri::AppHandle, target: &str) -> Option<(String, Vec<u8>)> {
    let (path, query) = target.split_once('?').map_or((target, ""), |(p, q)| (p, q));
    let path = if path == "/" || path.is_empty() { "/reader.html" } else { path };
    if path.contains("..") || path.contains('\\') {
        return None;
    }
    #[cfg(debug_assertions)]
    if let Some(r) = proxy_vite(path, query) {
        return Some(r);
    }
    let _ = query;
    let a = app.asset_resolver().get(path.to_string())?;
    Some((a.mime_type, a.bytes))
}

/// En desarrollo: pide el recurso al servidor de Vite (si esta en marcha).
#[cfg(debug_assertions)]
fn proxy_vite(path: &str, query: &str) -> Option<(String, Vec<u8>)> {
    let url = if query.is_empty() { format!("{VITE_DEV}{path}") } else { format!("{VITE_DEV}{path}?{query}") };
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_millis(300)).timeout(Duration::from_secs(5)).build();
    let resp = agent.get(&url).call().ok()?;
    let mime = resp.header("content-type").unwrap_or("application/octet-stream").to_string();
    let mut bytes = Vec::new();
    resp.into_reader().read_to_end(&mut bytes).ok()?;
    Some((mime, bytes))
}

// ---------------------------------------------------------------- WebSocket

fn ws_session(s: TcpStream) {
    let check = |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
        let ok = req.headers().get("origin").and_then(|o| o.to_str().ok()).map_or(true, is_local_host);
        if ok {
            Ok(resp)
        } else {
            let mut r = ErrorResponse::new(Some("Forbidden".to_string()));
            *r.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
            Err(r)
        }
    };
    let Ok(mut ws) = tungstenite::accept_hdr(s, check) else { return };
    let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(100)));

    // Alta del cliente y `hello` dentro del mismo bloqueo: no se pierde ninguna linea entre ambos.
    let (tx, rx) = channel::<String>();
    let first = {
        let mut st = state();
        st.clients.push(tx);
        hello(&st).to_string()
    };
    if ws.send(Message::text(first)).is_err() {
        return;
    }
    loop {
        match ws.read() {
            Ok(Message::Text(t)) => on_page_message(&t),
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(WsError::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(_) => break,
        }
        while let Ok(m) = rx.try_recv() {
            if ws.send(Message::text(m)).is_err() {
                return;
            }
        }
    }
}

fn on_page_message(text: &str) {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return };
    if v.get("type").and_then(Value::as_str) != Some("prefs") {
        let cb = state().on_page_cmd.clone();
        if let Some(cb) = cb {
            cb(v);
        }
        return;
    }
    let Some(reader) = v.get("reader").cloned() else { return };
    let cb = {
        let mut st = state();
        if let Some(p) = st.prefs.as_mut() {
            if let Ok(r) = serde_json::from_value(reader.clone()) {
                p.reader = r;
            }
        }
        st.on_page_prefs.clone()
    };
    if let Some(cb) = cb {
        cb(reader);
    }
}
