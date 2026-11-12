//! Servidor HTTP para la app movil (contrato en `docs/referencia/api-movil.md`).
//!
//! Solo lee: sirve una instantanea de la biblioteca (JSON) y las imagenes que aparecen en ella
//! (miniatura, vista previa reducida u original). No depende de Tauri: los datos los da un [`Provider`].
//! Cada peticion la atiende uno de los hilos trabajadores (varios llaman a `recv` del mismo servidor),
//! asi el trabajo de imagen nunca bloquea la aceptacion de conexiones.
use crate::config::Config;
use crate::sagas::Saga;
use crate::scan::Library;
use crate::thumbs;
use anyhow::{anyhow, Context, Result};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Version del contrato (`api` en `hello` y `snapshot`). Se queda en 1: la APK de la tablet rechaza cualquier otro
/// valor y las rutas `/api/v1` siguen siendo compatibles. Lo nuevo se anuncia con `API_REV`.
pub const API_VERSION: u32 = 1;
/// Revision de la API (`apiRev` en `hello`): sube cuando se anaden rutas o campos. 2 = dispositivos con token,
/// 3 = emparejamiento por solicitud y billete (sin codigo fijo).
pub const API_REV: u32 = 3;
/// Revision mas antigua que se acepta: un cliente que mande `X-Visor-Api` menor recibe 426.
pub const API_MIN: u32 = 3;
/// Mensaje del 426 (cliente con una version anterior al emparejamiento nuevo).
pub const UPDATE_MSG: &str = "Actualiza la app: este PC usa el emparejamiento nuevo";
/// Una solicitud de emparejamiento sin respuesta caduca a los 2 minutos.
const PAIR_TTL: Duration = Duration::from_secs(120);
/// Solicitudes pendientes a la vez; la siguiente recibe 429.
const PAIR_MAX_PENDING: usize = 3;
/// Espera maxima de `GET /pair/wait` (segundos).
const PAIR_WAIT_MAX: u64 = 25;
/// Una solicitud ya resuelta (aprobada o rechazada) espera este tiempo a que el dispositivo la recoja.
const PAIR_RESOLVED_KEEP: Duration = Duration::from_secs(60);
/// Un billete de un solo uso vale 5 minutos.
const TICKET_TTL: Duration = Duration::from_secs(300);
pub const DEFAULT_PORT: u16 = 47811;
/// Ancho maximo de la vista previa para el visor del movil.
pub const PREVIEW_WIDTH: u32 = 1920;
const PREVIEW_QUALITY: u8 = 85;
/// Antiguedad maxima de la instantanea antes de volver a escanear.
const SNAPSHOT_MAX_AGE: Duration = Duration::from_secs(30);
/// Intentos fallidos permitidos por IP y ventana en la que se cuentan.
const FAIL_LIMIT: u32 = 10;
const FAIL_WINDOW: Duration = Duration::from_secs(60);
/// Hilos que atienden peticiones.
const WORKERS: usize = 16;
/// Eventos que guarda el bus (los mas recientes).
const EVENT_BUFFER: usize = 200;
/// Espera maxima de `GET /events` (segundos).
const EVENTS_MAX_WAIT: u64 = 25;
/// Cuerpo maximo de las peticiones POST.
const MAX_BODY: u64 = 1024 * 1024;
/// Cuerpo maximo de `POST /call` de un PC emparejado (sube la media de las frases: capturas y audio).
const MAX_UPLOAD: u64 = 48 * 1024 * 1024;
/// Error de `Provider::call` para un comando que no se puede usar desde fuera del escritorio (da 403).
pub const CALL_NOT_ALLOWED: &str = "No disponible desde la tablet";
/// Prefijo del error de `Provider::call` cuando los argumentos no encajan con el comando (da 400).
pub const CALL_BAD_ARGS: &str = "Argumentos no válidos";
/// Alfabeto del codigo de emparejamiento (sin 0/O, 1/I/L ni otros ambiguos).
pub const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/* ---------- datos ---------- */

/// Ajustes del escritorio que necesita el movil.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConfig {
    pub goal_vn: u32,
    pub goal_jvn: u32,
    pub goal_pc: u32,
    pub goal_switch: u32,
    pub accent_mode: String,
    pub accent_fixed: u8,
}

impl RemoteConfig {
    pub fn from_config(c: &Config) -> RemoteConfig {
        RemoteConfig { goal_vn: c.goal_vn, goal_jvn: c.goal_jvn, goal_pc: c.goal_pc, goal_switch: c.goal_switch, accent_mode: c.accent_mode.clone(), accent_fixed: c.accent_fixed }
    }
}

/// Partida en curso lanzada desde el visor.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Playing {
    pub id: String,
    pub name: String,
    /// Inicio (ms desde epoch).
    pub since: i64,
    /// Nombre del dispositivo que juega (vacio = en el propio host).
    pub device: String,
}

/// Quien hace la peticion: un dispositivo emparejado (con su token) o el propio host.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Caller {
    /// Id del dispositivo (`''` = el propio host).
    pub device_id: String,
    pub name: String,
    /// `tablet` o `pc`.
    pub kind: String,
}

impl Caller {
    /// El propio equipo (el ayudante de streaming desde loopback).
    pub fn host() -> Caller {
        Caller { device_id: String::new(), name: String::new(), kind: "pc".into() }
    }
}

/// Todo lo que la app movil necesita de una vez (sin `api`, `generatedAt` ni `name`, que anade el servidor).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub config: RemoteConfig,
    /// Biblioteca sin los juegos ocultos.
    pub library: Library,
    pub favorites: Vec<String>,
    pub sagas: Vec<Saga>,
    pub playing: Option<Playing>,
}

/// Documento que se envia en `/api/v1/snapshot`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotDoc<'a> {
    api: u32,
    generated_at: i64,
    name: &'a str,
    #[serde(flatten)]
    snap: &'a Snapshot,
}

/// Solicitud de emparejamiento pendiente, tal como la ve la app del PC principal.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PairRequest {
    /// Identificador corto para aprobar o rechazar desde el PC (distinto del `requestId` secreto del dispositivo).
    pub id_public: String,
    pub name: String,
    /// `tablet` o `pc`.
    pub kind: String,
    /// Numero de 4 cifras que el dispositivo muestra igual, para reconocer la solicitud.
    pub digits: String,
    /// IP desde la que se pidio.
    pub ip: String,
    /// Caducidad (ms desde epoch).
    pub expires_ms: i64,
}

/// Billete de un solo uso para emparejar sin verse en la red (QR o escrito a mano).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Ticket {
    /// `ABCD-EFGH`.
    pub code: String,
    /// Caducidad (ms desde epoch).
    pub expires_ms: i64,
}

/// Fuente de los datos (la app de escritorio, o el ejemplo `serve` sin interfaz).
pub trait Provider: Send + Sync + 'static {
    /// Nombre del equipo que se muestra en el movil.
    fn name(&self) -> String;
    /// Identificador fijo del equipo (vacio = sin identificador).
    fn id(&self) -> String {
        String::new()
    }
    /// Escaneo nuevo de la biblioteca (ya sin ocultos).
    fn snapshot(&self) -> Result<Snapshot>;
    /// Cache de miniaturas (la misma del escritorio).
    fn thumbs_dir(&self) -> PathBuf;
    /// Cache de vistas previas de 1920 px.
    fn previews_dir(&self) -> PathBuf;
    /// Dispositivo emparejado con ese token (no revocado).
    fn device_by_token(&self, _token: &str) -> Option<Caller> {
        None
    }
    /// Da de alta un dispositivo y devuelve `(id, token)`.
    fn pair(&self, _name: &str, _kind: &str, _version: &str, _api: u32) -> Result<(String, String)> {
        anyhow::bail!("Este equipo no admite emparejar dispositivos")
    }
    /// El dispositivo deja de estar emparejado (su token ya no vale).
    fn unpair(&self, _caller: &Caller) -> Result<()> {
        anyhow::bail!("Este equipo no admite emparejar dispositivos")
    }
    /// Ha llegado una solicitud de emparejamiento (para avisar al usuario: modal, notificacion).
    fn pair_requested(&self, _req: &PairRequest) {}
    /// Ha cambiado la lista de solicitudes pendientes (llega, caduca, se aprueba, se rechaza o se cancela).
    fn pair_requests_changed(&self) {}
    /// Empareja el streaming (Moonlight/Artemis). Con `pin` (lo genero el dispositivo) se mete en Apollo y se
    /// devuelve `{"ok":true}`; sin `pin` se pide un OTP a Apollo y se devuelve `{"pin","passphrase","port"}`.
    fn stream_pair(&self, _caller: &Caller, _pin: Option<&str>) -> Result<serde_json::Value> {
        anyhow::bail!("Este equipo no puede emparejar el streaming")
    }
    /// Un dispositivo con token se ha conectado (version de su app y revision de la API, si las manda).
    fn device_seen(&self, _caller: &Caller, _version: &str, _api: u32) {}
    /// Lanza un juego por su id con el lanzador del escritorio (cuenta el tiempo de juego).
    fn launch(&self, _caller: &Caller, _id: &str) -> Result<()> {
        anyhow::bail!("Este equipo no puede lanzar juegos")
    }
    /// Hace una captura ahora, como el atajo del escritorio.
    fn capture(&self) -> Result<()> {
        anyhow::bail!("Este equipo no puede hacer capturas")
    }
    /// Ejecuta un comando del visor por su nombre (`POST /call`). `Err(CALL_NOT_ALLOWED)` = no permitido.
    fn call(&self, _caller: &Caller, _cmd: &str, _args: serde_json::Value) -> std::result::Result<serde_json::Value, String> {
        Err(CALL_NOT_ALLOWED.into())
    }
    /// Cierra el juego en marcha (`POST /close`). Devuelve `{ "closed": "none"|"asked"|"forced" }`.
    fn close(&self, _force: bool) -> Result<serde_json::Value> {
        anyhow::bail!("Este equipo no puede cerrar juegos")
    }
    /// Estado ligero de la sesion de juego (`GET /session`).
    fn session(&self) -> serde_json::Value {
        serde_json::json!({ "state": "idle" })
    }
    /// El ayudante avisa de que la tablet ha conectado (`POST /session/start`, solo loopback).
    fn session_start(&self, _w: u32, _h: u32, _fps: u32) -> Result<serde_json::Value> {
        anyhow::bail!("Este equipo no admite sesiones de streaming")
    }
    /// Juego del host que el PC cliente `caller` quiere copiar entero (`GET /install/*`).
    fn install_source(&self, _caller: &Caller, _id: &str) -> Result<crate::gamecopy::InstallSource> {
        anyhow::bail!("Este equipo no permite copiar juegos")
    }
    /// Carpetas de las que sirve `GET /asset` (cualquier archivo dentro de ellas).
    fn asset_roots(&self) -> Vec<PathBuf> {
        vec![self.thumbs_dir(), self.previews_dir()]
    }
}

/* ---------- eventos (long-poll) ---------- */

/// Un evento del escritorio reenviado a los clientes remotos.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Event {
    pub seq: u64,
    pub name: String,
    pub payload: serde_json::Value,
}

struct BusState {
    seq: u64,
    buf: VecDeque<Event>,
}

/// Cola circular de eventos con numero de secuencia; `poll` espera a los nuevos.
pub struct EventBus {
    st: Mutex<BusState>,
    cv: std::sync::Condvar,
}

impl Default for EventBus {
    fn default() -> Self {
        EventBus::new()
    }
}

impl EventBus {
    pub fn new() -> EventBus {
        EventBus { st: Mutex::new(BusState { seq: 0, buf: VecDeque::new() }), cv: std::sync::Condvar::new() }
    }
    pub fn push(&self, name: &str, payload: serde_json::Value) {
        let mut st = lock(&self.st);
        st.seq += 1;
        let seq = st.seq;
        st.buf.push_back(Event { seq, name: name.to_string(), payload });
        while st.buf.len() > EVENT_BUFFER {
            st.buf.pop_front();
        }
        drop(st);
        self.cv.notify_all();
    }
    /// Secuencia del ultimo evento (0 = ninguno aun).
    pub fn current(&self) -> u64 {
        lock(&self.st).seq
    }
    /// Eventos con `seq > since`; si no hay, espera hasta `wait` y devuelve lo que llegue. `since == 0` (o de un
    /// servidor anterior, mayor que la secuencia actual) devuelve solo la secuencia vigente.
    pub fn poll(&self, since: u64, wait: Duration) -> (u64, Vec<Event>) {
        let deadline = Instant::now() + wait;
        let mut st = lock(&self.st);
        if since == 0 || since > st.seq {
            return (st.seq, Vec::new());
        }
        loop {
            if st.seq > since {
                let ev = st.buf.iter().filter(|e| e.seq > since).cloned().collect();
                return (st.seq, ev);
            }
            let now = Instant::now();
            if now >= deadline {
                return (st.seq, Vec::new());
            }
            st = self.cv.wait_timeout(st, deadline - now).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
}

/// Bus unico del proceso: la app empuja aqui los eventos de Tauri y `GET /events` los sirve.
pub fn event_bus() -> &'static EventBus {
    static BUS: std::sync::OnceLock<EventBus> = std::sync::OnceLock::new();
    BUS.get_or_init(EventBus::new)
}

/* ---------- utilidades publicas ---------- */

/// Codigo de emparejamiento nuevo `XXXX-XXXX` (aleatorio del sistema).
pub fn new_code() -> String {
    let mut out = String::with_capacity(9);
    let limit = (256 / CODE_ALPHABET.len() * CODE_ALPHABET.len()) as u8; // sin sesgo: se descartan los bytes >= limit
    let mut buf = [0u8; 32];
    while out.len() < 9 {
        if getrandom::getrandom(&mut buf).is_err() {
            fallback_random(&mut buf);
        }
        for b in buf {
            if b >= limit {
                continue;
            }
            if out.len() == 4 {
                out.push('-');
            }
            out.push(CODE_ALPHABET[(b as usize) % CODE_ALPHABET.len()] as char);
            if out.len() == 9 {
                break;
            }
        }
    }
    out
}

/// Identificador fijo de un equipo: 16 bytes aleatorios en hexadecimal.
pub fn new_id() -> String {
    let mut buf = [0u8; 32];
    if getrandom::getrandom(&mut buf).is_err() {
        fallback_random(&mut buf);
    }
    hex(&buf[..16])
}

/// Token secreto de un dispositivo: 32 bytes aleatorios en hexadecimal (64 caracteres).
pub fn new_token() -> String {
    let mut buf = [0u8; 32];
    if getrandom::getrandom(&mut buf).is_err() {
        fallback_random(&mut buf);
    }
    hex(&buf)
}

/// SHA-256 de un texto, en hexadecimal (para guardar tokens sin dejarlos en claro).
pub fn sha256_hex(s: &str) -> String {
    hex(&Sha256::digest(s.as_bytes()))
}

/// Por si el generador del sistema fallara: hora, proceso y contador pasados por SHA-256.
fn fallback_random(buf: &mut [u8; 32]) {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut h = Sha256::new();
    h.update(nanos.to_le_bytes());
    h.update(std::process::id().to_le_bytes());
    h.update(COUNTER.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    h.update((buf.as_ptr() as usize).to_le_bytes());
    buf.copy_from_slice(&h.finalize());
}

/// Codigo para comparar: sin guiones ni espacios y en mayusculas.
pub fn normalize_code(s: &str) -> String {
    s.chars().filter(|c| *c != '-' && !c.is_whitespace()).flat_map(|c| c.to_uppercase()).collect()
}

/// Compara un codigo recibido con el del servidor (normalizados). Un codigo vacio nunca vale.
pub fn code_matches(expected: &str, given: &str) -> bool {
    let (a, b) = (normalize_code(expected), normalize_code(given));
    if a.is_empty() || a.len() != b.len() {
        return false;
    }
    // Comparacion en tiempo constante (para la misma longitud).
    a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Nombre del equipo (`COMPUTERNAME` en Windows).
pub fn host_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "PC".into())
}

fn is_lan(ip: &Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_unspecified() && !ip.is_link_local() && !ip.is_multicast()
}

/// IPv4 de la red local, la de la interfaz principal primero (truco de «conectar» un socket UDP: no se envia
/// nada) y despues las que resuelva el nombre del equipo (otros adaptadores), quitando duplicados.
pub fn local_ipv4s() -> Vec<String> {
    let mut out: Vec<Ipv4Addr> = Vec::new();
    let primary = UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| s.connect("8.8.8.8:80").map(|_| s))
        .and_then(|s| s.local_addr())
        .ok()
        .and_then(|a| match a.ip() {
            IpAddr::V4(v4) => Some(v4),
            _ => None,
        });
    if let Some(p) = primary.filter(is_lan) {
        out.push(p);
    }
    use std::net::ToSocketAddrs;
    if let Ok(addrs) = (host_name().as_str(), 0u16).to_socket_addrs() {
        for a in addrs {
            if let IpAddr::V4(v4) = a.ip() {
                if is_lan(&v4) && v4.is_private() && !out.contains(&v4) {
                    out.push(v4);
                }
            }
        }
    }
    out.into_iter().map(|ip| ip.to_string()).collect()
}

/// IP de Tailscale (CGNAT, 100.64.0.0/10).
fn is_tailscale(ip: &Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 100 && (o[1] & 0xC0) == 64
}

/// Direcciones `ip:puerto` con las que se puede llegar a este equipo: primero las IPv4 de casa y despues las de
/// Tailscale (100.64.0.0/10).
pub fn host_addrs(port: u16) -> Vec<String> {
    let mut home: Vec<Ipv4Addr> = Vec::new();
    let mut tail: Vec<Ipv4Addr> = Vec::new();
    for ip in local_ipv4s().iter().filter_map(|s| s.parse::<Ipv4Addr>().ok()) {
        if is_tailscale(&ip) {
            tail.push(ip);
        } else if !home.contains(&ip) {
            home.push(ip);
        }
    }
    // `local_ipv4s` solo da redes privadas: la de Tailscale sale del nombre del equipo (todos sus adaptadores).
    use std::net::ToSocketAddrs;
    if let Ok(addrs) = (host_name().as_str(), 0u16).to_socket_addrs() {
        for a in addrs {
            if let IpAddr::V4(v4) = a.ip() {
                if is_tailscale(&v4) && !tail.contains(&v4) {
                    tail.push(v4);
                }
            }
        }
    }
    home.into_iter().chain(tail).map(|ip| format!("{ip}:{port}")).collect()
}

/// Contenido del QR de emparejamiento (v2): direcciones del PC y billete de un solo uso.
pub fn qr_payload(id: &str, name: &str, addrs: &[String], port: u16, ticket: &str) -> String {
    serde_json::json!({ "v": 2, "app": "visor-capturas", "id": id, "name": name, "addrs": addrs, "port": port, "ticket": ticket }).to_string()
}

/// Rutas que se pueden servir: capturas, portadas, hero y logos de la instantanea.
pub fn allowed_paths(lib: &Library) -> HashSet<String> {
    let mut set = HashSet::new();
    for p in &lib.platforms {
        for t in &p.titles {
            set.extend(t.shots.iter().map(|s| s.path.clone()));
            set.extend([&t.cover, &t.hero, &t.logo].into_iter().flatten().cloned());
        }
    }
    set
}

/// `If-None-Match` coincide con la ETag (lista separada por comas, `*` o etiquetas debiles `W/`).
pub fn etag_matches(if_none_match: Option<&str>, etag: &str) -> bool {
    let Some(h) = if_none_match else { return false };
    h.split(',').map(str::trim).any(|t| t == "*" || t.trim_start_matches("W/") == etag)
}

/// El cliente acepta gzip (`Accept-Encoding`), salvo con `q=0`.
pub fn accepts_gzip(accept_encoding: Option<&str>) -> bool {
    let Some(h) = accept_encoding else { return false };
    h.split(',').any(|tok| {
        let mut parts = tok.split(';');
        let name = parts.next().unwrap_or("").trim();
        let q0 = parts.any(|p| {
            let p = p.trim().replace(' ', "");
            p == "q=0" || p == "q=0.0" || p == "q=0.00" || p == "q=0.000"
        });
        name.eq_ignore_ascii_case("gzip") && !q0
    })
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// `Mutex::lock` que no entra en panico si otro hilo lo envenenó (en release `panic = "abort"`).
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/* ---------- vistas previas ---------- */

/// Genera (si falta) la vista previa JPEG de como mucho 1920 px de ancho; misma clave que las miniaturas.
pub fn ensure_preview(cache_dir: &Path, src: &Path) -> Result<PathBuf> {
    let out = thumbs::thumb_path(cache_dir, src)?;
    if out.exists() {
        return Ok(out);
    }
    fs::create_dir_all(cache_dir)?;
    let img = image::open(src).with_context(|| format!("decodificando {}", src.display()))?;
    let (w, h) = (img.width(), img.height());
    let img = if w > PREVIEW_WIDTH {
        let nh = ((h as f64) * (PREVIEW_WIDTH as f64) / (w as f64)).round().max(1.0) as u32;
        img.resize_exact(PREVIEW_WIDTH, nh, FilterType::CatmullRom)
    } else {
        img
    };
    let rgb = img.to_rgb8();
    static N: AtomicU64 = AtomicU64::new(0);
    let tmp = out.with_extension(format!("tmp{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    {
        let f = fs::File::create(&tmp)?;
        let mut enc = JpegEncoder::new_with_quality(std::io::BufWriter::new(f), PREVIEW_QUALITY);
        enc.encode_image(&rgb)?;
    }
    fs::rename(&tmp, &out)?;
    Ok(out)
}

/* ---------- limite de intentos ---------- */

/// Cuenta los fallos de autenticacion por IP en ventanas de un minuto.
#[derive(Default)]
pub struct Throttle {
    map: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl Throttle {
    /// Ya hubo `FAIL_LIMIT` fallos en la ventana actual: se contesta 429 hasta que acabe.
    pub fn blocked(&self, ip: IpAddr, now: Instant) -> bool {
        lock(&self.map).get(&ip).is_some_and(|(start, n)| now.saturating_duration_since(*start) < FAIL_WINDOW && *n >= FAIL_LIMIT)
    }

    pub fn fail(&self, ip: IpAddr, now: Instant) {
        let mut m = lock(&self.map);
        if m.len() > 1024 {
            m.retain(|_, (start, _)| now.saturating_duration_since(*start) < FAIL_WINDOW);
        }
        let e = m.entry(ip).or_insert((now, 0));
        if now.saturating_duration_since(e.0) >= FAIL_WINDOW {
            *e = (now, 0);
        }
        e.1 += 1;
    }
}

/* ---------- instantanea en cache ---------- */

struct Cached {
    at: Instant,
    etag: String,
    json: Vec<u8>,
    gz: Vec<u8>,
    allowed: HashSet<String>,
}

fn build_cached(name: &str, snap: &Snapshot) -> Result<Cached> {
    // La ETag sale de los datos (sin `generatedAt`), asi un reescaneo sin cambios sigue dando 304.
    let data = serde_json::to_vec(snap)?;
    let etag = format!("\"{}\"", &hex(&Sha256::digest(&data))[..20]);
    let json = serde_json::to_vec(&SnapshotDoc { api: API_VERSION, generated_at: now_ms(), name, snap })?;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(&json)?;
    let gz = enc.finish()?;
    Ok(Cached { at: Instant::now(), etag, json, gz, allowed: allowed_paths(&snap.library) })
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/* ---------- peticiones y respuestas ---------- */

/// Peticion ya desmontada (independiente de tiny_http, para poder probar el enrutado).
pub struct Req {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    /// Nombres en minusculas.
    pub headers: Vec<(String, String)>,
    pub ip: IpAddr,
    /// Cuerpo de la peticion (solo las POST; vacio en el resto).
    pub body: Vec<u8>,
}

impl Req {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
    /// Separa ruta y query de la URL de la peticion.
    pub fn parse(method: &str, url: &str, headers: Vec<(String, String)>, ip: IpAddr) -> Req {
        let (path, q) = url.split_once('?').unwrap_or((url, ""));
        let query = q
            .split('&')
            .filter(|kv| !kv.is_empty())
            .map(|kv| {
                let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
                (percent_decode(k), percent_decode(v))
            })
            .collect();
        Req { method: method.to_ascii_uppercase(), path: percent_decode(path), query, headers, ip, body: Vec::new() }
    }
}

/// Decodifica `%XX` y `+` (espacio) de una URL; los escapes rotos se dejan tal cual.
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let h = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                match h {
                    Some(v) => {
                        out.push(v);
                        i += 3;
                        continue;
                    }
                    None => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub enum Body {
    Empty,
    Bytes(Vec<u8>),
    File(PathBuf),
    /// Trozo de un archivo: `(ruta, inicio, longitud)` (respuestas 206 a `Range`).
    FileRange(PathBuf, u64, u64),
}

pub struct Reply {
    pub status: u16,
    pub headers: Vec<(&'static str, String)>,
    pub body: Body,
}

impl Reply {
    fn json<T: Serialize>(status: u16, v: &T) -> Reply {
        let body = serde_json::to_vec(v).unwrap_or_else(|_| b"{}".to_vec());
        Reply { status, headers: vec![("Content-Type", "application/json; charset=utf-8".into())], body: Body::Bytes(body) }
    }
    fn error(status: u16, msg: &str) -> Reply {
        Reply::json(status, &serde_json::json!({ "error": msg }))
    }
    fn empty(status: u16) -> Reply {
        Reply { status, headers: vec![], body: Body::Empty }
    }
    fn with(mut self, k: &'static str, v: impl Into<String>) -> Reply {
        self.headers.push((k, v.into()));
        self
    }
}

fn content_type(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("avif") => "image/avif",
        Some("ico") => "image/x-icon",
        Some("svg") => "image/svg+xml",
        Some("mp3") => "audio/mpeg",
        Some("ogg") | Some("oga") | Some("opus") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("m4a") => "audio/mp4",
        Some("flac") => "audio/flac",
        Some("webm") => "video/webm",
        Some("mp4") => "video/mp4",
        _ => "application/octet-stream",
    }
}

/// Estado compartido por los hilos del servidor.
struct Inner {
    provider: Arc<dyn Provider>,
    /// Puerto en el que escucha (para las direcciones que se anuncian).
    port: u16,
    cache: Mutex<Option<Arc<Cached>>>,
    /// Un solo escaneo a la vez.
    building: Mutex<()>,
    throttle: Throttle,
    /// Cerrojos por ruta (repartidos) para no generar dos veces la misma miniatura a la vez.
    gen_locks: Vec<Mutex<()>>,
    /// Solicitudes de emparejamiento y billete activo (solo en memoria).
    pairing: Mutex<Pairing>,
    pairing_cv: Condvar,
}

enum ReqState {
    Pending,
    Approved { device_id: String, token: String },
    Rejected,
}

struct PendingReq {
    /// Secreto del dispositivo (`requestId`).
    secret: String,
    public: String,
    name: String,
    kind: String,
    version: String,
    api: u32,
    digits: String,
    ip: IpAddr,
    created: Instant,
    created_ms: i64,
    resolved: Option<Instant>,
    state: ReqState,
}

impl PendingReq {
    fn is_pending(&self) -> bool {
        matches!(self.state, ReqState::Pending)
    }
    fn expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.created) >= PAIR_TTL
    }
    fn view(&self) -> PairRequest {
        PairRequest {
            id_public: self.public.clone(),
            name: self.name.clone(),
            kind: self.kind.clone(),
            digits: self.digits.clone(),
            ip: self.ip.to_string(),
            expires_ms: self.created_ms + PAIR_TTL.as_millis() as i64,
        }
    }
}

struct TicketState {
    code: String,
    expires: Instant,
}

#[derive(Default)]
struct Pairing {
    reqs: Vec<PendingReq>,
    ticket: Option<TicketState>,
}

impl Pairing {
    /// Quita lo caducado (solicitudes sin responder, resueltas sin recoger y billete). `true` si cambio la lista
    /// de pendientes.
    fn purge(&mut self, now: Instant) -> bool {
        let before = self.reqs.iter().filter(|r| r.is_pending()).count();
        self.reqs.retain(|r| match r.resolved {
            None => !r.expired(now),
            Some(t) => now.saturating_duration_since(t) < PAIR_RESOLVED_KEEP,
        });
        if self.ticket.as_ref().is_some_and(|t| now >= t.expires) {
            self.ticket = None;
        }
        before != self.reqs.iter().filter(|r| r.is_pending()).count()
    }
}

/// `n` cifras decimales aleatorias, sin sesgo.
fn random_digits(n: usize) -> String {
    let mut out = String::new();
    while out.len() < n {
        let mut buf = [0u8; 32];
        if getrandom::getrandom(&mut buf).is_err() {
            fallback_random(&mut buf);
        }
        for b in buf {
            if b < 250 && out.len() < n {
                out.push((b'0' + b % 10) as char);
            }
        }
    }
    out
}

impl Inner {
    fn new(provider: Arc<dyn Provider>, port: u16) -> Inner {
        Inner {
            provider,
            port,
            pairing: Mutex::new(Pairing::default()),
            pairing_cv: Condvar::new(),
            cache: Mutex::new(None),
            building: Mutex::new(()),
            throttle: Throttle::default(),
            gen_locks: (0..16).map(|_| Mutex::new(())).collect(),
        }
    }

    /// Instantanea vigente (o una nueva si tiene mas de 30 s o se pide `fresh`).
    fn snapshot(&self, fresh: bool) -> Result<Arc<Cached>> {
        let asked = Instant::now();
        let usable = |c: &Arc<Cached>| if fresh { c.at >= asked } else { c.at.elapsed() < SNAPSHOT_MAX_AGE };
        if let Some(c) = lock(&self.cache).as_ref().filter(|c| usable(c)) {
            return Ok(c.clone());
        }
        let _b = lock(&self.building);
        // Otro hilo pudo terminar un escaneo mientras se esperaba.
        if let Some(c) = lock(&self.cache).as_ref().filter(|c| usable(c)) {
            return Ok(c.clone());
        }
        let snap = self.provider.snapshot()?;
        let c = Arc::new(build_cached(&self.provider.name(), &snap)?);
        *lock(&self.cache) = Some(c.clone());
        Ok(c)
    }

    /// Lista blanca de rutas: la de la ultima instantanea (si aun no hay ninguna, se escanea).
    fn whitelist(&self) -> Result<Arc<Cached>> {
        if let Some(c) = lock(&self.cache).as_ref() {
            return Ok(c.clone());
        }
        self.snapshot(false)
    }

    /// Direcciones con las que llegar a este equipo (de casa primero, Tailscale despues).
    fn addrs(&self) -> Vec<String> {
        host_addrs(self.port)
    }

    /// Avisa a la app de que cambio la lista de solicitudes (nunca con el cerrojo de `pairing` cogido).
    fn pairs_changed(&self) {
        self.provider.pair_requests_changed();
    }

    /// Solicitudes pendientes (sin las caducadas).
    fn pending_requests_at(&self, now: Instant) -> Vec<PairRequest> {
        let mut g = lock(&self.pairing);
        let changed = g.purge(now);
        let v = g.reqs.iter().filter(|r| r.is_pending()).map(PendingReq::view).collect();
        drop(g);
        if changed {
            self.pairs_changed();
        }
        v
    }

    /// Aprueba una solicitud: da de alta el dispositivo y deja el token listo para `GET /pair/wait`.
    fn approve_at(&self, id_public: &str, now: Instant) -> Result<()> {
        let (found, changed) = {
            let mut g = lock(&self.pairing);
            let changed = g.purge(now);
            let found = g.reqs.iter().find(|r| r.is_pending() && r.public == id_public).map(|r| (r.name.clone(), r.kind.clone(), r.version.clone(), r.api));
            (found, changed)
        };
        let Some((name, kind, version, api)) = found else {
            if changed {
                self.pairs_changed();
            }
            anyhow::bail!("La solicitud ya no existe (puede haber caducado)")
        };
        let (device_id, token) = self.provider.pair(&name, &kind, &version, api)?;
        let mut g = lock(&self.pairing);
        let Some(r) = g.reqs.iter_mut().find(|r| r.is_pending() && r.public == id_public) else {
            // Se cancelo mientras se daba de alta: no se deja un dispositivo huerfano.
            drop(g);
            let _ = self.provider.unpair(&Caller { device_id, name, kind });
            self.pairs_changed();
            anyhow::bail!("La solicitud ya no existe (puede haber caducado)");
        };
        r.state = ReqState::Approved { device_id, token };
        r.resolved = Some(now);
        drop(g);
        self.pairing_cv.notify_all();
        self.pairs_changed();
        Ok(())
    }

    fn reject_at(&self, id_public: &str, now: Instant) -> Result<()> {
        let mut g = lock(&self.pairing);
        g.purge(now);
        let Some(r) = g.reqs.iter_mut().find(|r| r.is_pending() && r.public == id_public) else {
            drop(g);
            self.pairs_changed();
            anyhow::bail!("La solicitud ya no existe (puede haber caducado)");
        };
        r.state = ReqState::Rejected;
        r.resolved = Some(now);
        drop(g);
        self.pairing_cv.notify_all();
        self.pairs_changed();
        Ok(())
    }

    fn new_ticket_at(&self, now: Instant) -> Ticket {
        let code = new_code();
        let expires_ms = now_ms() + TICKET_TTL.as_millis() as i64;
        lock(&self.pairing).ticket = Some(TicketState { code: code.clone(), expires: now + TICKET_TTL });
        Ticket { code, expires_ms }
    }

    /// Quien llama segun su cabecera `Authorization` (o `k`): el token de un dispositivo emparejado.
    fn caller(&self, req: &Req) -> Option<Caller> {
        let given = req
            .header("authorization")
            .and_then(|h| {
                let h = h.trim();
                (h.len() > 7 && h[..7].eq_ignore_ascii_case("bearer ")).then(|| h[7..].to_string())
            })
            .or_else(|| req.query.get("k").cloned())
            .unwrap_or_default();
        let given = given.trim();
        if given.is_empty() {
            return None;
        }
        let c = self.provider.device_by_token(given)?;
        self.provider.device_seen(&c, req.header("x-visor-version").unwrap_or("").trim(), client_api(req).unwrap_or(0));
        Some(c)
    }

    fn gen_lock(&self, key: &str) -> MutexGuard<'_, ()> {
        let i = (thumbs::fnv1a64(key.as_bytes()) as usize) % self.gen_locks.len();
        lock(&self.gen_locks[i])
    }
}

fn route(inner: &Inner, req: &Req) -> Reply {
    route_at(inner, req, Instant::now())
}

/// Como `route`, con la hora que se le da (las pruebas adelantan el reloj para caducar solicitudes y billetes).
fn route_at(inner: &Inner, req: &Req, now: Instant) -> Reply {
    if req.method == "OPTIONS" {
        return Reply::empty(204);
    }
    let post = req.method == "POST";
    if !post && req.method != "GET" && req.method != "HEAD" {
        return Reply::error(405, "Método no permitido");
    }
    let ep = match req.path.trim_end_matches('/') {
        "/api/v1/hello" => {
            return Reply::json(
                200,
                &serde_json::json!({ "app": "visor-capturas", "api": API_VERSION, "name": inner.provider.name(), "version": env!("CARGO_PKG_VERSION"), "id": inner.provider.id(), "apiRev": API_REV, "apiMin": API_MIN, "features": ["launch", "capture", "call", "events", "asset", "discover", "close", "session", "pair2", "devices", "install"], "addrs": inner.addrs() }),
            )
        }
        // El emparejamiento por codigo ya no existe: cualquier metodo recibe 426.
        "/api/v1/pair" => return Reply::error(426, UPDATE_MSG),
        "/api/v1/launch"
        | "/api/v1/capture"
        | "/api/v1/call"
        | "/api/v1/close"
        | "/api/v1/session/start"
        | "/api/v1/unpair"
        | "/api/v1/pair/request"
        | "/api/v1/pair/ticket"
        | "/api/v1/pair/cancel"
        | "/api/v1/stream/pair"
            if !post =>
        {
            return Reply::error(405, "Método no permitido")
        }
        "/api/v1/pair/request" => "pair_request",
        "/api/v1/pair/ticket" => "pair_ticket",
        "/api/v1/pair/cancel" => "pair_cancel",
        "/api/v1/stream/pair" => "stream_pair",
        "/api/v1/unpair" => "unpair",
        "/api/v1/launch" => "launch",
        "/api/v1/close" => "close",
        "/api/v1/session/start" => "session_start",
        "/api/v1/call" => "call",
        "/api/v1/capture" => "capture",
        _ if post => return Reply::error(405, "Método no permitido"),
        "/api/v1/pair/wait" => "pair_wait",
        "/api/v1/session" => "session",
        "/api/v1/snapshot" => "snapshot",
        "/api/v1/thumb" => "thumb",
        "/api/v1/preview" => "preview",
        "/api/v1/file" => "file",
        "/api/v1/events" => "events",
        "/api/v1/asset" => "asset",
        "/api/v1/install/list" => "install_list",
        "/api/v1/install/file" => "install_file",
        _ => return Reply::error(404, "No existe"),
    };
    if client_api(req).is_some_and(|a| a < API_MIN) {
        return Reply::error(426, UPDATE_MSG);
    }
    if inner.throttle.blocked(req.ip, now) {
        return Reply::error(429, "Demasiados intentos fallidos. Espera un minuto.");
    }
    match ep {
        "pair_request" => return pair_request_reply(inner, req, now),
        "pair_ticket" => return pair_ticket_reply(inner, req, now),
        "pair_cancel" => return pair_cancel_reply(inner, req),
        "pair_wait" => return pair_wait_reply(inner, req, now),
        _ => {}
    }
    // El ayudante de streaming habla desde el propio PC: sin codigo desde loopback.
    let local_ok = (ep == "session" || ep == "session_start") && req.ip.is_loopback();
    if ep == "session_start" && !local_ok {
        return Reply::error(403, "Solo desde el propio equipo");
    }
    let caller = if local_ok { Some(Caller::host()) } else { inner.caller(req) };
    let Some(caller) = caller else {
        inner.throttle.fail(req.ip, now);
        return Reply::error(401, "Este dispositivo no está emparejado con este PC.");
    };
    if ep == "stream_pair" {
        // Con `pin` el dispositivo lo genero y el PC lo mete en Apollo; sin cuerpo el PC pide un OTP a Apollo.
        let pin = serde_json::from_slice::<serde_json::Value>(&req.body)
            .ok()
            .and_then(|v| v.get("pin").and_then(|p| p.as_str().map(|p| p.trim().to_string())))
            .filter(|p| !p.is_empty());
        return match inner.provider.stream_pair(&caller, pin.as_deref()) {
            Ok(v) => Reply::json(200, &v),
            Err(e) => Reply::error(409, &format!("{e:#}")),
        };
    }
    if ep == "unpair" {
        return match inner.provider.unpair(&caller) {
            Ok(()) => Reply::json(200, &serde_json::json!({ "ok": true })),
            Err(e) => Reply::error(409, &format!("{e:#}")),
        };
    }
    if ep == "session" {
        return Reply::json(200, &inner.provider.session()).with("Cache-Control", "no-store");
    }
    if ep == "session_start" {
        let v = serde_json::from_slice::<serde_json::Value>(&req.body).unwrap_or(serde_json::Value::Null);
        let n = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        return match inner.provider.session_start(n("width"), n("height"), n("fps")) {
            Ok(r) => Reply::json(200, &r),
            Err(e) => Reply::error(409, &format!("{e:#}")),
        };
    }
    if ep == "close" {
        let force = serde_json::from_slice::<serde_json::Value>(&req.body).ok().and_then(|v| v.get("force").and_then(|f| f.as_bool())).unwrap_or(false);
        return match inner.provider.close(force) {
            Ok(r) => Reply::json(200, &r),
            Err(e) => Reply::error(409, &format!("{e:#}")),
        };
    }
    if ep == "launch" {
        let id = serde_json::from_slice::<serde_json::Value>(&req.body).ok().and_then(|v| v.get("id").and_then(|i| i.as_str().map(str::to_string))).filter(|i| !i.is_empty());
        let Some(id) = id else {
            return Reply::error(400, "Falta el id del juego");
        };
        return match inner.provider.launch(&caller, &id) {
            Ok(()) => Reply::json(200, &serde_json::json!({ "ok": true })),
            Err(e) => Reply::error(409, &format!("{e:#}")),
        };
    }
    if ep == "call" {
        return call_reply(inner, &caller, &req.body);
    }
    if ep == "events" {
        let num = |k: &str| req.query.get(k).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        let (seq, events) = event_bus().poll(num("since"), Duration::from_secs(num("wait").min(EVENTS_MAX_WAIT)));
        return Reply::json(200, &serde_json::json!({ "seq": seq, "events": events })).with("Cache-Control", "no-store");
    }
    if ep == "asset" {
        return asset_reply(inner, req);
    }
    if ep == "install_list" || ep == "install_file" {
        return install_reply(inner, &caller, req, ep == "install_list");
    }
    if ep == "capture" {
        return match inner.provider.capture() {
            Ok(()) => Reply::json(200, &serde_json::json!({ "ok": true })),
            Err(e) => Reply::error(409, &format!("{e:#}")),
        };
    }
    if ep == "snapshot" {
        let fresh = req.query.get("fresh").is_some_and(|v| v == "1" || v == "true");
        return match inner.snapshot(fresh) {
            Err(e) => Reply::error(500, &format!("No se pudo leer la biblioteca: {e:#}")),
            Ok(c) => snapshot_reply(&c, req),
        };
    }
    let Some(p) = req.query.get("p").filter(|p| !p.is_empty()) else {
        return Reply::error(400, "Falta el parámetro p");
    };
    let wl = match inner.whitelist() {
        Ok(c) => c,
        Err(e) => return Reply::error(500, &format!("No se pudo leer la biblioteca: {e:#}")),
    };
    if !wl.allowed.contains(p.as_str()) {
        return Reply::error(404, "Ruta desconocida");
    }
    let src = Path::new(p);
    if !src.is_file() {
        return Reply::error(404, "La imagen ya no existe");
    }
    let (file, ctype, cache) = match ep {
        "file" => (src.to_path_buf(), content_type(src), "private, max-age=3600"),
        kind => {
            let _g = inner.gen_lock(&format!("{kind}\u{0}{p}"));
            let r = if kind == "thumb" {
                thumbs::ensure_thumb_pooled(&inner.provider.thumbs_dir(), src)
            } else {
                let dir = inner.provider.previews_dir();
                thumbs::pool().install(|| ensure_preview(&dir, src))
            };
            match r {
                Ok(f) => (f, "image/jpeg", "private, max-age=604800"),
                Err(e) => return Reply::error(500, &format!("No se pudo preparar la imagen: {e:#}")),
            }
        }
    };
    // ETag: la clave de ruta + fecha + tamano del original (como las miniaturas).
    let etag = match thumbs::thumb_path(Path::new(""), src) {
        Ok(k) => format!("\"{}-{}\"", &ep[..1], k.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()),
        Err(_) => String::new(),
    };
    if !etag.is_empty() && etag_matches(req.header("if-none-match"), &etag) {
        return Reply::empty(304).with("ETag", etag).with("Cache-Control", cache);
    }
    let mut r = Reply { status: 200, headers: vec![("Content-Type", ctype.into()), ("Cache-Control", cache.into())], body: Body::File(file) };
    if !etag.is_empty() {
        r = r.with("ETag", etag);
    }
    r
}

/// Revision de la API que dice usar el cliente (`X-Visor-Api`), si la manda.
fn client_api(req: &Req) -> Option<u32> {
    req.header("x-visor-api").and_then(|v| v.trim().parse().ok())
}

/// Cuerpo JSON de una peticion de emparejamiento (`Null` si no lo es) y su campo `api`.
fn pair_body(req: &Req) -> serde_json::Value {
    serde_json::from_slice::<serde_json::Value>(&req.body).unwrap_or(serde_json::Value::Null)
}

/// Un cliente que no dice `api` en el cuerpo, o dice una menor que `API_MIN`, es de antes del emparejamiento nuevo.
fn body_too_old(v: &serde_json::Value) -> bool {
    v.get("api").and_then(|x| x.as_u64()).unwrap_or(0) < API_MIN as u64
}

/// `POST /pair/request` `{name, kind, appVersion, api}` -> `{requestId, digits, expiresMs}` (milisegundos que le
/// quedan a la solicitud). Queda pendiente hasta que el usuario la apruebe o rechace en el PC.
fn pair_request_reply(inner: &Inner, req: &Req, now: Instant) -> Reply {
    let v = pair_body(req);
    if body_too_old(&v) {
        return Reply::error(426, UPDATE_MSG);
    }
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let name: String = s("name").chars().take(60).collect();
    if name.is_empty() {
        return Reply::error(400, "Falta el nombre del dispositivo");
    }
    let kind = if s("kind") == "tablet" { "tablet" } else { "pc" };
    let api = v.get("api").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    let mut g = lock(&inner.pairing);
    let mut changed = g.purge(now);
    // Quien repite la solicitud (misma IP y nombre) sustituye a la anterior en vez de llenar los huecos.
    let before = g.reqs.len();
    g.reqs.retain(|r| !(r.is_pending() && r.ip == req.ip && r.name == name));
    changed |= g.reqs.len() != before;
    if g.reqs.iter().filter(|r| r.is_pending()).count() >= PAIR_MAX_PENDING {
        drop(g);
        inner.throttle.fail(req.ip, now);
        if changed {
            inner.pairs_changed();
        }
        return Reply::error(429, "Hay demasiadas solicitudes pendientes en este PC. Espera un momento.");
    }
    let p = PendingReq {
        secret: new_id(),
        public: new_id()[..8].to_string(),
        name,
        kind: kind.into(),
        version: s("appVersion"),
        api,
        digits: random_digits(4),
        ip: req.ip,
        created: now,
        created_ms: now_ms(),
        resolved: None,
        state: ReqState::Pending,
    };
    let view = p.view();
    let out = serde_json::json!({ "requestId": p.secret, "digits": p.digits, "expiresMs": PAIR_TTL.as_millis() as u64 });
    g.reqs.push(p);
    drop(g);
    inner.provider.pair_requested(&view);
    inner.pairs_changed();
    Reply::json(200, &out).with("Cache-Control", "no-store")
}

/// `GET /pair/wait?id=<requestId>[&wait=s]`: espera (hasta 25 s) a que el PC resuelva la solicitud.
/// `{status: pending|approved|rejected|expired}`; si `approved`, ademas `{deviceId, token, hostId, hostName, addrs}`
/// (una sola vez: la solicitud se borra). Una solicitud desconocida cuenta como `expired`.
fn pair_wait_reply(inner: &Inner, req: &Req, now: Instant) -> Reply {
    let Some(id) = req.query.get("id").filter(|i| !i.is_empty()) else {
        return Reply::error(400, "Falta el parámetro id");
    };
    let wait = req.query.get("wait").and_then(|v| v.parse::<u64>().ok()).unwrap_or(PAIR_WAIT_MAX).min(PAIR_WAIT_MAX);
    let started = Instant::now();
    let deadline = started + Duration::from_secs(wait);
    let status = |s: &str| Reply::json(200, &serde_json::json!({ "status": s })).with("Cache-Control", "no-store");
    let mut g = lock(&inner.pairing);
    loop {
        let at = now + started.elapsed();
        let Some(i) = g.reqs.iter().position(|r| &r.secret == id) else {
            return status("expired");
        };
        if g.reqs[i].is_pending() {
            if g.reqs[i].expired(at) {
                g.reqs.remove(i);
                drop(g);
                inner.pairs_changed();
                return status("expired");
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return status("pending");
            }
            g = inner.pairing_cv.wait_timeout(g, left.min(Duration::from_secs(1))).unwrap_or_else(|e| e.into_inner()).0;
            continue;
        }
        let r = g.reqs.remove(i);
        drop(g);
        return match r.state {
            ReqState::Approved { device_id, token } => Reply::json(
                200,
                &serde_json::json!({
                    "status": "approved", "deviceId": device_id, "token": token,
                    "hostId": inner.provider.id(), "hostName": inner.provider.name(), "addrs": inner.addrs(),
                }),
            )
            .with("Cache-Control", "no-store"),
            _ => status("rejected"),
        };
    }
}

/// `POST /pair/cancel?id=<requestId>`: el dispositivo retira su solicitud.
fn pair_cancel_reply(inner: &Inner, req: &Req) -> Reply {
    let id = req.query.get("id").cloned().unwrap_or_default();
    let mut g = lock(&inner.pairing);
    let before = g.reqs.len();
    g.reqs.retain(|r| r.secret != id);
    let removed = g.reqs.len() != before;
    drop(g);
    if removed {
        inner.pairing_cv.notify_all();
        inner.pairs_changed();
    }
    Reply::json(200, &serde_json::json!({ "ok": true }))
}

/// `POST /pair/ticket` `{ticket, name, kind, appVersion, api}`: con un billete valido da el token directamente
/// (`{deviceId, token, hostId, hostName, addrs}`) y el billete queda gastado. Si no vale, 401 y cuenta como fallo.
fn pair_ticket_reply(inner: &Inner, req: &Req, now: Instant) -> Reply {
    let v = pair_body(req);
    if body_too_old(&v) {
        return Reply::error(426, UPDATE_MSG);
    }
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let name = s("name");
    if name.is_empty() {
        return Reply::error(400, "Falta el nombre del dispositivo");
    }
    let mut g = lock(&inner.pairing);
    let good = g.ticket.as_ref().is_some_and(|t| now < t.expires && code_matches(&t.code, &s("ticket")));
    if good {
        g.ticket = None;
    }
    drop(g);
    if !good {
        inner.throttle.fail(req.ip, now);
        return Reply::error(401, "El billete no es válido o ha caducado.");
    }
    let api = v.get("api").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    match inner.provider.pair(&name, &s("kind"), &s("appVersion"), api) {
        Ok((id, token)) => Reply::json(
            200,
            &serde_json::json!({ "deviceId": id, "token": token, "hostId": inner.provider.id(), "hostName": inner.provider.name(), "addrs": inner.addrs() }),
        )
        .with("Cache-Control", "no-store"),
        Err(e) => Reply::error(400, &format!("{e:#}")),
    }
}

/// `POST /call`: `{"cmd": "...", "args": {...}}` -> resultado JSON del comando.
fn call_reply(inner: &Inner, caller: &Caller, body: &[u8]) -> Reply {
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) else {
        return Reply::error(400, "El cuerpo no es JSON");
    };
    let Some(cmd) = v.get("cmd").and_then(|c| c.as_str()).filter(|c| !c.is_empty()) else {
        return Reply::error(400, "Falta cmd");
    };
    let args = match v.get("args") {
        None | Some(serde_json::Value::Null) => serde_json::json!({}),
        Some(a) if a.is_object() => a.clone(),
        Some(_) => return Reply::error(400, "args debe ser un objeto"),
    };
    match inner.provider.call(caller, cmd, args) {
        Ok(r) => Reply::json(200, &r),
        Err(e) if e == CALL_NOT_ALLOWED => Reply::error(403, CALL_NOT_ALLOWED),
        Err(e) if e.starts_with(CALL_BAD_ARGS) => Reply::error(400, &e),
        Err(e) => Reply::error(500, &e),
    }
}

/// Resuelve `p` y comprueba que quede dentro de una de las carpetas permitidas (sin saltos con `..` ni enlaces).
pub fn resolve_asset(p: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    let real = fs::canonicalize(p).ok()?;
    if !real.is_file() {
        return None;
    }
    roots.iter().filter_map(|r| fs::canonicalize(r).ok()).any(|r| real.starts_with(&r)).then_some(real)
}

/// `GET /asset`: cualquier archivo de las carpetas de datos, con cache larga (la interfaz versiona la URL).
fn asset_reply(inner: &Inner, req: &Req) -> Reply {
    let Some(p) = req.query.get("p").filter(|p| !p.is_empty()) else {
        return Reply::error(400, "Falta el parámetro p");
    };
    let Some(real) = resolve_asset(p, &inner.provider.asset_roots()) else {
        return Reply::error(404, "Ruta desconocida");
    };
    let cache = "public, max-age=31536000, immutable";
    let Ok(meta) = fs::metadata(&real) else {
        return Reply::error(404, "El archivo ya no existe");
    };
    let mtime = meta.modified().ok().and_then(|m| m.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis()).unwrap_or(0);
    let etag = format!("\"a-{:x}-{:x}\"", meta.len(), mtime);
    if etag_matches(req.header("if-none-match"), &etag) {
        return Reply::empty(304).with("ETag", etag).with("Cache-Control", cache);
    }
    Reply { status: 200, headers: vec![("Content-Type", content_type(&real).into())], body: Body::File(real) }
        .with("Cache-Control", cache)
        .with("ETag", etag)
}

/// `GET /install/list?id=` (archivos de la carpeta del juego) y `GET /install/file?id=&f=` (uno, con `Range`).
fn install_reply(inner: &Inner, caller: &Caller, req: &Req, list: bool) -> Reply {
    let Some(id) = req.query.get("id").filter(|i| !i.is_empty()) else {
        return Reply::error(400, "Falta el id del juego");
    };
    let src = match inner.provider.install_source(caller, id) {
        Ok(s) => s,
        Err(e) => return Reply::error(409, &format!("{e:#}")),
    };
    if list {
        return match crate::gamecopy::copy_list(id, &src) {
            Ok(l) => Reply::json(200, &l).with("Cache-Control", "no-store"),
            Err(e) => Reply::error(500, &format!("{e:#}")),
        };
    }
    let Some(f) = req.query.get("f").and_then(|f| crate::gamecopy::resolve(&src.dir, f)) else {
        return Reply::error(404, "Ese archivo no es del juego");
    };
    let Ok(len) = fs::metadata(&f).map(|m| m.len()) else {
        return Reply::error(404, "El archivo ya no existe");
    };
    let base = Reply { status: 200, headers: vec![("Content-Type", "application/octet-stream".into()), ("Accept-Ranges", "bytes".into()), ("Cache-Control", "no-store".into())], body: Body::Empty };
    match req.header("range") {
        None => Reply { body: Body::File(f), ..base },
        Some(h) => match crate::gamecopy::parse_range(h, len) {
            Some((start, n)) => Reply { status: 206, body: Body::FileRange(f, start, n), ..base }
                .with("Content-Range", format!("bytes {start}-{}/{len}", start + n - 1)),
            None => Reply { status: 416, ..base }.with("Content-Range", format!("bytes */{len}")),
        },
    }
}

fn snapshot_reply(c: &Cached, req: &Req) -> Reply {
    if etag_matches(req.header("if-none-match"), &c.etag) {
        return Reply::empty(304).with("ETag", c.etag.clone()).with("Cache-Control", "no-cache").with("Vary", "Accept-Encoding");
    }
    let gz = accepts_gzip(req.header("accept-encoding"));
    let mut r = Reply {
        status: 200,
        headers: vec![
            ("Content-Type", "application/json; charset=utf-8".into()),
            ("ETag", c.etag.clone()),
            ("Cache-Control", "no-cache".into()),
            ("Vary", "Accept-Encoding".into()),
        ],
        body: Body::Bytes(if gz { c.gz.clone() } else { c.json.clone() }),
    };
    if gz {
        r = r.with("Content-Encoding", "gzip");
    }
    r
}

const CORS: [(&str, &str); 4] = [
    ("Access-Control-Allow-Origin", "*"),
    ("Access-Control-Allow-Headers", "Authorization, If-None-Match, Content-Type, X-Visor-Api, X-Visor-Version"),
    ("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
    ("Access-Control-Expose-Headers", "ETag"),
];

fn handle(inner: &Inner, mut rq: tiny_http::Request) {
    let headers = rq.headers().iter().map(|h| (h.field.as_str().as_str().to_ascii_lowercase(), h.value.as_str().to_string())).collect();
    let ip = rq.remote_addr().map(|a| a.ip()).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let mut req = Req::parse(rq.method().as_str(), rq.url(), headers, ip);
    if req.method == "POST" {
        // Cuerpo JSON (un id o los argumentos de un comando): se limita para que nadie llene la memoria.
        let max = if req.path == "/api/v1/call" && inner.caller(&req).is_some_and(|c| c.kind == "pc") { MAX_UPLOAD } else { MAX_BODY };
        let _ = std::io::Read::take(rq.as_reader(), max).read_to_end(&mut req.body);
    }
    let reply = route(inner, &req);
    let head_only = req.method == "HEAD";
    let mut resp = match reply.body {
        Body::File(p) => match fs::File::open(&p) {
            Ok(f) if !head_only => tiny_http::Response::from_file(f).with_status_code(reply.status).boxed(),
            Ok(_) => tiny_http::Response::empty(reply.status).boxed(),
            Err(_) => {
                let e = Reply::error(404, "La imagen ya no existe");
                let Body::Bytes(b) = e.body else { return };
                let mut r = tiny_http::Response::from_data(b).with_status_code(404).boxed();
                for (k, v) in CORS.iter().copied().chain(e.headers.iter().map(|(k, v)| (*k, v.as_str()))) {
                    if let Ok(h) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                        r.add_header(h);
                    }
                }
                let _ = rq.respond(r);
                return;
            }
        },
        Body::FileRange(p, start, n) if !head_only => {
            use std::io::Seek;
            match fs::File::open(&p).and_then(|mut f| f.seek(std::io::SeekFrom::Start(start)).map(|_| f)) {
                Ok(f) => tiny_http::Response::new(reply.status.into(), vec![], std::io::Read::take(f, n), Some(n as usize), None).boxed(),
                Err(_) => tiny_http::Response::empty(404).boxed(),
            }
        }
        Body::Bytes(b) if !head_only => tiny_http::Response::from_data(b).with_status_code(reply.status).boxed(),
        _ => tiny_http::Response::empty(reply.status).boxed(),
    };
    for (k, v) in CORS.iter().copied().chain(reply.headers.iter().map(|(k, v)| (*k, v.as_str()))) {
        if let Ok(h) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
            resp.add_header(h);
        }
    }
    let _ = rq.respond(resp);
}

/* ---------- servidor ---------- */

/// Servidor en marcha. Se para con [`RemoteServer::stop`] o al soltarlo.
pub struct RemoteServer {
    server: Arc<tiny_http::Server>,
    stop: Arc<AtomicBool>,
    inner: Arc<Inner>,
    addr: SocketAddr,
}

impl RemoteServer {
    pub fn start<P: Provider>(bind: SocketAddr, provider: P) -> Result<RemoteServer> {
        Self::start_arc(bind, Arc::new(provider))
    }

    pub fn start_arc(bind: SocketAddr, provider: Arc<dyn Provider>) -> Result<RemoteServer> {
        let server = tiny_http::Server::http(bind).map_err(|e| anyhow!("No se pudo abrir el puerto {}: {e}", bind.port()))?;
        let addr = server.server_addr().to_ip().unwrap_or(bind);
        let server = Arc::new(server);
        let stop = Arc::new(AtomicBool::new(false));
        let inner = Arc::new(Inner::new(provider, addr.port()));
        // Vigila las solicitudes: las que caducan sin que nadie las toque avisan a la app.
        {
            let (inner, stop) = (inner.clone(), stop.clone());
            std::thread::Builder::new()
                .name("remote-pairing".into())
                .spawn(move || {
                    while !stop.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(500));
                        let changed = lock(&inner.pairing).purge(Instant::now());
                        if changed {
                            inner.pairing_cv.notify_all();
                            inner.pairs_changed();
                        }
                    }
                })
                .context("creando el vigilante de solicitudes")?;
        }
        for i in 0..WORKERS {
            let (srv, stop, inner) = (server.clone(), stop.clone(), inner.clone());
            std::thread::Builder::new()
                .name(format!("remote-{i}"))
                .spawn(move || loop {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    match srv.recv() {
                        Ok(rq) => handle(&inner, rq),
                        Err(_) if stop.load(Ordering::SeqCst) => break,
                        Err(_) => std::thread::sleep(Duration::from_millis(20)),
                    }
                })
                .context("creando los hilos del servidor")?;
        }
        Ok(RemoteServer { server, stop, inner, addr })
    }

    /// Solicitudes de emparejamiento pendientes (las que no han caducado).
    pub fn pending_requests(&self) -> Vec<PairRequest> {
        self.inner.pending_requests_at(Instant::now())
    }

    /// Aprueba una solicitud (por su `id_public`): crea el dispositivo con `Provider::pair` y deja el token para
    /// que el dispositivo lo recoja con `GET /pair/wait`.
    pub fn approve(&self, id_public: &str) -> Result<()> {
        self.inner.approve_at(id_public, Instant::now())
    }

    /// Rechaza una solicitud (por su `id_public`).
    pub fn reject(&self, id_public: &str) -> Result<()> {
        self.inner.reject_at(id_public, Instant::now())
    }

    /// Billete de un solo uso nuevo (5 minutos); sustituye al anterior si lo habia.
    pub fn new_ticket(&self) -> Ticket {
        self.inner.new_ticket_at(Instant::now())
    }

    /// Anula el billete activo.
    pub fn cancel_ticket(&self) {
        lock(&self.inner.pairing).ticket = None;
    }

    /// Direcciones `ip:puerto` de este equipo (casa primero, Tailscale despues).
    pub fn addrs(&self) -> Vec<String> {
        self.inner.addrs()
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Deja de aceptar peticiones (las que esten en curso terminan). Se puede llamar varias veces.
    pub fn stop(&self) {
        if self.stop.swap(true, Ordering::SeqCst) {
            return;
        }
        for _ in 0..WORKERS {
            self.server.unblock();
        }
        self.inner.pairing_cv.notify_all();
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/* ---------- descubrimiento en la red local ---------- */

/// Puerto UDP en el que el visor contesta a «¿hay un visor por aqui?».
pub const DISCOVERY_PORT: u16 = 47812;
/// Mensaje que manda la tablet (difusion a toda la red local).
pub const DISCOVERY_QUERY: &[u8] = b"VISOR?";

/// Respondedor UDP: a cada `VISOR?` contesta con un JSON `{app, api, id, name, port}` (sin el codigo).
pub struct Discovery {
    stop: Arc<AtomicBool>,
    addr: SocketAddr,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Discovery {
    pub fn start(bind: SocketAddr, id: String, name: String, http_port: u16) -> Result<Discovery> {
        let sock = UdpSocket::bind(bind).map_err(|e| anyhow!("No se pudo abrir el puerto UDP {}: {e}", bind.port()))?;
        sock.set_read_timeout(Some(Duration::from_millis(200))).context("configurando el descubrimiento")?;
        let addr = sock.local_addr().unwrap_or(bind);
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let reply = serde_json::json!({ "app": "visor-capturas", "api": API_VERSION, "id": id, "name": name, "port": http_port }).to_string();
        let thread = std::thread::Builder::new()
            .name("remote-discovery".into())
            .spawn(move || {
                let mut buf = [0u8; 64];
                while !flag.load(Ordering::SeqCst) {
                    if let Ok((n, from)) = sock.recv_from(&mut buf) {
                        if buf[..n].starts_with(DISCOVERY_QUERY) {
                            let _ = sock.send_to(reply.as_bytes(), from);
                        }
                    }
                }
            })
            .context("creando el hilo de descubrimiento")?;
        Ok(Discovery { stop, addr, thread: Some(thread) })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{Platform, Shot, Title};
    use std::io::{Read, Write};
    use std::net::TcpStream;

    #[test]
    fn codes_have_the_right_shape_and_normalize() {
        let c = new_code();
        assert_eq!(c.len(), 9);
        assert_eq!(&c[4..5], "-");
        assert!(c.chars().filter(|c| *c != '-').all(|ch| CODE_ALPHABET.contains(&(ch as u8))));
        assert_ne!(new_code(), new_code());
        assert_eq!(normalize_code(" abcd-efgh "), "ABCDEFGH");
        assert!(code_matches("ABCD-EFGH", "abcdefgh"));
        assert!(code_matches("ABCD-EFGH", "ab cd - ef gh"));
        assert!(!code_matches("ABCD-EFGH", "ABCD-EFGJ"));
        assert!(!code_matches("ABCD-EFGH", "ABCD-EFG"));
        assert!(!code_matches("", ""), "sin codigo no se entra");
        assert!(!code_matches("--", ""));
    }

    #[test]
    fn throttle_blocks_after_ten_failures_for_the_minute() {
        let t = Throttle::default();
        let ip: IpAddr = "192.168.1.5".parse().unwrap();
        let other: IpAddr = "192.168.1.6".parse().unwrap();
        let t0 = Instant::now();
        for i in 0..10 {
            assert!(!t.blocked(ip, t0), "intento {i}");
            t.fail(ip, t0 + Duration::from_secs(i));
        }
        assert!(t.blocked(ip, t0 + Duration::from_secs(10)));
        assert!(t.blocked(ip, t0 + Duration::from_secs(59)));
        assert!(!t.blocked(other, t0 + Duration::from_secs(10)), "otra IP no");
        assert!(!t.blocked(ip, t0 + Duration::from_secs(60)), "acaba el minuto");
        // La ventana se reinicia con el siguiente fallo.
        t.fail(ip, t0 + Duration::from_secs(61));
        assert!(!t.blocked(ip, t0 + Duration::from_secs(62)));
    }

    #[test]
    fn etag_and_gzip_headers() {
        assert!(etag_matches(Some("\"abc\""), "\"abc\""));
        assert!(etag_matches(Some("\"x\", W/\"abc\""), "\"abc\""));
        assert!(etag_matches(Some("*"), "\"abc\""));
        assert!(!etag_matches(Some("\"abd\""), "\"abc\""));
        assert!(!etag_matches(None, "\"abc\""));
        assert!(accepts_gzip(Some("gzip, deflate, br")));
        assert!(accepts_gzip(Some("deflate, GZIP;q=0.5")));
        assert!(!accepts_gzip(Some("gzip;q=0, deflate")));
        assert!(!accepts_gzip(Some("identity")));
        assert!(!accepts_gzip(None));
    }

    #[test]
    fn discovery_answers_the_query_and_ignores_the_rest() {
        let d = Discovery::start("127.0.0.1:0".parse().unwrap(), "abc123".into(), "MiPC".into(), 47811).unwrap();
        let c = UdpSocket::bind("127.0.0.1:0").unwrap();
        c.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        c.send_to(b"hola", d.local_addr()).unwrap();
        c.send_to(DISCOVERY_QUERY, d.local_addr()).unwrap();
        let mut buf = [0u8; 512];
        let (n, _) = c.recv_from(&mut buf).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf[..n]).unwrap();
        assert_eq!((v["app"].as_str(), v["id"].as_str(), v["name"].as_str(), v["port"].as_u64()), (Some("visor-capturas"), Some("abc123"), Some("MiPC"), Some(47811)));
        assert!(v.get("code").is_none());
        drop(d);
    }

    #[test]
    fn ids_are_hex_and_unique() {
        let (a, b) = (new_id(), new_id());
        assert_eq!(a.len(), 32);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn url_decoding() {
        assert_eq!(percent_decode("C%3A%5CCapturas%5CJuego%20A%5C1.png"), "C:\\Capturas\\Juego A\\1.png");
        assert_eq!(percent_decode("a+b%"), "a b%");
        assert_eq!(percent_decode("%E3%81%82"), "あ");
        assert_eq!(percent_decode("%zz%4"), "%zz%4");
        let r = Req::parse("get", "/api/v1/thumb?p=D%3A%5Cx.png&k=ab-cd&&fresh", vec![], IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!((r.method.as_str(), r.path.as_str()), ("GET", "/api/v1/thumb"));
        assert_eq!(r.query.get("p").map(String::as_str), Some("D:\\x.png"));
        assert_eq!(r.query.get("k").map(String::as_str), Some("ab-cd"));
        assert_eq!(r.query.get("fresh").map(String::as_str), Some(""));
    }

    fn title(name: &str, shots: &[&str], cover: Option<&str>, hero: Option<&str>, logo: Option<&str>) -> Title {
        let mut t: Title = serde_json::from_value(serde_json::json!({ "name": name, "cover": null, "shots": [] })).unwrap();
        t.shots = shots.iter().map(|p| Shot { path: p.to_string(), mtime: 0, date: 0, width: 1, height: 1, size: 1 }).collect();
        t.cover = cover.map(Into::into);
        t.hero = hero.map(Into::into);
        t.logo = logo.map(Into::into);
        t
    }

    fn lib_with(titles: Vec<Title>) -> Library {
        Library { platforms: vec![Platform { id: "pc".into(), name: "PC".into(), folder: "PC".into(), accent: 4, icon: "pc".into(), titles }], hidden: vec![] }
    }

    #[test]
    fn whitelist_has_shots_and_art_only() {
        let lib = lib_with(vec![
            title("A", &["C:\\c\\a1.png", "C:\\c\\a2.png"], Some("C:\\c\\cover.jpg"), None, Some("C:\\c\\logo.png")),
            title("B", &[], None, Some("C:\\s\\hero.jpg"), None),
        ]);
        let set = allowed_paths(&lib);
        let mut v: Vec<_> = set.iter().map(String::as_str).collect();
        v.sort();
        assert_eq!(v, ["C:\\c\\a1.png", "C:\\c\\a2.png", "C:\\c\\cover.jpg", "C:\\c\\logo.png", "C:\\s\\hero.jpg"]);
    }

    /* --- servidor de verdad en 127.0.0.1 con un proveedor de prueba --- */

    struct Fake {
        lib: Library,
        dir: PathBuf,
        scans: Arc<AtomicU64>,
    }
    impl Provider for Fake {
        fn name(&self) -> String {
            "PC-PRUEBA".into()
        }
        fn snapshot(&self) -> Result<Snapshot> {
            self.scans.fetch_add(1, Ordering::SeqCst);
            Ok(Snapshot {
                config: RemoteConfig { goal_vn: 60, goal_jvn: 30, goal_pc: 60, goal_switch: 60, accent_mode: "art".into(), accent_fixed: 4 },
                library: self.lib.clone(),
                favorites: vec![],
                sagas: vec![Saga { id: "s1".into(), name: "Saga".into(), members: vec!["pc/A".into()], platform: None, color: None }],
                playing: None,
            })
        }
        fn thumbs_dir(&self) -> PathBuf {
            self.dir.join("thumbs")
        }
        fn previews_dir(&self) -> PathBuf {
            self.dir.join("previews")
        }
        fn device_by_token(&self, token: &str) -> Option<Caller> {
            (token == "ABCD-EFGH").then(|| Caller { device_id: "fake".into(), name: "Fake".into(), kind: "tablet".into() })
        }
        fn launch(&self, _caller: &Caller, id: &str) -> Result<()> {
            anyhow::ensure!(id == "pc/A", "Juego no encontrado");
            Ok(())
        }
        fn capture(&self) -> Result<()> {
            Ok(())
        }
        fn close(&self, force: bool) -> Result<serde_json::Value> {
            Ok(serde_json::json!({ "closed": if force { "forced" } else { "asked" } }))
        }
        fn session(&self) -> serde_json::Value {
            serde_json::json!({ "state": "idle", "game": null, "streamed": false, "last": null })
        }
        fn session_start(&self, _w: u32, _h: u32, _fps: u32) -> Result<serde_json::Value> {
            anyhow::bail!("No hay juego pendiente")
        }
        fn call(&self, caller: &Caller, cmd: &str, args: serde_json::Value) -> std::result::Result<serde_json::Value, String> {
            match cmd {
                "ping" => Ok(serde_json::json!({ "pong": args["n"] })),
                "whoami" => Ok(serde_json::json!({ "device": caller.device_id })),
                "boom" => Err("fallo".into()),
                _ => Err(CALL_NOT_ALLOWED.into()),
            }
        }
    }

    struct Resp {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }
    impl Resp {
        fn h(&self, k: &str) -> Option<&str> {
            self.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(k)).map(|(_, v)| v.as_str())
        }
    }

    fn get(addr: SocketAddr, method: &str, url: &str, headers: &[(&str, &str)]) -> Resp {
        let mut s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        let mut req = format!("{method} {url} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n");
        for (k, v) in headers {
            req += &format!("{k}: {v}\r\n");
        }
        req += "\r\n";
        s.write_all(req.as_bytes()).unwrap();
        let mut raw = Vec::new();
        s.read_to_end(&mut raw).unwrap();
        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&raw[..split]).into_owned();
        let mut lines = head.lines();
        let status = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
        let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect();
        let mut body = raw[split + 4..].to_vec();
        if headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("transfer-encoding") && v.contains("chunked")) {
            body = dechunk(&body);
        }
        Resp { status, headers, body }
    }

    fn dechunk(b: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 0;
        loop {
            let end = i + b[i..].windows(2).position(|w| w == b"\r\n").unwrap();
            let n = usize::from_str_radix(std::str::from_utf8(&b[i..end]).unwrap().trim(), 16).unwrap();
            if n == 0 {
                return out;
            }
            out.extend_from_slice(&b[end + 2..end + 2 + n]);
            i = end + 2 + n + 2;
        }
    }

    /// POST con cuerpo JSON (la funcion `get` de arriba no manda cuerpo).
    fn post(addr: SocketAddr, url: &str, token: Option<&str>, body: &str) -> (u16, String) {
        let mut s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        let auth = token.map(|t| format!("Authorization: Bearer {t}\r\n")).unwrap_or_default();
        let req = format!("POST {url} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len());
        s.write_all(req.as_bytes()).unwrap();
        let mut raw = Vec::new();
        s.read_to_end(&mut raw).unwrap();
        let text = String::from_utf8_lossy(&raw).into_owned();
        let status = text.split(' ').nth(1).unwrap().parse().unwrap();
        (status, text)
    }

    #[test]
    fn event_bus_buffer_and_poll() {
        let bus = EventBus::new();
        assert_eq!(bus.poll(0, Duration::from_millis(10)), (0, vec![]));
        for i in 0..(EVENT_BUFFER + 5) {
            bus.push("x", serde_json::json!(i));
        }
        let cur = bus.current();
        assert_eq!(cur as usize, EVENT_BUFFER + 5);
        // since == 0 solo da la secuencia; since desfasado (servidor reiniciado) tambien
        assert_eq!(bus.poll(0, Duration::from_millis(10)), (cur, vec![]));
        assert_eq!(bus.poll(cur + 100, Duration::from_millis(10)), (cur, vec![]));
        // el buffer descarta los mas viejos
        let (_, all) = bus.poll(1, Duration::from_millis(10));
        assert_eq!(all.len(), EVENT_BUFFER);
        assert_eq!(all.last().unwrap().seq, cur);
        // sin novedades espera y devuelve vacio
        let t = Instant::now();
        assert_eq!(bus.poll(cur, Duration::from_millis(80)).1.len(), 0);
        assert!(t.elapsed() >= Duration::from_millis(70));
    }

    #[test]
    fn event_bus_wakes_a_waiting_poll() {
        let bus = Arc::new(EventBus::new());
        bus.push("a", serde_json::json!(1));
        let b2 = bus.clone();
        let h = std::thread::spawn(move || b2.poll(1, Duration::from_secs(10)));
        std::thread::sleep(Duration::from_millis(100));
        bus.push("b", serde_json::json!({"k": 2}));
        let (seq, ev) = h.join().unwrap();
        assert_eq!(seq, 2);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].name, "b");
    }

    #[test]
    fn asset_paths_stay_inside_roots() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("root");
        let other = d.path().join("other");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&other).unwrap();
        fs::write(root.join("a.png"), b"x").unwrap();
        fs::write(other.join("secret.txt"), b"x").unwrap();
        let roots = vec![root.clone()];
        assert!(resolve_asset(root.join("a.png").to_str().unwrap(), &roots).is_some());
        assert!(resolve_asset(other.join("secret.txt").to_str().unwrap(), &roots).is_none());
        // salto con ..
        let sneaky = root.join("..").join("other").join("secret.txt");
        assert!(resolve_asset(sneaky.to_str().unwrap(), &roots).is_none());
        // una carpeta no es un archivo; una ruta que no existe tampoco
        assert!(resolve_asset(root.to_str().unwrap(), &roots).is_none());
        assert!(resolve_asset(root.join("no.png").to_str().unwrap(), &roots).is_none());
    }

    #[test]
    fn call_events_asset_over_http() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("thumbs")).unwrap();
        fs::write(d.path().join("thumbs").join("t.png"), b"png!").unwrap();
        let fake = Fake { lib: lib_with(vec![]), dir: d.path().to_path_buf(), scans: Arc::new(AtomicU64::new(0)) };
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), fake).unwrap();
        let a = srv.local_addr();
        let auth = [("Authorization", "Bearer ABCD-EFGH")];
        // call: sin codigo 401; cuerpo roto 400; comando del proveedor de prueba 200; el resto 403 / 500
        assert_eq!(post(a, "/api/v1/call", None, "{\"cmd\":\"ping\"}").0, 401);
        assert_eq!(post(a, "/api/v1/call", Some("ABCD-EFGH"), "no es json").0, 400);
        assert_eq!(post(a, "/api/v1/call", Some("ABCD-EFGH"), "{}").0, 400);
        let (st, text) = post(a, "/api/v1/call", Some("ABCD-EFGH"), "{\"cmd\":\"ping\",\"args\":{\"n\":2}}");
        assert_eq!(st, 200);
        assert!(text.contains("{\"pong\":2}"), "{text}");
        assert_eq!(post(a, "/api/v1/call", Some("ABCD-EFGH"), "{\"cmd\":\"exit_app\"}").0, 403);
        assert_eq!(post(a, "/api/v1/call", Some("ABCD-EFGH"), "{\"cmd\":\"boom\"}").0, 500);
        assert_eq!(get(a, "GET", "/api/v1/call", &auth).status, 405);
        // events: since ausente = secuencia vigente sin eventos; con novedades las devuelve
        event_bus().push("test-evento", serde_json::json!({"v": 1}));
        let r = get(a, "GET", "/api/v1/events", &auth);
        assert_eq!(r.status, 200);
        let cur = serde_json::from_slice::<serde_json::Value>(&r.body).unwrap()["seq"].as_u64().unwrap();
        event_bus().push("test-evento", serde_json::json!({"v": 2}));
        let r = get(a, "GET", &format!("/api/v1/events?since={cur}&wait=5"), &auth);
        let v = serde_json::from_slice::<serde_json::Value>(&r.body).unwrap();
        assert_eq!(v["events"][0]["payload"]["v"], 2);
        // asset: dentro de las carpetas 200 con cache inmutable y ETag; fuera 404
        let p = d.path().join("thumbs").join("t.png");
        let url = format!("/api/v1/asset?p={}&k=ABCD-EFGH", p.to_string_lossy().replace('\\', "/"));
        let r = get(a, "GET", &url, &[]);
        assert_eq!(r.status, 200);
        assert_eq!(r.h("content-type"), Some("image/png"));
        assert!(r.h("cache-control").unwrap().contains("immutable"));
        let etag = r.h("etag").unwrap().to_string();
        assert_eq!(get(a, "GET", &url, &[("If-None-Match", &etag)]).status, 304);
        let out = format!("/api/v1/asset?p={}&k=ABCD-EFGH", d.path().join("thumbs").join("..").join("x.png").to_string_lossy().replace('\\', "/"));
        assert_eq!(get(a, "GET", &out, &[]).status, 404);
        assert_eq!(get(a, "GET", "/api/v1/asset?p=x", &[]).status, 401);
    }

    /// Proveedor con dispositivos en memoria: `(token, dispositivo, quitado)`.
    struct Paired {
        dir: PathBuf,
        devs: Arc<Mutex<Vec<(String, Caller, bool)>>>,
        seen: Arc<Mutex<Vec<(String, String, u32)>>>,
        /// Avisos recibidos (`requested:<nombre>`, `changed`, `stream:<id>:<pin>`).
        notes: Arc<Mutex<Vec<String>>>,
    }
    impl Provider for Paired {
        fn name(&self) -> String {
            "HOST".into()
        }
        fn id(&self) -> String {
            "host-1".into()
        }
        fn snapshot(&self) -> Result<Snapshot> {
            let config = RemoteConfig { goal_vn: 1, goal_jvn: 1, goal_pc: 1, goal_switch: 1, accent_mode: "art".into(), accent_fixed: 0 };
            Ok(Snapshot { config, library: lib_with(vec![]), favorites: vec![], sagas: vec![], playing: None })
        }
        fn thumbs_dir(&self) -> PathBuf {
            self.dir.join("thumbs")
        }
        fn previews_dir(&self) -> PathBuf {
            self.dir.join("previews")
        }
        fn device_by_token(&self, token: &str) -> Option<Caller> {
            self.devs.lock().unwrap().iter().find(|(t, _, rev)| t == token && !rev).map(|(_, c, _)| c.clone())
        }
        fn pair(&self, name: &str, kind: &str, _version: &str, _api: u32) -> Result<(String, String)> {
            let mut d = self.devs.lock().unwrap();
            let id = format!("dev{}", d.len() + 1);
            let token = format!("tok{}", d.len() + 1);
            d.push((token.clone(), Caller { device_id: id.clone(), name: name.into(), kind: kind.into() }, false));
            Ok((id, token))
        }
        fn unpair(&self, caller: &Caller) -> Result<()> {
            for d in self.devs.lock().unwrap().iter_mut().filter(|d| d.1.device_id == caller.device_id) {
                d.2 = true;
            }
            Ok(())
        }
        fn device_seen(&self, caller: &Caller, version: &str, api: u32) {
            self.seen.lock().unwrap().push((caller.device_id.clone(), version.into(), api));
        }
        fn pair_requested(&self, req: &PairRequest) {
            self.notes.lock().unwrap().push(format!("requested:{}", req.name));
        }
        fn pair_requests_changed(&self) {
            self.notes.lock().unwrap().push("changed".into());
        }
        fn stream_pair(&self, caller: &Caller, pin: Option<&str>) -> Result<serde_json::Value> {
            self.notes.lock().unwrap().push(format!("stream:{}:{}", caller.device_id, pin.unwrap_or("-")));
            Ok(match pin {
                Some(_) => serde_json::json!({ "ok": true }),
                None => serde_json::json!({ "pin": "4321", "passphrase": "abc", "port": 47989 }),
            })
        }
        fn call(&self, caller: &Caller, cmd: &str, _args: serde_json::Value) -> std::result::Result<serde_json::Value, String> {
            match cmd {
                "whoami" => Ok(serde_json::json!({ "device": caller.device_id, "name": caller.name })),
                _ => Err(CALL_NOT_ALLOWED.into()),
            }
        }
        fn install_source(&self, caller: &Caller, id: &str) -> Result<crate::gamecopy::InstallSource> {
            anyhow::ensure!(caller.kind == "pc" && id == "g1", "no");
            Ok(crate::gamecopy::InstallSource { dir: self.dir.join("Juego"), exe: "game.exe".into(), args: String::new(), name: "Juego".into(), platform: "pc".into() })
        }
    }

    #[test]
    fn copy_game_with_resume() {
        use std::sync::atomic::AtomicBool;
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("Juego");
        fs::create_dir_all(g.join("data")).unwrap();
        let big: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        fs::write(g.join("game.exe"), b"MZ-exe").unwrap();
        fs::write(g.join("data/big.pak"), &big).unwrap();
        fs::write(g.join("data/empty.txt"), b"").unwrap();
        fs::write(d.path().join("secreto.txt"), b"no").unwrap();
        let devs = vec![
            ("tokpc".to_string(), Caller { device_id: "pc1".into(), name: "Portátil".into(), kind: "pc".into() }, false),
            ("toktab".to_string(), Caller { device_id: "tab1".into(), name: "Tablet".into(), kind: "tablet".into() }, false),
        ];
        let p = Paired { dir: d.path().to_path_buf(), devs: Arc::new(Mutex::new(devs)), seen: Arc::new(Mutex::new(vec![])), notes: Default::default() };
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), p).unwrap();
        let a = srv.local_addr();
        let addr = a.to_string();
        // sin token, otro tipo de dispositivo o fuera de la carpeta: no
        assert_eq!(get(a, "GET", "/api/v1/install/list?id=g1", &[]).status, 401);
        assert_eq!(get(a, "GET", "/api/v1/install/list?id=g1", &[("Authorization", "Bearer toktab")]).status, 409);
        assert_eq!(get(a, "GET", "/api/v1/install/file?id=g1&f=..%2Fsecreto.txt", &[("Authorization", "Bearer tokpc")]).status, 404);
        // Range: 206 con el trozo pedido; fuera del archivo, 416
        let r = get(a, "GET", "/api/v1/install/file?id=g1&f=data%2Fbig.pak", &[("Authorization", "Bearer tokpc"), ("Range", "bytes=10-19")]);
        assert_eq!(r.status, 206);
        assert_eq!(r.body, big[10..20]);
        assert_eq!(get(a, "GET", "/api/v1/install/file?id=g1&f=game.exe", &[("Authorization", "Bearer tokpc"), ("Range", "bytes=99-")]).status, 416);
        // copia entera, reanudando un archivo a medias
        let list = crate::gamecopy::fetch_list(&addr, "tokpc", "1.0", "g1").unwrap();
        assert_eq!((list.folder.as_str(), list.exe.as_str(), list.files.len(), list.total), ("Juego", "game.exe", 3, big.len() as u64 + 6));
        let dest = d.path().join("copia");
        fs::create_dir_all(dest.join("data")).unwrap();
        fs::write(dest.join("data/big.pak.visor-part"), &big[..123_456]).unwrap();
        let mut last = crate::gamecopy::Progress::default();
        crate::gamecopy::download(&addr, "tokpc", "1.0", &list, &dest, &AtomicBool::new(false), |p| last = p).unwrap();
        assert_eq!(fs::read(dest.join("data/big.pak")).unwrap(), big);
        assert_eq!(fs::read(dest.join("game.exe")).unwrap(), b"MZ-exe");
        assert!(dest.join("data/empty.txt").is_file());
        assert!(!dest.join("data/big.pak.visor-part").exists());
        assert_eq!((last.done, last.files_done), (list.total, 3));
        // otra vez: no descarga nada (ya esta todo)
        crate::gamecopy::download(&addr, "tokpc", "1.0", &list, &dest, &AtomicBool::new(false), |_| ()).unwrap();
    }

    fn paired_in(d: &Path) -> Paired {
        Paired { dir: d.to_path_buf(), devs: Default::default(), seen: Default::default(), notes: Default::default() }
    }

    /// Cuerpo JSON de la respuesta de `post` (texto HTTP entero).
    fn body_of(text: &str) -> serde_json::Value {
        serde_json::from_str(&text[text.find("\r\n\r\n").unwrap() + 4..]).unwrap()
    }

    fn json_of(r: &Resp) -> serde_json::Value {
        serde_json::from_slice(&r.body).unwrap()
    }

    /// Peticion armada a mano para `route_at` (sin red) con el reloj que se quiera.
    fn rq(method: &str, url: &str, ip: &str, body: &str) -> Req {
        let mut r = Req::parse(method, url, vec![], ip.parse().unwrap());
        r.body = body.as_bytes().to_vec();
        r
    }

    fn rj(r: &Reply) -> serde_json::Value {
        match &r.body {
            Body::Bytes(b) => serde_json::from_slice(b).unwrap_or(serde_json::Value::Null),
            _ => serde_json::Value::Null,
        }
    }

    const PAIR_BODY: &str = r#"{"name":"Tablet","kind":"tablet","appVersion":"2.0.3","api":3}"#;

    fn named(n: &str) -> String {
        format!(r#"{{"name":"{n}","kind":"tablet","appVersion":"2.0.3","api":3}}"#)
    }

    fn ticket_body(code: &str, api: u32) -> String {
        format!(r#"{{"ticket":"{code}","name":"Portátil","kind":"pc","appVersion":"2.0.3","api":{api}}}"#)
    }

    #[test]
    fn pair2_request_approve_and_wait_once() {
        let d = tempfile::tempdir().unwrap();
        let p = paired_in(d.path());
        let notes = p.notes.clone();
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), p).unwrap();
        let a = srv.local_addr();
        let hello = json_of(&get(a, "GET", "/api/v1/hello", &[]));
        assert_eq!((hello["apiRev"].as_u64(), hello["apiMin"].as_u64()), (Some(3), Some(3)));
        let feats: Vec<&str> = hello["features"].as_array().unwrap().iter().filter_map(|f| f.as_str()).collect();
        assert!(feats.contains(&"pair2") && !feats.contains(&"pair"), "{feats:?}");
        let addrs = hello["addrs"].as_array().expect("hello lleva addrs");
        assert!(addrs.iter().all(|x| x.as_str().unwrap().ends_with(&format!(":{}", a.port()))));

        let (st, text) = post(a, "/api/v1/pair/request", None, PAIR_BODY);
        assert_eq!(st, 200, "{text}");
        let b = body_of(&text);
        let rid = b["requestId"].as_str().unwrap().to_string();
        let digits = b["digits"].as_str().unwrap().to_string();
        assert_eq!(rid.len(), 32);
        assert!(digits.len() == 4 && digits.bytes().all(|c| c.is_ascii_digit()), "{digits}");
        assert_eq!(b["expiresMs"], 120_000);

        // sin resolver: pending
        let w = json_of(&get(a, "GET", &format!("/api/v1/pair/wait?id={rid}&wait=0"), &[]));
        assert_eq!(w["status"], "pending");
        let pend = srv.pending_requests();
        assert_eq!(pend.len(), 1);
        assert_eq!((pend[0].name.as_str(), pend[0].kind.as_str(), pend[0].ip.as_str()), ("Tablet", "tablet", "127.0.0.1"));
        assert_eq!(pend[0].digits, digits);
        assert_ne!(pend[0].id_public, rid, "el id publico no es el secreto");
        assert!(pend[0].expires_ms > now_ms());
        assert!(notes.lock().unwrap().contains(&"requested:Tablet".to_string()));
        assert!(notes.lock().unwrap().contains(&"changed".to_string()));

        // un wait largo se despierta con la aprobacion
        let rid2 = rid.clone();
        let h = std::thread::spawn(move || get(a, "GET", &format!("/api/v1/pair/wait?id={rid2}&wait=10"), &[]));
        std::thread::sleep(Duration::from_millis(150));
        srv.approve(&pend[0].id_public).unwrap();
        let w = json_of(&h.join().unwrap());
        assert_eq!(w["status"], "approved");
        assert_eq!((w["deviceId"].as_str(), w["token"].as_str(), w["hostId"].as_str(), w["hostName"].as_str()), (Some("dev1"), Some("tok1"), Some("host-1"), Some("HOST")));
        assert!(w["addrs"].is_array());
        // el token se entrega una sola vez
        assert_eq!(json_of(&get(a, "GET", &format!("/api/v1/pair/wait?id={rid}&wait=0"), &[]))["status"], "expired");
        assert!(srv.pending_requests().is_empty());
        let (st, text) = post(a, "/api/v1/call", Some("tok1"), "{\"cmd\":\"whoami\"}");
        assert_eq!(st, 200);
        assert!(text.contains("\"device\":\"dev1\""), "{text}");
        assert!(srv.approve("nada").is_err());
        assert_eq!(get(a, "GET", "/api/v1/pair/wait?wait=0", &[]).status, 400);
        assert_eq!(get(a, "GET", "/api/v1/pair/request", &[]).status, 405);
    }

    #[test]
    fn pair2_reject_and_cancel() {
        let d = tempfile::tempdir().unwrap();
        let p = paired_in(d.path());
        let devs = p.devs.clone();
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), p).unwrap();
        let a = srv.local_addr();
        let rid = body_of(&post(a, "/api/v1/pair/request", None, PAIR_BODY).1)["requestId"].as_str().unwrap().to_string();
        let id_public = srv.pending_requests()[0].id_public.clone();
        srv.reject(&id_public).unwrap();
        assert!(srv.reject(&id_public).is_err(), "ya resuelta");
        assert_eq!(json_of(&get(a, "GET", &format!("/api/v1/pair/wait?id={rid}&wait=0"), &[]))["status"], "rejected");
        assert_eq!(json_of(&get(a, "GET", &format!("/api/v1/pair/wait?id={rid}&wait=0"), &[]))["status"], "expired");
        assert!(devs.lock().unwrap().is_empty(), "rechazar no crea dispositivo");
        // cancelar
        let rid = body_of(&post(a, "/api/v1/pair/request", None, PAIR_BODY).1)["requestId"].as_str().unwrap().to_string();
        assert_eq!(srv.pending_requests().len(), 1);
        assert_eq!(post(a, &format!("/api/v1/pair/cancel?id={rid}"), None, "").0, 200);
        assert!(srv.pending_requests().is_empty());
        assert_eq!(json_of(&get(a, "GET", &format!("/api/v1/pair/wait?id={rid}&wait=0"), &[]))["status"], "expired");
    }

    #[test]
    fn pair2_request_expires_with_injected_clock() {
        let d = tempfile::tempdir().unwrap();
        let p = paired_in(d.path());
        let notes = p.notes.clone();
        let inner = Inner::new(Arc::new(p), 47811);
        let t0 = Instant::now();
        let ip = "192.168.1.20";
        let r = route_at(&inner, &rq("POST", "/api/v1/pair/request", ip, PAIR_BODY), t0);
        assert_eq!(r.status, 200);
        let rid = rj(&r)["requestId"].as_str().unwrap().to_string();
        let wait = |t: Instant| rj(&route_at(&inner, &rq("GET", &format!("/api/v1/pair/wait?id={rid}&wait=0"), ip, ""), t))["status"].as_str().unwrap().to_string();
        assert_eq!(wait(t0 + Duration::from_secs(119)), "pending");
        assert_eq!(inner.pending_requests_at(t0 + Duration::from_secs(119)).len(), 1);
        let changes = notes.lock().unwrap().iter().filter(|n| *n == "changed").count();
        assert_eq!(wait(t0 + Duration::from_secs(121)), "expired");
        assert!(notes.lock().unwrap().iter().filter(|n| *n == "changed").count() > changes, "caducar avisa");
        assert!(inner.pending_requests_at(t0 + Duration::from_secs(121)).is_empty());
        // una caducada tampoco se puede aprobar
        route_at(&inner, &rq("POST", "/api/v1/pair/request", ip, PAIR_BODY), t0);
        let id_public = inner.pending_requests_at(t0)[0].id_public.clone();
        assert!(inner.approve_at(&id_public, t0 + Duration::from_secs(121)).is_err());
        assert!(inner.pending_requests_at(t0 + Duration::from_secs(121)).is_empty());
    }

    #[test]
    fn pair2_fourth_pending_is_429() {
        let d = tempfile::tempdir().unwrap();
        let inner = Inner::new(Arc::new(paired_in(d.path())), 47811);
        let t0 = Instant::now();
        let ip = "192.168.1.20";
        for n in ["A", "B", "C"] {
            assert_eq!(route_at(&inner, &rq("POST", "/api/v1/pair/request", ip, &named(n)), t0).status, 200, "{n}");
        }
        assert_eq!(route_at(&inner, &rq("POST", "/api/v1/pair/request", ip, &named("D")), t0).status, 429);
        // repetir la misma solicitud (misma IP y nombre) la sustituye y no ocupa otro hueco
        assert_eq!(route_at(&inner, &rq("POST", "/api/v1/pair/request", ip, &named("A")), t0).status, 200);
        assert_eq!(inner.pending_requests_at(t0).len(), 3);
        // al caducar se liberan los huecos
        assert_eq!(route_at(&inner, &rq("POST", "/api/v1/pair/request", ip, &named("D")), t0 + Duration::from_secs(121)).status, 200);
        // sin nombre: 400
        assert_eq!(route_at(&inner, &rq("POST", "/api/v1/pair/request", ip, r#"{"name":" ","api":3}"#), t0).status, 400);
    }

    #[test]
    fn pair2_ticket_valid_used_twice_expired_and_wrong() {
        let d = tempfile::tempdir().unwrap();
        let inner = Inner::new(Arc::new(paired_in(d.path())), 47811);
        let t0 = Instant::now();
        let ip = "192.168.1.30";
        let use_ticket = |code: &str, api: u32, t: Instant| route_at(&inner, &rq("POST", "/api/v1/pair/ticket", ip, &ticket_body(code, api)), t);
        // sin billete activo
        assert_eq!(use_ticket("ABCD-EFGH", 3, t0).status, 401);
        let tk = inner.new_ticket_at(t0);
        assert_eq!(tk.code.len(), 9);
        assert!(tk.expires_ms > now_ms());
        assert_eq!(use_ticket("ZZZZ-ZZZZ", 3, t0).status, 401, "billete incorrecto");
        assert_eq!(use_ticket(&tk.code, 2, t0).status, 426, "api vieja, y no se gasta");
        // valido (se acepta en minusculas y sin guion)
        let r = use_ticket(&tk.code.to_lowercase().replace('-', ""), 3, t0 + Duration::from_secs(10));
        assert_eq!(r.status, 200);
        let v = rj(&r);
        assert_eq!((v["deviceId"].as_str(), v["token"].as_str(), v["hostId"].as_str()), (Some("dev1"), Some("tok1"), Some("host-1")));
        assert!(v["addrs"].is_array());
        // gastado
        assert_eq!(use_ticket(&tk.code, 3, t0 + Duration::from_secs(11)).status, 401);
        // caducado a los 5 minutos
        let tk = inner.new_ticket_at(t0);
        assert_eq!(use_ticket(&tk.code, 3, t0 + Duration::from_secs(301)).status, 401);
        // uno nuevo sustituye al anterior
        let (t1, t2) = (inner.new_ticket_at(t0), inner.new_ticket_at(t0));
        assert_eq!(use_ticket(&t1.code, 3, t0).status, 401);
        assert_eq!(use_ticket(&t2.code, 3, t0).status, 200);
        // sin nombre: 400
        let t3 = inner.new_ticket_at(t0);
        let body = format!(r#"{{"ticket":"{}","name":"","api":3}}"#, t3.code);
        assert_eq!(route_at(&inner, &rq("POST", "/api/v1/pair/ticket", ip, &body), t0).status, 400);
    }

    #[test]
    fn pair2_bad_tickets_are_throttled() {
        let d = tempfile::tempdir().unwrap();
        let inner = Inner::new(Arc::new(paired_in(d.path())), 47811);
        let t0 = Instant::now();
        let tk = inner.new_ticket_at(t0);
        for _ in 0..10 {
            assert_eq!(route_at(&inner, &rq("POST", "/api/v1/pair/ticket", "192.168.1.40", &ticket_body("ZZZZ-ZZZZ", 3)), t0).status, 401);
        }
        assert_eq!(route_at(&inner, &rq("POST", "/api/v1/pair/ticket", "192.168.1.40", &ticket_body(&tk.code, 3)), t0).status, 429);
        assert_eq!(route_at(&inner, &rq("POST", "/api/v1/pair/ticket", "192.168.1.41", &ticket_body(&tk.code, 3)), t0).status, 200, "otra IP no");
    }

    #[test]
    fn pair_token_revoke_and_old_clients() {
        let d = tempfile::tempdir().unwrap();
        let p = paired_in(d.path());
        let seen = p.seen.clone();
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), p).unwrap();
        let a = srv.local_addr();
        let tk = srv.new_ticket();
        let (st, text) = post(a, "/api/v1/pair/ticket", None, &ticket_body(&tk.code, 3));
        assert_eq!(st, 200, "{text}");
        let body = body_of(&text);
        assert_eq!((body["deviceId"].as_str(), body["token"].as_str(), body["hostId"].as_str()), (Some("dev1"), Some("tok1"), Some("host-1")));
        // el token entra y el proveedor sabe quien llama; tambien apunta la version
        let (st, text) = post(a, "/api/v1/call", Some("tok1"), "{\"cmd\":\"whoami\"}");
        assert_eq!(st, 200);
        assert!(text.contains("\"device\":\"dev1\""), "{text}");
        let r = get(a, "GET", "/api/v1/snapshot", &[("Authorization", "Bearer tok1"), ("X-Visor-Version", "2.0.3"), ("X-Visor-Api", "3")]);
        assert_eq!(r.status, 200);
        assert!(seen.lock().unwrap().contains(&("dev1".into(), "2.0.3".into(), 3)));
        // sin token, token falso o el viejo codigo: 401
        assert_eq!(get(a, "GET", "/api/v1/snapshot", &[]).status, 401);
        assert_eq!(get(a, "GET", "/api/v1/snapshot", &[("Authorization", "Bearer nada")]).status, 401);
        assert_eq!(post(a, "/api/v1/call", Some("ABCD-EFGH"), "{\"cmd\":\"whoami\"}").0, 401);
        // api vieja: 426 (con token valido o sin el), y el /pair de antes tambien
        for api in ["0", "2"] {
            assert_eq!(get(a, "GET", "/api/v1/snapshot", &[("Authorization", "Bearer tok1"), ("X-Visor-Api", api)]).status, 426);
        }
        let (st, text) = post(a, "/api/v1/pair", None, "{\"code\":\"ABCD-EFGH\",\"name\":\"Portátil\",\"api\":2}");
        assert_eq!(st, 426);
        assert!(text.contains(UPDATE_MSG), "{text}");
        assert_eq!(get(a, "GET", "/api/v1/pair", &[]).status, 426);
        assert_eq!(post(a, "/api/v1/pair/request", None, r#"{"name":"X","api":2}"#).0, 426);
        assert_eq!(post(a, "/api/v1/pair/request", None, r#"{"name":"X"}"#).0, 426);
        assert_eq!(post(a, "/api/v1/pair/ticket", None, &ticket_body("ABCD-EFGH", 2)).0, 426);
        // un billete anulado ya no vale
        let tk = srv.new_ticket();
        srv.cancel_ticket();
        assert_eq!(post(a, "/api/v1/pair/ticket", None, &ticket_body(&tk.code, 3)).0, 401);
        // quitarse: el token deja de valer
        assert_eq!(post(a, "/api/v1/unpair", Some("tok1"), "").0, 200);
        assert_eq!(post(a, "/api/v1/call", Some("tok1"), "{\"cmd\":\"whoami\"}").0, 401);
        assert_eq!(get(a, "GET", "/api/v1/snapshot", &[("Authorization", "Bearer tok1")]).status, 401);
    }

    #[test]
    fn stream_pair_goes_to_the_provider() {
        let d = tempfile::tempdir().unwrap();
        let mut p = paired_in(d.path());
        p.devs = Arc::new(Mutex::new(vec![("tokx".to_string(), Caller { device_id: "tab1".into(), name: "Tablet".into(), kind: "tablet".into() }, false)]));
        let notes = p.notes.clone();
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), p).unwrap();
        let a = srv.local_addr();
        assert_eq!(post(a, "/api/v1/stream/pair", None, "{\"pin\":\"1234\"}").0, 401);
        let (st, text) = post(a, "/api/v1/stream/pair", Some("tokx"), "{}");
        assert_eq!(st, 200);
        assert!(text.contains("\"passphrase\"") && text.contains("4321"), "{text}");
        let (st, text) = post(a, "/api/v1/stream/pair", Some("tokx"), "{\"pin\":\"1234\"}");
        assert_eq!(st, 200);
        assert!(text.contains("\"ok\":true"), "{text}");
        assert!(notes.lock().unwrap().contains(&"stream:tab1:1234".to_string()));
        assert!(notes.lock().unwrap().contains(&"stream:tab1:-".to_string()));
        assert_eq!(get(a, "GET", "/api/v1/stream/pair", &[("Authorization", "Bearer tokx")]).status, 405);
        // por defecto el proveedor no puede
        let fake = Fake { lib: lib_with(vec![]), dir: d.path().to_path_buf(), scans: Arc::new(AtomicU64::new(0)) };
        let srv2 = RemoteServer::start("127.0.0.1:0".parse().unwrap(), fake).unwrap();
        let (st, text) = post(srv2.local_addr(), "/api/v1/stream/pair", Some("ABCD-EFGH"), "{\"pin\":\"1234\"}");
        assert_eq!(st, 409);
        assert!(text.contains("no puede emparejar el streaming"), "{text}");
    }

    #[test]
    fn qr_payload_and_host_addrs() {
        let q = qr_payload("id1", "SALON", &["192.168.1.5:47811".to_string(), "100.64.0.2:47811".to_string()], 47811, "ABCD-EFGH");
        let v: serde_json::Value = serde_json::from_str(&q).unwrap();
        assert_eq!((v["v"].as_u64(), v["app"].as_str(), v["id"].as_str(), v["name"].as_str()), (Some(2), Some("visor-capturas"), Some("id1"), Some("SALON")));
        assert_eq!((v["port"].as_u64(), v["ticket"].as_str(), v["addrs"].as_array().map(|a| a.len())), (Some(47811), Some("ABCD-EFGH"), Some(2)));
        assert!(is_tailscale(&"100.64.0.1".parse().unwrap()) && is_tailscale(&"100.127.255.254".parse().unwrap()));
        assert!(!is_tailscale(&"100.128.0.1".parse().unwrap()) && !is_tailscale(&"100.63.255.255".parse().unwrap()) && !is_tailscale(&"192.168.1.1".parse().unwrap()));
        let addrs = host_addrs(1234);
        assert!(addrs.iter().all(|x| x.ends_with(":1234") && !x.starts_with("127.")), "{addrs:?}");
        // de casa primero, Tailscale despues
        let first_ts = addrs.iter().position(|x| is_tailscale(&x.split(':').next().unwrap().parse().unwrap()));
        if let Some(i) = first_ts {
            assert!(addrs[i..].iter().all(|x| is_tailscale(&x.split(':').next().unwrap().parse().unwrap())), "{addrs:?}");
        }
    }

    #[test]
    fn launch_and_capture_over_http() {
        let d = tempfile::tempdir().unwrap();
        let fake = Fake { lib: lib_with(vec![]), dir: d.path().to_path_buf(), scans: Arc::new(AtomicU64::new(0)) };
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), fake).unwrap();
        let a = srv.local_addr();
        let hello = get(a, "GET", "/api/v1/hello", &[]);
        assert!(String::from_utf8_lossy(&hello.body).contains("\"launch\""));
        // sin codigo: 401; con codigo: 200, 409 (el proveedor rechaza el id) o 400 (cuerpo sin id)
        assert_eq!(post(a, "/api/v1/capture", None, "").0, 401);
        assert_eq!(post(a, "/api/v1/launch", None, "{\"id\":\"pc/A\"}").0, 401);
        assert_eq!(post(a, "/api/v1/capture", Some("ABCD-EFGH"), "").0, 200);
        assert_eq!(post(a, "/api/v1/launch", Some("ABCD-EFGH"), "{\"id\":\"pc/A\"}").0, 200);
        assert_eq!(post(a, "/api/v1/launch", Some("ABCD-EFGH"), "{\"id\":\"otro\"}").0, 409);
        assert_eq!(post(a, "/api/v1/launch", Some("ABCD-EFGH"), "no es json").0, 400);
        // por GET no se puede lanzar ni capturar
        assert_eq!(get(a, "GET", "/api/v1/capture", &[("Authorization", "Bearer ABCD-EFGH")]).status, 405);
        // close: 401 sin codigo, 200 con codigo; session sin codigo desde loopback; session/start por GET = 405
        assert_eq!(post(a, "/api/v1/close", None, "{}").0, 401);
        let (st, text) = post(a, "/api/v1/close", Some("ABCD-EFGH"), "{\"force\":true}");
        assert_eq!(st, 200);
        assert!(text.contains("\"forced\""));
        assert_eq!(get(a, "GET", "/api/v1/session", &[]).status, 200);
        assert_eq!(post(a, "/api/v1/session/start", None, "{\"width\":1,\"height\":1,\"fps\":60}").0, 409);
        assert_eq!(get(a, "GET", "/api/v1/session/start", &[]).status, 405);
    }

    #[test]
    fn end_to_end_over_http() {
        let d = tempfile::tempdir().unwrap();
        let shot = d.path().join("caps").join("Juego A").join("2024-01-01 10-00-00.png");
        fs::create_dir_all(shot.parent().unwrap()).unwrap();
        image::RgbImage::from_fn(2400, 1200, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 7])).save(&shot).unwrap();
        let secret = d.path().join("secreto.png");
        fs::copy(&shot, &secret).unwrap();
        let shot_s = shot.to_string_lossy().into_owned();
        let scans = Arc::new(AtomicU64::new(0));
        let fake = Fake { lib: lib_with(vec![title("Juego A", &[&shot_s], None, None, None)]), dir: d.path().to_path_buf(), scans: scans.clone() };
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), fake).unwrap();
        let a = srv.local_addr();
        let enc = |s: &str| s.bytes().map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>();

        // hello sin codigo, OPTIONS sin codigo
        let r = get(a, "GET", "/api/v1/hello", &[]);
        assert_eq!(r.status, 200);
        assert_eq!(r.h("Access-Control-Allow-Origin"), Some("*"));
        let v: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        assert_eq!((v["app"].as_str(), v["api"].as_u64(), v["name"].as_str()), (Some("visor-capturas"), Some(1), Some("PC-PRUEBA")));
        assert_eq!(get(a, "OPTIONS", "/api/v1/snapshot", &[]).status, 204);

        // sin codigo / codigo malo
        assert_eq!(get(a, "GET", "/api/v1/snapshot", &[]).status, 401);
        assert_eq!(get(a, "GET", "/api/v1/snapshot", &[("Authorization", "Bearer ZZZZ-ZZZZ")]).status, 401);
        assert_eq!(scans.load(Ordering::SeqCst), 0, "sin codigo no se escanea");

        // snapshot con token
        let r = get(a, "GET", "/api/v1/snapshot", &[("Authorization", "Bearer ABCD-EFGH")]);
        assert_eq!(r.status, 200);
        let v: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        assert_eq!(v["api"], 1);
        assert_eq!(v["name"], "PC-PRUEBA");
        assert!(v["generatedAt"].as_i64().unwrap() > 0);
        assert_eq!(v["config"]["goalVn"], 60);
        assert_eq!(v["config"]["accentMode"], "art");
        assert_eq!(v["library"]["platforms"][0]["titles"][0]["shots"][0]["path"], shot_s.as_str());
        assert_eq!(v["sagas"][0]["members"][0], "pc/A");
        assert!(v["playing"].is_null());
        assert!(v["favorites"].as_array().unwrap().is_empty());
        let etag = r.h("ETag").unwrap().to_string();

        // 304 con la misma ETag; gzip si se acepta; la cache evita reescanear
        let r = get(a, "GET", "/api/v1/snapshot", &[("Authorization", "Bearer ABCD-EFGH"), ("If-None-Match", &etag)]);
        assert_eq!(r.status, 304);
        assert!(r.body.is_empty());
        let r = get(a, "GET", "/api/v1/snapshot", &[("Authorization", "Bearer ABCD-EFGH"), ("Accept-Encoding", "gzip")]);
        assert_eq!((r.status, r.h("Content-Encoding")), (200, Some("gzip")));
        let mut json = Vec::new();
        flate2::read::GzDecoder::new(&r.body[..]).read_to_end(&mut json).unwrap();
        assert!(serde_json::from_slice::<serde_json::Value>(&json).is_ok());
        assert_eq!(scans.load(Ordering::SeqCst), 1);
        let r = get(a, "GET", "/api/v1/snapshot?fresh=1", &[("Authorization", "Bearer ABCD-EFGH"), ("If-None-Match", &etag)]);
        assert_eq!(r.status, 304, "mismos datos, misma ETag");
        assert_eq!(scans.load(Ordering::SeqCst), 2);

        // imagenes con ?k=
        let q = enc(&shot_s);
        let r = get(a, "GET", &format!("/api/v1/thumb?p={q}&k=ABCD-EFGH"), &[]);
        assert_eq!((r.status, r.h("Content-Type")), (200, Some("image/jpeg")));
        assert_eq!(imagesize::blob_size(&r.body).unwrap().width, 480);
        let r = get(a, "GET", &format!("/api/v1/preview?p={q}&k=ABCD-EFGH"), &[]);
        assert_eq!(r.status, 200);
        assert_eq!(imagesize::blob_size(&r.body).unwrap().width, 1920);
        let et = r.h("ETag").unwrap().to_string();
        assert_eq!(get(a, "GET", &format!("/api/v1/preview?p={q}&k=ABCD-EFGH"), &[("If-None-Match", &et)]).status, 304);
        let r = get(a, "GET", &format!("/api/v1/file?p={q}&k=ABCD-EFGH"), &[]);
        assert_eq!((r.status, r.h("Content-Type")), (200, Some("image/png")));
        assert_eq!(r.body, fs::read(&shot).unwrap());

        // rutas fuera de la instantanea, sin p, endpoint desconocido
        assert_eq!(get(a, "GET", &format!("/api/v1/file?p={}&k=ABCD-EFGH", enc(&secret.to_string_lossy())), &[]).status, 404);
        assert_eq!(get(a, "GET", "/api/v1/file?k=ABCD-EFGH", &[]).status, 400);
        assert_eq!(get(a, "GET", "/api/v1/nada?k=ABCD-EFGH", &[]).status, 404);

        // el token se compara tal cual (ya no se normaliza como el codigo de antes)
        assert_eq!(get(a, "GET", &format!("/api/v1/file?p={q}&k=abcdefgh"), &[]).status, 401);

        // 429 tras 10 fallos (ya van 3)
        for _ in 0..7 {
            assert_eq!(get(a, "GET", "/api/v1/snapshot?k=MALO", &[]).status, 401);
        }
        let r = get(a, "GET", "/api/v1/snapshot?k=ABCD-EFGH", &[]);
        assert_eq!(r.status, 429, "bloqueado aunque el codigo sea bueno");
        assert!(String::from_utf8_lossy(&r.body).contains("error"));
        assert_eq!(get(a, "GET", "/api/v1/hello", &[]).status, 200, "hello no se bloquea");

        srv.stop();
        drop(srv);
        std::thread::sleep(Duration::from_millis(200));
        assert!(TcpStream::connect_timeout(&a, Duration::from_millis(500)).map(|mut s| {
            let _ = s.write_all(b"GET /api/v1/hello HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
            let mut buf = Vec::new();
            s.set_read_timeout(Some(Duration::from_secs(2))).ok();
            let _ = s.read_to_end(&mut buf);
            buf.is_empty()
        }).unwrap_or(true), "parado no contesta");
    }
}
