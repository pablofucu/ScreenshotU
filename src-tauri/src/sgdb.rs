//! Cliente de la API v2 de SteamGridDB (https://www.steamgriddb.com/api/v2). Llamadas bloqueantes:
//! los comandos las ejecutan fuera del hilo de la interfaz.
//!
//! Las respuestas se leen como JSON suelto y se extraen los campos a mano: la API manda `null` u otro tipo
//! en campos que "siempre" estan (nombres, estilos, autores...) y con una deserializacion estricta un solo
//! elemento raro tiraba abajo toda la peticion («read json») y, con ella, la descarga masiva.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use visor_core::art::{Art, ArtKind};
use visor_core::db::{data as dbd, Db};
use visor_core::sgdb::{self, url_segment, SgdbEntry};

use crate::state::AppState;

const API: &str = "https://www.steamgriddb.com/api/v2";
const MAX_IMAGE: u64 = 25 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameHit {
    pub id: u64,
    pub name: String,
    /// Año de salida, si se conoce.
    pub year: Option<i32>,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageHit {
    pub id: u64,
    pub url: String,
    pub thumb: String,
    pub width: u32,
    pub height: u32,
    pub mime: String,
    pub style: String,
    pub author: String,
}

/* ---------- errores y reintentos ---------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrKind {
    /// Clave rechazada o ausente: no tiene sentido seguir.
    Auth,
    /// Red caida, limite de peticiones, error 5xx o pagina de error: puede arreglarse esperando.
    Transient,
    /// SteamGridDB no tiene ese juego.
    NotFound,
    Other,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub kind: ErrKind,
    pub msg: String,
}

impl ApiError {
    fn new(kind: ErrKind, msg: impl Into<String>) -> Self {
        ApiError { kind, msg: msg.into() }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

const MSG_AUTH: &str = "SteamGridDB rechaza la clave de API: revisala en Ajustes";

/// Cuantas veces se reintenta una peticion pasajera y si se espacian las peticiones.
#[derive(Clone, Copy)]
struct Policy {
    retries: u32,
    pace: bool,
}
/// Selector de imagenes: responde rapido (un par de reintentos cortos).
const QUICK: Policy = Policy { retries: 2, pace: false };
/// Descarga masiva: paciencia con el limite de peticiones y pausa entre peticiones.
const BATCH: Policy = Policy { retries: 4, pace: true };
const PACE: Duration = Duration::from_millis(350);

/// Espera lo justo para no pasar de ~3 peticiones por segundo a la API.
fn pace() {
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    let wait = {
        let mut last = LAST.lock().unwrap();
        let now = Instant::now();
        let at = match *last {
            Some(t) if t + PACE > now => t + PACE,
            _ => now,
        };
        *last = Some(at);
        at - now
    };
    if !wait.is_zero() {
        std::thread::sleep(wait);
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(40))
        .user_agent(concat!("visor-capturas/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// Primeros caracteres de un cuerpo de respuesta, para el registro.
fn snippet(s: &str) -> String {
    s.chars().take(300).collect::<String>().replace(['\r', '\n'], " ")
}

enum Envelope {
    /// No es el JSON de SteamGridDB (o no es JSON).
    Unreadable(String),
    /// Respuesta valida con `success: false`.
    Rejected(String),
}

/// Abre el sobre `{success, data, errors}` de la API y devuelve `data` (o `Null` si no trae).
fn parse_envelope(body: &str) -> Result<Value, Envelope> {
    let v: Value = serde_json::from_str(body).map_err(|e| Envelope::Unreadable(e.to_string()))?;
    if !v.is_object() {
        return Err(Envelope::Unreadable("no es un objeto JSON".into()));
    }
    let ok = v.get("success").and_then(Value::as_bool).unwrap_or_else(|| v.get("data").is_some_and(|d| !d.is_null()));
    if ok {
        return Ok(v.get("data").cloned().unwrap_or(Value::Null));
    }
    let errors: Vec<String> = match v.get("errors") {
        Some(Value::Array(a)) => a.iter().map(|e| e.as_str().map(str::to_string).unwrap_or_else(|| e.to_string())).collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    };
    Err(Envelope::Rejected(if errors.is_empty() { "SteamGridDB no devolvio datos".into() } else { errors.join(", ") }))
}

/// Pide una URL de la API y devuelve su `data`. 429 y 5xx (y los fallos de red o paginas de error) se
/// reintentan con espera creciente, respetando `Retry-After`; la clave rechazada (401/403) corta de inmediato.
fn get_value(key: &str, url: &str, pol: Policy) -> Result<Value, ApiError> {
    if key.trim().is_empty() {
        return Err(ApiError::new(ErrKind::Auth, "Falta la clave de API de SteamGridDB (Ajustes → SteamGridDB)"));
    }
    let mut attempt = 0u32;
    loop {
        if pol.pace {
            pace();
        }
        let res = agent().get(url).set("Authorization", &format!("Bearer {}", key.trim())).call();
        let (err, hint): (ApiError, Option<u64>) = match res {
            Ok(resp) => {
                let code = resp.status();
                match resp.into_string() {
                    Ok(body) => match parse_envelope(&body) {
                        Ok(v) => return Ok(v),
                        Err(Envelope::Rejected(m)) => {
                            let kind = if m.to_lowercase().contains("not found") { ErrKind::NotFound } else { ErrKind::Other };
                            return Err(ApiError::new(kind, m));
                        }
                        Err(Envelope::Unreadable(m)) => {
                            eprintln!("SteamGridDB: respuesta ilegible (HTTP {code}) de {url}: {m}; cuerpo: {}", snippet(&body));
                            // Una pagina HTML o un cuerpo vacio es un fallo pasajero del servidor o de un proxy.
                            let blip = body.trim().is_empty() || body.trim_start().starts_with('<');
                            let kind = if blip { ErrKind::Transient } else { ErrKind::Other };
                            (ApiError::new(kind, format!("Respuesta ilegible de SteamGridDB (HTTP {code}): {m}")), None)
                        }
                    },
                    Err(e) => (ApiError::new(ErrKind::Transient, format!("Respuesta de SteamGridDB cortada: {e}")), None),
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                let hint = resp.header("Retry-After").and_then(|h| h.trim().parse::<u64>().ok());
                let body = resp.into_string().unwrap_or_default();
                eprintln!("SteamGridDB: HTTP {code} de {url}: {}", snippet(&body));
                match code {
                    401 | 403 => return Err(ApiError::new(ErrKind::Auth, MSG_AUTH)),
                    404 => return Err(ApiError::new(ErrKind::NotFound, "SteamGridDB no tiene ese juego")),
                    429 => (ApiError::new(ErrKind::Transient, "SteamGridDB pide esperar un poco (demasiadas peticiones)"), hint),
                    500..=599 => (ApiError::new(ErrKind::Transient, format!("SteamGridDB respondio con un error ({code})")), hint),
                    _ => return Err(ApiError::new(ErrKind::Other, format!("SteamGridDB respondio con un error ({code})"))),
                }
            }
            Err(ureq::Error::Transport(t)) => (ApiError::new(ErrKind::Transient, format!("Sin conexion con SteamGridDB: {t}")), None),
        };
        if err.kind != ErrKind::Transient || attempt >= pol.retries {
            return Err(err);
        }
        let wait = hint.unwrap_or(2u64 << attempt).clamp(1, 60);
        eprintln!("SteamGridDB: {}; reintento {} en {wait} s", err.msg, attempt + 1);
        std::thread::sleep(Duration::from_secs(wait));
        attempt += 1;
    }
}

/* ---------- lectura tolerante del JSON ---------- */

/// Numero entero sin signo venga como numero, decimal o texto.
fn as_u64(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64))
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

/// Texto de un campo; `null`, ausente o de otro tipo = vacio (los numeros se pasan a texto).
fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn number(v: Option<&Value>) -> u64 {
    v.and_then(as_u64).unwrap_or(0)
}

/// Elementos de una lista de `data` (cualquier otra cosa se toma por lista vacia). Un elemento que no se
/// puede leer se registra y se salta, sin afectar al resto.
fn list_of<T>(v: &Value, what: &str, f: impl Fn(&Value) -> Option<T>) -> Vec<T> {
    let Some(a) = v.as_array() else { return Vec::new() };
    a.iter()
        .filter_map(|item| {
            let r = f(item);
            if r.is_none() {
                eprintln!("SteamGridDB: se ignora un {what} ilegible: {}", snippet(&item.to_string()));
            }
            r
        })
        .collect()
}

fn parse_game(v: &Value) -> Option<GameHit> {
    let id = v.get("id").and_then(as_u64)?;
    let name = text(v.get("name"));
    Some(GameHit {
        id,
        name: if name.trim().is_empty() { format!("Juego #{id}") } else { name },
        year: v.get("release_date").and_then(Value::as_f64).filter(|t| *t > 0.0).map(|t| (1970.0 + t / 31_556_952.0) as i32),
        verified: v.get("verified").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn parse_image(v: &Value) -> Option<ImageHit> {
    let id = v.get("id").and_then(as_u64)?;
    let url = text(v.get("url"));
    if url.is_empty() {
        return None;
    }
    let thumb = text(v.get("thumb"));
    Some(ImageHit {
        id,
        thumb: if thumb.is_empty() { url.clone() } else { thumb },
        url,
        width: number(v.get("width")).min(u32::MAX as u64) as u32,
        height: number(v.get("height")).min(u32::MAX as u64) as u32,
        mime: text(v.get("mime")),
        style: text(v.get("style")),
        author: text(v.get("author").and_then(|a| a.get("name"))),
    })
}

/* ---------- busquedas ---------- */

/// Busca juegos por nombre (el primero es el mas probable).
pub fn search(key: &str, term: &str) -> Result<Vec<GameHit>, ApiError> {
    search_with(key, term, QUICK)
}

fn search_with(key: &str, term: &str, pol: Policy) -> Result<Vec<GameHit>, ApiError> {
    let term = sgdb::search_term(term);
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let v = match get_value(key, &format!("{API}/search/autocomplete/{}", url_segment(&term)), pol) {
        Ok(v) => v,
        Err(e) if e.kind == ErrKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    Ok(list_of(&v, "juego", parse_game))
}

/// Imagenes de SteamGridDB de un tipo para un juego (`page` empieza en 0). Lista vacia si no hay ninguna.
fn sgdb_list(key: &str, game_id: u64, kind: ArtKind, page: u32, pol: Policy) -> Result<Vec<ImageHit>, ApiError> {
    let q = match kind {
        // Portadas verticales (las de Steam y sus variantes de mayor resolucion).
        ArtKind::Cover => "grids/game/{id}?dimensions=600x900,342x482,660x930&types=static",
        ArtKind::Hero => "heroes/game/{id}?types=static",
        ArtKind::Logo => "logos/game/{id}?types=static",
    };
    let url = format!("{API}/{}&nsfw=false&humor=false&page={page}", q.replace("{id}", &game_id.to_string()));
    match get_value(key, &url, pol) {
        Ok(v) => Ok(list_of(&v, "grafismo", parse_image)),
        Err(e) if e.kind == ErrKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Imagenes de un tipo para un juego: primero el original de Steam (si lo hay), despues las de SteamGridDB.
pub fn images(key: &str, game_id: u64, kind: ArtKind, page: u32) -> Result<Vec<ImageHit>, ApiError> {
    let raw = sgdb_list(key, game_id, kind, page, QUICK)?;
    // Los originales de Steam no estan en SteamGridDB como imagenes propias: salen del CDN de Steam
    // y van los primeros, para que sean la opcion por defecto.
    let mut out = Vec::new();
    if page == 0 {
        if let Some(app) = steam_app(key, game_id, QUICK)? {
            out.extend(steam_hit(&app, kind, true));
        }
    }
    out.extend(raw);
    Ok(out)
}

const STEAM_CDN: &str = "https://shared.steamstatic.com/store_item_assets/steam/apps";

/// App de Steam enlazada a un juego de SteamGridDB, con los datos de sus assets localizados.
struct SteamApp {
    appid: u64,
    meta: Value,
}

fn parse_steam(data: &Value) -> Option<SteamApp> {
    let first = data.get("external_platform_data")?.get("steam")?.as_array()?.first()?;
    Some(SteamApp { appid: first.get("id").and_then(as_u64)?, meta: first.get("metadata").cloned().unwrap_or(Value::Null) })
}

/// Ruta (relativa al appid) del asset localizado en `metadata.<field>_full`: el 2x en ingles si lo hay.
fn steam_full_path(meta: &Value, field: &str) -> Option<String> {
    let full = meta.get(field)?;
    ["image2x", "image"].iter().find_map(|res| {
        let langs = full.get(res)?.as_object()?;
        langs.get("english").or_else(|| langs.values().next())?.as_str().map(str::to_string)
    })
}

/// App de Steam del juego, si SteamGridDB la enlaza. Solo falla con la clave rechazada: cualquier otro
/// problema (sin enlace, respuesta rara, red) deja el original de Steam sin ofrecer, nada mas.
fn steam_app(key: &str, game_id: u64, pol: Policy) -> Result<Option<SteamApp>, ApiError> {
    match get_value(key, &format!("{API}/games/id/{game_id}?platformdata=steam"), pol) {
        Ok(v) => Ok(parse_steam(&v)),
        Err(e) if e.kind == ErrKind::Auth => Err(e),
        Err(e) => {
            eprintln!("SteamGridDB platformdata {game_id}: {e}");
            Ok(None)
        }
    }
}

/// Asset original de Steam (portada, heroe o logo). Con `verify` comprueba que exista (una peticion HEAD por
/// ruta); sin ella devuelve la ruta mas probable y, si no existe, la descarga fallara y se probaran las demas.
fn steam_hit(app: &SteamApp, kind: ArtKind, verify: bool) -> Option<ImageHit> {
    let (field, fallback, w, h, mime) = match kind {
        ArtKind::Cover => ("library_capsule_full", "library_600x900_2x.jpg", 600, 900, "image/jpeg"),
        ArtKind::Hero => ("library_hero_full", "library_hero.jpg", 1920, 620, "image/jpeg"),
        ArtKind::Logo => ("library_logo_full", "logo.png", 0, 0, "image/png"),
    };
    let mut paths: Vec<String> = steam_full_path(&app.meta, field).into_iter().collect();
    paths.push(fallback.to_string());
    let mut urls = paths.into_iter().map(|p| format!("{STEAM_CDN}/{}/{p}", app.appid));
    // Algunos juegos no tienen todos los assets: con `verify` se usa la primera ruta que exista.
    let url = if verify { urls.find(|u| agent().head(u).call().is_ok())? } else { urls.next()? };
    Some(ImageHit {
        // Id propio (no choca con los de SteamGridDB) para el nombre del fichero en la cache.
        id: app.appid,
        thumb: url.clone(),
        url,
        width: w,
        height: h,
        mime: mime.into(),
        style: "official".into(),
        author: "Steam".into(),
    })
}

/// Solo se descargan imagenes del CDN de SteamGridDB o del de Steam (assets originales).
fn allowed(url: &str) -> bool {
    let rest = match url.strip_prefix("https://") {
        Some(r) => r,
        None => return false,
    };
    let host = rest.split('/').next().unwrap_or("");
    host == "steamgriddb.com" || host.ends_with(".steamgriddb.com") || host.ends_with(".steamstatic.com")
}

/// Baja los bytes de una imagen. El `bool` del error indica si merece otro intento.
fn fetch_bytes(url: &str) -> Result<(String, Vec<u8>), (bool, String)> {
    let resp = match agent().get(url).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(c, _)) if matches!(c, 403 | 404 | 410) => return Err((false, format!("La imagen ya no esta disponible ({c})"))),
        Err(ureq::Error::Status(c, _)) => return Err((true, format!("El servidor de imagenes respondio con un error ({c})"))),
        Err(ureq::Error::Transport(t)) => return Err((true, format!("Sin conexion para descargar la imagen: {t}"))),
    };
    let mime = resp.content_type().to_string();
    let mut bytes = Vec::new();
    resp.into_reader().take(MAX_IMAGE + 1).read_to_end(&mut bytes).map_err(|e| (true, format!("Descarga cortada: {e}")))?;
    if bytes.len() as u64 > MAX_IMAGE {
        return Err((false, "La imagen es demasiado grande".into()));
    }
    Ok((mime, bytes))
}

/// Descarga una imagen y la guarda como grafismo del titulo en la cache (un reintento si la red falla).
pub fn download(base: &Path, title_id: &str, kind: ArtKind, img: &ImageHit) -> Result<PathBuf, String> {
    if !allowed(&img.url) {
        return Err("Esa imagen no viene de SteamGridDB".into());
    }
    let mut attempt = 0;
    let (mime, bytes) = loop {
        match fetch_bytes(&img.url) {
            Ok(r) => break r,
            Err((retry, m)) if retry && attempt < 1 => {
                eprintln!("SteamGridDB: {m}; se reintenta {}", img.url);
                std::thread::sleep(Duration::from_millis(1500));
                attempt += 1;
            }
            Err((_, m)) => return Err(m),
        }
    };
    let mime = if img.mime.is_empty() { mime } else { img.mime.clone() };
    sgdb::store_image(base, title_id, kind, img.id, sgdb::ext_for(&mime, &img.url), &bytes).map_err(|e| e.to_string())
}

/* ---------- comandos de la interfaz ---------- */

fn key_of(state: &AppState) -> String {
    state.config.lock().unwrap().sgdb_key.clone()
}

fn kind_arg(kind: &str) -> Result<ArtKind, String> {
    sgdb::parse_kind(kind).ok_or_else(|| format!("Tipo de grafismo desconocido: {kind}"))
}

/// Guarda la entrada de un titulo en la base (solo esa fila).
fn save_entry(db: &Db, title_id: &str, e: &SgdbEntry) {
    crate::state::log_db_err("sgdb", db.with(|c| dbd::sgdb_put(c, title_id, e)));
}

/// Marca un titulo como revisado (la descarga automatica ya no lo vuelve a tocar).
fn touch(state: &AppState, title_id: String, f: impl FnOnce(&mut SgdbEntry)) {
    let mut s = state.sgdb.lock().unwrap();
    let e = s.entry(title_id.clone()).or_default();
    e.auto_done = true;
    f(e);
    save_entry(&state.db, &title_id, e);
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoTitle {
    id: String,
    name: String,
    /// Grafismos que le faltan: "cover" | "hero" | "logo".
    missing: Vec<String>,
}

/// Un titulo que de verdad se va a buscar y los tipos que le faltan de verdad.
struct Pending {
    id: String,
    name: String,
    kinds: Vec<ArtKind>,
}

#[derive(Clone, Serialize)]
struct Progress {
    done: usize,
    total: usize,
    name: String,
    /// Grafismos bajados hasta ahora y titulos que han fallado.
    images: usize,
    failed: usize,
}

/// Grafismos nuevos de un titulo: la interfaz los pone en su copia de la biblioteca sin reescanear.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtEvent {
    title_id: String,
    cover: Option<String>,
    hero: Option<String>,
    logo: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Done {
    /// Titulos revisados y titulos que habia que revisar.
    done: usize,
    total: usize,
    /// Grafismos descargados.
    images: usize,
    /// Juegos que no se encontraron en SteamGridDB.
    missing: Vec<String>,
    /// Juegos que fallaron (respuesta ilegible, red, descarga...) y el primer error, legible.
    failed: usize,
    first_error: Option<String>,
    /// La tanda se corto (clave rechazada, red caida...): lo que quedaba sigue pendiente.
    error: Option<String>,
}

fn art_of(a: &Art, k: ArtKind) -> bool {
    match k {
        ArtKind::Cover => a.cover.is_some(),
        ArtKind::Hero => a.hero.is_some(),
        ArtKind::Logo => a.logo.is_some(),
    }
}

/// Titulos que fallaron en esta sesion: la busqueda automatica no los reintenta en cada escaneo
/// (`force` si).
fn failed_set() -> &'static Mutex<HashSet<String>> {
    static S: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    S.get_or_init(Default::default)
}

/// Tipos que de verdad faltan en un titulo: los que la interfaz dice, menos los que ya hay en la cache de
/// grafismos (la interfaz puede ir por detras) y los que SteamGridDB ya dijo que no tiene. Sin `force`, un
/// titulo ya revisado no se toca.
fn pending_kinds(t: &AutoTitle, entry: Option<&SgdbEntry>, base: &Path, force: bool) -> Vec<ArtKind> {
    if !force && entry.is_some_and(|e| e.auto_done) {
        return Vec::new();
    }
    let cached = sgdb::find_cached(base, &t.id);
    let mut kinds: Vec<ArtKind> = Vec::new();
    for k in t.missing.iter().filter_map(|k| sgdb::parse_kind(k)) {
        if !kinds.contains(&k) && !art_of(&cached, k) && !entry.is_some_and(|e| e.has_tried(k)) {
            kinds.push(k);
        }
    }
    kinds
}

/// Resultado de un titulo que no fallo.
enum Found {
    Ok,
    /// SteamGridDB no tiene ningun juego con ese nombre.
    NotFound,
}

/// Busca y baja los grafismos que faltan de un titulo. Anota en `e` lo que averigua (juego, tipos sin
/// resultado); el que llama lo guarda. `images` cuenta cada imagen bajada, aunque el titulo acabe fallando.
fn run_title(key: &str, base: &Path, t: &Pending, e: &mut SgdbEntry, images: &mut usize) -> Result<Found, ApiError> {
    let gid = match e.game_id {
        Some(g) => g,
        None => match search_with(key, &t.name, BATCH)?.into_iter().next() {
            Some(h) => {
                e.game_id = Some(h.id);
                e.game_name = h.name;
                h.id
            }
            None => {
                for k in &t.kinds {
                    e.add_tried(*k);
                }
                return Ok(Found::NotFound);
            }
        },
    };
    // Una sola consulta por juego para los originales de Steam (antes una por tipo, mas HEAD por cada ruta).
    let steam = steam_app(key, gid, BATCH)?;
    let mut first_fail: Option<String> = None;
    for &kind in &t.kinds {
        // El original de Steam va primero. Sin comprobar si existe: si falla la descarga se prueba lo demas.
        if let Some(hit) = steam.as_ref().and_then(|a| steam_hit(a, kind, false)) {
            match download(base, &t.id, kind, &hit) {
                Ok(_) => {
                    *images += 1;
                    continue;
                }
                Err(m) => eprintln!("SteamGridDB {}: {m} ({})", t.name, hit.url),
            }
        }
        let list = sgdb_list(key, gid, kind, 0, BATCH)?;
        if list.is_empty() {
            e.add_tried(kind);
            continue;
        }
        let mut last_err = String::new();
        let mut ok = false;
        for img in list.iter().take(3) {
            match download(base, &t.id, kind, img) {
                Ok(_) => {
                    *images += 1;
                    ok = true;
                    break;
                }
                Err(m) => {
                    eprintln!("SteamGridDB {}: {m} ({})", t.name, img.url);
                    last_err = m;
                }
            }
        }
        if !ok {
            first_fail.get_or_insert(last_err);
        }
    }
    match first_fail {
        Some(m) => Err(ApiError::new(ErrKind::Other, m)),
        None => Ok(Found::Ok),
    }
}

/// Fallos de red seguidos tras los cuales se corta la tanda.
const MAX_NET_STREAK: u32 = 5;

struct BusyGuard(Arc<AtomicBool>);
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Descarga en segundo plano los grafismos que faltan (la primera imagen de cada tipo del juego mas
/// probable). Sin `force` se salta los titulos ya revisados; con `force` revisa tambien los que ya lo
/// estaban si les falta algo que SteamGridDB aun no ha dicho que no tiene. Un titulo que falla se anota y
/// sigue el siguiente; solo corta la tanda la clave rechazada o varios fallos de red seguidos. Avisa con
/// `sgdb-progress`, `sgdb-art` (por titulo con imagenes nuevas) y `sgdb-done`; devuelve cuantos titulos va a
/// revisar (0 = nada que hacer).
#[tauri::command]
pub fn sgdb_auto(app: AppHandle, state: State<'_, AppState>, titles: Vec<AutoTitle>, force: bool) -> Result<usize, String> {
    let key = key_of(&state);
    if key.is_empty() {
        return Err("Falta la clave de API de SteamGridDB (Ajustes → SteamGridDB)".into());
    }
    let base = state.sgdb_dir();
    if force {
        failed_set().lock().unwrap().clear();
    }
    let todo: Vec<Pending> = {
        let store = state.sgdb.lock().unwrap();
        let failed = failed_set().lock().unwrap();
        titles
            .into_iter()
            .filter(|t| !t.missing.is_empty() && (force || !failed.contains(&t.id)))
            .filter_map(|t| {
                let kinds = pending_kinds(&t, store.get(&t.id), &base, force);
                (!kinds.is_empty()).then_some(Pending { id: t.id, name: t.name, kinds })
            })
            .collect()
    };
    if todo.is_empty() {
        return Ok(0);
    }
    if state.sgdb_busy.swap(true, Ordering::SeqCst) {
        return Err("Ya se estan buscando grafismos en SteamGridDB".into());
    }
    let (store, busy, db) = (state.sgdb.clone(), state.sgdb_busy.clone(), state.db.clone());
    let total = todo.len();
    std::thread::spawn(move || {
        let _guard = BusyGuard(busy);
        let mut done = Done { done: 0, total, images: 0, missing: Vec::new(), failed: 0, first_error: None, error: None };
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut net_streak = 0u32;
            for (i, t) in todo.iter().enumerate() {
                let _ = app.emit("sgdb-progress", Progress { done: i, total, name: t.name.clone(), images: done.images, failed: done.failed });
                let orig = store.lock().unwrap().get(&t.id).cloned().unwrap_or_default();
                let mut e = orig.clone();
                let before = done.images;
                let r = run_title(&key, &base, t, &mut e, &mut done.images);
                if r.is_ok() {
                    e.auto_done = true;
                }
                // Cada titulo queda anotado al momento: si la tanda se corta, el reintento sigue por los que faltan.
                if e != orig {
                    save_entry(&db, &t.id, &e);
                    store.lock().unwrap().insert(t.id.clone(), e);
                }
                match r {
                    Ok(f) => {
                        net_streak = 0;
                        if matches!(f, Found::NotFound) {
                            done.missing.push(t.name.clone());
                        }
                    }
                    Err(err) if err.kind == ErrKind::Auth => {
                        done.error = Some(err.msg);
                        break;
                    }
                    Err(err) => {
                        eprintln!("SteamGridDB: {} fallo: {}", t.name, err.msg);
                        failed_set().lock().unwrap().insert(t.id.clone());
                        done.failed += 1;
                        done.first_error.get_or_insert_with(|| format!("{}: {}", t.name, err.msg));
                        if err.kind == ErrKind::Transient {
                            net_streak += 1;
                            if net_streak >= MAX_NET_STREAK {
                                done.error = Some(format!("Se corta la busqueda tras {MAX_NET_STREAK} fallos seguidos. {}", err.msg));
                                break;
                            }
                        } else {
                            net_streak = 0;
                        }
                    }
                }
                done.done = i + 1;
                if done.images > before {
                    let a = sgdb::find_cached(&base, &t.id);
                    let _ = app.emit("sgdb-art", ArtEvent { title_id: t.id.clone(), cover: a.cover, hero: a.hero, logo: a.logo });
                }
            }
        }));
        if run.is_err() {
            done.error.get_or_insert_with(|| "Error interno al buscar grafismos".into());
        }
        let _ = app.emit("sgdb-done", done);
    });
    Ok(total)
}

/// Busca juegos en SteamGridDB por nombre.
#[tauri::command]
pub async fn sgdb_search(state: State<'_, AppState>, term: String) -> Result<Vec<GameHit>, String> {
    let key = key_of(&state);
    tauri::async_runtime::spawn_blocking(move || search(&key, &term).map_err(|e| e.msg)).await.map_err(|e| e.to_string())?
}

/// Una pagina de imagenes de un tipo para un juego de SteamGridDB.
#[tauri::command]
pub async fn sgdb_images(state: State<'_, AppState>, game_id: u64, kind: String, page: u32) -> Result<Vec<ImageHit>, String> {
    let key = key_of(&state);
    let kind = kind_arg(&kind)?;
    tauri::async_runtime::spawn_blocking(move || images(&key, game_id, kind, page).map_err(|e| e.msg)).await.map_err(|e| e.to_string())?
}

/// Juego de SteamGridDB asociado a un titulo (si lo hay).
#[tauri::command]
pub fn sgdb_info(state: State<'_, AppState>, title_id: String) -> Option<SgdbEntry> {
    state.sgdb.lock().unwrap().get(&title_id).cloned()
}

/// Asocia un titulo a otro juego de SteamGridDB (para elegir entre sus imagenes).
#[tauri::command]
pub fn sgdb_link(state: State<'_, AppState>, title_id: String, game_id: u64, game_name: String) {
    touch(&state, title_id, |e| {
        // Otro juego: lo que le faltaba al anterior puede estar aqui.
        if e.game_id != Some(game_id) {
            e.tried.clear();
        }
        e.game_id = Some(game_id);
        e.game_name = game_name;
    });
}

/// Descarga la imagen elegida como grafismo del titulo (sustituye la anterior de ese tipo) y devuelve su ruta.
#[tauri::command]
pub async fn sgdb_apply(state: State<'_, AppState>, title_id: String, kind: String, image: ImageHit) -> Result<String, String> {
    let kind = kind_arg(&kind)?;
    let base = state.sgdb_dir();
    let id = title_id.clone();
    let p = tauri::async_runtime::spawn_blocking(move || download(&base, &id, kind, &image)).await.map_err(|e| e.to_string())??;
    touch(&state, title_id, |_| {});
    Ok(p.to_string_lossy().into_owned())
}

/// Usa una imagen del PC como grafismo del titulo (se copia a la cache, como las de SteamGridDB) y devuelve su ruta.
#[tauri::command]
pub async fn sgdb_local(state: State<'_, AppState>, title_id: String, kind: String, path: String) -> Result<String, String> {
    let kind = kind_arg(&kind)?;
    let base = state.sgdb_dir();
    let id = title_id.clone();
    let p = tauri::async_runtime::spawn_blocking(move || {
        let src = PathBuf::from(&path);
        let ext = src.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let ext = match ext.as_str() {
            "png" => "png",
            "webp" => "webp",
            "jpg" | "jpeg" => "jpg",
            _ => return Err("Elige una imagen PNG, JPG o WebP".to_string()),
        };
        let len = std::fs::metadata(&src).map_err(|e| format!("No se puede leer la imagen: {e}"))?.len();
        if len > MAX_IMAGE {
            return Err("La imagen es demasiado grande".into());
        }
        let bytes = std::fs::read(&src).map_err(|e| format!("No se puede leer la imagen: {e}"))?;
        // Id por la hora: cada eleccion tiene una ruta nueva y no choca con los ids de SteamGridDB.
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        sgdb::store_image(&base, &id, kind, stamp, ext, &bytes).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    touch(&state, title_id, |_| {});
    Ok(p.to_string_lossy().into_owned())
}

/// Quita el grafismo de SteamGridDB de un tipo (no se vuelve a descargar solo).
#[tauri::command]
pub fn sgdb_clear(state: State<'_, AppState>, title_id: String, kind: String) -> Result<(), String> {
    let kind = kind_arg(&kind)?;
    sgdb::clear_image(&state.sgdb_dir(), &title_id, kind);
    touch(&state, title_id, |e| e.add_tried(kind));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_steamgriddb_hosts() {
        assert!(super::allowed("https://cdn2.steamgriddb.com/grid/a.png"));
        assert!(!super::allowed("http://cdn2.steamgriddb.com/grid/a.png"));
        assert!(!super::allowed("https://evil.com/steamgriddb.com/a.png"));
        assert!(!super::allowed("https://steamgriddb.com.evil.com/a.png"));
    }

    fn data_of(body: &str) -> Value {
        match parse_envelope(body) {
            Ok(v) => v,
            Err(_) => panic!("el sobre deberia ser valido"),
        }
    }

    #[test]
    fn game_with_null_name_and_odd_types_is_still_read() {
        let v = data_of(
            r#"{"success":true,"data":[
              {"id":5268188,"name":"Aokana","release_date":1468540800,"verified":true,"types":["steam"]},
              {"id":2,"name":null,"release_date":null,"verified":null},
              {"id":"3","name":42,"release_date":"x"},
              {"name":"sin id"},
              "basura",
              {"id":6}
            ]}"#,
        );
        let g = list_of(&v, "juego", parse_game);
        assert_eq!(g.iter().map(|g| g.id).collect::<Vec<_>>(), vec![5268188, 2, 3, 6], "los elementos sin id se saltan, no tumban la lista");
        assert_eq!(g[0].name, "Aokana");
        assert_eq!(g[0].year, Some(2016));
        assert!(g[0].verified);
        assert_eq!(g[1].name, "Juego #2", "name null");
        assert_eq!(g[1].year, None);
        assert!(!g[1].verified);
        assert_eq!(g[2].name, "42", "name numerico");
    }

    #[test]
    fn image_without_author_or_nulls_is_still_read() {
        let v = data_of(
            r#"{"success":true,"data":[
              {"id":10,"url":"https://cdn2.steamgriddb.com/grid/a.png","thumb":"https://cdn2.steamgriddb.com/thumb/a.jpg","width":600,"height":900,"mime":"image/png","style":"alternate","author":{"name":"pepe","steam64":"1"}},
              {"id":11,"url":"https://cdn2.steamgriddb.com/grid/b.png","thumb":null,"width":null,"height":"900","mime":null,"style":null},
              {"id":12,"url":"https://cdn2.steamgriddb.com/grid/c.png","author":null},
              {"id":13,"url":null},
              {"id":14,"url":"https://cdn2.steamgriddb.com/grid/d.png","author":{"name":null}}
            ]}"#,
        );
        let l = list_of(&v, "grafismo", parse_image);
        assert_eq!(l.iter().map(|i| i.id).collect::<Vec<_>>(), vec![10, 11, 12, 14], "sin url no hay imagen");
        assert_eq!((l[0].author.as_str(), l[0].width, l[0].height), ("pepe", 600, 900));
        assert_eq!(l[1].thumb, l[1].url, "sin miniatura se usa la url");
        assert_eq!((l[1].width, l[1].height, l[1].mime.as_str(), l[1].style.as_str()), (0, 900, "", ""));
        assert_eq!(l[2].author, "");
        assert_eq!(l[3].author, "");
    }

    #[test]
    fn envelope_variants() {
        assert_eq!(data_of(r#"{"success":true,"data":[]}"#), json!([]));
        assert_eq!(data_of(r#"{"success":true,"data":null}"#), Value::Null, "sin datos = lista vacia, no error");
        assert_eq!(data_of(r#"{"data":[1]}"#), json!([1]), "sin success pero con datos");
        match parse_envelope(r#"{"success":false,"errors":["Game not found"]}"#) {
            Err(Envelope::Rejected(m)) => assert_eq!(m, "Game not found"),
            _ => panic!("deberia rechazarse"),
        }
        match parse_envelope(r#"{"success":false,"errors":null}"#) {
            Err(Envelope::Rejected(m)) => assert_eq!(m, "SteamGridDB no devolvio datos"),
            _ => panic!("deberia rechazarse"),
        }
        assert!(matches!(parse_envelope("<html>429 Too Many Requests</html>"), Err(Envelope::Unreadable(_))));
        assert!(matches!(parse_envelope("[1,2]"), Err(Envelope::Unreadable(_))));
        assert!(matches!(parse_envelope(""), Err(Envelope::Unreadable(_))));
        assert!(list_of(&Value::Null, "juego", parse_game).is_empty());
    }

    #[test]
    fn steam_app_with_numeric_or_missing_fields() {
        let v = data_of(
            r#"{"success":true,"data":{"id":1,"name":"X","external_platform_data":{"steam":[{"id":"1245620","metadata":{
              "library_capsule_full":{"image2x":{"english":"abc/library_capsule_2x.jpg","french":"def.jpg"}}}}]}}}"#,
        );
        let app = parse_steam(&v).unwrap();
        assert_eq!(app.appid, 1245620);
        assert_eq!(steam_full_path(&app.meta, "library_capsule_full").as_deref(), Some("abc/library_capsule_2x.jpg"));
        assert_eq!(steam_full_path(&app.meta, "library_hero_full"), None);
        let hit = steam_hit(&app, ArtKind::Cover, false).unwrap();
        assert!(hit.url.ends_with("/1245620/abc/library_capsule_2x.jpg") && hit.style == "official");
        let hero = steam_hit(&app, ArtKind::Hero, false).unwrap();
        assert!(hero.url.ends_with("/1245620/library_hero.jpg"), "sin ruta localizada, la de siempre");
        // Id numerico, sin metadatos, sin enlace a Steam o con `steam: null`.
        assert_eq!(parse_steam(&data_of(r#"{"success":true,"data":{"external_platform_data":{"steam":[{"id":77}]}}}"#)).unwrap().appid, 77);
        assert!(parse_steam(&data_of(r#"{"success":true,"data":{"external_platform_data":{"steam":null}}}"#)).is_none());
        assert!(parse_steam(&data_of(r#"{"success":true,"data":{"external_platform_data":null}}"#)).is_none());
        assert!(parse_steam(&data_of(r#"{"success":true,"data":{}}"#)).is_none());
    }

    #[test]
    fn pending_skips_what_is_on_disk_or_known_missing() {
        let dir = std::env::temp_dir().join(format!("visor-sgdb-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _c = Cleanup(dir.clone());
        let d = &dir;
        let t = |m: &[&str]| AutoTitle { id: "pc/hades".into(), name: "Hades".into(), missing: m.iter().map(|s| s.to_string()).collect() };
        let all = ["cover", "hero", "logo"];
        assert_eq!(pending_kinds(&t(&all), None, d, false).len(), 3);
        // La interfaz va por detras: la portada ya esta en la cache.
        sgdb::store_image(d, "pc/hades", ArtKind::Cover, 1, "png", b"x").unwrap();
        assert_eq!(pending_kinds(&t(&all), None, d, false), vec![ArtKind::Hero, ArtKind::Logo]);
        // SteamGridDB no tiene logo: no vuelve a contar como pendiente, ni con `force`.
        let mut e = SgdbEntry { game_id: Some(1), auto_done: true, ..Default::default() };
        e.add_tried(ArtKind::Logo);
        assert!(pending_kinds(&t(&all), Some(&e), d, false).is_empty(), "revisado y sin force: no se toca");
        assert_eq!(pending_kinds(&t(&all), Some(&e), d, true), vec![ArtKind::Hero]);
        assert!(pending_kinds(&t(&["logo"]), Some(&e), d, true).is_empty());
        // Duplicados y tipos desconocidos.
        assert_eq!(pending_kinds(&t(&["hero", "hero", "xx"]), None, d, true), vec![ArtKind::Hero]);
    }
}
