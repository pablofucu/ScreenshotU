//! Logica del visor de capturas independiente de Tauri, para poder probarla con `cargo test`.
pub mod achievements;
pub mod art;
pub mod audio;
pub mod backup;
pub mod capture;
pub mod catalog;
pub mod config;
pub mod db;
pub mod dates;
pub mod gamedirs;
pub mod games;
pub mod importer;
pub mod jp;
pub mod library;
pub mod magpie;
pub mod palette;
pub mod relpath;
pub mod remote;
pub mod sagas;
pub mod saves;
pub mod scan;
pub mod sgdb;
pub mod thumbs;
pub mod userdata;

pub use config::{Config, PlatformCfg};
pub use scan::{Library, Platform, Shot, Title};
