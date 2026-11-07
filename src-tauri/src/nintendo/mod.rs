//! Integracion con Nintendo Switch (solo escritorio Windows). La logica pura esta en
//! `visor_core::nintendo`; aqui va lo que toca el sistema (credenciales, ventana de login, red).

pub mod auth;
pub mod history;
pub mod import;
pub mod login;
pub mod secrets;
pub mod sync;

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use visor_core::db::{data as dbd, Db};
use visor_core::nintendo::store::{self, HistorySaved};

use history::SyncError;

/// Minimo entre dos peticiones reales al historial (la importacion y el hilo periodico no piden dos veces).
pub const MIN_FETCH_GAP: Duration = Duration::from_secs(120);

fn session() -> &'static Mutex<auth::Session> {
    static S: OnceLock<Mutex<auth::Session>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(auth::Session::new()))
}

fn last_fetch() -> &'static Mutex<Option<Instant>> {
    static L: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(None))
}

/// Resultado de [`refresh`]. `fetched: false` si se reutilizo una descarga reciente.
#[derive(Debug, Default)]
pub struct Refresh {
    pub fetched: bool,
    pub saved: HistorySaved,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Unico punto de entrada para descargar y guardar el historial. Serializa las descargas y anota
/// en `meta` la ultima sincronizacion (`nintendo.last_sync_ms`) y el ultimo error (`nintendo.last_error`).
pub fn refresh(db: &Db) -> Result<Refresh, SyncError> {
    // El candado de la ultima peticion se mantiene toda la descarga: otra llamada espera y luego ve que es reciente.
    let mut last = last_fetch().lock().unwrap_or_else(|e| e.into_inner());
    if last.is_some_and(|t| t.elapsed() < MIN_FETCH_GAP) {
        return Ok(Refresh::default());
    }
    let result = {
        let mut s = session().lock().unwrap_or_else(|e| e.into_inner());
        history::fetch(&mut s)
    };
    match result {
        Ok(ph) => {
            *last = Some(Instant::now());
            let now = now_ms();
            let saved = db
                .with(|c| {
                    let saved = store::save_history(c, &ph.title_upserts(), &ph.daily_rows(), now)?;
                    dbd::meta_set(c, "nintendo.last_sync_ms", &now.to_string())?;
                    dbd::meta_set(c, "nintendo.last_error", "")?;
                    Ok::<_, anyhow::Error>(saved)
                })
                .map_err(|e| SyncError::Transient(format!("no se pudo guardar el historial: {e}")))?;
            Ok(Refresh { fetched: true, saved })
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = db.with(|c| dbd::meta_set(c, "nintendo.last_error", &msg));
            Err(e)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RefreshReport {
    pub fetched: bool,
    pub titles: usize,
    pub days: usize,
    pub new: Vec<String>,
    pub changed: Vec<String>,
}

/// Descarga y guarda el historial. Sirve para probar a mano hasta que exista la interfaz.
#[tauri::command]
pub async fn nintendo_refresh(state: tauri::State<'_, crate::state::AppState>) -> Result<RefreshReport, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || refresh(&db))
        .await
        .map_err(|e| e.to_string())?
        .map(|r| RefreshReport { fetched: r.fetched, titles: r.saved.titles, days: r.saved.days, new: r.saved.new, changed: r.saved.changed })
        .map_err(|e| e.to_string())
}
