//! Acceso desde el movil: arranca/para el servidor de `visor_core::remote` segun la configuracion y le da los
//! datos de la app (la misma biblioteca que `scan_library`, sin ocultos).
use crate::state::{self, AppState};
use serde::Serialize;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Listener, Manager, State};
use tauri_plugin_notification::{Action, ActionType, NotificationExt};
use visor_core::db::data as dbd;
use visor_core::devices;
use visor_core::remote::{self, Caller, Discovery, PairRequest, Playing, Provider, RemoteConfig, RemoteServer, Snapshot};

/// Respondedor UDP de "hay un visor por aqui?" (vive mientras el servidor esta activado).
static DISCOVERY: Mutex<Option<Discovery>> = Mutex::new(None);

struct AppProvider {
    app: AppHandle,
}

/// Nombre de este equipo para los demas dispositivos: el que le puso el usuario o, si no, el de Windows.
pub fn display_name(st: &AppState) -> String {
    let n = st.config.lock().unwrap().device_name.trim().to_string();
    if n.is_empty() {
        remote::host_name()
    } else {
        n
    }
}

/// Nombre de un dispositivo para mostrarlo (`''` = este equipo).
pub fn device_label(st: &AppState, id: &str) -> String {
    match id {
        "" => String::new(),
        _ => st.db.with(|c| devices::get(c, id)).ok().flatten().map(|d| d.name).unwrap_or_else(|| "Otro dispositivo".into()),
    }
}

fn caller_of(d: devices::Device) -> Caller {
    Caller { device_id: d.id, name: d.name, kind: d.kind }
}

impl Provider for AppProvider {
    fn name(&self) -> String {
        display_name(&self.app.state::<AppState>())
    }

    fn id(&self) -> String {
        self.app.state::<AppState>().config.lock().unwrap().remote_id.clone()
    }

    fn snapshot(&self) -> anyhow::Result<Snapshot> {
        let st = self.app.state::<AppState>();
        let (mut lib, found, cfg) = crate::scan_full(&st);
        *st.games.lock().unwrap() = found;
        // El movil nunca ve los juegos ocultos (aunque el escritorio los muestre atenuados).
        for p in lib.platforms.iter_mut() {
            p.titles.retain(|t| !t.hidden);
        }
        lib.hidden.clear();
        let favorites = st.favorites.lock().unwrap().clone();
        let sagas = state::log_db("sagas", st.db.with(dbd::sagas_load));
        let active = st.active.lock().unwrap().clone();
        let playing = active.map(|a| Playing { device: device_label(&st, &a.device), id: a.id, name: a.name, since: a.since });
        Ok(Snapshot { config: RemoteConfig::from_config(&cfg), library: lib, favorites, sagas, playing })
    }

    fn thumbs_dir(&self) -> PathBuf {
        self.app.state::<AppState>().thumbs_dir()
    }

    fn previews_dir(&self) -> PathBuf {
        self.app.state::<AppState>().previews_dir()
    }

    fn device_by_token(&self, token: &str) -> Option<Caller> {
        let st = self.app.state::<AppState>();
        st.db.with(|c| devices::by_token(c, token)).ok().flatten().map(caller_of)
    }

    fn pair(&self, name: &str, kind: &str, version: &str, api: u32) -> anyhow::Result<(String, String)> {
        let st = self.app.state::<AppState>();
        let r = st.db.with(|c| devices::create(c, name, kind, version, api))?;
        let _ = self.app.emit("devices-changed", ());
        Ok(r)
    }

    fn unpair(&self, caller: &Caller) -> anyhow::Result<()> {
        revoke_device(&self.app, &caller.device_id)
    }

    fn pair_requested(&self, req: &PairRequest) {
        let _ = self.app.emit("pair-request", PairRequestView::from(req));
        notify_request(&self.app, req);
    }

    fn pair_requests_changed(&self) {
        // Sin lista adjunta: este aviso puede llegar con el servidor cogido, la interfaz pide `pair_pending`.
        let _ = self.app.emit("pair-requests-changed", ());
    }

    fn stream_pair(&self, caller: &Caller, pin: Option<&str>) -> anyhow::Result<serde_json::Value> {
        use crate::apollo::web;
        if !web::creds_present() {
            anyhow::bail!("Falta configurar Apollo en el PC principal: hay que guardar su usuario y contraseña en Ajustes.");
        }
        let name = caller.name.clone();
        // Los clientes que ya habia, para reconocer despues el nuevo aunque comparta nombre con uno antiguo.
        let before: Vec<String> = web::clients().map(|v| v.into_iter().map(|c| c.uuid).collect()).unwrap_or_default();
        let (out, secs) = match pin {
            Some(pin) => {
                web::pair_pin(pin, &name).map_err(|e| anyhow::anyhow!(e))?;
                (serde_json::json!({ "ok": true }), 90)
            }
            None => {
                let (pin, passphrase) = web::otp_new(&name).map_err(|e| anyhow::anyhow!(e))?;
                (serde_json::json!({ "pin": pin, "passphrase": passphrase, "port": web::stream_port() }), 190)
            }
        };
        // El cliente aparece en Apollo cuando termina el emparejamiento (en el flujo OTP, cuando acaba Artemis):
        // se busca en segundo plano para no dejar el hilo del servidor esperando.
        let (app, id) = (self.app.clone(), caller.device_id.clone());
        std::thread::spawn(move || link_apollo_client(&app, &id, &name, &before, secs));
        Ok(out)
    }

    fn device_seen(&self, caller: &Caller, version: &str, api: u32) {
        let st = self.app.state::<AppState>();
        state::log_db_err("dispositivos", st.db.with(|c| devices::touch(c, &caller.device_id, version, api)));
    }

    /// Lanzar sin streaming es jugar en el host: la partida es del host aunque la pida otro dispositivo.
    fn launch(&self, _caller: &Caller, id: &str) -> anyhow::Result<()> {
        let id = id.to_string();
        let app = self.app.clone();
        // El lanzador toca la ventana y la pantalla principal: va en el hilo principal, como el comando del visor.
        let (tx, rx) = std::sync::mpsc::channel();
        self.app
            .run_on_main_thread(move || {
                let st = app.state::<AppState>();
                let mut game = st.games.lock().unwrap().iter().find(|g| g.id == id).cloned();
                if game.is_none() {
                    // Aun no hay escaneo (o el juego es nuevo): se escanea igual que la instantanea.
                    let (_, found, _) = crate::scan_full(&st);
                    game = found.iter().find(|g| g.id == id).cloned();
                    *st.games.lock().unwrap() = found;
                }
                let r = match game {
                    Some(g) => crate::launcher::start(&app, g),
                    None => Err("Juego no encontrado".to_string()),
                };
                let _ = tx.send(r);
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        rx.recv_timeout(std::time::Duration::from_secs(30)).map_err(|_| anyhow::anyhow!("El visor no respondió a tiempo"))?.map_err(|e| anyhow::anyhow!(e))
    }

    fn capture(&self) -> anyhow::Result<()> {
        // snap::trigger ya trabaja en su propio hilo y no necesita el principal.
        crate::snap::trigger(&self.app);
        Ok(())
    }

    fn overlay(&self) -> anyhow::Result<()> {
        crate::quickmenu::toggle(&self.app).map_err(|e| anyhow::anyhow!(e))
    }

    fn close(&self, force: bool) -> anyhow::Result<serde_json::Value> {
        crate::stream::close(&self.app, force).map_err(|e| anyhow::anyhow!(e))
    }

    fn session(&self) -> serde_json::Value {
        crate::stream::session_json()
    }

    fn session_start(&self, w: u32, h: u32, fps: u32) -> anyhow::Result<serde_json::Value> {
        crate::stream::session_start(&self.app, w, h, fps).map_err(|e| anyhow::anyhow!(e))
    }

    fn call(&self, caller: &Caller, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        crate::remote_call::dispatch(&self.app, caller, cmd, args)
    }

    fn install_source(&self, caller: &Caller, id: &str) -> anyhow::Result<visor_core::gamecopy::InstallSource> {
        crate::installs::install_source(&self.app.state::<AppState>(), caller, id).map_err(|e| anyhow::anyhow!(e))
    }

    fn asset_roots(&self) -> Vec<PathBuf> {
        let st = self.app.state::<AppState>();
        let mut v = vec![st.thumbs_dir(), st.previews_dir(), st.paths.art.clone(), st.paths.saves.clone(), st.paths.glossary.clone()];
        v.extend(st.paths.captures.clone());
        // Raiz de la biblioteca de juegos (capturas, portadas, hero y logos que viven junto a ellas).
        if let Some(r) = st.config.lock().unwrap().root.as_deref().filter(|r| !r.trim().is_empty()) {
            v.push(PathBuf::from(r));
        }
        v
    }
}

/// Eventos de Tauri que se reenvian a las tablets (`GET /api/v1/events`). Quedan fuera los de la ventana del PC:
/// el texthook (`th-*`), el lector de novelas (`novel-prefs`) y el aprendizaje del mando (`learn-pad`).
const FORWARDED_EVENTS: &[&str] = &[
    "achievement-unlocked",
    "achievements-rarity-updated",
    "audio-outputs",
    "capture-error",
    "capture-saved",
    "game-exited",
    "game-started",
    "import-progress",
    "library-progress",
    "magpie-status",
    "saves-copied",
    "session-summary",
    "sgdb-art",
    "sgdb-done",
    "sgdb-progress",
    "thumb-ready",
];

/// Escucha los eventos de `FORWARDED_EVENTS` y los deja en el bus de `visor_core::remote`. Se llama una vez.
pub fn forward_events(app: &AppHandle) {
    setup_notifications(app);
    for name in FORWARDED_EVENTS {
        app.listen_any(*name, move |ev| {
            let payload = serde_json::from_str(ev.payload()).unwrap_or(serde_json::Value::Null);
            remote::event_bus().push(name, payload);
        });
    }
}

/// El acceso desde el móvil está activo; el guard de apply() sirve como interruptor de desarrollo.
const REMOTE_AVAILABLE: bool = true;

/// Pone el servidor como dice la configuracion: lo arranca, lo para o lo reinicia si cambio el puerto.
/// Si esta activado y no hay identificador del PC, genera uno y lo guarda.
pub fn apply(app: &AppHandle) {
    if !REMOTE_AVAILABLE {
        return;
    }
    let st = app.state::<AppState>();
    let (enabled, port, id) = {
        let mut c = st.config.lock().unwrap();
        if c.remote_id.is_empty() {
            c.remote_id = remote::new_id();
            if let Err(e) = c.save(&st.config_file()) {
                eprintln!("config (identificador del PC): {e:#}");
            }
        }
        (c.remote_enabled, c.remote_port, c.remote_id.clone())
    };
    let mut cur = st.remote.lock().unwrap();
    if !enabled {
        if let Some(s) = cur.take() {
            s.stop();
        }
        *DISCOVERY.lock().unwrap() = None;
        *st.remote_error.lock().unwrap() = None;
        return;
    }
    if let Some(s) = cur.as_ref() {
        if s.local_addr().port() == port {
            return;
        }
    }
    if let Some(s) = cur.take() {
        s.stop();
    }
    *DISCOVERY.lock().unwrap() = None; // libera el puerto UDP antes de abrirlo con el puerto nuevo
    let bind = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));
    match RemoteServer::start(bind, AppProvider { app: app.clone() }) {
        Ok(s) => {
            *cur = Some(s);
            *st.remote_error.lock().unwrap() = None;
            // Que la tablet pueda encontrar el PC por su identificador aunque cambie de IP.
            start_discovery(&st, id, port);
        }
        Err(e) => {
            eprintln!("servidor del movil: {e:#}");
            *st.remote_error.lock().unwrap() = Some(format!("{e:#}"));
        }
    }
}

/// Respondedor UDP con el nombre de ahora (se reinicia si cambia el nombre del equipo).
fn start_discovery(st: &AppState, id: String, port: u16) {
    *DISCOVERY.lock().unwrap() = None;
    match Discovery::start(SocketAddr::from((Ipv4Addr::UNSPECIFIED, remote::DISCOVERY_PORT)), id, display_name(st), port) {
        Ok(d) => *DISCOVERY.lock().unwrap() = Some(d),
        Err(e) => eprintln!("descubrimiento de la tablet: {e:#}"),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    enabled: bool,
    running: bool,
    port: u16,
    /// IPv4 de la red local (la principal primero).
    addresses: Vec<String>,
    /// Identificador fijo del PC.
    id: String,
    /// Nombre de este equipo para los demas dispositivos.
    name: String,
    /// Obsoleto (el QR nuevo lo da `pair_ticket_new`): siempre vacio.
    link: String,
    /// Obsoleto: siempre vacio.
    qr: String,
    /// Ultimo error al abrir el puerto (p. ej. ya en uso).
    error: Option<String>,
}

fn info(st: &AppState) -> RemoteInfo {
    let (enabled, port, id) = {
        let c = st.config.lock().unwrap();
        (c.remote_enabled, c.remote_port, c.remote_id.clone())
    };
    let running = st.remote.lock().unwrap().is_some();
    let addresses = if running { remote::local_ipv4s() } else { Vec::new() };
    let name = display_name(st);
    RemoteInfo {
        enabled,
        running,
        port,
        addresses,
        id,
        name,
        link: String::new(),
        qr: String::new(),
        error: st.remote_error.lock().unwrap().clone(),
    }
}

#[tauri::command]
pub fn remote_info(state: State<'_, AppState>) -> RemoteInfo {
    info(&state)
}

/* --- dispositivos emparejados (solo desde el escritorio del host; no estan en `remote_call::dispatch`) --- */

#[tauri::command]
pub fn devices_list(state: State<'_, AppState>) -> Result<Vec<devices::Device>, String> {
    state.db.with(devices::list).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn device_rename(app: AppHandle, state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    state.db.with(|c| devices::rename(c, &id, &name)).map_err(|e| e.to_string())?;
    let _ = app.emit("devices-changed", ());
    Ok(())
}

/// Quita un dispositivo: su token deja de valer y tendra que emparejarse otra vez.
#[tauri::command]
pub fn device_revoke(app: AppHandle, id: String) -> Result<(), String> {
    revoke_device(&app, &id).map_err(|e| e.to_string())
}

/// Revoca el token de un dispositivo y, si tenia el streaming emparejado, lo quita tambien de Apollo (en segundo
/// plano: si Apollo falla se registra y se sigue, quitar el dispositivo no debe fallar por eso).
fn revoke_device(app: &AppHandle, id: &str) -> anyhow::Result<()> {
    let st = app.state::<AppState>();
    let apollo_id = st.db.with(|c| devices::apollo_id(c, id)).ok().flatten();
    st.db.with(|c| devices::revoke(c, id))?;
    let _ = app.emit("devices-changed", ());
    if let Some(uuid) = apollo_id {
        std::thread::spawn(move || {
            if let Err(e) = crate::apollo::web::unpair_client(&uuid) {
                eprintln!("apollo (quitar cliente): {e}");
            }
        });
    }
    Ok(())
}

/// Nombre de este equipo para los demas dispositivos (vacio = el de Windows).
#[tauri::command]
pub fn set_device_name(state: State<'_, AppState>, name: String) -> Result<RemoteInfo, String> {
    let name: String = name.trim().chars().take(60).collect();
    let (id, port) = {
        let mut c = state.config.lock().unwrap();
        let mut next = c.clone();
        next.device_name = name;
        next.save(&state.config_file()).map_err(|e| e.to_string())?;
        *c = next;
        (c.remote_id.clone(), c.remote_port)
    };
    if state.remote.lock().unwrap().is_some() {
        start_discovery(&state, id, port);
    }
    Ok(info(&state))
}

/* --- solicitudes de emparejamiento, billetes y credenciales de Apollo --- */

/// Solicitud pendiente tal como la ve la interfaz (evento `pair-request` y comando `pair_pending`).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PairRequestView {
    id: String,
    name: String,
    kind: String,
    digits: String,
    ip: String,
    expires_ms: i64,
}

impl From<&PairRequest> for PairRequestView {
    fn from(r: &PairRequest) -> Self {
        Self { id: r.id_public.clone(), name: r.name.clone(), kind: r.kind.clone(), digits: r.digits.clone(), ip: r.ip.clone(), expires_ms: r.expires_ms }
    }
}

const NOTIF_TYPE: &str = "pair-request";

/// Registra los botones de la notificacion y el manejador de sus pulsaciones. Va antes de mostrar ninguna:
/// sin manejador registrado, Windows no pinta los botones.
fn setup_notifications(app: &AppHandle) {
    let n = app.notification();
    let buttons = vec![Action::builder("approve", "Aprobar").build(), Action::builder("reject", "Rechazar").build()];
    if let Err(e) = n.register_action_types(vec![ActionType::builder(NOTIF_TYPE).actions(buttons).build()]) {
        eprintln!("notificacion de solicitudes: {e}");
    }
    let handle = app.clone();
    let _ = n.on_action(move |a| {
        let req = a.notification().and_then(|n| n.extra().get("req")).and_then(|v| v.as_str()).map(str::to_string);
        match (a.action_id(), req) {
            ("approve", Some(id)) => {
                let st = handle.state::<AppState>();
                let r = st.remote.lock().unwrap().as_ref().map(|s| s.approve(&id));
                if let Some(Err(e)) = r {
                    eprintln!("aprobar desde la notificacion: {e:#}");
                }
            }
            ("reject", Some(id)) => {
                let st = handle.state::<AppState>();
                let r = st.remote.lock().unwrap().as_ref().map(|s| s.reject(&id));
                if let Some(Err(e)) = r {
                    eprintln!("rechazar desde la notificacion: {e:#}");
                }
            }
            // Pulsar la notificacion (no un boton): trae el visor al frente, donde esta el modal.
            _ => crate::tray::show_main(&handle),
        }
    });
}

/// Notificacion normal de Windows: «<nombre> quiere conectarse» con el numero de 4 cifras y dos botones.
fn notify_request(app: &AppHandle, req: &PairRequest) {
    let r = app
        .notification()
        .builder()
        .title(format!("{} quiere conectarse", req.name))
        .body(format!("Comprueba que en su pantalla sale el número {}.", req.digits))
        .action_type_id(NOTIF_TYPE)
        .extra("req", req.id_public.clone())
        .show();
    if let Err(e) = r {
        eprintln!("notificacion de solicitud: {e}");
    }
}

/// Busca en Apollo el cliente recien emparejado y guarda su uuid en el dispositivo. Reintenta cada 2 s hasta `secs`.
fn link_apollo_client(app: &AppHandle, device_id: &str, name: &str, before: &[String], secs: u64) {
    use crate::apollo::web;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    let mut found = None;
    loop {
        if let Ok(list) = web::clients() {
            found = list.into_iter().find(|c| c.name == name && !before.contains(&c.uuid)).map(|c| c.uuid);
        }
        if found.is_some() || std::time::Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    // Sin cliente nuevo (p. ej. se repitio el emparejamiento con el mismo nombre): se usa el que ya hubiera.
    let found = found.or_else(|| web::client_uuid_by_name(name).ok().flatten());
    match found {
        Some(uuid) => {
            let st = app.state::<AppState>();
            state::log_db_err("dispositivos (id de Apollo)", st.db.with(|c| devices::set_apollo_id(c, device_id, Some(&uuid))));
            // Apollo solo da permiso de lanzar al primer cliente: sin esto Moonlight falla con «Permission denied» (503).
            if let Err(e) = web::grant_full(&uuid) {
                eprintln!("apollo: no se pudieron dar permisos a «{name}»: {e}");
            }
            let _ = app.emit("devices-changed", ());
        }
        None => eprintln!("apollo: no aparecio el cliente «{name}» tras emparejar"),
    }
}

#[tauri::command]
pub fn pair_pending(state: State<'_, AppState>) -> Vec<PairRequestView> {
    state.remote.lock().unwrap().as_ref().map(|s| s.pending_requests().iter().map(PairRequestView::from).collect()).unwrap_or_default()
}

#[tauri::command]
pub fn pair_approve(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let g = state.remote.lock().unwrap();
    g.as_ref().ok_or("El acceso desde otros dispositivos está desactivado")?.approve(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pair_reject(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let g = state.remote.lock().unwrap();
    g.as_ref().ok_or("El acceso desde otros dispositivos está desactivado")?.reject(&id).map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketInfo {
    /// Billete `ABCD-EFGH` (para escribirlo a mano).
    ticket: String,
    expires_ms: i64,
    /// Texto del QR v2 (JSON) para pintarlo en la interfaz.
    qr: String,
}

#[tauri::command]
pub fn pair_ticket_new(state: State<'_, AppState>) -> Result<TicketInfo, String> {
    let id = state.config.lock().unwrap().remote_id.clone();
    let name = display_name(&state);
    let g = state.remote.lock().unwrap();
    let s = g.as_ref().ok_or("El acceso desde otros dispositivos está desactivado")?;
    let t = s.new_ticket();
    let qr = remote::qr_payload(&id, &name, &s.addrs(), s.local_addr().port(), &t.code);
    Ok(TicketInfo { ticket: t.code, expires_ms: t.expires_ms, qr })
}

#[tauri::command]
pub fn pair_ticket_cancel(state: State<'_, AppState>) {
    if let Some(s) = state.remote.lock().unwrap().as_ref() {
        s.cancel_ticket();
    }
}

/// Direcciones `ip:puerto` de este PC (vacio si el servidor no esta en marcha).
#[tauri::command]
pub fn host_addrs(state: State<'_, AppState>) -> Vec<String> {
    state.remote.lock().unwrap().as_ref().map(|s| s.addrs()).unwrap_or_default()
}

/// Estado de la API web de Apollo y si hay credenciales guardadas (la contrasena nunca sale del backend).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApolloWebInfo {
    /// `noCredentials`, `badCredentials`, `apolloOff`, `incompatible` u `ok`.
    state: crate::apollo::web::WebState,
    has_credentials: bool,
}

fn apollo_web_info() -> ApolloWebInfo {
    use crate::apollo::web;
    ApolloWebInfo { state: web::web_check(), has_credentials: web::creds_present() }
}

#[tauri::command]
pub async fn apollo_creds_set(user: String, pass: String) -> Result<ApolloWebInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::apollo::web::creds_set(&user, &pass)?;
        Ok(apollo_web_info())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apollo_creds_clear() -> Result<ApolloWebInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::apollo::web::creds_clear()?;
        Ok(apollo_web_info())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apollo_web_state() -> Result<ApolloWebInfo, String> {
    tauri::async_runtime::spawn_blocking(apollo_web_info).await.map_err(|e| e.to_string())
}
