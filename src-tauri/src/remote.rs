//! Acceso desde el movil: arranca/para el servidor de `visor_core::remote` segun la configuracion y le da los
//! datos de la app (la misma biblioteca que `scan_library`, sin ocultos).
use crate::state::{self, AppState};
use serde::Serialize;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use tauri::{AppHandle, Listener, Manager, State};
use visor_core::db::data as dbd;
use visor_core::remote::{self, Playing, Provider, RemoteConfig, RemoteServer, Snapshot};

struct AppProvider {
    app: AppHandle,
}

impl Provider for AppProvider {
    fn name(&self) -> String {
        remote::host_name()
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
        let playing = st.active.lock().unwrap().as_ref().map(|a| Playing { id: a.id.clone(), name: a.name.clone(), since: a.since });
        Ok(Snapshot { config: RemoteConfig::from_config(&cfg), library: lib, favorites, sagas, playing })
    }

    fn thumbs_dir(&self) -> PathBuf {
        self.app.state::<AppState>().thumbs_dir()
    }

    fn previews_dir(&self) -> PathBuf {
        self.app.state::<AppState>().previews_dir()
    }

    fn launch(&self, id: &str) -> anyhow::Result<()> {
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

    fn call(&self, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        crate::remote_call::dispatch(&self.app, cmd, args)
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
    "sgdb-art",
    "sgdb-done",
    "sgdb-progress",
    "thumb-ready",
];

/// Escucha los eventos de `FORWARDED_EVENTS` y los deja en el bus de `visor_core::remote`. Se llama una vez.
pub fn forward_events(app: &AppHandle) {
    for name in FORWARDED_EVENTS {
        app.listen_any(*name, move |ev| {
            let payload = serde_json::from_str(ev.payload()).unwrap_or(serde_json::Value::Null);
            remote::event_bus().push(name, payload);
        });
    }
}

/// El acceso desde el móvil está activo; el guard de apply() sirve como interruptor de desarrollo.
const REMOTE_AVAILABLE: bool = true;

/// Pone el servidor como dice la configuracion: lo arranca, lo para, lo reinicia si cambio el puerto o le
/// pasa el codigo nuevo. Si esta activado y no hay codigo, genera uno y lo guarda.
pub fn apply(app: &AppHandle) {
    if !REMOTE_AVAILABLE {
        return;
    }
    let st = app.state::<AppState>();
    let (enabled, port, token) = {
        let mut c = st.config.lock().unwrap();
        if c.remote_enabled && c.remote_token.is_empty() {
            c.remote_token = remote::new_code();
            if let Err(e) = c.save(&st.config_file()) {
                eprintln!("config (codigo del movil): {e:#}");
            }
        }
        (c.remote_enabled, c.remote_port, c.remote_token.clone())
    };
    let mut cur = st.remote.lock().unwrap();
    if !enabled {
        if let Some(s) = cur.take() {
            s.stop();
        }
        *st.remote_error.lock().unwrap() = None;
        return;
    }
    if let Some(s) = cur.as_ref() {
        if s.local_addr().port() == port {
            s.set_token(&token);
            return;
        }
    }
    if let Some(s) = cur.take() {
        s.stop();
    }
    let bind = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));
    match RemoteServer::start(bind, token, AppProvider { app: app.clone() }) {
        Ok(s) => {
            *cur = Some(s);
            *st.remote_error.lock().unwrap() = None;
        }
        Err(e) => {
            eprintln!("servidor del movil: {e:#}");
            *st.remote_error.lock().unwrap() = Some(format!("{e:#}"));
        }
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
    code: String,
    /// Ultimo error al abrir el puerto (p. ej. ya en uso).
    error: Option<String>,
}

fn info(st: &AppState) -> RemoteInfo {
    let (enabled, port, code) = {
        let c = st.config.lock().unwrap();
        (c.remote_enabled, c.remote_port, c.remote_token.clone())
    };
    let running = st.remote.lock().unwrap().is_some();
    RemoteInfo {
        enabled,
        running,
        port,
        addresses: if running { remote::local_ipv4s() } else { Vec::new() },
        code,
        error: st.remote_error.lock().unwrap().clone(),
    }
}

#[tauri::command]
pub fn remote_info(state: State<'_, AppState>) -> RemoteInfo {
    info(&state)
}

/// Genera un codigo de emparejamiento nuevo (el movil tendra que escribirlo otra vez).
#[tauri::command]
pub fn remote_new_code(app: AppHandle, state: State<'_, AppState>) -> Result<RemoteInfo, String> {
    {
        let mut c = state.config.lock().unwrap();
        let mut next = c.clone();
        next.remote_token = remote::new_code();
        next.save(&state.config_file()).map_err(|e| e.to_string())?;
        *c = next;
    }
    apply(&app);
    Ok(info(&state))
}
