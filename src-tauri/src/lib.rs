//! Capa Tauri: solo traduce comandos de la interfaz a llamadas a `visor-core`.
mod ai;
mod anki;
mod audio;
mod backup;
mod display;
mod glossary;
mod launcher;
mod library;
mod magpie;
mod mouse;
mod novel;
mod outputs;
mod overlay;
mod pad;
mod peek;
mod platform;
mod reader;
mod remote;
mod saves;
mod sgdb;
mod shots;
mod snap;
mod state;
mod texthook;
mod voice;

use state::AppState;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use visor_core::capture::{combo_mask, mouse_button_index};
use visor_core::config::Config;
use visor_core::db::{data as dbd, Db};
use visor_core::art::ArtKind;
use visor_core::games::{self, ManualGame};
use visor_core::gamedirs;
use visor_core::importer::{self, ArtCopy, ImportReport};
use visor_core::palette::PaletteCache;
use visor_core::scan::ScanOpts;
use visor_core::userdata::{self, HiddenEntry};
use visor_core::{catalog, relpath, scan, thumbs, Library};

/// El protocolo `asset:` solo sirve rutas dentro de un ambito. La raiz la elige el usuario en
/// tiempo de ejecucion, asi que se autoriza aqui (solo lectura de imagenes por la webview).
fn allow_root(app: &AppHandle, root: &str) {
    let _ = app.asset_protocol_scope().allow_directory(Path::new(root), true);
}

/// Registra (o quita) los atajos globales: captura de pantalla, grabacion del audio del juego y cambio de salida de
/// audio. Si uno falla se sigue con los demas y se devuelven todos los errores. Un atajo repetido solo se registra una vez.
fn apply_hotkeys(app: &AppHandle, capture: &str, record: &str, audio: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::Shortcut;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut errs = Vec::new();
    let mut done: Vec<Shortcut> = Vec::new();
    for (key, what) in [(capture, "captura"), (record, "grabación"), (audio, "cambio de audio")] {
        if key.trim().is_empty() {
            continue;
        }
        if let Ok(sc) = key.parse::<Shortcut>() {
            if done.contains(&sc) {
                continue;
            }
            done.push(sc);
        }
        if let Err(e) = gs.register(key) {
            errs.push(format!("No se pudo registrar el atajo de {what} {key}: {e}"));
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join(" "))
    }
}

/// Dos atajos distintos no pueden ser el mismo (el segundo no se registraria).
fn hotkey_clash(a: &str, b: &str) -> bool {
    use tauri_plugin_global_shortcut::Shortcut;
    !a.trim().is_empty() && matches!((a.parse::<Shortcut>(), b.parse::<Shortcut>()), (Ok(x), Ok(y)) if x == y)
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(app: AppHandle, state: State<'_, AppState>, config: Config) -> Result<(), String> {
    if let Some(r) = &config.root {
        allow_root(&app, r);
    }
    let mut config = config;
    let (old_hotkey, old_record, old_audio) = {
        let c = state.config.lock().unwrap();
        (c.capture_hotkey.clone(), c.record_hotkey.clone(), c.audio_hotkey.clone())
    };
    // Si el atajo nuevo no se puede registrar (otro programa lo usa), se mantiene el anterior y se avisa.
    let mut hotkey_err = None;
    if old_hotkey != config.capture_hotkey || old_record != config.record_hotkey || old_audio != config.audio_hotkey {
        let clash = if hotkey_clash(&config.capture_hotkey, &config.record_hotkey) {
            Some("El atajo de grabación no puede ser el mismo que el de captura.")
        } else if hotkey_clash(&config.capture_hotkey, &config.audio_hotkey) || hotkey_clash(&config.record_hotkey, &config.audio_hotkey) {
            Some("El atajo de audio no puede repetir el de captura ni el de grabación.")
        } else {
            None
        };
        let r = match clash {
            Some(m) => Err(m.to_string()),
            None => apply_hotkeys(&app, &config.capture_hotkey, &config.record_hotkey, &config.audio_hotkey),
        };
        if let Err(e) = r {
            let _ = apply_hotkeys(&app, &old_hotkey, &old_record, &old_audio);
            config.capture_hotkey = old_hotkey;
            config.record_hotkey = old_record;
            config.audio_hotkey = old_audio;
            hotkey_err = Some(e);
        }
    }
    // El codigo del movil lo gestiona el nucleo (`remote_new_code`): se ignora el que mande la interfaz.
    config.remote_token = state.config.lock().unwrap().remote_token.clone();
    if config.remote_enabled && config.remote_token.is_empty() {
        config.remote_token = visor_core::remote::new_code();
    }
    // Las capturas van siempre en la biblioteca: la interfaz no elige la raiz.
    config.root = state.config.lock().unwrap().root.clone();
    // Sin biblioteca encontrada no se escribe nada (crearia la carpeta en un disco que no esta).
    if state.lib_mode != state::LibMode::Missing {
        config.save(&state.config_file()).map_err(|e| e.to_string())?;
    }
    state.combo_mask.store(combo_mask(&config.capture_combo), Ordering::Relaxed);
    state.audio_mask.store(combo_mask(&config.audio_combo), Ordering::Relaxed);
    state.mouse_button.store(mouse_button_index(&config.capture_mouse), Ordering::Relaxed);
    let root_changed = state.config.lock().unwrap().root != config.root;
    let new_root = config.root.clone();
    *state.config.lock().unwrap() = config;
    if root_changed {
        *state.favorites.lock().unwrap() = load_favorites(&state.db, new_root.as_deref().map(Path::new));
    }
    remote::apply(&app);
    hotkey_err.map_or(Ok(()), Err)
}

/// Detecta juegos y escanea toda la biblioteca (tambien lo oculto, marcado `hidden`), sincroniza el catalogo y
/// guarda la cache de acentos. Bloquea: llamarla fuera del hilo de la interfaz. La usan `scan_library` y el
/// servidor del movil, para que los dos vean lo mismo. Devuelve la biblioteca, los juegos detectados y la raiz.
pub(crate) fn scan_full(state: &AppState) -> (Library, Vec<games::Game>, Config) {
    let cfg = state.config.lock().unwrap().clone();
    let manual = state::log_db("juegos", state.db.with(dbd::manual_load));
    let stats = state.stats.lock().unwrap().clone();
    let hidden = state.hidden.lock().unwrap().clone();
    let prefs = state.prefs.lock().unwrap().clone();
    let sgdb_dir = state.sgdb_dir();
    let found = games::all_games(&manual);
    let opts = ScanOpts {
        hidden: &hidden,
        // Se escanea todo (tambien lo oculto) para que el catalogo este completo; se filtra despues.
        show_hidden: true,
        prefs: Some(&prefs),
        palettes: Some(&state.palettes),
        sgdb_dir: Some(&sgdb_dir),
    };
    let root = cfg.root.as_deref().map(Path::new);
    let lib = scan::scan_library_ex(root, &cfg.platforms, &found, &stats, &opts);
    state.palettes.save(&state.palettes_file());
    if let Some(r) = root {
        let now = chrono::Local::now().timestamp_millis();
        state::log_db_err("catalogo", state.db.with(|c| catalog::sync(c, r, &lib, now)).map(|_| ()));
    }
    (lib, found, cfg)
}

/// Detecta juegos y escanea la biblioteca en un hilo aparte para no bloquear la interfaz.
#[tauri::command]
async fn scan_library(app: AppHandle, state: State<'_, AppState>) -> Result<Library, String> {
    if let Some(r) = &state.config.lock().unwrap().root {
        allow_root(&app, r);
    }
    let app2 = app.clone();
    let (mut lib, found, cfg) = tauri::async_runtime::spawn_blocking(move || scan_full(&app2.state::<AppState>()))
        .await
        .map_err(|e| e.to_string())?;
    if !cfg.show_hidden {
        for p in lib.platforms.iter_mut() {
            p.titles.retain(|t| !t.hidden);
        }
    }
    *state.games.lock().unwrap() = found;
    if let Some(r) = cfg.root {
        start_hasher(state.db.clone(), PathBuf::from(r), state.hashing.clone());
    }
    Ok(lib)
}

/// Calcula en segundo plano las huellas que falten en el catalogo (una sola tanda a la vez).
fn start_hasher(db: Arc<Db>, root: PathBuf, busy: Arc<std::sync::atomic::AtomicBool>) {
    if busy.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let r = catalog::hash_pending(&db, &root, || false);
        busy.store(false, Ordering::SeqCst);
        match r {
            Ok(0) => {}
            Ok(n) => eprintln!("catalogo: {n} huellas nuevas"),
            Err(e) => eprintln!("catalogo: {e:#}"),
        }
    });
}

/// Favoritas en memoria (rutas absolutas). En la base se guardan portables (relativas a la raiz):
/// al cargar se pasan a portables las absolutas que caigan dentro de la raiz y se resuelven todas.
fn load_favorites(db: &Db, root: Option<&Path>) -> Vec<String> {
    if let Some(r) = root {
        state::log_db_err("favoritas", db.with(|c| dbd::favorites_relativize(c, r)).map(|_| ()));
    }
    state::log_db("favoritas", db.with(dbd::favorites_load)).iter().map(|p| relpath::resolved(root, p)).collect()
}

/// Raiz configurada, si la hay.
fn root_of(state: &AppState) -> Option<PathBuf> {
    state.config.lock().unwrap().root.as_ref().map(PathBuf::from)
}

/// Devuelve la ruta de la miniatura de una imagen (la genera si no existe).
#[tauri::command]
async fn get_thumb(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let dir = state.thumbs_dir();
    tauri::async_runtime::spawn_blocking(move || {
        thumbs::ensure_thumb_pooled(&dir, Path::new(&path))
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Clone, serde::Serialize)]
struct ThumbReady {
    src: String,
    thumb: String,
}

/// Precalienta miniaturas en el pool de rayon; avisa con el evento `thumb-ready`.
#[tauri::command]
fn prefetch_thumbs(app: AppHandle, state: State<'_, AppState>, paths: Vec<String>) {
    thumbs::prefetch(state.thumbs_dir(), paths, move |src, th| {
        let _ = app.emit(
            "thumb-ready",
            ThumbReady { src: src.to_string(), thumb: th.to_string_lossy().into_owned() },
        );
    });
}

#[tauri::command]
fn get_favorites(state: State<'_, AppState>) -> Vec<String> {
    state.favorites.lock().unwrap().clone()
}

#[tauri::command]
fn set_favorite(state: State<'_, AppState>, path: String, on: bool) -> Result<(), String> {
    let mut f = state.favorites.lock().unwrap();
    f.retain(|p| p != &path);
    if on {
        f.push(path.clone());
    }
    let key = relpath::stored(root_of(&state).as_deref(), &path);
    state.db.with(|c| dbd::favorite_set(c, &key, on)).map_err(|e| e.to_string())
}

/// Lanza un juego detectado por el ultimo escaneo.
#[tauri::command]
fn launch_game(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let game = state.games.lock().unwrap().iter().find(|g| g.id == id).cloned().ok_or("Juego no encontrado: vuelve a escanear")?;
    launcher::start(&app, game)
}

/// Generacion del aviso de meta pendiente: uno nuevo (otra partida) anula el anterior.
static GOAL_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Aviso sobre el juego cuando la partida en curso llegue a la meta diaria de lectura. La app calcula la hora (`at`,
/// ms reales) con las partidas de hoy; aqui se espera y, si la partida de `id` sigue en marcha, sale el cartel.
#[tauri::command]
fn goal_notice(app: AppHandle, id: String, at: i64, text: String) {
    let gen = GOAL_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = std::thread::Builder::new().name("goal-notice".into()).spawn(move || loop {
        if GOAL_GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        let same = app.state::<AppState>().active.lock().unwrap().as_ref().map(|a| a.id == id).unwrap_or(false);
        if !same {
            return;
        }
        let now = chrono_now_ms();
        if now >= at {
            overlay::goal(&text);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis((at - now).clamp(50, 5000) as u64));
    });
}

/// Abre en el explorador la carpeta de capturas de un titulo (solo dentro de la raiz configurada).
#[tauri::command]
fn open_folder(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let root = state.config.lock().unwrap().root.clone().ok_or("Sin carpeta raiz")?;
    let p = PathBuf::from(&path);
    if !games::path_inside(&path, &root) {
        return Err("Esa carpeta esta fuera de la biblioteca".into());
    }
    if !p.is_dir() {
        return Err("La carpeta aun no existe: se creara con la primera captura".into());
    }
    launcher::open_folder(&p)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewGame {
    /// "exe" | "emulator" | "captures"
    kind: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    exe: String,
    #[serde(default)]
    args: String,
    #[serde(default)]
    rom: String,
    #[serde(default)]
    platform: String,
    /// Grafismos opcionales que se COPIAN a la carpeta del juego.
    #[serde(default)]
    hero: String,
    #[serde(default)]
    logo: String,
    #[serde(default)]
    cover: String,
}

#[derive(serde::Serialize)]
struct AddReport {
    /// Avisos para el usuario (grafismos que ya existian, que no se pudieron copiar...).
    notes: Vec<String>,
}

/// Anade una entrada a la tabla de juegos (juego .exe, emulador + ROM o solo capturas) y copia los grafismos elegidos a la carpeta del juego. Nunca mueve ni modifica los originales.
#[tauri::command]
fn add_game(state: State<'_, AppState>, game: NewGame) -> Result<AddReport, String> {
    let cfg = state.config.lock().unwrap().clone();
    let name = game.name.trim().to_string();
    let mut notes = Vec::new();
    let mut entry = ManualGame { name: name.clone(), kind: game.kind.clone(), platform: game.platform.clone(), ..Default::default() };
    match game.kind.as_str() {
        "exe" | "emulator" | "captures" => {}
        _ => return Err("Tipo de juego desconocido".into()),
    }
    if name.is_empty() {
        return Err("Falta el nombre".into());
    }
    let plat = cfg.platforms.iter().find(|p| p.id == game.platform).ok_or("Plataforma desconocida")?.clone();
    match game.kind.as_str() {
        "exe" | "emulator" => {
            if !Path::new(&game.exe).is_file() {
                return Err("El ejecutable no existe".into());
            }
            if game.kind == "emulator" && !Path::new(&game.rom).is_file() {
                return Err("La ROM no existe".into());
            }
            entry.id = games::new_manual_id();
            entry.exe = game.exe.clone();
            entry.args = game.args.clone();
            entry.rom = game.rom.clone();
        }
        _ => entry.id = games::new_manual_id(),
    }
    let arts = [(ArtKind::Hero, &game.hero, "hero"), (ArtKind::Logo, &game.logo, "logo"), (ArtKind::Cover, &game.cover, "portada")];
    let want_dir = game.kind == "captures" || arts.iter().any(|(_, p, _)| !p.is_empty());
    if want_dir {
        let root = cfg.root.clone().ok_or("Elige primero la carpeta de la biblioteca")?;
        let root_p = Path::new(&root);
        let dir = importer::title_dir(root_p, &plat.folder, &games::sanitize_folder_name(&name));
        if !importer::is_within_root(root_p, &dir) {
            return Err("La carpeta de capturas no existe o esta fuera de la biblioteca".into());
        }
        std::fs::create_dir_all(&dir).map_err(|e| format!("No se pudo crear la carpeta del juego: {e}"))?;
        for (kind, src, label) in arts {
            if src.is_empty() {
                continue;
            }
            match importer::copy_art(&dir, kind, Path::new(src)) {
                Ok(ArtCopy::Copied(_)) => {}
                Ok(ArtCopy::Exists) => notes.push(format!("Ya habia un {label} en la carpeta: no se sobrescribe")),
                Err(e) => notes.push(format!("{label}: {e}")),
            }
        }
    }
    state.db.with(|c| dbd::manual_add(c, &entry)).map_err(|e| e.to_string())?;
    Ok(AddReport { notes })
}

/// Busca juegos en las subcarpetas de un directorio (una por juego) para el asistente de carpeta de juegos.
#[tauri::command]
async fn scan_games_dir(state: State<'_, AppState>, dir: String) -> Result<gamedirs::DirScan, String> {
    let existing = state.db.with(dbd::manual_load).map_err(|e| e.to_string())?;
    if !Path::new(&dir).is_dir() {
        return Err("Esa carpeta no existe".into());
    }
    tauri::async_runtime::spawn_blocking(move || gamedirs::scan_game_dirs(Path::new(&dir), &existing))
        .await
        .map_err(|e| e.to_string())
}

/// Oculta o muestra un titulo. Solo escribe en la base; los archivos del juego no se tocan.
#[tauri::command]
fn set_hidden(state: State<'_, AppState>, id: String, name: String, platform: String, hidden: bool) -> Result<(), String> {
    let mut l = state.hidden.lock().unwrap();
    let entry = HiddenEntry { id, name, platform };
    userdata::set_hidden(&mut l, entry.clone(), hidden);
    state.db.with(|c| dbd::hidden_set(c, &entry, hidden)).map_err(|e| e.to_string())
}

/// Fija (o quita con `None`) el color de acento manual de un titulo.
#[tauri::command]
fn set_accent_override(state: State<'_, AppState>, id: String, color: Option<String>) -> Result<(), String> {
    let mut p = state.prefs.lock().unwrap();
    userdata::set_accent(&mut p, &id, color.as_deref())?;
    state.db.with(|c| dbd::pref_save(c, &id, p.get(&id))).map_err(|e| e.to_string())
}

/// Recuerda el orden de capturas de un titulo.
#[tauri::command]
fn set_sort_pref(state: State<'_, AppState>, id: String, mode: String) -> Result<(), String> {
    let mut p = state.prefs.lock().unwrap();
    userdata::set_sort(&mut p, &id, &mode)?;
    state.db.with(|c| dbd::pref_save(c, &id, p.get(&id))).map_err(|e| e.to_string())
}

/// Estado de un titulo ("none" | "pending" | "playing" | "finished" | "dropped"). Terminado guarda la fecha.
#[tauri::command]
fn set_title_status(state: State<'_, AppState>, id: String, status: String) -> Result<(), String> {
    let now = chrono_now_ms();
    let mut p = state.prefs.lock().unwrap();
    userdata::set_status(&mut p, &id, &status, now)?;
    let cur = p.get(&id).and_then(|x| x.status.as_deref().map(|s| (s.to_string(), x.finished_ms)));
    state.db.with(|c| dbd::status_save(c, &id, cur.as_ref().map(|(s, f)| (s.as_str(), *f)), now)).map_err(|e| e.to_string())
}

fn chrono_now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Elige el escalado con Magpie de un juego ("off" | "auto" | "2x" | "4x"). Se aplica al lanzarlo.
#[tauri::command]
fn set_magpie_mode(state: State<'_, AppState>, id: String, mode: String) -> Result<(), String> {
    let mut p = state.prefs.lock().unwrap();
    userdata::set_magpie_mode(&mut p, &id, &mode)?;
    state.db.with(|c| dbd::pref_save(c, &id, p.get(&id))).map_err(|e| e.to_string())
}

/// Donde esta Magpie, si esta abierto y si tiene los modelos CuNNy2.
#[tauri::command]
async fn magpie_status(state: State<'_, AppState>) -> Result<magpie::MagpieStatus, String> {
    let configured = state.config.lock().unwrap().magpie_exe.clone();
    tauri::async_runtime::spawn_blocking(move || magpie::status(&configured)).await.map_err(|e| e.to_string())
}

/// Sagas guardadas (agrupaciones virtuales de juegos).
#[tauri::command]
fn get_sagas(state: State<'_, AppState>) -> Vec<visor_core::sagas::Saga> {
    state::log_db("sagas", state.db.with(dbd::sagas_load))
}

/// Guarda la lista completa de sagas y devuelve la lista saneada. No toca capturas.
#[tauri::command]
fn save_sagas(state: State<'_, AppState>, sagas: Vec<visor_core::sagas::Saga>) -> Result<Vec<visor_core::sagas::Saga>, String> {
    let clean = visor_core::sagas::sanitize(&sagas);
    state.db.with(|c| dbd::sagas_replace(c, &clean)).map_err(|e| e.to_string())?;
    Ok(clean)
}

#[derive(Clone, serde::Serialize)]
struct Progress {
    done: usize,
    total: usize,
}

/// Copia imagenes a la carpeta de un titulo (nombre `Carpeta_AAAA-MM-DD_HH-MM-SS.ext`). Emite `import-progress`.
#[tauri::command]
async fn import_shots(app: AppHandle, state: State<'_, AppState>, dir: String, paths: Vec<String>) -> Result<ImportReport, String> {
    let root = state.config.lock().unwrap().root.clone().ok_or("Elige primero la carpeta de la biblioteca")?;
    tauri::async_runtime::spawn_blocking(move || {
        let sources: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        importer::import_shots(Path::new(&root), Path::new(&dir), &sources, |done, total| {
            let _ = app.emit("import-progress", Progress { done, total });
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
struct MoveReport {
    /// Pares (ruta anterior, ruta nueva).
    moved: Vec<(String, String)>,
    failed: Vec<String>,
}

/// Mueve capturas a la carpeta de otro titulo, siempre dentro de la raiz de la biblioteca.
#[tauri::command]
fn move_shots(state: State<'_, AppState>, paths: Vec<String>, target_dir: String) -> Result<MoveReport, String> {
    let root = state.config.lock().unwrap().root.clone().ok_or("Sin carpeta raiz")?;
    let root_p = Path::new(&root);
    let mut rep = MoveReport { moved: Vec::new(), failed: Vec::new() };
    for p in paths {
        match importer::move_shot(root_p, Path::new(&p), Path::new(&target_dir)) {
            Ok(n) => rep.moved.push((p, n.to_string_lossy().into_owned())),
            Err(e) => rep.failed.push(format!("{}: {e}", Path::new(&p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(p.clone()))),
        }
    }
    // Las favoritas y la fila del catalogo (con su huella) siguen a la captura.
    let mut f = state.favorites.lock().unwrap();
    for (old, new) in &rep.moved {
        let (old_k, new_k) = (relpath::stored(Some(root_p), old), relpath::stored(Some(root_p), new));
        if let Some(i) = f.iter().position(|x| x == old) {
            f[i] = new.clone();
            state.db.with(|c| dbd::favorite_rename(c, &old_k, &new_k)).map_err(|e| e.to_string())?;
        }
        state::log_db_err("catalogo", state.db.with(|c| catalog::rename(c, &old_k, &new_k)));
    }
    Ok(rep)
}

/// Muestra una captura en el Explorador (solo dentro de la raiz).
#[tauri::command]
fn reveal_in_explorer(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let root = state.config.lock().unwrap().root.clone().ok_or("Sin carpeta raiz")?;
    let p = PathBuf::from(&path);
    if !importer::is_within_root(Path::new(&root), &p) || !p.exists() {
        return Err("Ese archivo no esta en la biblioteca".into());
    }
    launcher::reveal(&p)
}

/// Quita una entrada manual de la tabla de juegos (no toca ningun archivo del usuario).
#[tauri::command]
fn remove_game(state: State<'_, AppState>, id: String) -> Result<(), String> {
    if !state.db.with(|c| dbd::manual_remove(c, &id)).map_err(|e| e.to_string())? {
        return Err("Solo se pueden quitar los juegos anadidos a mano".into());
    }
    Ok(())
}

/// Pasa un juego anadido a mano a otra categoria: mueve su carpeta de capturas (si existe) y cambia su
/// plataforma. El id no cambia, asi que horas, estado, sagas y frases lo siguen solos.
#[tauri::command]
fn move_game_platform(state: State<'_, AppState>, id: String, platform: String) -> Result<(), String> {
    let cfg = state.config.lock().unwrap().clone();
    let new_plat = cfg.platforms.iter().find(|p| p.id == platform).ok_or("Categoria desconocida")?.clone();
    let list = state.db.with(dbd::manual_load).map_err(|e| e.to_string())?;
    let g = list.iter().find(|g| g.id == id).ok_or("Solo se pueden mover los juegos anadidos a mano")?;
    if g.platform == platform {
        return Err("Ya esta en esa categoria".into());
    }
    let old_plat = cfg.platforms.iter().find(|p| p.id == g.platform).ok_or("Categoria de origen desconocida")?;
    let folder = g.folder.clone().filter(|f| !f.trim().is_empty()).unwrap_or_else(|| games::sanitize_folder_name(&g.name));

    let mut moved: Option<(PathBuf, PathBuf)> = None;
    if let Some(root) = cfg.root.as_deref() {
        let root_p = Path::new(root);
        let src = importer::title_dir(root_p, &old_plat.folder, &folder);
        if src.is_dir() {
            let dst = importer::title_dir(root_p, &new_plat.folder, &folder);
            if dst.exists() {
                return Err(format!("Ya hay una carpeta «{folder}» en {}", new_plat.name));
            }
            if !importer::is_within_root(root_p, &src) || !importer::is_within_root(root_p, &dst) {
                return Err("La carpeta esta fuera de la biblioteca".into());
            }
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("No se pudo crear la carpeta de la categoria: {e}"))?;
            }
            std::fs::rename(&src, &dst).map_err(|e| format!("No se pudo mover la carpeta (¿esta abierta en otro programa?): {e}"))?;
            moved = Some((src, dst));
        }
    }
    if let Err(e) = state.db.with(|c| dbd::manual_set_platform(c, &id, &platform)) {
        if let Some((src, dst)) = &moved {
            let _ = std::fs::rename(dst, src);
        }
        return Err(e.to_string());
    }
    for h in state.hidden.lock().unwrap().iter_mut().filter(|h| h.id == id) {
        h.platform = platform.clone();
    }
    if let (Some((src, dst)), Some(root)) = (&moved, cfg.root.as_deref()) {
        let root_p = Path::new(root);
        let (old_k, new_k) = (relpath::stored(Some(root_p), &src.to_string_lossy()), relpath::stored(Some(root_p), &dst.to_string_lossy()));
        state::log_db_err("catalogo", state.db.with(|c| dbd::dir_moved(c, &old_k, &new_k)));
        for f in state.favorites.lock().unwrap().iter_mut() {
            if let Some(rest) = relpath::to_rel(root_p, f).and_then(|r| r.strip_prefix(&format!("{old_k}/")).map(str::to_string)) {
                *f = relpath::to_abs(root_p, &format!("{new_k}/{rest}"));
            }
        }
    }
    Ok(())
}

/// Hace una captura ahora (igual que el atajo o el mando).
#[tauri::command]
fn capture_now(app: AppHandle) {
    snap::trigger(&app);
}

/// Salidas de audio de Windows (reproduccion) activas, con la predeterminada marcada.
#[tauri::command]
async fn audio_outputs() -> Result<Vec<outputs::AudioOutput>, String> {
    tauri::async_runtime::spawn_blocking(outputs::list).await.map_err(|e| e.to_string())?
}

/// Pone una salida como predeterminada de Windows. Devuelve la lista actualizada.
#[tauri::command]
async fn set_audio_output(app: AppHandle, id: String) -> Result<Vec<outputs::AudioOutput>, String> {
    tauri::async_runtime::spawn_blocking(move || outputs::select(&app, &id)).await.map_err(|e| e.to_string())?
}

/// Ajustes espera una combinacion del mando (`on`) o deja de esperarla. El hilo del mando emite `learn-pad`.
#[tauri::command]
fn learn_pad(state: State<'_, AppState>, on: bool) {
    state.learning_pad.store(on, Ordering::Relaxed);
}

/// Suelta el atajo global mientras Ajustes espera una tecla (si no, Windows se la come) y lo vuelve a
/// registrar despues.
#[tauri::command]
fn hotkey_suspend(app: AppHandle, state: State<'_, AppState>, suspend: bool) -> Result<(), String> {
    if suspend {
        let _ = app.global_shortcut().unregister_all();
        Ok(())
    } else {
        let (key, rec, audio) = {
            let c = state.config.lock().unwrap();
            (c.capture_hotkey.clone(), c.record_hotkey.clone(), c.audio_hotkey.clone())
        };
        apply_hotkeys(&app, &key, &rec, &audio)
    }
}

/// Lleva la ventana del visor (a pantalla completa si lo estaba) a un monitor.
fn move_window_to(w: &tauri::WebviewWindow, m: &display::Monitor) {
    let full = w.is_fullscreen().unwrap_or(false);
    if full {
        let _ = w.set_fullscreen(false);
    }
    let _ = w.set_position(tauri::PhysicalPosition::new(m.x + 40, m.y + 40));
    if full {
        let _ = w.set_fullscreen(true);
    }
}

/// Texto del monitor donde esta el visor para Ajustes («1 de 2 · 2560×1440 · principal»).
#[tauri::command]
fn window_monitor(app: AppHandle) -> String {
    let mons = display::monitors();
    let cur = display::of_window(launcher::main_hwnd(&app));
    match cur.and_then(|c| mons.iter().position(|m| m.device == c.device).map(|i| (i, c))) {
        Some((i, c)) => c.label(i, mons.len()),
        None => "Desconocido".into(),
    }
}

/// Pasa el visor al monitor siguiente (`dir` = 1) o anterior (-1) y lo recuerda para la proxima vez.
#[tauri::command]
fn next_monitor(app: AppHandle, state: State<'_, AppState>, dir: i32) -> String {
    let mons = display::monitors();
    let Some(w) = app.get_webview_window("main") else { return String::new() };
    if mons.len() > 1 {
        let cur = display::of_window(launcher::main_hwnd(&app));
        let i = cur.and_then(|c| mons.iter().position(|m| m.device == c.device)).unwrap_or(0) as i32;
        let n = mons.len() as i32;
        let m = &mons[((i + dir) % n + n) as usize % mons.len()];
        move_window_to(&w, m);
        display::save_window_monitor(&state.config_dir, &m.device);
    }
    window_monitor(app)
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    app.exit(0);
}

/// Modo auxiliar (reiniciar Magpie elevado). Lo llama `main` antes de abrir la aplicacion.
pub fn helper_main() -> Option<i32> {
    magpie::helper_main()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        // El atajo de grabacion empieza/para el audio del juego, el de audio cambia de salida;
                        // cualquier otro captura la pantalla.
                        let (rec, out) = {
                            let st = app.state::<AppState>();
                            let c = st.config.lock().unwrap();
                            (c.record_hotkey.clone(), c.audio_hotkey.clone())
                        };
                        let is = |k: &str| k.parse::<tauri_plugin_global_shortcut::Shortcut>().map(|r| &r == shortcut).unwrap_or(false);
                        if is(&rec) {
                            audio::toggle(app, None);
                        } else if is(&out) {
                            outputs::cycle_with_feedback(app, 0);
                        } else {
                            snap::trigger(app);
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            // Donde estan los datos: la biblioteca, lo de antes (asistente pendiente) o una biblioteca que no se encuentra.
            let (lib_mode, lib_dir) = library::resolve(&config_dir);
            let early = library::early_paths(&lib_mode, &lib_dir, &config_dir, &cache_dir);
            let mut notices: Vec<backup::Notice> = Vec::new();
            // Una restauracion preparada se aplica ANTES de leer la configuracion y abrir la base.
            if lib_mode != state::LibMode::Missing {
                if let Some(r) = visor_core::backup::apply_pending_restore(&early.data, &early.glossary) {
                    notices.push(backup::restore_notice(r.map_err(|e| format!("{e:#}"))));
                }
            }
            let mut config = Config::load(&early.config_file());
            let paths = match lib_mode {
                state::LibMode::Legacy => visor_core::library::DataPaths::legacy(&config_dir, &cache_dir, &config),
                _ => early,
            };
            config.root = match lib_mode {
                state::LibMode::Library => paths.captures.as_ref().map(|p| p.to_string_lossy().into_owned()),
                state::LibMode::Missing => None,
                state::LibMode::Legacy => config.root,
            };
            if let Some(n) = library::copied_notice(&config_dir) {
                notices.push(n);
            }
            // Base de datos (la primera vez importa los JSON antiguos, que se quedan como respaldo). Si no se
            // puede abrir, se sigue con una base en memoria: la app funciona pero esta sesion no guarda nada.
            let mut db_in_memory = false;
            let db = if lib_mode == state::LibMode::Missing {
                db_in_memory = true;
                notices.push(library::missing_notice(&lib_dir));
                Db::open_memory()?
            } else {
                match Db::open(&paths.data) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("No se pudo abrir visor.db: {e:#}. Los datos de esta sesion no se guardaran.");
                        db_in_memory = true;
                        notices.push(backup::db_failed_notice(&format!("{e:#}")));
                        Db::open_memory()?
                    }
                }
            };
            let db = Arc::new(db);
            let favorites = load_favorites(&db, config.root.as_deref().map(Path::new));
            let stats = state::log_db("estadisticas", db.with(dbd::stats_load));
            let hidden = state::log_db("ocultos", db.with(dbd::hidden_load));
            let prefs = state::log_db("preferencias", db.with(dbd::prefs_load));
            let sgdb_store = state::log_db("sgdb", db.with(dbd::sgdb_load));
            let palettes = Arc::new(PaletteCache::load(&cache_dir.join("palettes.json")));
            if let Some(r) = &config.root {
                allow_root(app.handle(), r);
            }
            let _ = app.asset_protocol_scope().allow_directory(&cache_dir, true);
            // Grafismos de SteamGridDB y capturas y audio del glosario (la pantalla de repaso los enseña con el protocolo asset).
            let _ = app.asset_protocol_scope().allow_directory(&paths.art, true);
            let _ = app.asset_protocol_scope().allow_directory(&paths.glossary, true);
            let hotkey = config.capture_hotkey.clone();
            let rec_hotkey = config.record_hotkey.clone();
            let audio_hotkey = config.audio_hotkey.clone();
            let mask = combo_mask(&config.capture_combo);
            let audio_mask = combo_mask(&config.audio_combo);
            let mouse = mouse_button_index(&config.capture_mouse);
            let auto_backup = config.auto_backup && !db_in_memory;
            let (db_for_backup, dir_for_backup) = (db.clone(), paths.db_backups.clone());
            app.manage(AppState {
                config_dir,
                paths,
                lib_mode,
                lib_dir,
                lib_busy: std::sync::atomic::AtomicBool::new(false),
                notices: Mutex::new(notices),
                cache_dir,
                db,
                config: Mutex::new(config),
                favorites: Mutex::new(favorites),
                games: Mutex::new(Vec::new()),
                stats: Mutex::new(stats),
                hidden: Mutex::new(hidden),
                prefs: Mutex::new(prefs),
                palettes,
                active: Mutex::new(None),
                combo_mask: std::sync::atomic::AtomicU32::new(mask),
                audio_mask: std::sync::atomic::AtomicU32::new(audio_mask),
                mouse_button: std::sync::atomic::AtomicU8::new(mouse),
                learning_pad: std::sync::atomic::AtomicBool::new(false),
                sgdb: Arc::new(Mutex::new(sgdb_store)),
                sgdb_busy: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                hashing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                saves_manifest: Mutex::new(None),
                saves_busy: std::sync::atomic::AtomicBool::new(false),
                remote: Mutex::new(None),
                remote_error: Mutex::new(None),
            });
            // Servidor para la app movil, si esta activado.
            remote::apply(app.handle());
            if let Err(e) = apply_hotkeys(app.handle(), &hotkey, &rec_hotkey, &audio_hotkey) {
                eprintln!("{e}");
            }
            // Copia automatica de la base (cada 7 dias, se conservan 5), en segundo plano para no retrasar el arranque.
            if auto_backup {
                std::thread::spawn(move || {
                    state::log_db_err("copia automatica", visor_core::backup::auto_backup_today(&db_for_backup, &dir_for_backup).map(|_| ()));
                });
            }
            overlay::init(app.handle());
            // Si el visor se cerro a mitad de partida, el monitor principal sigue cambiado: se restaura.
            let st = app.state::<AppState>();
            display::restore_primary(&st.config_dir);
            // Se abre en el monitor del cursor (si esta activado en Ajustes) o en el de la ultima vez.
            if let Some(w) = app.get_webview_window("main") {
                let on_cursor = st.config.lock().unwrap().open_on_cursor;
                let target = on_cursor.then(display::of_cursor).flatten().or_else(|| {
                    let saved = display::saved_window_monitor(&st.config_dir)?;
                    display::monitors().into_iter().find(|m| m.device.eq_ignore_ascii_case(&saved))
                });
                if let Some(m) = target {
                    let _ = w.set_position(tauri::PhysicalPosition::new(m.x + 40, m.y + 40));
                }
                let _ = w.set_fullscreen(true);
                let _ = w.show();
                let _ = w.set_focus();
            }
            novel::init(app.handle());
            pad::spawn(app.handle().clone());
            mouse::spawn(app.handle().clone());
            Ok(())
        })
        .on_window_event(|w, e| {
            // Recordar el monitor si el usuario mueve el visor (p. ej. Win+Mayus+flecha).
            if let (tauri::WindowEvent::Moved(_), "main") = (e, w.label()) {
                if let (Some(st), Some(m)) = (w.try_state::<AppState>(), display::of_window(launcher::main_hwnd(w.app_handle()))) {
                    display::save_window_monitor(&st.config_dir, &m.device);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            window_monitor,
            next_monitor,
            get_config,
            save_config,
            scan_library,
            get_thumb,
            prefetch_thumbs,
            get_favorites,
            set_favorite,
            launch_game,
            goal_notice,
            open_folder,
            add_game,
            scan_games_dir,
            set_hidden,
            set_accent_override,
            set_sort_pref,
            set_magpie_mode,
            set_title_status,
            backup::backup_create,
            backup::restore_prepare,
            backup::restore_cancel,
            backup::restore_apply,
            backup::startup_notices,
            library::library_info,
            library::library_plan,
            library::library_copy,
            library::library_use,
            library::library_restart,
            saves::saves_info,
            saves::saves_set_enabled,
            saves::saves_add,
            saves::saves_remove,
            saves::saves_backup,
            saves::saves_restore,
            saves::saves_restore_file,
            saves::saves_copy_files,
            saves::saves_set_note,
            saves::saves_set_pinned,
            saves::saves_delete_copy,
            saves::saves_export_copy,
            saves::saves_open_dir,
            saves::saves_status,
            saves::saves_update_manifest,
            remote::remote_info,
            remote::remote_new_code,
            magpie_status,
            get_sagas,
            save_sagas,
            import_shots,
            move_shots,
            reveal_in_explorer,
            remove_game,
            move_game_platform,
            capture_now,
            sgdb::sgdb_auto,
            sgdb::sgdb_search,
            sgdb::sgdb_images,
            sgdb::sgdb_info,
            sgdb::sgdb_link,
            sgdb::sgdb_apply,
            sgdb::sgdb_local,
            sgdb::sgdb_clear,
            learn_pad,
            audio_outputs,
            set_audio_output,
            hotkey_suspend,
            exit_app,
            texthook::commands::th_status,
            texthook::commands::th_select_thread,
            texthook::commands::th_select_name_thread,
            texthook::commands::th_insert_code,
            texthook::commands::th_remove_hook,
            texthook::commands::th_search_text,
            texthook::commands::th_search_cancel,
            texthook::commands::th_reinject,
            texthook::commands::th_detach,
            glossary::phrases_list,
            glossary::phrases_get,
            glossary::phrases_update,
            glossary::phrases_grade,
            glossary::phrases_delete,
            glossary::phrases_due_count,
            glossary::phrases_media_dir,
            anki::anki_test,
            anki::anki_create_deck,
            anki::anki_apply_style,
            anki::anki_fields,
            anki::anki_guess_map,
            novel::novel_get_prefs,
            novel::novel_save_prefs,
            novel::novel_open_reader,
            novel::novel_reader_url,
            novel::novel_yomitan_css,
            novel::readstats::readstats_get,
            novel::readstats::readstats_live
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicacion");
}
