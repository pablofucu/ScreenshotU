//! Comandos de partidas guardadas: deteccion (Ludusavi + motores + manuales), copias y restauracion, descarga de
//! la base de Ludusavi y copia automatica al cerrar un juego. La logica esta en `visor_core::saves`.
use crate::state::AppState;
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use visor_core::games::{Game, Launch, Source};
use visor_core::saves::archive::{self, CopyKind, CopyReq, RestoreReport, SaveCopy};
use visor_core::saves::manifest::{self as mf, IndexMeta};
use visor_core::saves::{self as core, LocKind, Manifest, SaveEnv, SaveLoc};

/// Carpetas del usuario: las de entorno, con Documentos real (puede estar en OneDrive) y la carpeta de Steam.
fn sys_env() -> SaveEnv {
    let mut env = SaveEnv::from_env();
    #[cfg(windows)]
    {
        if let Some(d) = known_documents() {
            env.documents = d;
        }
        env.steam = steam_path();
    }
    env
}

#[cfg(windows)]
fn known_documents() -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath, KNOWN_FOLDER_FLAG};
    unsafe {
        let p = SHGetKnownFolderPath(&FOLDERID_Documents, KNOWN_FOLDER_FLAG(0), None).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const _));
        s.map(PathBuf::from)
    }
}

#[cfg(windows)]
fn steam_path() -> Option<PathBuf> {
    use windows::core::w;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
    let mut buf = [0u16; 1024];
    let mut len = (buf.len() * 2) as u32;
    let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, w!("Software\\Valve\\Steam"), w!("SteamPath"), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut len)) };
    if r.is_err() {
        return None;
    }
    let n = (len as usize / 2).saturating_sub(1);
    let p = PathBuf::from(String::from_utf16_lossy(&buf[..n.min(buf.len())]));
    p.is_dir().then_some(p)
}

/// Carpeta raiz de las copias: la elegida en Ajustes o `<config>/partidas`.
pub fn saves_root(state: &AppState) -> PathBuf {
    let d = state.config.lock().unwrap().saves_dir.clone();
    if d.trim().is_empty() {
        state.config_dir.join("partidas")
    } else {
        PathBuf::from(d)
    }
}

/// Base de Ludusavi en memoria (se lee del indice en cache la primera vez). `None` si aun no se descargo.
fn manifest(state: &AppState) -> Option<Arc<Manifest>> {
    let mut m = state.saves_manifest.lock().unwrap();
    if m.is_none() {
        *m = mf::load_index(&state.cache_dir).map(Arc::new);
    }
    m.clone()
}

fn game_of(state: &AppState, id: &str) -> Result<Game, String> {
    state.games.lock().unwrap().iter().find(|g| g.id == id).cloned().ok_or_else(|| "Ese juego ya no está en la lista".to_string())
}

/// Ejecutable para reconocer el juego (los emuladores no cuentan: su ejecutable es el del emulador).
fn exe_of(g: &Game) -> Option<PathBuf> {
    match (&g.source, &g.launch) {
        (Source::Manual, Launch::Exe { exe, .. }) if !exe.trim().is_empty() => Some(PathBuf::from(exe)),
        _ => None,
    }
}

fn detect(db: &visor_core::db::Db, manifest: Option<&Manifest>, g: &Game) -> core::Detection {
    let rows = crate::state::log_db("partidas", db.with(|c| core::rows_load(c, &g.id)));
    core::detect(&g.name, exe_of(g).as_deref(), manifest, &sys_env(), &rows)
}

fn now_ms() -> i64 {
    chrono::Local::now().timestamp_millis()
}
fn stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string()
}

fn enabled_locs(locs: &[SaveLoc]) -> Vec<(String, LocKind)> {
    locs.iter().filter(|l| l.enabled && l.exists).map(|l| (l.path.clone(), l.kind)).collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavesInfo {
    pub game_id: String,
    pub name: String,
    pub matched: Option<String>,
    pub engine: Option<String>,
    pub locs: Vec<SaveLoc>,
    pub copies: Vec<SaveCopy>,
    /// Carpeta de las copias de este juego.
    pub dir: String,
    /// Juegos en la base de Ludusavi (0 = sin descargar).
    pub manifest_games: usize,
    /// Tiene ejecutable con el que reconocerlo (si no, solo valen las rutas manuales).
    pub has_exe: bool,
    /// Copias automaticas que se conservan (ademas de las fijadas).
    pub keep: usize,
}

#[tauri::command]
pub async fn saves_info(state: State<'_, AppState>, id: String) -> Result<SavesInfo, String> {
    let g = game_of(&state, &id)?;
    let (db, m) = (state.db.clone(), manifest(&state));
    let dir = archive::game_dir(&saves_root(&state), &g.id, &g.name);
    tauri::async_runtime::spawn_blocking(move || {
        let det = detect(&db, m.as_deref(), &g);
        SavesInfo {
            game_id: g.id.clone(),
            name: g.name.clone(),
            matched: det.matched,
            engine: det.engine,
            locs: det.locs,
            copies: archive::list(&dir),
            dir: dir.to_string_lossy().into_owned(),
            manifest_games: m.map(|m| m.len()).unwrap_or(0),
            has_exe: exe_of(&g).is_some(),
            keep: archive::KEEP_AUTO,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn saves_set_enabled(state: State<'_, AppState>, id: String, path: String, kind: String, enabled: bool) -> Result<(), String> {
    state.db.with(|c| core::row_set_enabled(c, &id, &path, LocKind::parse(&kind), enabled)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn saves_add(state: State<'_, AppState>, id: String, path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err(format!("No existe {path}"));
    }
    state.db.with(|c| core::row_add_manual(c, &id, &path, LocKind::Path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn saves_remove(state: State<'_, AppState>, id: String, path: String) -> Result<bool, String> {
    state.db.with(|c| core::row_remove_manual(c, &id, &path)).map_err(|e| e.to_string())
}

/// Copia manual de las ubicaciones activas.
#[tauri::command]
pub async fn saves_backup(state: State<'_, AppState>, id: String) -> Result<SaveCopy, String> {
    let g = game_of(&state, &id)?;
    let (db, m) = (state.db.clone(), manifest(&state));
    let dir = archive::game_dir(&saves_root(&state), &g.id, &g.name);
    tauri::async_runtime::spawn_blocking(move || {
        let locs = enabled_locs(&detect(&db, m.as_deref(), &g).locs);
        if locs.is_empty() {
            return Err("No hay ninguna ubicación de partidas activa con archivos".to_string());
        }
        let st = stamp();
        let req = CopyReq { dir: &dir, game_id: &g.id, game_name: &g.name, locs: &locs, kind: CopyKind::Manual, now_ms: now_ms(), stamp: &st, skip_if_same: false };
        archive::create(&req).map_err(|e| format!("{e:#}"))?.ok_or_else(|| "Las ubicaciones están vacías: no hay nada que copiar".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Restaura una copia del juego (no con el juego abierto). Antes guarda lo que hay.
#[tauri::command]
pub async fn saves_restore(state: State<'_, AppState>, id: String, file: String) -> Result<RestoreReport, String> {
    if state.active.lock().unwrap().as_ref().map(|a| a.id == id).unwrap_or(false) {
        return Err("Cierra el juego antes de restaurar sus partidas".into());
    }
    let g = game_of(&state, &id)?;
    let dir = archive::game_dir(&saves_root(&state), &g.id, &g.name);
    if !Path::new(&file).starts_with(&dir) {
        return Err("Esa copia no es de este juego".into());
    }
    tauri::async_runtime::spawn_blocking(move || archive::restore(Path::new(&file), &dir, now_ms(), &stamp()).map_err(|e| format!("{e:#}")))
        .await
        .map_err(|e| e.to_string())?
}

/// Carpeta de copias del juego y ruta de una de ellas (que debe estar dentro).
fn copy_of(state: &AppState, id: &str, file: &str) -> Result<(PathBuf, PathBuf), String> {
    let g = game_of(state, id)?;
    let dir = archive::game_dir(&saves_root(state), &g.id, &g.name);
    if !Path::new(file).starts_with(&dir) {
        return Err("Esa copia no es de este juego".into());
    }
    Ok((dir, PathBuf::from(file)))
}

/// Restaura un solo archivo de una copia (antes guarda lo que hay, como la restauracion completa).
#[tauri::command]
pub async fn saves_restore_file(state: State<'_, AppState>, id: String, file: String, entry: String) -> Result<RestoreReport, String> {
    if state.active.lock().unwrap().as_ref().map(|a| a.id == id).unwrap_or(false) {
        return Err("Cierra el juego antes de restaurar sus partidas".into());
    }
    let (dir, zip) = copy_of(&state, &id, &file)?;
    tauri::async_runtime::spawn_blocking(move || archive::restore_one(&zip, &dir, now_ms(), &stamp(), &entry).map_err(|e| format!("{e:#}")))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn saves_set_note(state: State<'_, AppState>, id: String, file: String, note: String) -> Result<(), String> {
    let (dir, zip) = copy_of(&state, &id, &file)?;
    archive::set_meta(&dir, &zip, Some(&note), None).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn saves_set_pinned(state: State<'_, AppState>, id: String, file: String, pinned: bool) -> Result<(), String> {
    let (dir, zip) = copy_of(&state, &id, &file)?;
    archive::set_meta(&dir, &zip, None, Some(pinned)).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn saves_delete_copy(state: State<'_, AppState>, id: String, file: String) -> Result<(), String> {
    let (dir, zip) = copy_of(&state, &id, &file)?;
    archive::delete_copy(&dir, &zip).map_err(|e| format!("{e:#}"))
}

/// Copia el .zip de la copia a `dest` (elegido con el dialogo de guardar de la interfaz).
#[tauri::command]
pub async fn saves_export_copy(state: State<'_, AppState>, id: String, file: String, dest: String) -> Result<(), String> {
    let (_, zip) = copy_of(&state, &id, &file)?;
    tauri::async_runtime::spawn_blocking(move || std::fs::copy(&zip, &dest).map(|_| ()).map_err(|e| format!("No se pudo exportar la copia: {e}")))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyLocInfo {
    pub path: String,
    pub kind: LocKind,
    /// Nombre corto legible («AppData › RenPy › Juego»).
    pub place: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyFilesInfo {
    pub locs: Vec<CopyLocInfo>,
    pub files: Vec<archive::CopyFile>,
}

/// Archivos de una copia comparados con los de ahora.
#[tauri::command]
pub async fn saves_copy_files(state: State<'_, AppState>, id: String, file: String) -> Result<CopyFilesInfo, String> {
    let (_, zip) = copy_of(&state, &id, &file)?;
    let g = game_of(&state, &id)?;
    let (db, m) = (state.db.clone(), manifest(&state));
    tauri::async_runtime::spawn_blocking(move || {
        let r = archive::compare(&zip).map_err(|e| format!("{e:#}"))?;
        // Los nombres cortos salen de las ubicaciones detectadas; si ya no se detecta alguna, de la carpeta del juego.
        let det = detect(&db, m.as_deref(), &g);
        let env = sys_env();
        let gdir = exe_of(&g).and_then(|e| e.parent().map(Path::to_path_buf));
        let locs = r
            .locs
            .into_iter()
            .map(|l| {
                let place = det
                    .locs
                    .iter()
                    .find(|d| core::path_key(&d.path) == core::path_key(&l.path))
                    .map(|d| d.place.clone())
                    .unwrap_or_else(|| core::place_name(&l.path, l.kind, &env, gdir.as_deref()));
                CopyLocInfo { path: l.path, kind: l.kind, place }
            })
            .collect();
        Ok(CopyFilesInfo { locs, files: r.files })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Abre en el Explorador la carpeta de copias del juego.
#[tauri::command]
pub fn saves_open_dir(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let g = game_of(&state, &id)?;
    let dir = archive::game_dir(&saves_root(&state), &g.id, &g.name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    crate::launcher::open_folder(&dir)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavesStatus {
    pub games: usize,
    pub fetched_ms: i64,
    pub default_dir: String,
    pub busy: bool,
}

fn status_of(state: &AppState) -> SavesStatus {
    let meta = mf::load_meta(&state.cache_dir).unwrap_or_default();
    SavesStatus {
        games: meta.games,
        fetched_ms: meta.fetched_ms,
        default_dir: state.config_dir.join("partidas").to_string_lossy().into_owned(),
        busy: state.saves_busy.load(Ordering::SeqCst),
    }
}

#[tauri::command]
pub fn saves_status(state: State<'_, AppState>) -> SavesStatus {
    status_of(&state)
}

/// Descarga (o actualiza, si cambio) la base de Ludusavi y la resume en el indice de la cache.
#[tauri::command]
pub async fn saves_update_manifest(state: State<'_, AppState>) -> Result<SavesStatus, String> {
    if state.saves_busy.swap(true, Ordering::SeqCst) {
        return Err("Ya se está descargando la base de Ludusavi".into());
    }
    let cache = state.cache_dir.clone();
    let old = mf::load_meta(&cache).filter(|_| cache.join(mf::INDEX_FILE).is_file());
    let r = tauri::async_runtime::spawn_blocking(move || -> Result<Option<Vec<mf::MEntry>>, String> {
        let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(120)).build();
        let mut req = agent.get(mf::MANIFEST_URL);
        if let Some(e) = old.as_ref().and_then(|m| m.etag.as_deref()) {
            req = req.set("If-None-Match", e);
        }
        let resp = req.call().map_err(|e| format!("No se pudo descargar la base de Ludusavi: {e}"))?;
        let mut meta = old.unwrap_or_default();
        meta.fetched_ms = now_ms();
        if resp.status() == 304 {
            mf::save_meta(&cache, &meta).map_err(|e| e.to_string())?;
            return Ok(None);
        }
        let etag = resp.header("ETag").map(str::to_string);
        let mut yaml = String::new();
        resp.into_reader().read_to_string(&mut yaml).map_err(|e| format!("Descarga interrumpida: {e}"))?;
        let idx = mf::build_index(&yaml).map_err(|e| format!("{e:#}"))?;
        mf::save_index(&cache, &idx, &IndexMeta { etag, fetched_ms: meta.fetched_ms, games: idx.len() }).map_err(|e| e.to_string())?;
        Ok(Some(idx))
    })
    .await
    .map_err(|e| e.to_string());
    state.saves_busy.store(false, Ordering::SeqCst);
    if let Some(idx) = r?? {
        *state.saves_manifest.lock().unwrap() = Some(Arc::new(Manifest::new(idx)));
    }
    Ok(status_of(&state))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SavesCopied {
    id: String,
    name: String,
    message: String,
    error: bool,
}

/// Al cerrar un juego: si esta activada la copia automatica y sus partidas cambiaron, hace una copia y poda las
/// automaticas antiguas. En un hilo aparte; avisa a la interfaz solo si copio algo o fallo.
pub fn after_session(app: &AppHandle, game: &Game) {
    let st = app.state::<AppState>();
    if !st.config.lock().unwrap().saves_auto {
        return;
    }
    let (app, g) = (app.clone(), game.clone());
    std::thread::spawn(move || {
        let st = app.state::<AppState>();
        let m = manifest(&st);
        let locs = enabled_locs(&detect(&st.db, m.as_deref(), &g).locs);
        if locs.is_empty() {
            return;
        }
        let dir = archive::game_dir(&saves_root(&st), &g.id, &g.name);
        let s = stamp();
        let req = CopyReq { dir: &dir, game_id: &g.id, game_name: &g.name, locs: &locs, kind: CopyKind::Auto, now_ms: now_ms(), stamp: &s, skip_if_same: true };
        let ev = match archive::create(&req) {
            Ok(None) => return,
            Ok(Some(_)) => {
                archive::prune(&dir, archive::KEEP_AUTO);
                SavesCopied { id: g.id.clone(), name: g.name.clone(), message: format!("Partidas de {} copiadas", g.name), error: false }
            }
            Err(e) => SavesCopied { id: g.id.clone(), name: g.name.clone(), message: format!("No se pudieron copiar las partidas de {}: {e:#}", g.name), error: true },
        };
        let _ = app.emit("saves-copied", ev);
    });
}
