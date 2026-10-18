//! Logica del visor de capturas independiente de Tauri, para poder probarla con `cargo test`.
pub mod art;
pub mod audio;
pub mod backup;
pub mod capture;
pub mod config;
pub mod db;
pub mod dates;
pub mod gamedirs;
pub mod games;
pub mod importer;
pub mod jp;
pub mod magpie;
pub mod palette;
pub mod sagas;
pub mod scan;
pub mod sgdb;
pub mod thumbs;
pub mod userdata;

pub use config::{Config, PlatformCfg};
pub use scan::{Library, Platform, Shot, Title};
