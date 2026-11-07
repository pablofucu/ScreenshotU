//! Hilo de sincronizacion periodica con Nintendo (SW-18). Solo llama a `nintendo::refresh`, que escribe
//! unicamente las tablas `switch_*` y `meta`: nunca crea ni cambia fichas (`games`).

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use visor_core::db::data as dbd;
use visor_core::nintendo::{import, merge, next_delay, store, FIRST_SYNC_DELAY};
use visor_core::scan::{self, TitleTimes};

use super::history::SyncError;
use super::{auth, refresh};
use crate::state::AppState;

/// Evento que se emite tras cada sincronizacion (`Synced`).
pub const EVENT: &str = "nintendo-synced";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Synced {
    changed: usize,
    new_titles: usize,
    expired: bool,
    error: Option<String>,
}

fn sender() -> &'static Mutex<Option<Sender<()>>> {
    static S: OnceLock<Mutex<Option<Sender<()>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None))
}

/// Un paso del bucle: sincroniza y devuelve cuanto esperar y el aviso para la interfaz.
fn step(app: &AppHandle, streak: &mut u32) -> (Duration, Option<Synced>) {
    if !auth::is_connected() {
        *streak = 0;
        return (next_delay(0), None);
    }
    let state = app.state::<AppState>();
    let db = state.db.clone();
    let min = state.config.lock().unwrap().nintendo_min_minutes as i64;
    let result = refresh(&db);
    let (wait, changed, expired, error) = match result {
        Ok(r) => {
            *streak = 0;
            (next_delay(0), r.saved.changed.len(), false, None)
        }
        Err(SyncError::NotConnected) => (next_delay(0), 0, false, None),
        Err(e @ SyncError::Expired(_)) => (next_delay(0), 0, true, Some(e.to_string())),
        Err(e) => {
            *streak += 1;
            (next_delay(*streak), 0, false, Some(e.to_string()))
        }
    };
    let new_titles = db.with(|c| store::titles_load(c)).map(|t| import::pending_count(&t, min)).unwrap_or(0);
    (wait, Some(Synced { changed, new_titles, expired, error }))
}

/// Arranca el hilo. Espera `FIRST_SYNC_DELAY` y luego `next_delay`; `nintendo_sync_now` lo despierta al momento.
pub fn spawn(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<()>();
    *sender().lock().unwrap() = Some(tx);
    let _ = std::thread::Builder::new().name("nintendo-sync".into()).spawn(move || {
        let mut wait = FIRST_SYNC_DELAY;
        let mut streak = 0u32;
        loop {
            match rx.recv_timeout(wait) {
                Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            // Si llegaron varias ordenes seguidas, una sola sincronizacion basta.
            while rx.try_recv().is_ok() {}
            let out = catch_unwind(AssertUnwindSafe(|| step(&app, &mut streak)));
            match out {
                Ok((w, ev)) => {
                    wait = w;
                    if let Some(ev) = ev {
                        let _ = app.emit(EVENT, ev);
                    }
                }
                Err(_) => {
                    eprintln!("[nintendo-sync] panico durante la sincronizacion");
                    streak += 1;
                    wait = next_delay(streak);
                }
            }
        }
    });
}

/// Pide una sincronizacion inmediata (la descarga real sigue limitada por `MIN_FETCH_GAP`).
#[tauri::command]
pub fn nintendo_sync_now() {
    if let Some(tx) = sender().lock().unwrap().as_ref() {
        let _ = tx.send(());
    }
}

/// Tiempos de los titulos de Switch con tiempo de consola, para parchear la interfaz sin reescanear el disco.
#[tauri::command]
pub fn nintendo_title_times(state: State<'_, AppState>) -> Result<Vec<TitleTimes>, String> {
    let map = state.db.with(|c| merge::console_times(c, chrono::Local::now())).map_err(|e| e.to_string())?;
    let manual = state.db.with(dbd::manual_load).map_err(|e| e.to_string())?;
    let games = visor_core::games::all_games(&manual);
    let folders: Vec<String> =
        state.db.with(store::titles_load).map_err(|e| e.to_string())?.into_iter().filter_map(|t| t.folder).collect();
    let stats = state.stats.lock().unwrap().clone();
    Ok(scan::title_times(&games, &stats, &map, &folders))
}
