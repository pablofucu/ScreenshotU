//! Partidas guardadas: donde guarda cada juego sus partidas y copias .zip para restaurarlas.
//!
//! Las ubicaciones salen de tres fuentes, por este orden: la base de Ludusavi ([`manifest`]), las reglas por
//! motor ([`engines`]: Ren'Py, RPG Maker, KiriKiri, Unity...) y las que el usuario anade a mano. Se recalculan
//! cada vez; en la base (tabla `save_paths`) solo se guardan las manuales y las detectadas que el usuario
//! desactiva. Las copias estan en [`archive`]. Los archivos del juego solo se leen, salvo al restaurar.
pub mod archive;
pub mod engines;
pub mod glob;
pub mod manifest;

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub use manifest::Manifest;

/// Migracion que crea la tabla de rutas de partidas.
pub const SCHEMA: &str = r#"
-- Rutas de partidas por juego: las anadidas a mano (manual = 1) y las detectadas que el usuario desactivo.
CREATE TABLE save_paths (
    game_id TEXT NOT NULL,
    path    TEXT NOT NULL,
    kind    TEXT NOT NULL,
    manual  INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (game_id, path)
);
"#;

/// Archivos que se miden como mucho por ubicacion (para no recorrer discos enteros).
const MEASURE_CAP: usize = 50_000;
/// Coincidencias de un comodin que se tienen en cuenta.
pub const MATCH_CAP: usize = 2_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LocKind {
    /// Carpeta, archivo o patron con comodines.
    Path,
    /// Clave del registro de Windows (`HKEY_CURRENT_USER\...`).
    Registry,
}

impl LocKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LocKind::Path => "path",
            LocKind::Registry => "registry",
        }
    }
    pub fn parse(s: &str) -> LocKind {
        if s == "registry" {
            LocKind::Registry
        } else {
            LocKind::Path
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Ludusavi,
    Engine,
    Manual,
}

/// Una ubicacion de partidas con lo que ocupa ahora.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveLoc {
    pub path: String,
    pub kind: LocKind,
    pub origin: Origin,
    /// «Ren'Py», «Ludusavi · partidas», «Añadida a mano»...
    pub label: String,
    /// Nombre legible: «AppData › RenPy › DDLC-1454445547», «Carpeta del juego › game › saves».
    pub place: String,
    pub enabled: bool,
    pub exists: bool,
    pub files: usize,
    pub size: u64,
    /// Ultima modificacion (ms desde epoch), 0 si no se sabe.
    pub modified_ms: i64,
}

/// Carpetas del usuario que usan los marcadores de Ludusavi y los motores.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveEnv {
    pub home: PathBuf,
    pub appdata: PathBuf,
    pub local_appdata: PathBuf,
    pub documents: PathBuf,
    pub public: PathBuf,
    pub program_data: PathBuf,
    pub win_dir: PathBuf,
    pub user_name: String,
    /// Carpeta de Steam (para `<root>/userdata/...`), si esta instalado.
    pub steam: Option<PathBuf>,
}

impl SaveEnv {
    /// Desde las variables de entorno. Documentos se supone en `%USERPROFILE%\Documents` (la capa de Windows
    /// pone la real, que puede estar en OneDrive) y Steam sin resolver.
    pub fn from_env() -> SaveEnv {
        let v = |k: &str| std::env::var_os(k).map(PathBuf::from);
        let home = v("USERPROFILE").or_else(|| v("HOME")).unwrap_or_default();
        SaveEnv {
            appdata: v("APPDATA").unwrap_or_else(|| home.join("AppData/Roaming")),
            local_appdata: v("LOCALAPPDATA").unwrap_or_else(|| home.join("AppData/Local")),
            documents: home.join("Documents"),
            public: v("PUBLIC").unwrap_or_else(|| PathBuf::from(r"C:\Users\Public")),
            program_data: v("PROGRAMDATA").unwrap_or_else(|| PathBuf::from(r"C:\ProgramData")),
            win_dir: v("WINDIR").unwrap_or_else(|| PathBuf::from(r"C:\Windows")),
            user_name: std::env::var("USERNAME").unwrap_or_default(),
            steam: None,
            home,
        }
    }
}

/// Lo que se sabe de las partidas de un juego.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    /// Juego de Ludusavi reconocido y como («Hades II (carpeta del juego)»).
    pub matched: Option<String>,
    /// Motor reconocido.
    pub engine: Option<String>,
    pub locs: Vec<SaveLoc>,
}

/// Fila de `save_paths`.
#[derive(Debug, Clone, PartialEq)]
pub struct PathRow {
    pub path: String,
    pub kind: LocKind,
    pub manual: bool,
    pub enabled: bool,
}

/// Clave de comparacion de rutas (sin mayusculas y con `/`).
pub fn path_key(p: &str) -> String {
    let mut s = p.replace('\\', "/").to_lowercase();
    while s.ends_with('/') {
        s.pop();
    }
    s
}

/// Parte fija de un patron: los componentes hasta el primero con comodines.
pub fn literal_prefix(pattern: &str) -> PathBuf {
    let norm = pattern.replace('\\', "/");
    let mut out: Vec<&str> = Vec::new();
    for c in norm.split('/') {
        if glob::has_magic(c) {
            break;
        }
        out.push(c);
    }
    let s = out.join("/");
    PathBuf::from(if s.ends_with(':') { format!("{s}/") } else { s })
}

/// Detecta las ubicaciones de un juego. `exe` es su ejecutable (los emuladores y las entradas de solo capturas
/// no tienen: para esos solo cuentan las manuales). Las detectadas que no existen no se devuelven.
pub fn detect(name: &str, exe: Option<&Path>, manifest: Option<&Manifest>, env: &SaveEnv, rows: &[PathRow]) -> Detection {
    let mut out = Detection::default();
    let mut cands: Vec<(String, LocKind, Origin, String)> = Vec::new();
    // Carpeta del juego, para nombrar las rutas que caen dentro.
    let mut game_dir: Option<PathBuf> = exe.and_then(Path::parent).map(Path::to_path_buf);
    if let Some(m) = manifest.and_then(|m| m.find(name, exe)) {
        if !m.base.as_os_str().is_empty() {
            game_dir = Some(m.base.clone());
        }
        out.matched = Some(format!("{} ({})", m.entry.title, m.how));
        for r in manifest::resolve(&m, env) {
            let what = if r.tags.iter().any(|t| t == "save") {
                "partidas"
            } else if r.tags.iter().any(|t| t == "config") {
                "configuración"
            } else {
                "datos"
            };
            cands.push((r.pattern, r.kind, Origin::Ludusavi, format!("Ludusavi · {what}")));
        }
    }
    if let Some(x) = exe {
        let (eng, locs) = engines::detect(x, name, env);
        out.engine = eng.map(str::to_string);
        for l in locs {
            cands.push((l.pattern, l.kind, Origin::Engine, l.engine.to_string()));
        }
    }
    // Detectadas: solo las que existen, sin repetir ni incluir una que ya este dentro de otra.
    let mut kept: Vec<SaveLoc> = Vec::new();
    for (path, kind, origin, label) in cands {
        let loc = measure(path, kind, origin, label);
        if !loc.exists {
            continue;
        }
        let covered = kept.iter().any(|k| {
            k.kind == loc.kind
                && (path_key(&k.path) == path_key(&loc.path)
                    || (loc.kind == LocKind::Path && !glob::has_magic(&k.path) && glob::inside_or_same(&literal_prefix(&loc.path), Path::new(&k.path))))
        });
        if covered {
            continue;
        }
        // Una nueva que contiene a otras anteriores las sustituye.
        if loc.kind == LocKind::Path && !glob::has_magic(&loc.path) {
            kept.retain(|k| k.kind != LocKind::Path || !glob::inside_or_same(&literal_prefix(&k.path), Path::new(&loc.path)));
        }
        kept.push(loc);
    }
    for r in rows.iter().filter(|r| r.manual) {
        if !kept.iter().any(|k| path_key(&k.path) == path_key(&r.path)) {
            kept.push(measure(r.path.clone(), r.kind, Origin::Manual, "Añadida a mano".into()));
        }
    }
    for k in kept.iter_mut() {
        if let Some(r) = rows.iter().find(|r| path_key(&r.path) == path_key(&k.path)) {
            k.enabled = r.enabled;
        }
        k.place = place_name(&k.path, k.kind, env, game_dir.as_deref());
    }
    out.locs = kept;
    out
}

/// Mide una ubicacion (existe, archivos, tamano y ultima modificacion).
pub fn measure(path: String, kind: LocKind, origin: Origin, label: String) -> SaveLoc {
    let mut loc = SaveLoc { path, kind, origin, label, place: String::new(), enabled: true, exists: false, files: 0, size: 0, modified_ms: 0 };
    if kind == LocKind::Registry {
        loc.exists = registry::exists(&loc.path);
        return loc;
    }
    for m in glob::expand(&loc.path, MATCH_CAP) {
        loc.exists = true;
        for e in WalkDir::new(&m).into_iter().flatten().filter(|e| e.file_type().is_file()).take(MEASURE_CAP) {
            let Ok(md) = e.metadata() else { continue };
            loc.files += 1;
            loc.size += md.len();
            loc.modified_ms = loc.modified_ms.max(crate::scan::mtime_ms(&md));
        }
    }
    loc
}

/// Nombre legible de una ubicacion: la carpeta conocida en la que empieza y el resto separado por «›».
pub fn place_name(path: &str, kind: LocKind, env: &SaveEnv, game_dir: Option<&Path>) -> String {
    let join = |head: &str, rest: &str, sep: char| -> String {
        std::iter::once(head).chain(rest.split(sep)).filter(|c| !c.is_empty()).collect::<Vec<_>>().join(" › ")
    };
    if kind == LocKind::Registry {
        let p = path.replace('/', "\\");
        let up = p.to_uppercase();
        let rest = [r"HKEY_CURRENT_USER\SOFTWARE\", r"HKEY_CURRENT_USER\"].iter().find(|x| up.starts_with(**x)).map(|x| &p[x.len()..]).unwrap_or(&p);
        return join("Registro", rest, '\\');
    }
    let low = env.home.join("AppData").join("LocalLow");
    let saved = env.home.join("Saved Games");
    let mut roots: Vec<(&str, &Path)> = vec![
        ("LocalLow", &low),
        ("AppData Local", &env.local_appdata),
        ("AppData", &env.appdata),
        ("Documentos", &env.documents),
        ("Partidas guardadas", &saved),
        ("Usuario", &env.home),
    ];
    if let Some(s) = env.steam.as_deref() {
        roots.push(("Steam", s));
    }
    if let Some(g) = game_dir {
        roots.push(("Carpeta del juego", g));
    }
    let key = path_key(path);
    let norm = path.replace('\\', "/");
    // Gana la carpeta conocida mas larga que contenga la ruta (las claves son ASCII en minusculas: se corta por
    // numero de componentes, no de bytes).
    let best = roots
        .iter()
        .map(|(n, r)| (*n, path_key(&r.to_string_lossy())))
        .filter(|(_, r)| !r.is_empty() && (key == *r || key.starts_with(&format!("{r}/"))))
        .max_by_key(|(_, r)| r.len());
    match best {
        Some((n, r)) => {
            let skip = r.split('/').count();
            join(n, &norm.split('/').skip(skip).collect::<Vec<_>>().join("/"), '/')
        }
        None => join("", &norm, '/'),
    }
}

/* ---------- tabla save_paths ---------- */

pub fn rows_load(c: &Connection, game_id: &str) -> Result<Vec<PathRow>> {
    let mut st = c.prepare("SELECT path, kind, manual, enabled FROM save_paths WHERE game_id = ?1 ORDER BY rowid")?;
    let rows = st
        .query_map([game_id], |r| Ok(PathRow { path: r.get(0)?, kind: LocKind::parse(&r.get::<_, String>(1)?), manual: r.get::<_, i64>(2)? != 0, enabled: r.get::<_, i64>(3)? != 0 }))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Activa o desactiva una ubicacion (detectada o manual).
pub fn row_set_enabled(c: &Connection, game_id: &str, path: &str, kind: LocKind, enabled: bool) -> Result<()> {
    let existing: Option<(String, bool)> = rows_load(c, game_id)?.into_iter().find(|r| path_key(&r.path) == path_key(path)).map(|r| (r.path, r.manual));
    match existing {
        Some((p, true)) => {
            c.execute("UPDATE save_paths SET enabled = ?3 WHERE game_id = ?1 AND path = ?2", params![game_id, p, enabled])?;
        }
        Some((p, false)) if enabled => {
            // Detectada y activada otra vez: es lo normal, no hace falta recordarla.
            c.execute("DELETE FROM save_paths WHERE game_id = ?1 AND path = ?2", params![game_id, p])?;
        }
        Some(_) => {}
        None if !enabled => {
            c.execute("INSERT INTO save_paths(game_id, path, kind, manual, enabled) VALUES (?1, ?2, ?3, 0, 0)", params![game_id, path, kind.as_str()])?;
        }
        None => {}
    }
    Ok(())
}

/// Anade una ruta manual (si ya estaba, la activa).
pub fn row_add_manual(c: &Connection, game_id: &str, path: &str, kind: LocKind) -> Result<()> {
    if let Some(r) = rows_load(c, game_id)?.into_iter().find(|r| path_key(&r.path) == path_key(path)) {
        c.execute("UPDATE save_paths SET manual = 1, enabled = 1 WHERE game_id = ?1 AND path = ?2", params![game_id, r.path])?;
        return Ok(());
    }
    c.execute("INSERT INTO save_paths(game_id, path, kind, manual, enabled) VALUES (?1, ?2, ?3, 1, 1)", params![game_id, path, kind.as_str()])?;
    Ok(())
}

/// Quita una ruta manual. Devuelve si habia alguna.
pub fn row_remove_manual(c: &Connection, game_id: &str, path: &str) -> Result<bool> {
    let Some(r) = rows_load(c, game_id)?.into_iter().find(|r| r.manual && path_key(&r.path) == path_key(path)) else { return Ok(false) };
    Ok(c.execute("DELETE FROM save_paths WHERE game_id = ?1 AND path = ?2", params![game_id, r.path])? > 0)
}

/* ---------- registro de Windows (con reg.exe, sin ventana) ---------- */

pub mod registry {
    use anyhow::{bail, Result};
    use std::path::Path;
    use std::process::{Command, Stdio};

    fn reg(args: &[&std::ffi::OsStr]) -> std::io::Result<std::process::ExitStatus> {
        let mut c = Command::new("reg");
        c.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        c.status()
    }

    pub fn exists(key: &str) -> bool {
        cfg!(windows) && reg(&["query".as_ref(), key.as_ref()]).map(|s| s.success()).unwrap_or(false)
    }

    /// Exporta la clave (con sus subclaves) a un .reg.
    pub fn export(key: &str, dest: &Path) -> Result<()> {
        let st = reg(&["export".as_ref(), key.as_ref(), dest.as_os_str(), "/y".as_ref()])?;
        if !st.success() {
            bail!("no se pudo exportar la clave del registro {key}");
        }
        Ok(())
    }

    pub fn import(file: &Path) -> Result<()> {
        let st = reg(&["import".as_ref(), file.as_os_str()])?;
        if !st.success() {
            bail!("no se pudo importar {} al registro", file.display());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn env(home: &Path) -> SaveEnv {
        SaveEnv {
            home: home.to_path_buf(),
            appdata: home.join("AppData/Roaming"),
            local_appdata: home.join("AppData/Local"),
            documents: home.join("Documents"),
            public: home.join("Public"),
            program_data: home.join("ProgramData"),
            win_dir: home.join("Windows"),
            user_name: "yo".into(),
            steam: None,
        }
    }

    #[test]
    fn friendly_place_names() {
        let e = SaveEnv { steam: Some(PathBuf::from("C:/Steam")), ..env(Path::new("C:/Users/yo")) };
        let g = Path::new("D:/Juegos/DDLC");
        let p = |s: &str, k| place_name(s, k, &e, Some(g));
        assert_eq!(p("C:/Users/yo/AppData/Roaming/RenPy/DDLC-1", LocKind::Path), "AppData › RenPy › DDLC-1");
        assert_eq!(p(r"C:\Users\yo\AppData\LocalLow\Team\Game", LocKind::Path), "LocalLow › Team › Game");
        assert_eq!(p("D:/Juegos/DDLC/game/saves", LocKind::Path), "Carpeta del juego › game › saves");
        assert_eq!(p("D:/Juegos/DDLC/Save*.rvdata2", LocKind::Path), "Carpeta del juego › Save*.rvdata2");
        assert_eq!(p("C:/Steam/userdata/*/1/remote", LocKind::Path), "Steam › userdata › * › 1 › remote");
        assert_eq!(p("E:/Otro/sitio", LocKind::Path), "E: › Otro › sitio");
        assert_eq!(p(r"HKEY_CURRENT_USER\Software\Team Salvato\DDLC", LocKind::Registry), "Registro › Team Salvato › DDLC");
    }

    #[test]
    fn prefixes_and_keys() {
        assert_eq!(literal_prefix("C:/a/b/*.sav"), PathBuf::from("C:/a/b"));
        assert_eq!(literal_prefix("C:/*/x"), PathBuf::from("C:/"));
        assert_eq!(path_key(r"C:\A\B\"), "c:/a/b");
    }

    #[test]
    fn merges_sources_dedupes_and_applies_rows() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("kk");
        fs::create_dir_all(g.join("savedata")).unwrap();
        fs::write(g.join("data.xp3"), b"x").unwrap();
        fs::write(g.join("savedata/01.ksd"), b"12345").unwrap();
        let exe = g.join("kk.exe");
        let e = env(&d.path().join("home"));
        let det = detect("kk", Some(&exe), None, &e, &[]);
        assert_eq!(det.engine.as_deref(), Some("KiriKiri"));
        assert_eq!(det.locs.len(), 1, "la carpeta savedata solo una vez: {:?}", det.locs);
        let l = &det.locs[0];
        assert_eq!((l.origin, l.files, l.size, l.enabled), (Origin::Engine, 1, 5, true));
        // Filas: una detectada desactivada y una manual (que no existe: tambien sale).
        let db = crate::db::Db::open_memory().unwrap();
        db.with(|c| {
            row_set_enabled(c, "g", &l.path, LocKind::Path, false).unwrap();
            row_add_manual(c, "g", "Z:/no/existe", LocKind::Path).unwrap();
        });
        let rows = db.with(|c| rows_load(c, "g")).unwrap();
        let det = detect("kk", Some(&exe), None, &e, &rows);
        assert_eq!(det.locs.len(), 2);
        assert!(!det.locs[0].enabled);
        assert_eq!((det.locs[1].origin, det.locs[1].exists), (Origin::Manual, false));
        // Reactivar una detectada borra su fila; quitar la manual tambien.
        db.with(|c| row_set_enabled(c, "g", &l.path.to_uppercase(), LocKind::Path, true)).unwrap();
        assert!(db.with(|c| row_remove_manual(c, "g", "z:\\no\\existe")).unwrap());
        assert!(db.with(|c| rows_load(c, "g")).unwrap().is_empty());
    }

    #[test]
    fn outer_folder_replaces_inner_ones() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("rm");
        fs::write(d.path().join("x"), b"").unwrap();
        fs::create_dir_all(g.join("www/js")).unwrap();
        fs::write(g.join("www/js/rpg_core.js"), b"").unwrap();
        fs::create_dir_all(g.join("www/save")).unwrap();
        fs::write(g.join("www/save/file1.rpgsave"), b"abc").unwrap();
        let det = detect("rm", Some(&g.join("Game.exe")), None, &env(d.path()), &[]);
        // Motor (www/save) y carpeta generica (www/save, a dos niveles) son la misma.
        assert_eq!(det.locs.len(), 1);
        assert_eq!(det.locs[0].label, "RPG Maker MV");
    }
}
