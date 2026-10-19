//! Estado compartido de la aplicacion.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8};
use std::sync::{Arc, Mutex};
use visor_core::config::Config;
use visor_core::db::Db;
use visor_core::games::{Game, Stats};
use visor_core::palette::PaletteCache;
use visor_core::sgdb::SgdbStore;
use visor_core::userdata::{HiddenEntry, TitlePrefs};

/// Partida en curso lanzada desde el visor.
#[derive(Clone)]
pub struct Active {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub folder: String,
    /// PIDs de los procesos del juego (se actualizan mientras se vigila).
    pub pids: Arc<Mutex<Vec<u32>>>,
}

/// Registra un error de la base y devuelve el valor por defecto (para lecturas al arrancar o en hilos sin donde informar).
pub fn log_db<T: Default, E: std::fmt::Display>(what: &str, r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| {
        eprintln!("visor.db ({what}): {e}");
        T::default()
    })
}

/// Escritura en la base desde un hilo donde no hay a quien devolver el error: se registra y se sigue.
pub fn log_db_err<E: std::fmt::Display>(what: &str, r: Result<(), E>) {
    if let Err(e) = r {
        eprintln!("visor.db ({what}): {e}");
    }
}

pub struct AppState {
    pub config_dir: PathBuf,
    /// Avisos para la interfaz que nacen al arrancar (base en memoria, restauracion aplicada...). Se entregan una vez.
    pub notices: Mutex<Vec<crate::backup::Notice>>,
    pub cache_dir: PathBuf,
    /// Base SQLite compartida (`<config>/visor.db`): favoritas, juegos, estadisticas, ocultos, preferencias,
    /// sagas, SteamGridDB, novelas y glosario. Los `Mutex` de abajo son copias en memoria de lo que se lee a menudo;
    /// cada cambio se escribe tambien en la base (fila a fila). Orden de bloqueo: primero el `Mutex`, luego `db.with`.
    pub db: Arc<Db>,
    pub config: Mutex<Config>,
    pub favorites: Mutex<Vec<String>>,
    pub games: Mutex<Vec<Game>>,
    pub stats: Mutex<Stats>,
    pub hidden: Mutex<Vec<HiddenEntry>>,
    pub prefs: Mutex<TitlePrefs>,
    pub palettes: Arc<PaletteCache>,
    pub active: Mutex<Option<Active>>,
    /// Mascara de la combinacion de captura (`capture::combo_mask`, 0 = desactivada); la lee el hilo del mando.
    pub combo_mask: AtomicU32,
    /// Boton del raton que captura (`capture::mouse_button_index`, 0 = ninguno).
    pub mouse_button: AtomicU8,
    /// Ajustes esta esperando una combinacion del mando: el hilo del mando informa en vez de capturar.
    pub learning_pad: AtomicBool,
    /// Juego de SteamGridDB asociado a cada titulo (tabla `sgdb`).
    pub sgdb: Arc<Mutex<SgdbStore>>,
    /// Hay una descarga automatica de SteamGridDB en marcha.
    pub sgdb_busy: Arc<AtomicBool>,
    /// Hay un hilo calculando huellas del catalogo de capturas.
    pub hashing: Arc<AtomicBool>,
    /// Base de partidas de Ludusavi (se carga de la cache la primera vez que hace falta).
    pub saves_manifest: Mutex<Option<Arc<visor_core::saves::Manifest>>>,
    /// Hay una descarga de la base de Ludusavi en marcha.
    pub saves_busy: AtomicBool,
}

impl AppState {
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }
    pub fn palettes_file(&self) -> PathBuf {
        self.cache_dir.join("palettes.json")
    }
    /// Grafismos descargados de SteamGridDB (una carpeta por titulo).
    pub fn sgdb_dir(&self) -> PathBuf {
        self.cache_dir.join("sgdb")
    }
    pub fn thumbs_dir(&self) -> PathBuf {
        self.cache_dir.join("thumbs")
    }
}
