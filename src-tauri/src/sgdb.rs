//! Cliente de la API v2 de SteamGridDB (https://www.steamgriddb.com/api/v2). Llamadas bloqueantes:
//! los comandos las ejecutan fuera del hilo de la interfaz.
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};
use visor_core::art::ArtKind;
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

#[derive(Deserialize)]
struct Resp<T> {
    #[serde(default)]
    success: bool,
    data: Option<T>,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Deserialize)]
struct RawGame {
    id: u64,
    name: String,
    release_date: Option<i64>,
    #[serde(default)]
    verified: bool,
}

#[derive(Deserialize)]
struct RawAuthor {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct RawImage {
    id: u64,
    url: String,
    #[serde(default)]
    thumb: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    mime: String,
    #[serde(default)]
    style: String,
    author: Option<RawAuthor>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(40))
        .user_agent(concat!("visor-capturas/", env!("CARGO_PKG_VERSION")))
        .build()
}

fn err_text(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(401, _) | ureq::Error::Status(403, _) => "SteamGridDB rechaza la clave de API: revisala en Ajustes".into(),
        ureq::Error::Status(404, _) => "SteamGridDB no tiene ese juego".into(),
        ureq::Error::Status(429, _) => "SteamGridDB pide esperar un poco (demasiadas peticiones)".into(),
        ureq::Error::Status(c, _) => format!("SteamGridDB respondio con un error ({c})"),
        ureq::Error::Transport(t) => format!("Sin conexion con SteamGridDB: {t}"),
    }
}

fn get<T: for<'de> Deserialize<'de>>(key: &str, url: &str) -> Result<T, String> {
    if key.trim().is_empty() {
        return Err("Falta la clave de API de SteamGridDB (Ajustes → SteamGridDB)".into());
    }
    let r: Resp<T> = agent()
        .get(url)
        .set("Authorization", &format!("Bearer {}", key.trim()))
        .call()
        .map_err(err_text)?
        .into_json()
        .map_err(|e| format!("Respuesta de SteamGridDB no valida: {e}"))?;
    match (r.success, r.data) {
        (true, Some(d)) => Ok(d),
        _ => Err(if r.errors.is_empty() { "SteamGridDB no devolvio datos".into() } else { r.errors.join(", ") }),
    }
}

/// Busca juegos por nombre (el primero es el mas probable).
pub fn search(key: &str, term: &str) -> Result<Vec<GameHit>, String> {
    let term = sgdb::search_term(term);
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let raw: Vec<RawGame> = get(key, &format!("{API}/search/autocomplete/{}", url_segment(&term)))?;
    Ok(raw
        .into_iter()
        .map(|g| GameHit {
            id: g.id,
            name: g.name,
            year: g.release_date.filter(|t| *t > 0).map(|t| (1970.0 + t as f64 / 31_556_952.0) as i32),
            verified: g.verified,
        })
        .collect())
}

/// Imagenes de un tipo para un juego (`page` empieza en 0). Lista vacia si no hay ninguna.
pub fn images(key: &str, game_id: u64, kind: ArtKind, page: u32) -> Result<Vec<ImageHit>, String> {
    let q = match kind {
        // Portadas verticales (las de Steam y sus variantes de mayor resolucion).
        ArtKind::Cover => "grids/game/{id}?dimensions=600x900,342x482,660x930&types=static",
        ArtKind::Hero => "heroes/game/{id}?types=static",
        ArtKind::Logo => "logos/game/{id}?types=static",
    };
    let url = format!("{API}/{}&nsfw=false&humor=false&page={page}", q.replace("{id}", &game_id.to_string()));
    let raw: Vec<RawImage> = match get(key, &url) {
        Ok(v) => v,
        Err(e) if e.contains("no tiene ese juego") => Vec::new(),
        Err(e) => return Err(e),
    };
    // Los originales de Steam no estan en SteamGridDB como imagenes propias: salen del CDN de Steam
    // y van los primeros, para que sean la opcion por defecto.
    let mut out = if page == 0 { steam_official(key, game_id, kind) } else { Vec::new() };
    out.extend(raw
        .into_iter()
        .map(|i| ImageHit {
            thumb: if i.thumb.is_empty() { i.url.clone() } else { i.thumb },
            id: i.id,
            url: i.url,
            width: i.width,
            height: i.height,
            mime: i.mime,
            style: i.style,
            author: i.author.map(|a| a.name).unwrap_or_default(),
        }));
    Ok(out)
}

const STEAM_CDN: &str = "https://shared.steamstatic.com/store_item_assets/steam/apps";

#[derive(Deserialize)]
struct RawGameInfo {
    #[serde(default)]
    external_platform_data: Option<RawPlatforms>,
}

#[derive(Deserialize)]
struct RawPlatforms {
    #[serde(default)]
    steam: Vec<RawSteam>,
}

#[derive(Deserialize)]
struct RawSteam {
    id: String,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

/// Ruta (relativa al appid) del asset localizado en `metadata.<field>_full`: el 2x en ingles si lo hay.
fn steam_full_path(meta: &serde_json::Value, field: &str) -> Option<String> {
    let full = meta.get(field)?;
    ["image2x", "image"].iter().find_map(|res| {
        let langs = full.get(res)?.as_object()?;
        langs.get("english").or_else(|| langs.values().next())?.as_str().map(str::to_string)
    })
}

/// Assets originales de Steam (portada, heroe o logo) del juego, si SteamGridDB lo enlaza a una app de Steam.
fn steam_official(key: &str, game_id: u64, kind: ArtKind) -> Vec<ImageHit> {
    let info: RawGameInfo = match get(key, &format!("{API}/games/id/{game_id}?platformdata=steam")) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("SteamGridDB platformdata {game_id}: {e}");
            return Vec::new();
        }
    };
    let Some(app) = info.external_platform_data.and_then(|p| p.steam.into_iter().next()) else { return Vec::new() };
    let Ok(appid) = app.id.parse::<u64>() else { return Vec::new() };
    let meta = app.metadata.unwrap_or_default();
    let (field, fallback, w, h, mime) = match kind {
        ArtKind::Cover => ("library_capsule_full", "library_600x900_2x.jpg", 600, 900, "image/jpeg"),
        ArtKind::Hero => ("library_hero_full", "library_hero.jpg", 1920, 620, "image/jpeg"),
        ArtKind::Logo => ("library_logo_full", "logo.png", 0, 0, "image/png"),
    };
    let mut paths: Vec<String> = steam_full_path(&meta, field).into_iter().collect();
    paths.push(fallback.to_string());
    // Algunos juegos no tienen todos los assets: se usa la primera ruta que exista.
    let Some(url) = paths
        .into_iter()
        .map(|p| format!("{STEAM_CDN}/{appid}/{p}"))
        .find(|u| agent().head(u).call().is_ok())
    else {
        return Vec::new();
    };
    vec![ImageHit {
        // Id propio (no choca con los de SteamGridDB) para el nombre del fichero en la cache.
        id: appid,
        thumb: url.clone(),
        url,
        width: w,
        height: h,
        mime: mime.into(),
        style: "official".into(),
        author: "Steam".into(),
    }]
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

/// Descarga una imagen y la guarda como grafismo del titulo en la cache.
pub fn download(base: &Path, title_id: &str, kind: ArtKind, img: &ImageHit) -> Result<PathBuf, String> {
    if !allowed(&img.url) {
        return Err("Esa imagen no viene de SteamGridDB".into());
    }
    let resp = agent().get(&img.url).call().map_err(err_text)?;
    let mime = resp.content_type().to_string();
    let mut bytes = Vec::new();
    resp.into_reader().take(MAX_IMAGE + 1).read_to_end(&mut bytes).map_err(|e| format!("Descarga cortada: {e}"))?;
    if bytes.len() as u64 > MAX_IMAGE {
        return Err("La imagen es demasiado grande".into());
    }
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

#[derive(Clone, Serialize)]
struct Progress {
    done: usize,
    total: usize,
    name: String,
}

#[derive(Clone, Serialize)]
struct Done {
    /// Grafismos descargados.
    images: usize,
    /// Juegos que no se encontraron en SteamGridDB.
    missing: Vec<String>,
    error: Option<String>,
}

/// Descarga en segundo plano los grafismos que faltan (la primera imagen de cada tipo del juego mas
/// probable). Sin `force` se salta los titulos ya revisados. Avisa con `sgdb-progress` y `sgdb-done`;
/// devuelve cuantos titulos va a revisar (0 = nada que hacer).
#[tauri::command]
pub fn sgdb_auto(app: AppHandle, state: State<'_, AppState>, titles: Vec<AutoTitle>, force: bool) -> Result<usize, String> {
    let key = key_of(&state);
    if key.is_empty() {
        return Err("Falta la clave de API de SteamGridDB (Ajustes → SteamGridDB)".into());
    }
    let todo: Vec<AutoTitle> = {
        let store = state.sgdb.lock().unwrap();
        titles
            .into_iter()
            .filter(|t| !t.missing.is_empty() && (force || !store.get(&t.id).map(|e| e.auto_done).unwrap_or(false)))
            .collect()
    };
    if todo.is_empty() {
        return Ok(0);
    }
    if state.sgdb_busy.swap(true, Ordering::SeqCst) {
        return Err("Ya se estan buscando grafismos en SteamGridDB".into());
    }
    let (store, busy, db, base) = (state.sgdb.clone(), state.sgdb_busy.clone(), state.db.clone(), state.sgdb_dir());
    let total = todo.len();
    std::thread::spawn(move || {
        let mut done = Done { images: 0, missing: Vec::new(), error: None };
        let mark = |id: &str, game_id: Option<u64>, game_name: String| {
            let mut s = store.lock().unwrap();
            let e = SgdbEntry { game_id, game_name, auto_done: true };
            save_entry(&db, id, &e);
            s.insert(id.to_string(), e);
        };
        'titles: for (i, t) in todo.iter().enumerate() {
            let _ = app.emit("sgdb-progress", Progress { done: i, total, name: t.name.clone() });
            let known = store.lock().unwrap().get(&t.id).cloned().unwrap_or_default();
            let (gid, gname) = match known.game_id {
                Some(g) => (g, known.game_name),
                None => match search(&key, &t.name) {
                    Ok(hits) => match hits.into_iter().next() {
                        Some(h) => (h.id, h.name),
                        None => {
                            done.missing.push(t.name.clone());
                            mark(&t.id, None, String::new());
                            continue;
                        }
                    },
                    Err(e) => {
                        done.error = Some(e);
                        break 'titles;
                    }
                },
            };
            for kind in t.missing.iter().filter_map(|k| sgdb::parse_kind(k)) {
                match images(&key, gid, kind, 0) {
                    Ok(list) => {
                        if let Some(img) = list.first() {
                            match download(&base, &t.id, kind, img) {
                                Ok(_) => done.images += 1,
                                Err(e) => eprintln!("SteamGridDB {}: {e}", t.name),
                            }
                        }
                    }
                    Err(e) => {
                        done.error = Some(e);
                        break 'titles;
                    }
                }
            }
            mark(&t.id, Some(gid), gname);
        }
        busy.store(false, Ordering::SeqCst);
        let _ = app.emit("sgdb-done", done);
    });
    Ok(total)
}

/// Busca juegos en SteamGridDB por nombre.
#[tauri::command]
pub async fn sgdb_search(state: State<'_, AppState>, term: String) -> Result<Vec<GameHit>, String> {
    let key = key_of(&state);
    tauri::async_runtime::spawn_blocking(move || search(&key, &term)).await.map_err(|e| e.to_string())?
}

/// Una pagina de imagenes de un tipo para un juego de SteamGridDB.
#[tauri::command]
pub async fn sgdb_images(state: State<'_, AppState>, game_id: u64, kind: String, page: u32) -> Result<Vec<ImageHit>, String> {
    let key = key_of(&state);
    let kind = kind_arg(&kind)?;
    tauri::async_runtime::spawn_blocking(move || images(&key, game_id, kind, page)).await.map_err(|e| e.to_string())?
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
    sgdb::clear_image(&state.sgdb_dir(), &title_id, kind_arg(&kind)?);
    touch(&state, title_id, |_| {});
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_steamgriddb_hosts() {
        assert!(super::allowed("https://cdn2.steamgriddb.com/grid/a.png"));
        assert!(!super::allowed("http://cdn2.steamgriddb.com/grid/a.png"));
        assert!(!super::allowed("https://evil.com/steamgriddb.com/a.png"));
        assert!(!super::allowed("https://steamgriddb.com.evil.com/a.png"));
    }
}
