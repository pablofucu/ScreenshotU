//! Grafismos de los juegos recien importados desde Nintendo (SW-25). Lanza el lote de SteamGridDB
//! cuando esta libre; la caratula del CDN de Nintendo solo se usa si SteamGridDB no deja ninguna.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use visor_core::art::ArtKind;
use visor_core::nintendo::base_name;
use visor_core::nintendo::import::Created;
use visor_core::sgdb;

use crate::sgdb::{run_batch, Pending};
use crate::state::AppState;

const POLL: Duration = Duration::from_secs(5);
const MAX_WAIT: Duration = Duration::from_secs(600);

/// Busca portada, fondo y logo de las fichas nuevas. No hace nada sin clave o con la descarga automatica apagada.
pub fn spawn(app: AppHandle, created: Vec<Created>) {
    if created.is_empty() {
        return;
    }
    {
        let st = app.state::<AppState>();
        let cfg = st.config.lock().unwrap();
        if cfg.sgdb_key.is_empty() || !cfg.sgdb_auto {
            return;
        }
    }
    let _ = std::thread::Builder::new().name("nintendo-art".into()).spawn(move || {
        let start = Instant::now();
        loop {
            let st = app.state::<AppState>();
            let base = st.sgdb_dir();
            let todo: Vec<Pending> = created
                .iter()
                .filter_map(|c| {
                    let cached = sgdb::find_cached(&base, &c.game_id);
                    let kinds: Vec<ArtKind> = [(ArtKind::Cover, cached.cover.is_some()), (ArtKind::Hero, cached.hero.is_some()), (ArtKind::Logo, cached.logo.is_some())]
                        .into_iter()
                        .filter(|(_, has)| !has)
                        .map(|(k, _)| k)
                        .collect();
                    (!kinds.is_empty()).then(|| Pending {
                        id: c.game_id.clone(),
                        name: c.name.clone(),
                        term: base_name(&c.name),
                        kinds,
                        fallback_cover: c.image_url.clone(),
                    })
                })
                .collect();
            if todo.is_empty() {
                return;
            }
            if !st.sgdb_busy.load(Ordering::SeqCst) {
                match run_batch(app.clone(), todo) {
                    Ok(_) => return,
                    // Otro lote se adelanto entre la comprobacion y el arranque: se espera.
                    Err(m) if m.starts_with("Ya se estan") => {}
                    Err(m) => {
                        eprintln!("[nintendo-art] {m}");
                        return;
                    }
                }
            }
            drop(st);
            if start.elapsed() > MAX_WAIT {
                return;
            }
            std::thread::sleep(POLL);
        }
    });
}
