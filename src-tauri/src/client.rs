//! Modo cliente PC: este equipo se empareja con un «PC principal» (el host, con la biblioteca) y la interfaz habla
//! con él por su API. Lo guardado vive en `<config_dir>/client.json`, fuera de `Config`: `save_config` va al host y
//! no debe pisarlo ni llevarlo allí. Estos comandos son solo locales (nunca en `remote_call::dispatch`).
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use visor_core::config::write_atomic;
use visor_core::remote_client as rc;

const FILE: &str = "client.json";

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClientCfg {
    pub host_id: String,
    pub host_name: String,
    /// `ip:puerto` del host en orden de preferencia (la de casa primero; despues la de Tailscale u otra red).
    /// Se usa la primera que conteste (ver `probe`).
    pub host_addrs: Vec<String>,
    /// MAC del host para encenderlo (Wake-on-LAN).
    pub host_mac: String,
    pub device_id: String,
    pub device_name: String,
    pub token: String,
    /// Ruta de Moonlight elegida a mano (vacia = buscarlo donde se instala).
    pub moonlight_path: String,
}

impl ClientCfg {
    /// La configuracion de cliente, si este equipo esta emparejado con un host.
    pub fn load(config_dir: &Path) -> Option<ClientCfg> {
        let s = std::fs::read_to_string(config_dir.join(FILE)).ok()?;
        let c: ClientCfg = serde_json::from_str(&s).ok()?;
        (!c.token.is_empty() && !c.host_addrs.is_empty()).then_some(c)
    }

    fn save(&self, config_dir: &Path) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        write_atomic(&config_dir.join(FILE), &json).map_err(|e| format!("{e:#}"))
    }

    /// La direccion que contesto la ultima vez (sin red: lo que diga la cache), o la primera.
    fn addr(&self) -> &str {
        let active = ACTIVE.lock().unwrap().as_ref().filter(|a| a.host_id == self.host_id).map(|a| a.addr.clone());
        active
            .and_then(|a| self.host_addrs.iter().find(|x| **x == a))
            .or(self.host_addrs.first())
            .map(String::as_str)
            .unwrap_or("")
    }
}

/// Direccion del host que contesto (fase 5: varias direcciones, casa y Tailscale).
struct Active {
    host_id: String,
    addr: String,
    at: std::time::Instant,
}
static ACTIVE: std::sync::Mutex<Option<Active>> = std::sync::Mutex::new(None);
/// Cuanto vale la direccion elegida antes de volver a probarlas.
const ACTIVE_TTL: Duration = Duration::from_secs(90);

/// Prueba las direcciones del host en orden con `hello` y se queda con la primera que sea ese host.
/// Con una sola direccion no hace nada. `force` = probar aunque la elegida sea reciente.
fn probe(cfg: &ClientCfg, force: bool) -> Option<String> {
    if cfg.host_addrs.len() < 2 {
        return None;
    }
    if !force {
        let g = ACTIVE.lock().unwrap();
        if let Some(a) = g.as_ref().filter(|a| a.host_id == cfg.host_id && a.at.elapsed() < ACTIVE_TTL) {
            if cfg.host_addrs.contains(&a.addr) {
                return Some(a.addr.clone());
            }
        }
    }
    for a in &cfg.host_addrs {
        if rc::hello(a).is_ok_and(|h| cfg.host_id.is_empty() || h.id == cfg.host_id) {
            *ACTIVE.lock().unwrap() = Some(Active { host_id: cfg.host_id.clone(), addr: a.clone(), at: std::time::Instant::now() });
            return Some(a.clone());
        }
    }
    None
}

/// Moonlight elegido a mano o, si no, donde lo deja su instalador.
fn moonlight(cfg: &ClientCfg) -> Option<PathBuf> {
    let manual = PathBuf::from(cfg.moonlight_path.trim());
    if !cfg.moonlight_path.trim().is_empty() {
        return manual.is_file().then_some(manual);
    }
    ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
        .iter()
        .filter_map(|v| std::env::var_os(v))
        .flat_map(|d| {
            let d = PathBuf::from(d);
            [d.join("Moonlight Game Streaming").join("Moonlight.exe"), d.join("Programs").join("Moonlight Game Streaming").join("Moonlight.exe")]
        })
        .find(|p| p.is_file())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    /// `host` (sin emparejar: este equipo tiene su biblioteca) o `client`.
    mode: &'static str,
    host_id: String,
    host_name: String,
    host_addr: String,
    /// Todas las direcciones guardadas (la de casa y las de fuera).
    host_addrs: Vec<String>,
    host_mac: String,
    device_id: String,
    device_name: String,
    moonlight_path: String,
    /// Moonlight que se usara (vacio = no se encuentra).
    moonlight_found: String,
    /// Nombre propuesto para este equipo al emparejarlo.
    suggested_name: String,
}

fn info_of(cfg: Option<&ClientCfg>, st: &AppState) -> ClientInfo {
    let def = ClientCfg::default();
    let c = cfg.unwrap_or(&def);
    ClientInfo {
        mode: if cfg.is_some() { "client" } else { "host" },
        host_id: c.host_id.clone(),
        host_name: c.host_name.clone(),
        host_addr: c.addr().to_string(),
        host_addrs: c.host_addrs.clone(),
        host_mac: c.host_mac.clone(),
        device_id: c.device_id.clone(),
        device_name: c.device_name.clone(),
        moonlight_path: c.moonlight_path.clone(),
        moonlight_found: moonlight(c).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
        suggested_name: crate::remote::display_name(st),
    }
}

fn version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

fn current(st: &AppState) -> Result<ClientCfg, String> {
    ClientCfg::load(&st.config_dir).ok_or_else(|| "Este equipo no está conectado a un PC principal".into())
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

/// Modo de este equipo y con qué host está emparejado (sin el token).
#[tauri::command]
pub fn client_info(state: State<'_, AppState>) -> ClientInfo {
    info_of(ClientCfg::load(&state.config_dir).as_ref(), &state)
}

#[derive(Serialize)]
pub struct ClientEndpoint {
    /// `http://ip:puerto`.
    base: String,
    token: String,
    #[serde(rename = "hostId")]
    host_id: String,
}

/// Dirección y token para que la interfaz hable con el host (`None` en modo host). Con varias direcciones, la que conteste.
#[tauri::command]
pub async fn client_endpoint(state: State<'_, AppState>) -> Result<Option<ClientEndpoint>, String> {
    let Some(c) = ClientCfg::load(&state.config_dir) else { return Ok(None) };
    let c = blocking(move || {
        probe(&c, false);
        Ok(c)
    })
    .await?;
    Ok(Some(ClientEndpoint { base: format!("http://{}", rc::normalize_addr(c.addr())), token: c.token.clone(), host_id: c.host_id.clone() }))
}

/// Cambia las direcciones del PC principal (la primera se prueba antes; p. ej. la de casa y luego la de Tailscale).
#[tauri::command]
pub fn client_set_addrs(state: State<'_, AppState>, addrs: Vec<String>) -> Result<ClientInfo, String> {
    let mut cfg = current(&state)?;
    let mut out: Vec<String> = Vec::new();
    for a in addrs.iter().flat_map(|a| a.split([',', ';', ' '])) {
        let a = rc::normalize_addr(a);
        if !a.is_empty() && !out.contains(&a) {
            out.push(a);
        }
    }
    if out.is_empty() {
        return Err("Hace falta al menos una dirección".into());
    }
    out.truncate(6);
    cfg.host_addrs = out;
    cfg.save(&state.config_dir)?;
    *ACTIVE.lock().unwrap() = None;
    Ok(info_of(Some(&cfg), &state))
}

/// Visores en la red local (1,5 s esperando respuestas), sin este mismo equipo.
#[tauri::command]
pub async fn client_discover(state: State<'_, AppState>) -> Result<Vec<rc::Found>, String> {
    let me = state.config.lock().unwrap().remote_id.clone();
    let found = blocking(|| rc::discover(Duration::from_millis(1500)).map_err(|e| format!("{e:#}"))).await?;
    Ok(found.into_iter().filter(|f| me.is_empty() || f.id != me).collect())
}

/// Se empareja con el host de `addr` usando su código y guarda el token. La interfaz recarga después.
#[tauri::command]
pub async fn client_pair(app: AppHandle, state: State<'_, AppState>, addr: String, code: String, device_name: String) -> Result<ClientInfo, String> {
    let addr = rc::normalize_addr(&addr);
    if addr.is_empty() {
        return Err("Escribe la dirección del PC principal".into());
    }
    let name: String = device_name.trim().chars().take(60).collect();
    if name.is_empty() {
        return Err("Ponle un nombre a este equipo".into());
    }
    let me = state.config.lock().unwrap().remote_id.clone();
    let config_dir = state.config_dir.clone();
    let ver = version(&app);
    let moonlight_path = ClientCfg::load(&config_dir).map(|c| c.moonlight_path).unwrap_or_default();
    let cfg = blocking(move || {
        let h = rc::hello(&addr).map_err(|e| format!("{e:#}"))?;
        // Mismo id = es este equipo, o los dos abren la misma biblioteca por red.
        if !me.is_empty() && h.id == me {
            return Err("Esa dirección es este mismo equipo (o los dos usan la misma biblioteca por red)".into());
        }
        let (_, p) = rc::pair(&addr, code.trim(), &name, &ver).map_err(|e| format!("{e:#}"))?;
        let mac = rc::call(&addr, &p.token, &ver, "pc_mac", serde_json::Value::Null)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        let cfg = ClientCfg {
            host_id: p.host_id,
            host_name: p.host_name,
            host_addrs: vec![addr],
            host_mac: mac,
            device_id: p.device_id,
            device_name: name,
            token: p.token,
            moonlight_path,
        };
        cfg.save(&config_dir)?;
        Ok(cfg)
    })
    .await?;
    Ok(info_of(Some(&cfg), &state))
}

/// Deja de ser cliente: avisa al host (si contesta) y borra `client.json`. La interfaz recarga después.
#[tauri::command]
pub async fn client_unpair(app: AppHandle, state: State<'_, AppState>) -> Result<ClientInfo, String> {
    let cfg = current(&state)?;
    let ver = version(&app);
    let addr = cfg.addr().to_string();
    let token = cfg.token.clone();
    // Si el host esta apagado se desempareja igual aqui; alli se puede quitar desde «Dispositivos».
    let _ = blocking(move || rc::unpair(&addr, &token, &ver).map_err(|e| format!("{e:#}"))).await;
    let f = state.config_dir.join(FILE);
    if f.exists() {
        std::fs::remove_file(&f).map_err(|e| e.to_string())?;
    }
    Ok(info_of(None, &state))
}

/// Abre Moonlight con la app de Apollo que devolvió `launch_stream` («Juego actual»).
#[tauri::command]
pub fn client_open_stream(state: State<'_, AppState>, app: String) -> Result<(), String> {
    let cfg = current(&state)?;
    let exe = moonlight(&cfg).ok_or("No se encuentra Moonlight: instálalo o elige dónde está en Ajustes → Este equipo")?;
    let host = rc::addr_host(cfg.addr());
    // Sin app «Juego actual» en Apollo, el escritorio (como la tablet).
    let app = if app.trim().is_empty() { "Desktop".to_string() } else { app };
    std::process::Command::new(exe).args(["stream", &host, &app]).spawn().map_err(|e| format!("No se pudo abrir Moonlight: {e}"))?;
    Ok(())
}

/// Enciende el PC principal (Wake-on-LAN, solo en la red local).
#[tauri::command]
pub fn client_wake(state: State<'_, AppState>) -> Result<(), String> {
    let cfg = current(&state)?;
    if cfg.host_mac.is_empty() {
        return Err("No se sabe la MAC del PC principal: empareja de nuevo con él encendido".into());
    }
    rc::wake(&cfg.host_mac).map_err(|e| format!("{e:#}"))
}

/// Ruta de Moonlight (vacía = buscarlo donde lo instala su instalador).
#[tauri::command]
pub fn client_set_moonlight(state: State<'_, AppState>, path: String) -> Result<ClientInfo, String> {
    let mut cfg = current(&state)?;
    cfg.moonlight_path = path.trim().to_string();
    cfg.save(&state.config_dir)?;
    Ok(info_of(Some(&cfg), &state))
}

/* ------------------------------------------------------------------ */
/* Juegos instalados en este equipo (fase 2)                            */
/* ------------------------------------------------------------------ */
//
// El juego se guarda en la base local con el id del titulo del host, asi que la biblioteca (que viene del host)
// y lo instalado aqui casan por `Title.id`. Las partidas que se juegan aqui se suben solas al host.

fn host_call(cfg: &ClientCfg, ver: &str, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    probe(cfg, false);
    match rc::call(cfg.addr(), &cfg.token, ver, cmd, args.clone()) {
        // Sin respuesta: quiza ha cambiado de red (casa <-> Tailscale); se prueba otra direccion una vez.
        Err(e) if cfg.host_addrs.len() > 1 && e.chain().any(|c| c.downcast_ref::<rc::Unreachable>().is_some()) => {
            let before = cfg.addr().to_string();
            match probe(cfg, true) {
                Some(a) if a != before => rc::call(&a, &cfg.token, ver, cmd, args).map_err(|e| format!("{e:#}")),
                _ => Err(format!("{e:#}")),
            }
        }
        r => r.map_err(|e| format!("{e:#}")),
    }
}

fn file_name(p: &str) -> String {
    Path::new(p).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

fn parent_name(p: &str) -> String {
    Path::new(p).parent().and_then(|d| d.file_name()).map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalAdd {
    /// Titulo del host elegido tras `match_title`; `None` = que el host lo cree en `platform`.
    pub game_id: Option<String>,
    /// Categoria del host (nunca se crean categorias).
    pub platform: String,
    pub name: String,
    /// "exe" | "emulator".
    pub kind: String,
    pub exe: String,
    pub args: String,
    pub rom: String,
    pub workdir: String,
    /// Tamaño de la instalacion (0 = no se sabe).
    pub size_bytes: i64,
}

/// Un juego instalado en este equipo (para casarlo con la biblioteca del host).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalGame {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub kind: String,
    pub exe: String,
    pub rom: String,
}

/// Añade un juego instalado aqui: el host lo apunta (o crea el titulo en esa categoria) y aqui se guarda con su id.
#[tauri::command]
pub async fn client_add_game(app: AppHandle, state: State<'_, AppState>, game: LocalAdd) -> Result<String, String> {
    let cfg = current(&state)?;
    if game.kind != "exe" && game.kind != "emulator" {
        return Err("Tipo de juego desconocido".into());
    }
    if !Path::new(&game.exe).is_file() {
        return Err("El ejecutable no existe".into());
    }
    if game.kind == "emulator" && !Path::new(&game.rom).is_file() {
        return Err("La ROM no existe".into());
    }
    let name = game.name.trim().to_string();
    if name.is_empty() {
        return Err("Falta el nombre".into());
    }
    let ver = version(&app);
    let main = if game.kind == "emulator" { game.rom.clone() } else { game.exe.clone() };
    let args = serde_json::json!({
        "gameId": game.game_id,
        "platform": game.platform,
        "name": name,
        "exeName": file_name(&main),
        "folderName": parent_name(&main),
        "sizeBytes": game.size_bytes.max(0),
    });
    let id = blocking(move || host_call(&cfg, &ver, "register_install", args)).await?;
    let id = id.as_str().filter(|s| !s.is_empty()).ok_or("El PC principal no devolvió el juego")?.to_string();
    let entry = visor_core::games::ManualGame {
        id: id.clone(),
        name,
        kind: game.kind,
        exe: game.exe,
        args: game.args,
        rom: game.rom,
        platform: game.platform,
        workdir: game.workdir.trim().to_string(),
        ..Default::default()
    };
    state
        .db
        .with(|c| {
            visor_core::db::data::manual_remove(c, &id)?;
            visor_core::db::data::manual_add(c, &entry)
        })
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Juegos instalados en este equipo.
#[tauri::command]
pub fn client_local_games(state: State<'_, AppState>) -> Result<Vec<LocalGame>, String> {
    let list = state.db.with(visor_core::db::data::manual_load).map_err(|e| e.to_string())?;
    Ok(list
        .into_iter()
        .filter(|g| g.kind == "exe" || g.kind == "emulator")
        .map(|g| LocalGame { id: g.id, name: g.name, platform: g.platform, kind: g.kind, exe: g.exe, rom: g.rom })
        .collect())
}

/// Lanza aqui un juego instalado en este equipo.
#[tauri::command]
pub fn client_launch(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let manual = state.db.with(visor_core::db::data::manual_load).map_err(|e| e.to_string())?;
    let found = visor_core::games::all_games(&manual);
    let game = found.iter().find(|g| g.id == id).cloned().ok_or("Ese juego no está instalado en este equipo")?;
    *state.games.lock().unwrap() = found;
    crate::launcher::start(&app, game)
}

/// Quita un juego de este equipo (no toca sus archivos) y se lo dice al host si responde.
#[tauri::command]
pub async fn client_remove_game(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.db.with(|c| visor_core::db::data::manual_remove(c, &id)).map_err(|e| e.to_string())?;
    if let Ok(cfg) = current(&state) {
        let ver = version(&app);
        let args = serde_json::json!({ "gameId": id });
        let _ = blocking(move || host_call(&cfg, &ver, "unregister_install", args)).await;
    }
    Ok(())
}

/// Sube al host las partidas jugadas aqui que falten. Devuelve cuantas se subieron.
fn sync_once(app: &AppHandle) -> Result<usize, String> {
    let st = app.state::<AppState>();
    let Some(cfg) = ClientCfg::load(&st.config_dir) else { return Ok(0) };
    let mine: std::collections::HashSet<String> =
        st.db.with(visor_core::db::data::manual_load).map_err(|e| e.to_string())?.into_iter().map(|g| g.id).collect();
    let pend = st.db.with(|c| visor_core::installs::unsynced(c, 2000)).map_err(|e| e.to_string())?;
    // Solo los juegos apuntados en el host (los demas no los aceptaria y se reenviarian siempre).
    let pend: Vec<_> = pend.into_iter().filter(|s| mine.contains(&s.game_id)).take(200).collect();
    let ver = version(app);
    let now = chrono::Utc::now().timestamp_millis();
    let mut n = 0;
    if !pend.is_empty() {
        let res = host_call(&cfg, &ver, "sessions_upload", serde_json::json!({ "sessions": pend }))?;
        let uids: Vec<String> = serde_json::from_value(res).map_err(|e| e.to_string())?;
        st.db.with(|c| visor_core::installs::mark_synced(c, &uids, now)).map_err(|e| e.to_string())?;
        n += uids.len();
    }
    n += sync_reading(&st, &cfg, &ver, &mine, now)?;
    Ok(n)
}

/// Sube las sesiones de lectura y las frases del glosario de los juegos de este equipo (fase 4), con su media.
fn sync_reading(st: &AppState, cfg: &ClientCfg, ver: &str, mine: &std::collections::HashSet<String>, now: i64) -> Result<usize, String> {
    use visor_core::readsync;
    let mut n = 0;
    let reads = st.db.with(|c| readsync::unsynced_reads(c, 2000)).map_err(|e| e.to_string())?;
    let reads: Vec<_> = reads.into_iter().filter(|r| mine.contains(&r.game_id)).take(300).collect();
    if !reads.is_empty() {
        let res = host_call(cfg, ver, "reads_upload", serde_json::json!({ "reads": reads }))?;
        let uids: Vec<String> = serde_json::from_value(res).map_err(|e| e.to_string())?;
        st.db.with(|c| readsync::mark_reads_synced(c, &uids, now)).map_err(|e| e.to_string())?;
        n += uids.len();
    }
    let phrases = st.db.with(|c| readsync::unsynced_phrases(c, 2000)).map_err(|e| e.to_string())?;
    let phrases: Vec<_> = phrases.into_iter().filter(|p| mine.contains(&p.game_id)).take(100).collect();
    if phrases.is_empty() {
        return Ok(n);
    }
    #[derive(Deserialize)]
    struct Up {
        saved: Vec<(String, String)>,
        missing: Vec<String>,
    }
    let res = host_call(cfg, ver, "phrases_upload", serde_json::json!({ "phrases": phrases }))?;
    let up: Up = serde_json::from_value(res).map_err(|e| e.to_string())?;
    // Primero la media que le falta al host; si una no sube, sus frases quedan pendientes para la proxima vez.
    let dir = st.paths.glossary.join("media");
    let mut failed: Vec<String> = Vec::new();
    for m in &up.missing {
        let Some(f) = readsync::media_path(&dir, m) else { continue };
        let Ok(bytes) = std::fs::read(&f) else { continue }; // aqui tampoco esta: no hay nada que subir
        let args = serde_json::json!({ "name": m, "data": readsync::encode_media(&bytes) });
        if let Err(e) = rc::call_long(cfg.addr(), &cfg.token, ver, "phrase_media_upload", args) {
            eprintln!("cliente: no se pudo subir {m}: {e:#}");
            failed.push(m.clone());
        }
    }
    let ok: Vec<(String, String)> = up
        .saved
        .into_iter()
        .filter(|k| {
            phrases
                .iter()
                .find(|p| p.game_id == k.0 && p.id == k.1)
                .is_none_or(|p| readsync::media_of(p).iter().all(|m| !failed.contains(m)))
        })
        .collect();
    st.db.with(|c| readsync::mark_phrases_synced(c, &ok, now)).map_err(|e| e.to_string())?;
    Ok(n + ok.len())
}

/// Sube ya las partidas pendientes (p. ej. al acabar una).
#[tauri::command]
pub async fn client_sync(app: AppHandle) -> Result<usize, String> {
    blocking(move || sync_once(&app)).await
}

/// Hilo que sube las partidas locales al host cada minuto (si este equipo es cliente).
pub fn spawn_sync(app: AppHandle) {
    let _ = std::thread::Builder::new().name("client-sync".into()).spawn(move || {
        std::thread::sleep(Duration::from_secs(15));
        loop {
            if let Err(e) = sync_once(&app) {
                eprintln!("cliente: no se pudieron subir las partidas: {e}");
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

/* ------------------------------------------------------------------ */
/* Copiar un juego entero del PC principal (fase 3b)                    */
/* ------------------------------------------------------------------ */

static COPY_CANCEL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static COPYING: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Que copiaria `client_copy_game` (carpeta, ejecutable, tamaño y archivos).
#[tauri::command]
pub async fn client_copy_info(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<visor_core::gamecopy::CopyList, String> {
    let cfg = current(&state)?;
    let ver = version(&app);
    let mut l = blocking(move || {
        probe(&cfg, false);
        visor_core::gamecopy::fetch_list(cfg.addr(), &cfg.token, &ver, &id).map_err(|e| format!("{e:#}"))
    })
    .await?;
    l.files.clear();
    Ok(l)
}

/// Nombre de carpeta valido en Windows.
fn folder_name(s: &str) -> String {
    let n: String = s.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c }).collect();
    let n = n.trim().trim_end_matches('.').to_string();
    if n.is_empty() { "Juego".into() } else { n }
}

/// Copia el juego del host a `dest\<carpeta del juego>` (reanuda si se corto) y lo apunta como instalado aqui.
/// Avisa del progreso con `client-copy-progress`. Devuelve el id del juego.
#[tauri::command]
pub async fn client_copy_game(app: AppHandle, state: State<'_, AppState>, id: String, dest: String) -> Result<String, String> {
    use tauri::Emitter;
    let cfg = current(&state)?;
    if !Path::new(&dest).is_dir() {
        return Err("La carpeta de destino no existe".into());
    }
    {
        let mut c = COPYING.lock().unwrap();
        if c.is_some() {
            return Err("Ya se está copiando un juego".into());
        }
        *c = Some(id.clone());
    }
    COPY_CANCEL.store(false, std::sync::atomic::Ordering::Relaxed);
    let ver = version(&app);
    let (app2, id2) = (app.clone(), id.clone());
    let res = blocking(move || {
        probe(&cfg, false);
        let list = visor_core::gamecopy::fetch_list(cfg.addr(), &cfg.token, &ver, &id2).map_err(|e| format!("{e:#}"))?;
        let dir = Path::new(&dest).join(folder_name(&list.folder));
        let mut last = std::time::Instant::now() - Duration::from_secs(1);
        let r = visor_core::gamecopy::download(cfg.addr(), &cfg.token, &ver, &list, &dir, &COPY_CANCEL, |p| {
            if last.elapsed() >= Duration::from_millis(250) || p.files_done == p.files {
                last = std::time::Instant::now();
                let _ = app2.emit("client-copy-progress", serde_json::json!({ "id": id2, "progress": p }));
            }
        });
        r.map_err(|e| format!("{e:#}"))?;
        Ok((list, dir))
    })
    .await;
    *COPYING.lock().unwrap() = None;
    let (list, dir) = res?;
    let exe = dir.join(visor_core::gamecopy::safe_rel(&list.exe).ok_or("El ejecutable del juego no es válido")?);
    let game = LocalAdd {
        game_id: Some(id),
        platform: list.platform,
        name: list.name,
        kind: "exe".into(),
        exe: exe.to_string_lossy().into_owned(),
        args: list.args,
        size_bytes: list.total as i64,
        ..Default::default()
    };
    client_add_game(app, state, game).await
}

/// Para la copia en curso (lo ya bajado se aprovecha al volver a copiar).
#[tauri::command]
pub fn client_copy_cancel() {
    COPY_CANCEL.store(true, std::sync::atomic::Ordering::Relaxed);
}
