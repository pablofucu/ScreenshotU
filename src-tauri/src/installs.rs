//! Host multi-PC: juegos instalados en los PCs cliente y partidas que esos clientes juegan en local.
//!
//! El cliente nunca crea categorias ni ids: busca el titulo del host en una categoria que ya existe
//! (`match_title`) y lo apunta como instalado (`register_install`); si no existe, el host lo crea como juego
//! «solo capturas» en esa misma categoria, sin crear carpetas.
use serde::Deserialize;
use std::path::Path;
use visor_core::db::data as dbd;
use visor_core::games::{self, Launch, ManualGame};
use visor_core::installs::{self, Candidate, Install, MatchItem, MatchQuery, RemoteSession};
use visor_core::remote::Caller;
use visor_core::scan::{InstalledOn, Library};

use crate::state::AppState;

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn pc_caller(caller: &Caller) -> Result<&str, String> {
    if caller.device_id.is_empty() || caller.kind != "pc" {
        return Err("Solo un PC emparejado puede apuntar juegos instalados".into());
    }
    Ok(&caller.device_id)
}

/// Ultima biblioteca guardada (la que ve la interfaz al abrir); si no hay, se escanea.
fn library(state: &AppState) -> (Library, Vec<games::Game>) {
    #[derive(Deserialize)]
    struct Snap {
        root: String,
        lib: Library,
    }
    let root = state.config.lock().unwrap().root.clone();
    let snap = std::fs::read(state.cache_dir.join("library-snapshot.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Snap>(&b).ok())
        .filter(|s| Some(&s.root) == root.as_ref());
    match snap {
        Some(s) => {
            let manual = crate::state::log_db("juegos", state.db.with(dbd::manual_load));
            (s.lib, games::all_games(&manual))
        }
        None => {
            let (lib, found, _) = crate::scan_full(state);
            (lib, found)
        }
    }
}

fn file_name(p: &str) -> String {
    Path::new(p).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Titulos de la categoria `platform` con lo que sirve para reconocerlos.
fn items(state: &AppState, platform: &str) -> Vec<MatchItem> {
    let (lib, found) = library(state);
    let Some(p) = lib.platforms.iter().find(|p| p.id == platform) else { return Vec::new() };
    p.titles
        .iter()
        .map(|t| {
            let g = t.game_id.as_ref().and_then(|id| found.iter().find(|g| &g.id == id));
            let exe_name = match g.map(|g| &g.launch) {
                Some(Launch::Exe { exe, .. }) => file_name(exe),
                Some(Launch::Emulator { rom, .. }) => file_name(rom),
                _ => String::new(),
            };
            let steam_app_id = t.game_id.as_deref().and_then(|id| id.strip_prefix("steam:")).map(str::to_string);
            MatchItem { id: t.id.clone(), name: t.name.clone(), folder: t.folder.clone(), exe_name, steam_app_id }
        })
        .collect()
}

fn check_platform(state: &AppState, platform: &str) -> Result<(), String> {
    if state.config.lock().unwrap().platforms.iter().any(|p| p.id == platform) {
        Ok(())
    } else {
        Err("Esa categoria no existe en el PC principal".into())
    }
}

/// Candidatos del host (solo de esa categoria) para un juego que el cliente quiere añadir.
pub fn match_title(state: &AppState, platform: &str, q: &MatchQuery) -> Result<Vec<Candidate>, String> {
    check_platform(state, platform)?;
    Ok(installs::match_title(&items(state, platform), q))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegisterArgs {
    /// Titulo del host elegido; `None` = crear uno nuevo en `platform`.
    pub game_id: Option<String>,
    pub platform: String,
    pub name: String,
    pub exe_name: String,
    pub folder_name: String,
    pub size_bytes: i64,
}

/// Apunta que el PC que llama tiene instalado el juego. Devuelve el id del titulo en el host.
pub fn register_install(state: &AppState, caller: &Caller, a: RegisterArgs) -> Result<String, String> {
    let device = pc_caller(caller)?;
    let id = match a.game_id.filter(|g| !g.trim().is_empty()) {
        Some(id) => id,
        None => {
            check_platform(state, &a.platform)?;
            let name = a.name.trim().to_string();
            if name.is_empty() {
                return Err("Falta el nombre".into());
            }
            let entry = ManualGame {
                id: games::new_manual_id(),
                name,
                kind: "captures".into(),
                platform: a.platform.clone(),
                ..Default::default()
            };
            state.db.with(|c| dbd::manual_add(c, &entry)).map_err(|e| e.to_string())?;
            entry.id
        }
    };
    let row = Install {
        game_id: id.clone(),
        device_id: device.to_string(),
        exe_name: a.exe_name,
        folder_name: a.folder_name,
        size_bytes: a.size_bytes.max(0),
        ..Default::default()
    };
    state.db.with(|c| installs::upsert(c, &row, now_ms())).map_err(|e| e.to_string())?;
    Ok(id)
}

pub fn unregister_install(state: &AppState, caller: &Caller, game_id: &str) -> Result<bool, String> {
    let device = pc_caller(caller)?;
    state.db.with(|c| installs::remove(c, game_id, device)).map_err(|e| e.to_string())
}

/// Guarda las partidas locales que sube un cliente. Devuelve los `uid` guardados (el cliente los marca enviados).
/// Solo se aceptan juegos que ese PC tiene apuntados como instalados.
pub fn sessions_upload(state: &AppState, caller: &Caller, sessions: Vec<RemoteSession>) -> Result<Vec<String>, String> {
    let device = pc_caller(caller)?;
    let mut ok = Vec::new();
    for s in sessions.iter().take(500) {
        let saved = state
            .db
            .with(|c| anyhow::Ok(installs::has(c, &s.game_id, device)? && installs::put_remote_session(c, device, s)?))
            .map_err(|e| e.to_string())?;
        if saved {
            ok.push(s.uid.clone());
        }
    }
    if !ok.is_empty() {
        // Las partidas de los clientes cuentan en el tiempo de juego del host: se recarga lo que hay en memoria.
        if let Ok(stats) = state.db.with(dbd::stats_load) {
            *state.stats.lock().unwrap() = stats;
        }
    }
    Ok(ok)
}

/// Rellena `installed_on` de los titulos con los PCs cliente que los tienen instalados.
pub fn fill_installed(state: &AppState, lib: &mut Library) {
    let Ok((by, names)) = state.db.with(|c| anyhow::Ok((installs::by_game(c)?, visor_core::devices::names(c)?))) else { return };
    if by.is_empty() {
        return;
    }
    for t in lib.platforms.iter_mut().flat_map(|p| p.titles.iter_mut()) {
        if let Some(devs) = by.get(&t.id) {
            t.installed_on = devs
                .iter()
                .map(|d| InstalledOn { device_id: d.clone(), name: names.get(d).cloned().unwrap_or_default() })
                .collect();
        }
    }
}
