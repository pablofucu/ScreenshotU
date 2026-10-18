//! Capturas del juego para la lectura y servidor de medios. Al llegar cada frase se toma UNA captura de la ventana del
//! juego, solo en memoria (las 3 ultimas, como JPEG reducido); se guarda en disco solo cuando el usuario pide Anki o ★,
//! asi la imagen corresponde a la frase aunque el juego ya haya avanzado. Tambien sirve a la pagina de lectura
//! `/shot/<linea>`, `/audio/<linea>/<rec|synth>` y `/media/<archivo>`.
use crate::glossary;
use crate::platform;
use crate::state::AppState;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use visor_core::capture::rgba_to_jpeg;

const KEEP: usize = 3;
const MAX_W: u32 = 960;
const QUALITY: u8 = 82;

struct Shot {
    line_id: u64,
    at: i64,
    jpeg: Arc<Vec<u8>>,
}

static RING: Mutex<VecDeque<Shot>> = Mutex::new(VecDeque::new());
static BUSY: AtomicBool = AtomicBool::new(false);

/// Olvida las capturas (juego nuevo o cerrado).
pub fn clear() {
    RING.lock().unwrap_or_else(|e| e.into_inner()).clear();
    publish();
}

fn refs() -> Vec<serde_json::Value> {
    RING.lock().unwrap_or_else(|e| e.into_inner()).iter().rev().map(|s| json!({ "lineId": s.line_id, "at": s.at })).collect()
}

fn publish() {
    let r = refs();
    crate::reader::server::set_extra("shots", json!(r));
    crate::reader::server::send(json!({ "type": "shots", "shots": r }));
}

/// Toma la captura de una frase recien llegada (en un hilo; si aun hay otra en curso, se salta).
pub fn capture_line(app: &AppHandle, line_id: u64) {
    let (pids, scaled) = {
        let st = app.state::<AppState>();
        let pids = st.active.lock().unwrap().as_ref().map(|a| a.pids.lock().unwrap().clone()).unwrap_or_default();
        let scaled = st.config.lock().unwrap().capture_magpie != "source";
        (pids, scaled)
    };
    if pids.is_empty() || BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = std::thread::Builder::new().name("reader-shot".into()).spawn(move || {
        let r = platform::grab(&pids, scaled).and_then(|g| rgba_to_jpeg(g.width, g.height, &g.rgba, MAX_W, QUALITY).map_err(|e| e.to_string()));
        BUSY.store(false, Ordering::SeqCst);
        match r {
            Ok(jpeg) => {
                {
                    let mut ring = RING.lock().unwrap_or_else(|e| e.into_inner());
                    ring.push_back(Shot { line_id, at: glossary::now_ms(), jpeg: Arc::new(jpeg) });
                    while ring.len() > KEEP {
                        ring.pop_front();
                    }
                }
                publish();
            }
            Err(e) => eprintln!("captura de lectura: {e}"),
        }
    });
}

fn in_memory(line_id: u64) -> Option<Arc<Vec<u8>>> {
    RING.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|s| s.line_id == line_id).map(|s| s.jpeg.clone())
}

/// Guarda en disco la captura de una linea (`media/shot-<frase>.jpg`) y devuelve el nombre de archivo.
pub fn save(app: &AppHandle, line_id: u64, phrase_id: &str) -> Option<String> {
    let jpeg = in_memory(line_id)?;
    let name = format!("shot-{phrase_id}.jpg");
    let dir = glossary::media_dir(app);
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::write(dir.join(&name), jpeg.as_slice()).ok()?;
    Some(name)
}

/// ★ o Anki: guarda la captura de la frase en el glosario si aun no tiene.
pub fn keep_for_phrase(app: &AppHandle, line_id: u64) {
    if in_memory(line_id).is_none() {
        return;
    }
    let _ = glossary::update_line(app, line_id, |p| {
        if let Some(f) = save(app, line_id, &p.id) {
            p.shot = Some(f);
        }
    });
}

fn mime_of(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

/// Archivo de la carpeta de medios por nombre (sin rutas).
fn media_file(app: &AppHandle, name: &str) -> Option<(String, Vec<u8>)> {
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
        return None;
    }
    let bytes = std::fs::read(glossary::media_dir(app).join(name)).ok()?;
    Some((mime_of(name).into(), bytes))
}

/// Rutas HTTP de medios de la pagina de lectura. `path` sin la query.
pub fn http(app: &AppHandle, path: &str) -> Option<(String, Vec<u8>)> {
    if let Some(id) = path.strip_prefix("/shot/") {
        let id: u64 = id.parse().ok()?;
        if let Some(j) = in_memory(id) {
            return Some(("image/jpeg".into(), j.as_ref().clone()));
        }
        let p = glossary::find_by_line(app, id)?;
        return media_file(app, p.shot.as_deref()?);
    }
    if let Some(rest) = path.strip_prefix("/audio/") {
        let (id, kind) = rest.split_once('/')?;
        let p = glossary::find_by_line(app, id.parse().ok()?)?;
        let f = if kind == "synth" { p.synth } else { p.audio }?;
        return media_file(app, &f);
    }
    if let Some(name) = path.strip_prefix("/media/") {
        return media_file(app, name);
    }
    None
}
