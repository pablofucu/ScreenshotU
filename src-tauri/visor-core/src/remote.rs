//! Servidor HTTP para la app movil (contrato en `docs/api-movil.md`).
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
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Version del contrato (`api` en `hello` y `snapshot`).
pub const API_VERSION: u32 = 1;
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
const WORKERS: usize = 8;
/// Alfabeto del codigo de emparejamiento (sin 0/O, 1/I/L ni otros ambiguos).
pub const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/* ---------- datos ---------- */

/// Ajustes del escritorio que necesita el movil.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConfig {
    pub goal_vn: u32,
    pub goal_jvn: u32,
    pub accent_mode: String,
    pub accent_fixed: u8,
}

impl RemoteConfig {
    pub fn from_config(c: &Config) -> RemoteConfig {
        RemoteConfig { goal_vn: c.goal_vn, goal_jvn: c.goal_jvn, accent_mode: c.accent_mode.clone(), accent_fixed: c.accent_fixed }
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

/// Fuente de los datos (la app de escritorio, o el ejemplo `serve` sin interfaz).
pub trait Provider: Send + Sync + 'static {
    /// Nombre del equipo que se muestra en el movil.
    fn name(&self) -> String;
    /// Escaneo nuevo de la biblioteca (ya sin ocultos).
    fn snapshot(&self) -> Result<Snapshot>;
    /// Cache de miniaturas (la misma del escritorio).
    fn thumbs_dir(&self) -> PathBuf;
    /// Cache de vistas previas de 1920 px.
    fn previews_dir(&self) -> PathBuf;
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
        Req { method: method.to_ascii_uppercase(), path: percent_decode(path), query, headers, ip }
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
        _ => "application/octet-stream",
    }
}

/// Estado compartido por los hilos del servidor.
struct Inner {
    provider: Arc<dyn Provider>,
    token: Arc<RwLock<String>>,
    cache: Mutex<Option<Arc<Cached>>>,
    /// Un solo escaneo a la vez.
    building: Mutex<()>,
    throttle: Throttle,
    /// Cerrojos por ruta (repartidos) para no generar dos veces la misma miniatura a la vez.
    gen_locks: Vec<Mutex<()>>,
}

impl Inner {
    fn new(provider: Arc<dyn Provider>, token: Arc<RwLock<String>>) -> Inner {
        Inner {
            provider,
            token,
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

    fn authorized(&self, req: &Req) -> bool {
        let given = req
            .header("authorization")
            .and_then(|h| {
                let h = h.trim();
                (h.len() > 7 && h[..7].eq_ignore_ascii_case("bearer ")).then(|| h[7..].to_string())
            })
            .or_else(|| req.query.get("k").cloned())
            .unwrap_or_default();
        let token = self.token.read().unwrap_or_else(|e| e.into_inner());
        code_matches(&token, &given)
    }

    fn gen_lock(&self, key: &str) -> MutexGuard<'_, ()> {
        let i = (thumbs::fnv1a64(key.as_bytes()) as usize) % self.gen_locks.len();
        lock(&self.gen_locks[i])
    }
}

fn route(inner: &Inner, req: &Req) -> Reply {
    if req.method == "OPTIONS" {
        return Reply::empty(204);
    }
    if req.method != "GET" && req.method != "HEAD" {
        return Reply::error(405, "Método no permitido");
    }
    let ep = match req.path.trim_end_matches('/') {
        "/api/v1/hello" => {
            return Reply::json(
                200,
                &serde_json::json!({ "app": "visor-capturas", "api": API_VERSION, "name": inner.provider.name(), "version": env!("CARGO_PKG_VERSION") }),
            )
        }
        "/api/v1/snapshot" => "snapshot",
        "/api/v1/thumb" => "thumb",
        "/api/v1/preview" => "preview",
        "/api/v1/file" => "file",
        _ => return Reply::error(404, "No existe"),
    };
    let now = Instant::now();
    if inner.throttle.blocked(req.ip, now) {
        return Reply::error(429, "Demasiados intentos con un código incorrecto. Espera un minuto.");
    }
    if !inner.authorized(req) {
        inner.throttle.fail(req.ip, now);
        return Reply::error(401, "Falta el código de emparejamiento o no es correcto.");
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
    ("Access-Control-Allow-Headers", "Authorization, If-None-Match, Content-Type"),
    ("Access-Control-Allow-Methods", "GET, OPTIONS"),
    ("Access-Control-Expose-Headers", "ETag"),
];

fn handle(inner: &Inner, rq: tiny_http::Request) {
    let headers = rq.headers().iter().map(|h| (h.field.as_str().as_str().to_ascii_lowercase(), h.value.as_str().to_string())).collect();
    let ip = rq.remote_addr().map(|a| a.ip()).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let req = Req::parse(rq.method().as_str(), rq.url(), headers, ip);
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
    token: Arc<RwLock<String>>,
    addr: SocketAddr,
}

impl RemoteServer {
    pub fn start<P: Provider>(bind: SocketAddr, token: String, provider: P) -> Result<RemoteServer> {
        Self::start_arc(bind, token, Arc::new(provider))
    }

    pub fn start_arc(bind: SocketAddr, token: String, provider: Arc<dyn Provider>) -> Result<RemoteServer> {
        let server = tiny_http::Server::http(bind).map_err(|e| anyhow!("No se pudo abrir el puerto {}: {e}", bind.port()))?;
        let addr = server.server_addr().to_ip().unwrap_or(bind);
        let server = Arc::new(server);
        let stop = Arc::new(AtomicBool::new(false));
        let token = Arc::new(RwLock::new(token));
        let inner = Arc::new(Inner::new(provider, token.clone()));
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
        Ok(RemoteServer { server, stop, token, addr })
    }

    /// Cambia el codigo de emparejamiento sin reiniciar.
    pub fn set_token(&self, token: &str) {
        *self.token.write().unwrap_or_else(|e| e.into_inner()) = token.to_string();
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
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        self.stop();
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
                config: RemoteConfig { goal_vn: 60, goal_jvn: 30, accent_mode: "art".into(), accent_fixed: 4 },
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
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), "ABCD-EFGH".into(), fake).unwrap();
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

        // snapshot con codigo normalizado
        let r = get(a, "GET", "/api/v1/snapshot", &[("Authorization", "Bearer abcd efgh")]);
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
        let r = get(a, "GET", &format!("/api/v1/thumb?p={q}&k=abcdefgh"), &[]);
        assert_eq!((r.status, r.h("Content-Type")), (200, Some("image/jpeg")));
        assert_eq!(imagesize::blob_size(&r.body).unwrap().width, 480);
        let r = get(a, "GET", &format!("/api/v1/preview?p={q}&k=abcdefgh"), &[]);
        assert_eq!(r.status, 200);
        assert_eq!(imagesize::blob_size(&r.body).unwrap().width, 1920);
        let et = r.h("ETag").unwrap().to_string();
        assert_eq!(get(a, "GET", &format!("/api/v1/preview?p={q}&k=abcdefgh"), &[("If-None-Match", &et)]).status, 304);
        let r = get(a, "GET", &format!("/api/v1/file?p={q}&k=abcdefgh"), &[]);
        assert_eq!((r.status, r.h("Content-Type")), (200, Some("image/png")));
        assert_eq!(r.body, fs::read(&shot).unwrap());

        // rutas fuera de la instantanea, sin p, endpoint desconocido
        assert_eq!(get(a, "GET", &format!("/api/v1/file?p={}&k=abcdefgh", enc(&secret.to_string_lossy())), &[]).status, 404);
        assert_eq!(get(a, "GET", "/api/v1/file?k=abcdefgh", &[]).status, 400);
        assert_eq!(get(a, "GET", "/api/v1/nada?k=abcdefgh", &[]).status, 404);

        // cambio de codigo en caliente
        srv.set_token("WXYZ-2345");
        assert_eq!(get(a, "GET", &format!("/api/v1/file?p={q}&k=abcdefgh"), &[]).status, 401);
        assert_eq!(get(a, "GET", &format!("/api/v1/file?p={q}&k=wxyz2345"), &[]).status, 200);

        // 429 tras 10 fallos (ya van 3)
        for _ in 0..7 {
            assert_eq!(get(a, "GET", "/api/v1/snapshot?k=MALO", &[]).status, 401);
        }
        let r = get(a, "GET", "/api/v1/snapshot?k=WXYZ-2345", &[]);
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
