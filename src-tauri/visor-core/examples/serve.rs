//! Servidor del movil sin la interfaz de escritorio, con los datos reales del usuario (para probar la app
//! Android). Solo lee la biblioteca; la base se abre como en la app (la primera vez importaria los JSON antiguos).
//!
//! ```text
//! cargo run -p visor-core --example serve --release -- [carpeta_config] [puerto] [codigo]
//! ```
//! Por defecto: `%APPDATA%\app.visorcapturas.desktop`, 47811 y `TEST-CODE`. Las miniaturas salen de la cache
//! del escritorio (`%LOCALAPPDATA%\app.visorcapturas.desktop\thumbs`) y las vistas previas van a `previews`.
use anyhow::Result;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use visor_core::config::Config;
use visor_core::db::{data as dbd, Db};
use visor_core::games;
use visor_core::palette::PaletteCache;
use visor_core::relpath;
use visor_core::remote::{self, Provider, RemoteConfig, RemoteServer, Snapshot};
use visor_core::scan::{self, ScanOpts};

const APP_ID: &str = "app.visorcapturas.desktop";

struct Real {
    config_dir: PathBuf,
    cache_dir: PathBuf,
    db: Db,
}

fn log<T: Default>(what: &str, r: Result<T>) -> T {
    r.unwrap_or_else(|e| {
        eprintln!("visor.db ({what}): {e:#}");
        T::default()
    })
}

impl Provider for Real {
    fn name(&self) -> String {
        remote::host_name()
    }

    fn snapshot(&self) -> Result<Snapshot> {
        let t0 = std::time::Instant::now();
        let cfg = Config::load(&self.config_dir.join("config.json"));
        let root = cfg.root.as_deref().map(Path::new);
        let manual = log("juegos", self.db.with(dbd::manual_load));
        let stats = log("estadisticas", self.db.with(dbd::stats_load));
        let hidden = log("ocultos", self.db.with(dbd::hidden_load));
        let prefs = log("preferencias", self.db.with(dbd::prefs_load));
        let sagas = log("sagas", self.db.with(dbd::sagas_load));
        let favorites: Vec<String> = log("favoritas", self.db.with(dbd::favorites_load)).iter().map(|p| relpath::resolved(root, p)).collect();
        let palettes = PaletteCache::load(&self.cache_dir.join("palettes.json"));
        let sgdb_dir = self.cache_dir.join("sgdb");
        let found = games::all_games(&manual);
        let opts = ScanOpts { hidden: &hidden, show_hidden: false, prefs: Some(&prefs), palettes: Some(&palettes), sgdb_dir: Some(&sgdb_dir) };
        let mut library = scan::scan_library_ex(root, &cfg.platforms, &found, &stats, &opts);
        library.hidden.clear();
        let shots: usize = library.platforms.iter().flat_map(|p| &p.titles).map(|t| t.shots.len()).sum();
        let titles: usize = library.platforms.iter().map(|p| p.titles.len()).sum();
        eprintln!("escaneo: {titles} juegos, {shots} capturas en {:?}", t0.elapsed());
        Ok(Snapshot { config: RemoteConfig::from_config(&cfg), library, favorites, sagas, playing: None })
    }

    fn thumbs_dir(&self) -> PathBuf {
        self.cache_dir.join("thumbs")
    }

    fn previews_dir(&self) -> PathBuf {
        self.cache_dir.join("previews")
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let env_dir = |var: &str| std::env::var_os(var).map(PathBuf::from).unwrap_or_default().join(APP_ID);
    let config_dir = args.first().filter(|a| !a.is_empty()).map(PathBuf::from).unwrap_or_else(|| env_dir("APPDATA"));
    let port: u16 = args.get(1).and_then(|p| p.parse().ok()).unwrap_or(remote::DEFAULT_PORT);
    let code = args.get(2).cloned().unwrap_or_else(|| "TEST-CODE".into());
    let cache_dir = env_dir("LOCALAPPDATA");
    let db = Db::open(&config_dir)?;
    let srv = RemoteServer::start(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)), code.clone(), Real { config_dir: config_dir.clone(), cache_dir, db })?;
    println!("Datos: {}", config_dir.display());
    println!("Codigo: {code}");
    for ip in remote::local_ipv4s() {
        println!("http://{ip}:{port}/api/v1/hello");
    }
    println!("http://127.0.0.1:{}/api/v1/hello  (Ctrl+C para salir)", srv.local_addr().port());
    loop {
        std::thread::park();
    }
}
