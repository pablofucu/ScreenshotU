//! Modo cliente PC: este equipo se empareja con un «PC principal» (el host, con la biblioteca) y la interfaz habla
//! con él por su API. Lo guardado vive en `<config_dir>/client.json`, fuera de `Config`: `save_config` va al host y
//! no debe pisarlo ni llevarlo allí. Estos comandos son solo locales (nunca en `remote_call::dispatch`).
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
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
    /// Carpeta de configuracion (no se guarda): para renovar las direcciones desde `probe`.
    #[serde(skip)]
    dir: PathBuf,
}

impl ClientCfg {
    /// Lo guardado en `client.json` aunque el token ya no valga (para conservar la ruta de Moonlight y las direcciones).
    fn raw(config_dir: &Path) -> Option<ClientCfg> {
        let s = std::fs::read_to_string(config_dir.join(FILE)).ok()?;
        let mut c: ClientCfg = serde_json::from_str(&s).ok()?;
        c.dir = config_dir.to_path_buf();
        Some(c)
    }

    /// La configuracion de cliente, si este equipo esta emparejado con un host.
    pub fn load(config_dir: &Path) -> Option<ClientCfg> {
        Self::raw(config_dir).filter(|c| !c.token.is_empty() && !c.host_addrs.is_empty())
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

/// Normaliza, quita repetidas y se queda con 6 como mucho; `used` (la que ha contestado) se anade al final si falta.
fn merge_addrs(list: &[String], used: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for a in list.iter().map(String::as_str).chain(std::iter::once(used)) {
        let a = rc::normalize_addr(a);
        if !a.is_empty() && !out.contains(&a) {
            out.push(a);
        }
    }
    out.truncate(6);
    out
}

/// Prueba las direcciones del host en orden con `hello` y se queda con la primera que sea ese host. En cada `hello`
/// renueva las direcciones guardadas con las que anuncia el PC. `force` = probar aunque la elegida sea reciente.
fn probe(cfg: &ClientCfg, force: bool) -> Option<String> {
    if !force {
        let g = ACTIVE.lock().unwrap();
        if let Some(a) = g.as_ref().filter(|a| a.host_id == cfg.host_id && a.at.elapsed() < ACTIVE_TTL) {
            if cfg.host_addrs.contains(&a.addr) {
                return Some(a.addr.clone());
            }
        }
    }
    for a in &cfg.host_addrs {
        let Ok(h) = rc::hello(a) else { continue };
        if !(cfg.host_id.is_empty() || h.id == cfg.host_id) {
            continue;
        }
        let fresh = merge_addrs(&h.addrs, a);
        if fresh != cfg.host_addrs && !cfg.dir.as_os_str().is_empty() {
            if let Some(mut saved) = ClientCfg::raw(&cfg.dir).filter(|c| c.host_id == cfg.host_id) {
                saved.host_addrs = fresh;
                let _ = saved.save(&cfg.dir);
            }
        }
        *ACTIVE.lock().unwrap() = Some(Active { host_id: cfg.host_id.clone(), addr: a.clone(), at: std::time::Instant::now() });
        return Some(a.clone());
    }
    None
}

static HANDLE: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

/// Este equipo ya no esta emparejado (el PC principal rechazo el token): se borra el token guardado y se avisa a la
/// interfaz con `client-unpaired { hostName, hostId }`.
fn mark_unpaired(dir: &Path) {
    let Some(mut c) = ClientCfg::raw(dir) else { return };
    if c.token.is_empty() {
        return;
    }
    c.token.clear();
    let _ = c.save(dir);
    *ACTIVE.lock().unwrap() = None;
    if let Some(app) = HANDLE.get() {
        let _ = app.emit("client-unpaired", serde_json::json!({ "hostName": c.host_name, "hostId": c.host_id }));
    }
}

/// Pasa un error del PC principal al frontend: un 401 borra el token y avisa; un 426 empieza por "Actualiza el visor".
fn host_err(dir: &Path, e: String) -> String {
    if e.contains("no está emparejado") {
        mark_unpaired(dir);
    } else if e.contains("versión más nueva") {
        return format!("Actualiza el visor: {e}");
    }
    e
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
pub async fn client_endpoint(app: AppHandle, state: State<'_, AppState>) -> Result<Option<ClientEndpoint>, String> {
    let _ = HANDLE.set(app);
    let Some(c) = ClientCfg::load(&state.config_dir) else { return Ok(None) };
    let c = blocking(move || {
        probe(&c, false);
        Ok(c)
    })
    .await?;
    Ok(Some(ClientEndpoint { base: format!("http://{}", rc::normalize_addr(c.addr())), token: c.token.clone(), host_id: c.host_id.clone() }))
}

/// La interfaz recibió un 401 del PC principal directamente: borra el token y avisa con `client-unpaired`.
#[tauri::command]
pub fn client_mark_unpaired(app: AppHandle, state: State<'_, AppState>) {
    let _ = HANDLE.set(app);
    mark_unpaired(&state.config_dir);
}

/// Visores en la red local (1,5 s esperando respuestas), sin este mismo equipo.
#[tauri::command]
pub async fn client_discover(state: State<'_, AppState>) -> Result<Vec<rc::Found>, String> {
    let me = state.config.lock().unwrap().remote_id.clone();
    let found = blocking(|| rc::discover(Duration::from_millis(1500)).map_err(|e| format!("{e:#}"))).await?;
    Ok(found.into_iter().filter(|f| me.is_empty() || f.id != me).collect())
}

/// Guarda la conexion tras emparejar. Las direcciones son las del PC (de casa primero) mas la usada.
fn save_paired(config_dir: &Path, addr: &str, h: &rc::Hello, p: rc::Paired, name: String, ver: &str) -> Result<ClientCfg, String> {
    let addrs = merge_addrs(if p.addrs.is_empty() { &h.addrs } else { &p.addrs }, addr);
    let mac = rc::call(addr, &p.token, ver, "pc_mac", serde_json::Value::Null).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
    let moonlight_path = ClientCfg::raw(config_dir).map(|c| c.moonlight_path).unwrap_or_default();
    let cfg = ClientCfg {
        host_id: p.host_id,
        host_name: p.host_name,
        host_addrs: addrs,
        host_mac: mac,
        device_id: p.device_id,
        device_name: name,
        token: p.token,
        moonlight_path,
        dir: config_dir.to_path_buf(),
    };
    cfg.save(config_dir)?;
    *ACTIVE.lock().unwrap() = None;
    Ok(cfg)
}

/// Error si `addr` es este mismo equipo (o los dos abren la misma biblioteca por red).
fn not_me(me: &str, h: &rc::Hello) -> Result<(), String> {
    if !me.is_empty() && h.id == me {
        return Err("Esa dirección es este mismo equipo (o los dos usan la misma biblioteca por red)".into());
    }
    Ok(())
}

fn clean_addr(addr: &str) -> Result<String, String> {
    let addr = rc::normalize_addr(addr);
    if addr.is_empty() {
        return Err("Escribe la dirección del PC principal".into());
    }
    Ok(addr)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairRequestOut {
    request_id: String,
    digits: String,
    expires_ms: u64,
    host_name: String,
}

/// Pide entrar en el PC de `addr`: este devuelve un número de 4 cifras que se muestra igual en los dos y se aprueba allí.
#[tauri::command]
pub async fn client_pair_request(app: AppHandle, state: State<'_, AppState>, addr: String) -> Result<PairRequestOut, String> {
    let _ = HANDLE.set(app.clone());
    let addr = clean_addr(&addr)?;
    let me = state.config.lock().unwrap().remote_id.clone();
    let name: String = crate::remote::display_name(&state).chars().take(60).collect();
    let ver = version(&app);
    blocking(move || {
        let (h, r) = rc::pair_request(&addr, &name, "pc", &ver).map_err(|e| format!("{e:#}"))?;
        not_me(&me, &h)?;
        Ok(PairRequestOut { request_id: r.request_id, digits: r.digits, expires_ms: r.expires_ms, host_name: h.name })
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairWaitOut {
    /// `pending` (volver a llamar), `approved`, `rejected` o `expired`.
    status: &'static str,
    /// Con `approved`: la conexion ya guardada (la interfaz recarga después).
    info: Option<ClientInfo>,
}

/// Espera (hasta ~25 s) la respuesta del PC. Con `approved` guarda token, id, nombre y direcciones del PC.
#[tauri::command]
pub async fn client_pair_wait(app: AppHandle, state: State<'_, AppState>, addr: String, request_id: String) -> Result<PairWaitOut, String> {
    let _ = HANDLE.set(app.clone());
    let addr = clean_addr(&addr)?;
    let config_dir = state.config_dir.clone();
    let name: String = crate::remote::display_name(&state).chars().take(60).collect();
    let ver = version(&app);
    let st = blocking(move || {
        let status = rc::pair_wait(&addr, &request_id, &ver).map_err(|e| format!("{e:#}"))?;
        Ok(match status {
            rc::PairStatus::Pending => ("pending", None),
            rc::PairStatus::Rejected => ("rejected", None),
            rc::PairStatus::Expired => ("expired", None),
            rc::PairStatus::Approved(p) => {
                let h = rc::hello(&addr).unwrap_or_default();
                ("approved", Some(save_paired(&config_dir, &addr, &h, p, name, &ver)?))
            }
        })
    })
    .await?;
    Ok(PairWaitOut { status: st.0, info: st.1.as_ref().map(|c| info_of(Some(c), &state)) })
}

/// Retira la solicitud pendiente.
#[tauri::command]
pub async fn client_pair_cancel(app: AppHandle, addr: String, request_id: String) -> Result<(), String> {
    let addr = clean_addr(&addr)?;
    let ver = version(&app);
    blocking(move || rc::pair_cancel(&addr, &request_id, &ver).map_err(|e| format!("{e:#}"))).await
}

/// Se empareja con un billete de un solo uso (código o enlace del PC principal) y guarda la conexión.
#[tauri::command]
pub async fn client_pair_ticket(app: AppHandle, state: State<'_, AppState>, addr: String, ticket: String) -> Result<ClientInfo, String> {
    let _ = HANDLE.set(app.clone());
    let addr = clean_addr(&addr)?;
    let me = state.config.lock().unwrap().remote_id.clone();
    let name: String = crate::remote::display_name(&state).chars().take(60).collect();
    let config_dir = state.config_dir.clone();
    let ver = version(&app);
    let cfg = blocking(move || {
        let (h, p) = rc::pair_with_ticket(&addr, ticket.trim(), &name, "pc", &ver).map_err(|e| format!("{e:#}"))?;
        not_me(&me, &h)?;
        save_paired(&config_dir, &addr, &h, p, name, &ver)
    })
    .await?;
    Ok(info_of(Some(&cfg), &state))
}

/// Empareja Moonlight con el streaming del PC principal: genera un PIN, abre `Moonlight.exe pair <host> --pin <pin>`
/// (ventana visible; su código de salida es siempre 0) y a la vez se lo pasa al PC principal, cuya respuesta dice si
/// ha ido bien.
#[tauri::command]
pub async fn client_stream_pair(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let _ = HANDLE.set(app.clone());
    let cfg = current(&state)?;
    let exe = moonlight(&cfg).ok_or("No se encuentra Moonlight: instálalo o elige dónde está en Ajustes → Este equipo")?;
    let ver = version(&app);
    let dir = state.config_dir.clone();
    blocking(move || {
        probe(&cfg, false);
        let addr = cfg.addr().to_string();
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() ^ (d.as_secs() as u32).rotate_left(13)).unwrap_or(1234);
        let pin = format!("{:04}", nanos % 10000);
        let mut child = std::process::Command::new(exe)
            .args(["pair", &rc::addr_host(&addr), "--pin", &pin])
            .spawn()
            .map_err(|e| format!("No se pudo abrir Moonlight: {e}"))?;
        match rc::stream_pair(&addr, &cfg.token, &ver, Some(&pin)) {
            Ok(v) if v["ok"].as_bool().unwrap_or(false) => Ok(()),
            Ok(v) => {
                let _ = child.kill();
                Err(v["error"].as_str().map(String::from).unwrap_or_else(|| "El PC principal no pudo emparejar el streaming".into()))
            }
            Err(e) => {
                let _ = child.kill();
                Err(host_err(&dir, format!("{e:#}")))
            }
        }
    })
    .await
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

/// Abre Moonlight con la app de Apollo que devolvió `launch_stream` («Juego actual»)
/// y vigila la sesión para cerrar Moonlight y devolver el visor al frente cuando acabe o se corte.
#[tauri::command]
pub fn client_open_stream(handle: AppHandle, state: State<'_, AppState>, app: String) -> Result<(), String> {
    let cfg = current(&state)?;
    let exe = moonlight(&cfg).ok_or("No se encuentra Moonlight: instálalo o elige dónde está en Ajustes → Este equipo")?;
    let host = rc::addr_host(cfg.addr());
    // Sin app «Juego actual» en Apollo, el escritorio (como la tablet).
    let app = if app.trim().is_empty() { "Desktop".to_string() } else { app };
    let child = std::process::Command::new(exe).args(["stream", &host, &app]).spawn().map_err(|e| format!("No se pudo abrir Moonlight: {e}"))?;
    start_stream_watch(handle, cfg, child);
    Ok(())
}

/// Cierra Moonlight si se abrió desde el visor y lo devuelve al primer plano.
#[tauri::command]
pub fn client_close_stream(app: AppHandle) {
    if !STREAM_ACTIVE.swap(false, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    STREAM_EPOCH.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    close_moonlight_processes();
    restore_visor_window(&app);
}

static STREAM_EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static STREAM_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static MOONLIGHT_CHILD: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

fn restore_visor_window(app: &AppHandle) {
    crate::launcher::allow_foreground();
    crate::tray::show_main(app);
}

fn scan_moonlight_processes(kill: bool) -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let mut found = false;
    for p in sys.processes().values() {
        if p.name().to_string_lossy().eq_ignore_ascii_case("Moonlight.exe") {
            found = true;
            if kill {
                p.kill();
            }
        }
    }
    found
}

fn close_moonlight_processes() {
    if let Some(mut c) = MOONLIGHT_CHILD.lock().unwrap().take() {
        let _ = c.kill();
        let _ = c.wait();
    }
    scan_moonlight_processes(true);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchAction {
    Continue,
    CloseAndReturn,
}

struct StreamWatchState {
    seen_active: bool,
    saw_pending: bool,
    idle_polls: u32,
    fails: u32,
}

impl StreamWatchState {
    fn new() -> Self {
        Self { seen_active: false, saw_pending: false, idle_polls: 0, fails: 0 }
    }

    fn step(&mut self, poll_ok: bool, state: &str) -> WatchAction {
        if !poll_ok {
            self.fails += 1;
            if self.fails >= 5 {
                return WatchAction::CloseAndReturn;
            }
            return WatchAction::Continue;
        }
        self.fails = 0;
        if state == "starting" || state == "playing" {
            self.seen_active = true;
            self.idle_polls = 0;
            return WatchAction::Continue;
        }
        if state == "pending" {
            self.saw_pending = true;
            self.idle_polls = 0;
            return WatchAction::Continue;
        }
        self.idle_polls += 1;
        if self.seen_active || self.saw_pending || self.idle_polls >= 3 {
            return WatchAction::CloseAndReturn;
        }
        WatchAction::Continue
    }
}

fn start_stream_watch(app: AppHandle, cfg: ClientCfg, child: std::process::Child) {
    let epoch = STREAM_EPOCH.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    STREAM_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
    *MOONLIGHT_CHILD.lock().unwrap() = Some(child);
    let ver = version(&app);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        let mut watch = StreamWatchState::new();
        let started = std::time::Instant::now();
        loop {
            if STREAM_EPOCH.load(std::sync::atomic::Ordering::SeqCst) != epoch || !STREAM_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            let child_exited = {
                let mut g = MOONLIGHT_CHILD.lock().unwrap();
                match g.as_mut() {
                    Some(c) => match c.try_wait() {
                        Ok(Some(_)) | Err(_) => {
                            *g = None;
                            true
                        }
                        Ok(None) => false,
                    },
                    None => true,
                }
            };
            if child_exited && started.elapsed() >= Duration::from_secs(2) && !scan_moonlight_processes(false) {
                if STREAM_ACTIVE.swap(false, std::sync::atomic::Ordering::SeqCst) {
                    restore_visor_window(&app);
                }
                break;
            }
            let res = match rc::session(cfg.addr(), &cfg.token, &ver) {
                Err(e) if cfg.host_addrs.len() > 1 && e.chain().any(|c| c.downcast_ref::<rc::Unreachable>().is_some()) => {
                    let before = cfg.addr().to_string();
                    match probe(&cfg, true) {
                        Some(a) if a != before => rc::session(&a, &cfg.token, &ver),
                        _ => Err(e),
                    }
                }
                r => r,
            };
            let act = match res {
                Ok(v) => watch.step(true, v["state"].as_str().unwrap_or("idle")),
                Err(_) => watch.step(false, ""),
            };
            if act == WatchAction::CloseAndReturn {
                if STREAM_ACTIVE.swap(false, std::sync::atomic::Ordering::SeqCst) {
                    close_moonlight_processes();
                    restore_visor_window(&app);
                }
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
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
                Some(a) if a != before => rc::call(&a, &cfg.token, ver, cmd, args).map_err(|e| host_err(&cfg.dir, format!("{e:#}"))),
                _ => Err(host_err(&cfg.dir, format!("{e:#}"))),
            }
        }
        r => r.map_err(|e| host_err(&cfg.dir, format!("{e:#}"))),
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

/// `true` si este equipo esta emparejado como cliente de un PC principal.
pub fn is_paired(app: &AppHandle) -> bool {
    ClientCfg::load(&app.state::<AppState>().config_dir).is_some()
}

/// `true` si Moonlight esta en primer plano en este equipo.
pub fn moonlight_in_foreground() -> bool {
    crate::platform::foreground_info()
        .map(|(_, info)| info.exe_name.eq_ignore_ascii_case("moonlight.exe"))
        .unwrap_or(false)
}

/// Pide al PC principal abrir o cerrar su overlay sobre el streaming.
pub fn toggle_remote_overlay(app: &AppHandle) {
    let dir = app.state::<AppState>().config_dir.clone();
    let Some(cfg) = ClientCfg::load(&dir) else { return };
    let ver = version(app);
    std::thread::spawn(move || {
        let _ = rc::overlay(cfg.addr(), &cfg.token, &ver);
    });
}

fn spawn_overlay_hotkey(app: AppHandle) {
    let app_vk = app.clone();
    crate::input::spawn_key_sink(
        move || {
            let hk = app_vk.state::<AppState>().config.lock().unwrap().overlay_hotkey.clone();
            crate::input::reserved_fkey_vk(&hk)
        },
        move || {
            if is_paired(&app) && moonlight_in_foreground() {
                toggle_remote_overlay(&app);
            }
        },
    );
}

/// Hilo que sube las partidas locales al host cada minuto (si este equipo es cliente).
pub fn spawn_sync(app: AppHandle) {
    let _ = HANDLE.set(app.clone());
    spawn_overlay_hotkey(app.clone());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vigilancia_moonlight_cierra_al_terminar_o_caer_host() {
        let mut w = StreamWatchState::new();
        assert_eq!(w.step(true, "pending"), WatchAction::Continue);
        assert_eq!(w.step(true, "playing"), WatchAction::Continue);
        assert_eq!(w.step(true, "idle"), WatchAction::CloseAndReturn);

        let mut caido = StreamWatchState::new();
        assert_eq!(caido.step(true, "playing"), WatchAction::Continue);
        for _ in 0..4 {
            assert_eq!(caido.step(false, ""), WatchAction::Continue);
        }
        assert_eq!(caido.step(true, "playing"), WatchAction::Continue);
        for _ in 0..4 {
            assert_eq!(caido.step(false, ""), WatchAction::Continue);
        }
        assert_eq!(caido.step(false, ""), WatchAction::CloseAndReturn);

        let mut ya_libre = StreamWatchState::new();
        assert_eq!(ya_libre.step(true, "idle"), WatchAction::Continue);
        assert_eq!(ya_libre.step(true, "idle"), WatchAction::Continue);
        assert_eq!(ya_libre.step(true, "idle"), WatchAction::CloseAndReturn);
    }
}
