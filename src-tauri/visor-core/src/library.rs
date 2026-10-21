//! Biblioteca del visor: una sola carpeta, elegida por el usuario (puede estar en otro disco), con todo lo que
//! la app guarda o importa. Asi basta con copiar esa carpeta para tener una copia completa o llevarsela a otro equipo.
//!
//! ```text
//! <Biblioteca>/
//!   visor.db, config.json
//!   Capturas/<plataforma>/<juego>/...   (la antigua «carpeta raiz»)
//!   Partidas/<juego>/*.zip
//!   Grafismos/<titulo>/...              (SteamGridDB)
//!   Glosario/media/...                  (capturas y audio de las frases)
//!   Diccionarios/                       (Yomitan)
//!   Copias de la base/visor-AAAA-MM-DD.db
//! ```
//!
//! En la carpeta de configuracion de Windows solo queda [`POINTER_FILE`] (donde esta la biblioteca) y lo que es
//! propio del equipo (monitor, perfil del lector, texthook); en la cache, lo que se puede regenerar (miniaturas,
//! acentos, base de Ludusavi). Antes de la biblioteca ([`DataPaths::legacy`]) todo estaba repartido: el asistente
//! lo copia con [`copy_into`], que nunca borra ni modifica el origen.
use crate::config::{write_atomic, Config};
use crate::db::{Db, DB_FILE};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

/// En la carpeta de configuracion de Windows: donde esta la biblioteca.
pub const POINTER_FILE: &str = "biblioteca.json";
pub const CONFIG_FILE: &str = "config.json";
pub const CAPTURES: &str = "Capturas";
pub const SAVES: &str = "Partidas";
pub const ART: &str = "Grafismos";
pub const GLOSSARY: &str = "Glosario";
pub const DICTS: &str = "Diccionarios";
pub const DB_BACKUPS: &str = "Copias de la base";
/// Marca de una copia a medias en el destino: se puede retomar (lo ya copiado con el mismo tamano se salta).
pub const COPYING: &str = ".copiando";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Pointer {
    pub dir: String,
    /// Biblioteca de la que se copio la actual ("" = ninguna o ya se aviso): se avisa una vez de que sigue ahi.
    pub previous: String,
}

pub fn read_pointer(config_dir: &Path) -> Option<Pointer> {
    let p: Pointer = serde_json::from_str(&fs::read_to_string(config_dir.join(POINTER_FILE)).ok()?).ok()?;
    (!p.dir.trim().is_empty()).then_some(p)
}

pub fn write_pointer(config_dir: &Path, p: &Pointer) -> Result<()> {
    write_atomic(&config_dir.join(POINTER_FILE), serde_json::to_string_pretty(p)?.as_bytes())
}

/// Donde vive cada tipo de dato.
#[derive(Debug, Clone, PartialEq)]
pub struct DataPaths {
    /// `visor.db` y `config.json`.
    pub data: PathBuf,
    /// Capturas (una subcarpeta por plataforma). `None` = aun sin elegir (solo antes de la biblioteca).
    pub captures: Option<PathBuf>,
    pub saves: PathBuf,
    pub art: PathBuf,
    pub glossary: PathBuf,
    pub dicts: PathBuf,
    pub db_backups: PathBuf,
}

impl DataPaths {
    pub fn library(dir: &Path) -> DataPaths {
        DataPaths {
            data: dir.to_path_buf(),
            captures: Some(dir.join(CAPTURES)),
            saves: dir.join(SAVES),
            art: dir.join(ART),
            glossary: dir.join(GLOSSARY),
            dicts: dir.join(DICTS),
            db_backups: dir.join(DB_BACKUPS),
        }
    }

    /// Como estaba antes de la biblioteca: datos en la configuracion de Windows, grafismos en la cache y capturas y
    /// partidas donde dijera la configuracion.
    pub fn legacy(config_dir: &Path, cache_dir: &Path, cfg: &Config) -> DataPaths {
        let saves = cfg.saves_dir.trim();
        DataPaths {
            data: config_dir.to_path_buf(),
            captures: cfg.root.as_deref().filter(|r| !r.trim().is_empty()).map(PathBuf::from),
            saves: if saves.is_empty() { config_dir.join("partidas") } else { PathBuf::from(saves) },
            art: cache_dir.join("sgdb"),
            glossary: config_dir.join("phrases"),
            dicts: config_dir.join("dicts"),
            db_backups: config_dir.join("backups"),
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.data.join(CONFIG_FILE)
    }

    /// Carpetas que se copian a una biblioteca nueva: (clave, nombre para el usuario, origen, destino).
    fn sources(&self, to: &DataPaths) -> Vec<(&'static str, &'static str, PathBuf, PathBuf)> {
        let mut v = Vec::new();
        if let (Some(a), Some(b)) = (&self.captures, &to.captures) {
            v.push(("captures", "Capturas", a.clone(), b.clone()));
        }
        v.push(("saves", "Copias de partidas", self.saves.clone(), to.saves.clone()));
        v.push(("art", "Grafismos de SteamGridDB", self.art.clone(), to.art.clone()));
        v.push(("glossary", "Glosario (capturas y audio de frases)", self.glossary.clone(), to.glossary.clone()));
        v.push(("dicts", "Diccionarios", self.dicts.clone(), to.dicts.clone()));
        v.push(("dbbackups", "Copias automáticas de la base", self.db_backups.clone(), to.db_backups.clone()));
        v
    }
}

/// Que hay en la carpeta elegida como destino.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DestKind {
    /// No existe o esta vacia: se copia todo.
    New,
    /// Ya es una biblioteca del visor: se usa tal cual, sin copiar nada.
    Existing,
    /// Una copia que se corto: se retoma.
    Resume,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub key: String,
    pub label: String,
    pub from: String,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub dest: String,
    pub kind: DestKind,
    /// Solo lo que existe y tiene algo (en `Existing`, vacio).
    pub items: Vec<PlanItem>,
    pub files: u64,
    pub bytes: u64,
}

/// Comparacion de rutas de Windows: sin distinguir mayusculas, `/` = `\` y sin `\` final.
fn norm(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\").to_lowercase();
    s.trim_end_matches('\\').to_string()
}

/// `inner` es `outer` o esta dentro.
fn within(inner: &Path, outer: &Path) -> bool {
    let (i, o) = (norm(inner), norm(outer));
    i == o || i.starts_with(&format!("{o}\\"))
}

fn is_library(dir: &Path) -> bool {
    dir.join(DB_FILE).is_file()
}

/// Comprueba la carpeta elegida: ruta absoluta, que no sea ni este dentro de lo que se va a copiar y que no exista,
/// este vacia, sea una biblioteca o una copia a medias.
pub fn check_dest(dest: &Path, from: &DataPaths) -> Result<DestKind> {
    if !dest.is_absolute() || dest.components().any(|c| c == Component::ParentDir) {
        bail!("Elige una carpeta con su ruta completa");
    }
    if norm(dest) == norm(&from.data) {
        bail!("Esa ya es la carpeta de la biblioteca");
    }
    let mut srcs = vec![from.data.clone()];
    srcs.extend(from.sources(&DataPaths::library(dest)).into_iter().map(|s| s.2));
    if srcs.iter().any(|s| within(dest, s)) {
        bail!("La biblioteca no puede estar dentro de una de las carpetas que se copian");
    }
    if !dest.exists() {
        return Ok(DestKind::New);
    }
    if !dest.is_dir() {
        bail!("Eso es un archivo, no una carpeta");
    }
    if dest.join(COPYING).is_file() {
        return Ok(DestKind::Resume);
    }
    if is_library(dest) {
        return Ok(DestKind::Existing);
    }
    if fs::read_dir(dest).with_context(|| format!("no se puede leer {}", dest.display()))?.next().is_some() {
        bail!("La carpeta tiene que estar vacía (o ser una biblioteca del visor)");
    }
    Ok(DestKind::New)
}

fn count(dir: &Path) -> (u64, u64) {
    let mut n = (0, 0);
    if dir.is_dir() {
        for e in WalkDir::new(dir).follow_links(false).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
            n.0 += 1;
            n.1 += e.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    n
}

/// Que se copiaria de `from` a `dest` y cuanto ocupa. La base cuenta con el tamano de su archivo.
pub fn plan(dest: &Path, from: &DataPaths) -> Result<Plan> {
    let kind = check_dest(dest, from)?;
    let mut items = Vec::new();
    if kind != DestKind::Existing {
        let db_bytes = fs::metadata(from.data.join(DB_FILE)).map(|m| m.len()).unwrap_or(0);
        items.push(PlanItem { key: "db".into(), label: "Base de datos y ajustes".into(), from: from.data.to_string_lossy().into_owned(), files: 2, bytes: db_bytes });
        for (key, label, src, _) in from.sources(&DataPaths::library(dest)) {
            let (files, bytes) = count(&src);
            if files > 0 {
                items.push(PlanItem { key: key.into(), label: label.into(), from: src.to_string_lossy().into_owned(), files, bytes });
            }
        }
    }
    let (files, bytes) = items.iter().fold((0, 0), |a, i| (a.0 + i.files, a.1 + i.bytes));
    Ok(Plan { dest: dest.to_string_lossy().into_owned(), kind, items, files, bytes })
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopyProgress {
    pub label: String,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub done_files: u64,
    pub total_files: u64,
}

/// Copia todo lo de `from` a la biblioteca `dest` (crea las carpetas de [`DataPaths::library`]) sin tocar el origen:
/// las carpetas archivo a archivo (comprobando el tamano; lo que ya este con el mismo tamano se salta, asi se puede
/// retomar), la configuracion sin las rutas antiguas (`root`, `saves_dir`) y, al final, una instantanea de la base
/// viva. Mientras dura hay una marca [`COPYING`] en el destino.
pub fn copy_into(from: &DataPaths, dest: &Path, db: &Db, cfg: &Config, mut progress: impl FnMut(&CopyProgress)) -> Result<()> {
    let p = plan(dest, from)?;
    if p.kind == DestKind::Existing {
        bail!("Esa carpeta ya es una biblioteca: se usa tal cual, sin copiar");
    }
    let to = DataPaths::library(dest);
    for d in [Some(&to.data), to.captures.as_ref(), Some(&to.saves), Some(&to.art), Some(&to.glossary), Some(&to.dicts), Some(&to.db_backups)].into_iter().flatten() {
        fs::create_dir_all(d).with_context(|| format!("No se pudo crear {}", d.display()))?;
    }
    fs::write(dest.join(COPYING), b"")?;
    let mut st = CopyProgress { label: String::new(), done_bytes: 0, total_bytes: p.bytes, done_files: 0, total_files: p.files };
    for (_, label, src, dst) in from.sources(&to) {
        if !src.is_dir() {
            continue;
        }
        st.label = label.into();
        progress(&st);
        for e in WalkDir::new(&src).follow_links(false).into_iter() {
            let e = e.with_context(|| format!("leyendo {}", src.display()))?;
            let rel = e.path().strip_prefix(&src)?;
            let target = dst.join(rel);
            if e.file_type().is_dir() {
                fs::create_dir_all(&target).with_context(|| format!("creando {}", target.display()))?;
                continue;
            }
            if !e.file_type().is_file() {
                continue;
            }
            let len = e.metadata()?.len();
            let same = fs::metadata(&target).map(|m| m.len() == len).unwrap_or(false);
            if !same {
                fs::copy(e.path(), &target).with_context(|| format!("copiando {}", e.path().display()))?;
                let got = fs::metadata(&target)?.len();
                if got != len {
                    bail!("{} se copio incompleto ({got} de {len} bytes)", e.path().display());
                }
            }
            st.done_bytes += len;
            st.done_files += 1;
            progress(&st);
        }
    }
    st.label = "Base de datos y ajustes".into();
    progress(&st);
    let mut c = cfg.clone();
    c.root = None;
    c.saves_dir.clear();
    c.save(&to.config_file())?;
    db.backup_to(&to.data.join(DB_FILE))?;
    st.done_files = st.total_files;
    st.done_bytes = st.total_bytes;
    progress(&st);
    fs::remove_file(dest.join(COPYING))?;
    Ok(())
}

/// Espacio libre que se pide por encima de lo que se copia (la base crece, el disco no debe quedar lleno).
pub const FREE_MARGIN: u64 = 200 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::data::favorite_set;

    fn legacy_setup() -> (tempfile::TempDir, DataPaths, Db, Config) {
        let t = tempfile::tempdir().unwrap();
        let (cfg_dir, cache, caps) = (t.path().join("cfg"), t.path().join("cache"), t.path().join("caps"));
        fs::create_dir_all(caps.join("PC/Juego")).unwrap();
        fs::write(caps.join("PC/Juego/a.png"), b"12345").unwrap();
        fs::create_dir_all(cfg_dir.join("phrases/media")).unwrap();
        fs::write(cfg_dir.join("phrases/media/f.mp3"), b"mp3").unwrap();
        fs::create_dir_all(cfg_dir.join("partidas/Juego [x]")).unwrap();
        fs::write(cfg_dir.join("partidas/Juego [x]/c.zip"), b"zip").unwrap();
        fs::create_dir_all(cache.join("sgdb/pc_juego")).unwrap();
        fs::write(cache.join("sgdb/pc_juego/hero-1.png"), b"h").unwrap();
        let db = Db::open_path(&cfg_dir.join(DB_FILE)).unwrap();
        db.with(|c| favorite_set(c, "PC/Juego/a.png", true)).unwrap();
        let cfg = Config { root: Some(caps.to_string_lossy().into_owned()), sound: true, ..Config::default() };
        let from = DataPaths::legacy(&cfg_dir, &cache, &cfg);
        (t, from, db, cfg)
    }

    #[test]
    fn pointer_roundtrip_and_missing() {
        let d = tempfile::tempdir().unwrap();
        assert!(read_pointer(d.path()).is_none());
        write_pointer(d.path(), &Pointer { dir: "D:\\Visor".into(), previous: String::new() }).unwrap();
        assert_eq!(read_pointer(d.path()).unwrap().dir, "D:\\Visor");
        fs::write(d.path().join(POINTER_FILE), r#"{"dir":"  "}"#).unwrap();
        assert!(read_pointer(d.path()).is_none(), "sin carpeta no hay biblioteca");
    }

    #[test]
    fn legacy_paths_follow_the_old_config() {
        let cfg = Config { root: Some("D:\\Caps".into()), saves_dir: "E:\\Copias".into(), ..Config::default() };
        let p = DataPaths::legacy(Path::new("C:\\cfg"), Path::new("C:\\cache"), &cfg);
        assert_eq!(p.captures.as_deref(), Some(Path::new("D:\\Caps")));
        assert_eq!(p.saves, PathBuf::from("E:\\Copias"));
        assert_eq!(p.art, Path::new("C:\\cache").join("sgdb"));
        let p = DataPaths::legacy(Path::new("C:\\cfg"), Path::new("C:\\cache"), &Config::default());
        assert_eq!((p.captures, p.saves), (None, Path::new("C:\\cfg").join("partidas")));
    }

    #[test]
    fn dest_checks() {
        let (t, from, _db, _) = legacy_setup();
        assert_eq!(check_dest(&t.path().join("nueva"), &from).unwrap(), DestKind::New);
        assert!(check_dest(Path::new("relativa"), &from).is_err());
        assert!(check_dest(&t.path().join("caps/PC"), &from).unwrap_err().to_string().contains("dentro"));
        assert!(check_dest(&t.path().join("cfg"), &from).is_err());
        let full = t.path().join("llena");
        fs::create_dir_all(&full).unwrap();
        fs::write(full.join("x.txt"), b"x").unwrap();
        assert!(check_dest(&full, &from).unwrap_err().to_string().contains("vacía"));
        fs::write(full.join(DB_FILE), b"").unwrap();
        assert_eq!(check_dest(&full, &from).unwrap(), DestKind::Existing);
        fs::write(full.join(COPYING), b"").unwrap();
        assert_eq!(check_dest(&full, &from).unwrap(), DestKind::Resume);
    }

    #[test]
    fn copies_everything_without_touching_the_source() {
        let (t, from, db, cfg) = legacy_setup();
        let dest = t.path().join("Biblioteca");
        let p = plan(&dest, &from).unwrap();
        assert_eq!(p.items.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(), ["db", "captures", "saves", "art", "glossary"]);
        let mut last = None;
        copy_into(&from, &dest, &db, &cfg, |s| last = Some(s.clone())).unwrap();
        let last = last.unwrap();
        assert_eq!((last.done_files, last.done_bytes), (last.total_files, last.total_bytes));
        let lib = DataPaths::library(&dest);
        assert_eq!(fs::read(lib.captures.as_ref().unwrap().join("PC/Juego/a.png")).unwrap(), b"12345");
        assert!(lib.glossary.join("media/f.mp3").is_file());
        assert!(lib.saves.join("Juego [x]/c.zip").is_file());
        assert!(lib.art.join("pc_juego/hero-1.png").is_file());
        assert!(lib.dicts.is_dir() && lib.db_backups.is_dir());
        assert!(!dest.join(COPYING).exists());
        // La configuracion pierde las rutas antiguas y conserva lo demas; la base copiada tiene los datos.
        let c = Config::load(&lib.config_file());
        assert!(c.root.is_none() && c.sound);
        let copy = Db::open_path(&dest.join(DB_FILE)).unwrap();
        assert_eq!(copy.with(crate::db::data::favorites_load).unwrap(), vec!["PC/Juego/a.png".to_string()]);
        // El origen sigue igual y el destino ya es una biblioteca.
        assert!(t.path().join("caps/PC/Juego/a.png").is_file());
        assert_eq!(check_dest(&dest, &from).unwrap(), DestKind::Existing);
    }

    #[test]
    fn interrupted_copy_resumes() {
        let (t, from, db, cfg) = legacy_setup();
        let dest = t.path().join("Biblioteca");
        fs::create_dir_all(dest.join("Capturas/PC/Juego")).unwrap();
        fs::write(dest.join("Capturas/PC/Juego/a.png"), b"12").unwrap();
        fs::write(dest.join(COPYING), b"").unwrap();
        assert_eq!(plan(&dest, &from).unwrap().kind, DestKind::Resume);
        copy_into(&from, &dest, &db, &cfg, |_| {}).unwrap();
        assert_eq!(fs::read(dest.join("Capturas/PC/Juego/a.png")).unwrap(), b"12345", "lo copiado a medias se repite");
    }

    #[test]
    fn library_to_library_copy() {
        let (t, from, db, cfg) = legacy_setup();
        let a = t.path().join("A");
        copy_into(&from, &a, &db, &cfg, |_| {}).unwrap();
        let lib_a = DataPaths::library(&a);
        assert!(check_dest(&a, &lib_a).unwrap_err().to_string().contains("ya es"));
        let b = t.path().join("B");
        copy_into(&lib_a, &b, &db, &cfg, |_| {}).unwrap();
        assert!(b.join("Capturas/PC/Juego/a.png").is_file());
        assert!(b.join("Glosario/media/f.mp3").is_file());
    }
}
