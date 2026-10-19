//! Copias de seguridad: el .zip manual (base + configuracion + opcionalmente los medios del glosario),
//! la restauracion (preparar, validar y aplicar al siguiente arranque) y las copias automaticas de la base.
//!
//! La restauracion **no** sustituye la base con la app en marcha: la conexion esta abierta y hay copias en
//! memoria de casi todo. Se extrae el .zip a `<config>/restore-pending/`, se valida, se deja un archivo
//! `LISTO` y se reinicia; al arrancar, antes de abrir la base, `apply_pending_restore` hace el cambio.
use crate::db::{self, validate_file, Db, DbInfo, DB_FILE};
use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Nombre de la base dentro del .zip.
pub const ZIP_DB: &str = "visor.db";
/// Configuracion dentro del .zip.
pub const ZIP_CONFIG: &str = "config.json";
/// Carpeta de medios del glosario (capturas y audio de frases), dentro del .zip y de la carpeta de configuracion.
pub const MEDIA_DIR: &str = "phrases";
/// Carpeta donde se deja lo extraido hasta el proximo arranque.
pub const PENDING_DIR: &str = "restore-pending";
/// Marca de que lo extraido esta completo y validado.
pub const READY_FILE: &str = "LISTO";
/// Copia de la base previa a una restauracion.
pub const BEFORE_RESTORE: &str = "visor.db.antes-de-restaurar";
/// Carpeta de las copias automaticas.
pub const AUTO_DIR: &str = "backups";
/// Cada cuantos dias se hace una copia automatica y cuantas se conservan.
pub const AUTO_EVERY_DAYS: i64 = 7;
pub const AUTO_KEEP: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    /// Tamano del .zip.
    pub size: u64,
    /// Archivos de medios incluidos.
    pub media_files: usize,
}

fn add_file(zip: &mut zip::ZipWriter<File>, name: &str, path: &Path) -> Result<()> {
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(name, opts)?;
    let mut f = File::open(path).with_context(|| format!("leyendo {}", path.display()))?;
    io::copy(&mut f, zip)?;
    Ok(())
}

/// Crea el .zip en `dest`: `visor.db` (instantanea consistente), `config.json` si existe y, con
/// `include_media`, todo `phrases/`. Escribe en un temporal y renombra, asi que no deja un zip a medias.
pub fn create_backup(db: &Db, config_dir: &Path, dest: &Path, include_media: bool) -> Result<BackupInfo> {
    if let Some(dir) = dest.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir).with_context(|| format!("creando {}", dir.display()))?;
    }
    let tmp_dir = tempdir_in(dest.parent().unwrap_or(Path::new(".")), "visor-copia")?;
    let r = (|| -> Result<BackupInfo> {
        let snap = tmp_dir.join(ZIP_DB);
        db.backup_to(&snap)?;
        let mut part_name = dest.file_name().map(|n| n.to_os_string()).unwrap_or_default();
        part_name.push(".part");
        let part = dest.with_file_name(part_name);
        let mut media_files = 0;
        {
            let mut zip = zip::ZipWriter::new(File::create(&part).with_context(|| format!("creando {}", part.display()))?);
            add_file(&mut zip, ZIP_DB, &snap)?;
            let cfg = config_dir.join(ZIP_CONFIG);
            if cfg.is_file() {
                add_file(&mut zip, ZIP_CONFIG, &cfg)?;
            }
            if include_media {
                let root = config_dir.join(MEDIA_DIR);
                if root.is_dir() {
                    for e in walkdir::WalkDir::new(&root).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                        if !e.file_type().is_file() {
                            continue;
                        }
                        let rel = e.path().strip_prefix(&root).unwrap_or(e.path());
                        let name = format!("{}/{}", MEDIA_DIR, rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"));
                        add_file(&mut zip, &name, e.path())?;
                        media_files += 1;
                    }
                }
            }
            zip.finish()?.flush()?;
        }
        if let Err(e) = fs::rename(&part, dest) {
            let _ = fs::remove_file(&part);
            return Err(e).with_context(|| format!("escribiendo {}", dest.display()));
        }
        Ok(BackupInfo { size: fs::metadata(dest)?.len(), media_files })
    })();
    let _ = fs::remove_dir_all(&tmp_dir);
    r
}

fn tempdir_in(parent: &Path, prefix: &str) -> Result<PathBuf> {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let p = parent.join(format!(".{prefix}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&p).with_context(|| format!("creando {}", p.display()))?;
    Ok(p)
}

/// Lo que se sabe de un .zip validado, para pedir confirmacion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreInfo {
    pub db: DbInfo,
    pub has_config: bool,
    pub media_files: usize,
}

/// Ruta relativa segura dentro del .zip (sin `..`, ni rutas absolutas, ni unidades) o `None`.
pub(crate) fn safe_rel(name: &str) -> Option<PathBuf> {
    let name = name.replace('\\', "/");
    let mut out = PathBuf::new();
    for part in name.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            p if p.contains(':') => return None,
            p => out.push(p),
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn pending_dir(config_dir: &Path) -> PathBuf {
    config_dir.join(PENDING_DIR)
}

/// Abre el .zip, extrae solo `visor.db`, `config.json` y `phrases/**` a `<config>/restore-pending/`, valida la base
/// (SQLite del visor, integra y de version no posterior) y deja la marca `LISTO`. No toca nada mas.
/// Si algo falla, borra lo extraido.
pub fn stage_restore(zip_path: &Path, config_dir: &Path) -> Result<RestoreInfo> {
    let pend = pending_dir(config_dir);
    let _ = fs::remove_dir_all(&pend);
    fs::create_dir_all(&pend).with_context(|| format!("creando {}", pend.display()))?;
    let r = extract_and_validate(zip_path, &pend);
    if r.is_err() {
        let _ = fs::remove_dir_all(&pend);
    }
    r
}

fn extract_and_validate(zip_path: &Path, pend: &Path) -> Result<RestoreInfo> {
    let file = File::open(zip_path).with_context(|| format!("no se puede abrir {}", zip_path.display()))?;
    let mut zip = zip::ZipArchive::new(file).context("no es un archivo .zip valido")?;
    let (mut has_db, mut has_config, mut media_files) = (false, false, 0usize);
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(rel) = safe_rel(entry.name()) else { continue };
        let first = rel.components().next().map(|c| c.as_os_str().to_string_lossy().into_owned()).unwrap_or_default();
        let is_db = rel == Path::new(ZIP_DB);
        let is_cfg = rel == Path::new(ZIP_CONFIG);
        let is_media = first == MEDIA_DIR && rel.components().count() > 1;
        if !(is_db || is_cfg || is_media) {
            continue;
        }
        let out = pend.join(&rel);
        if let Some(dir) = out.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut f = File::create(&out).with_context(|| format!("extrayendo {}", rel.display()))?;
        io::copy(&mut entry, &mut f)?;
        has_db |= is_db;
        has_config |= is_cfg;
        media_files += is_media as usize;
    }
    if !has_db {
        bail!("el .zip no contiene visor.db: no es una copia de seguridad del visor");
    }
    let info = validate_file(&pend.join(ZIP_DB))?;
    if has_config && serde_json::from_slice::<serde_json::Value>(&fs::read(pend.join(ZIP_CONFIG))?).is_err() {
        bail!("el config.json de la copia esta danado");
    }
    fs::write(pend.join(READY_FILE), b"ok")?;
    Ok(RestoreInfo { db: info, has_config, media_files })
}

/// Descarta una restauracion preparada (el usuario no confirmo).
pub fn cancel_restore(config_dir: &Path) {
    let _ = fs::remove_dir_all(pending_dir(config_dir));
}

/// Resultado de aplicar una restauracion pendiente al arrancar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    /// Donde quedo la base anterior.
    pub previous_db: Option<PathBuf>,
}

fn remove_sidecars(db_path: &Path) {
    for ext in ["-wal", "-shm"] {
        let mut n = db_path.as_os_str().to_os_string();
        n.push(ext);
        let _ = fs::remove_file(PathBuf::from(n));
    }
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    for e in walkdir::WalkDir::new(from).into_iter().filter_map(|e| e.ok()) {
        let rel = e.path().strip_prefix(from)?;
        let target = to.join(rel);
        if e.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if e.file_type().is_file() {
            if let Some(d) = target.parent() {
                fs::create_dir_all(d)?;
            }
            fs::copy(e.path(), &target)?;
        }
    }
    Ok(())
}

/// Si hay una restauracion lista en `<config>/restore-pending/`, la aplica. **Debe llamarse antes de abrir la base.**
/// Pasos: (1) guarda la base actual como `visor.db.antes-de-restaurar` (abriendola para que se recupere el `-wal`);
/// (2) copia la restaurada junto a `visor.db` y la renombra encima (atomico), quitando `-wal`/`-shm`; (3) igual con
/// `config.json` (el anterior queda como `config.json.antes-de-restaurar`); (4) mezcla `phrases/`. Al final borra lo
/// pendiente, salga bien o mal, para no repetirlo en cada arranque. `None` = no habia nada pendiente.
pub fn apply_pending_restore(config_dir: &Path) -> Option<Result<Restored>> {
    let pend = pending_dir(config_dir);
    if !pend.is_dir() {
        return None;
    }
    if !pend.join(READY_FILE).is_file() {
        // Extraccion a medias (la app se cerro): se descarta.
        let _ = fs::remove_dir_all(&pend);
        return None;
    }
    let r = apply_inner(config_dir, &pend);
    let _ = fs::remove_dir_all(&pend);
    Some(r)
}

fn apply_inner(config_dir: &Path, pend: &Path) -> Result<Restored> {
    // Se vuelve a validar: entre preparar y arrancar el archivo podria haberse tocado.
    validate_file(&pend.join(ZIP_DB))?;
    let live = config_dir.join(DB_FILE);
    let before = config_dir.join(BEFORE_RESTORE);
    let mut previous_db = None;
    if live.is_file() {
        let _ = fs::remove_file(&before);
        remove_sidecars(&before);
        let snap = (|| -> Result<()> {
            let conn = rusqlite::Connection::open(&live)?;
            db::backup_to(&conn, &before)
        })();
        if snap.is_err() {
            // Base ilegible: al menos se conservan sus bytes.
            let _ = fs::remove_file(&before);
            fs::copy(&live, &before).context("guardando la base actual antes de restaurar")?;
        }
        previous_db = Some(before);
    }
    let staged = config_dir.join("visor.db.restaurando");
    fs::copy(pend.join(ZIP_DB), &staged).context("copiando la base restaurada")?;
    remove_sidecars(&live);
    if let Err(e) = fs::rename(&staged, &live) {
        let _ = fs::remove_file(&staged);
        return Err(e).context("sustituyendo visor.db");
    }
    let new_cfg = pend.join(ZIP_CONFIG);
    if new_cfg.is_file() {
        let cfg = config_dir.join(ZIP_CONFIG);
        if cfg.is_file() {
            let _ = fs::copy(&cfg, config_dir.join("config.json.antes-de-restaurar"));
        }
        let tmp = config_dir.join("config.json.restaurando");
        fs::copy(&new_cfg, &tmp)?;
        fs::rename(&tmp, &cfg).context("sustituyendo config.json")?;
    }
    let media = pend.join(MEDIA_DIR);
    if media.is_dir() {
        copy_dir(&media, &config_dir.join(MEDIA_DIR))?;
    }
    Ok(Restored { previous_db })
}

/* ------------------------------------------------------------------ */
/* Copias automaticas                                                   */
/* ------------------------------------------------------------------ */

fn auto_name(d: NaiveDate) -> String {
    format!("visor-{}.db", d.format("%Y-%m-%d"))
}

fn parse_auto_name(name: &str) -> Option<NaiveDate> {
    let s = name.strip_prefix("visor-")?.strip_suffix(".db")?;
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Copias automaticas que hay en `dir`, de la mas reciente a la mas antigua.
pub fn list_auto(dir: &Path) -> Vec<(NaiveDate, PathBuf)> {
    let mut v: Vec<(NaiveDate, PathBuf)> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| parse_auto_name(&e.file_name().to_string_lossy()).map(|d| (d, e.path())))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v
}

/// Conserva las `keep` copias mas recientes y borra el resto. Devuelve cuantas borro.
pub fn rotate_auto(dir: &Path, keep: usize) -> usize {
    list_auto(dir).into_iter().skip(keep).filter(|(_, p)| fs::remove_file(p).is_ok()).count()
}

/// Si la ultima copia automatica de `<config>/backups/` tiene `AUTO_EVERY_DAYS` dias o mas (o no hay ninguna),
/// guarda `visor-AAAA-MM-DD.db` y rota a `AUTO_KEEP`. Devuelve la ruta creada.
pub fn auto_backup_if_due(db: &Db, config_dir: &Path, today: NaiveDate) -> Result<Option<PathBuf>> {
    let dir = config_dir.join(AUTO_DIR);
    if let Some((last, _)) = list_auto(&dir).first() {
        if (today - *last).num_days() < AUTO_EVERY_DAYS {
            return Ok(None);
        }
    }
    let dest = dir.join(auto_name(today));
    db.backup_to(&dest)?;
    rotate_auto(&dir, AUTO_KEEP);
    Ok(Some(dest))
}

/// `auto_backup_if_due` con la fecha local de hoy.
pub fn auto_backup_today(db: &Db, config_dir: &Path) -> Result<Option<PathBuf>> {
    auto_backup_if_due(db, config_dir, chrono::Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::data::*;

    fn setup() -> (tempfile::TempDir, Db) {
        let d = tempfile::tempdir().unwrap();
        let db = Db::open_path(&d.path().join(DB_FILE)).unwrap();
        db.with(|c| favorite_set(c, "uno.png", true)).unwrap();
        fs::write(d.path().join("config.json"), r#"{"sound":true}"#).unwrap();
        fs::create_dir_all(d.path().join("phrases/juego")).unwrap();
        fs::write(d.path().join("phrases/juego/a.png"), b"png").unwrap();
        (d, db)
    }

    fn names(p: &Path) -> Vec<String> {
        let mut z = zip::ZipArchive::new(File::open(p).unwrap()).unwrap();
        let mut v: Vec<String> = (0..z.len()).map(|i| z.by_index(i).unwrap().name().to_string()).collect();
        v.sort();
        v
    }

    #[test]
    fn zip_with_and_without_media() {
        let (d, db) = setup();
        let out = tempfile::tempdir().unwrap();
        let a = out.path().join("sin.zip");
        let i = create_backup(&db, d.path(), &a, false).unwrap();
        assert_eq!(names(&a), vec!["config.json", "visor.db"]);
        assert_eq!((i.media_files, i.size > 0), (0, true));
        let b = out.path().join("con.zip");
        let i = create_backup(&db, d.path(), &b, true).unwrap();
        assert_eq!(names(&b), vec!["config.json", "phrases/juego/a.png", "visor.db"]);
        assert_eq!(i.media_files, 1);
        // No quedan temporales junto al destino.
        let left: Vec<_> = fs::read_dir(out.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(left.len(), 2, "{left:?}");
    }

    #[test]
    fn stage_and_apply_restore_roundtrip() {
        let (d, db) = setup();
        let out = tempfile::tempdir().unwrap();
        let z = out.path().join("c.zip");
        create_backup(&db, d.path(), &z, true).unwrap();
        // Estado posterior a la copia que debe perderse al restaurar.
        db.with(|c| favorite_set(c, "dos.png", true)).unwrap();
        fs::write(d.path().join("config.json"), r#"{"sound":false}"#).unwrap();
        fs::remove_file(d.path().join("phrases/juego/a.png")).unwrap();

        let info = stage_restore(&z, d.path()).unwrap();
        assert_eq!((info.has_config, info.media_files, info.db.version), (true, 1, db::latest_version()));
        // Como al reiniciar: la conexion vieja ya no existe cuando se aplica.
        drop(db);
        apply_pending_restore(d.path()).expect("habia una pendiente").unwrap();
        let r = Db::open_path(&d.path().join(DB_FILE)).unwrap();
        assert_eq!(r.with(favorites_load).unwrap(), vec!["uno.png"]);
        assert_eq!(fs::read_to_string(d.path().join("config.json")).unwrap(), r#"{"sound":true}"#);
        assert!(d.path().join("phrases/juego/a.png").is_file());
        // La base anterior se conserva y no queda nada pendiente.
        let prev = Db::open_path(&d.path().join(BEFORE_RESTORE)).unwrap();
        assert_eq!(prev.with(favorites_load).unwrap(), vec!["uno.png", "dos.png"]);
        assert!(d.path().join("config.json.antes-de-restaurar").is_file());
        assert!(!d.path().join(PENDING_DIR).exists());
        assert!(apply_pending_restore(d.path()).is_none());
    }

    #[test]
    fn stage_rejects_bad_zips_and_cleans_up() {
        let (d, _db) = setup();
        let out = tempfile::tempdir().unwrap();
        // No es un zip.
        let t = out.path().join("t.zip");
        fs::write(&t, "hola").unwrap();
        assert!(stage_restore(&t, d.path()).is_err());
        // Zip sin visor.db.
        let z = out.path().join("sin.zip");
        {
            let mut w = zip::ZipWriter::new(File::create(&z).unwrap());
            w.start_file("otra.txt", zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(b"x").unwrap();
            w.finish().unwrap();
        }
        assert!(stage_restore(&z, d.path()).unwrap_err().to_string().contains("visor.db"));
        // visor.db que no es del visor.
        let z2 = out.path().join("mala.zip");
        {
            let mut w = zip::ZipWriter::new(File::create(&z2).unwrap());
            w.start_file("visor.db", zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(b"no soy sqlite").unwrap();
            w.finish().unwrap();
        }
        assert!(stage_restore(&z2, d.path()).is_err());
        assert!(!d.path().join(PENDING_DIR).exists(), "se limpia lo extraido");
    }

    #[test]
    fn stage_rejects_future_version() {
        let d = tempfile::tempdir().unwrap();
        let fut = d.path().join("visor.db");
        {
            let db = Db::open_path(&fut).unwrap();
            db.with(|c| c.execute_batch(&format!("PRAGMA user_version = {}", db::latest_version() + 1))).unwrap();
        }
        let z = d.path().join("f.zip");
        {
            let mut w = zip::ZipWriter::new(File::create(&z).unwrap());
            add_file_raw(&mut w, "visor.db", &fut);
            w.finish().unwrap();
        }
        let cfg = tempfile::tempdir().unwrap();
        assert!(stage_restore(&z, cfg.path()).unwrap_err().to_string().contains("mas nueva"));
    }

    fn add_file_raw(w: &mut zip::ZipWriter<File>, name: &str, p: &Path) {
        w.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(&fs::read(p).unwrap()).unwrap();
    }

    #[test]
    fn unsafe_paths_are_ignored() {
        assert_eq!(safe_rel("phrases/a/b.png"), Some(PathBuf::from("phrases/a/b.png")));
        assert_eq!(safe_rel("phrases\\a.png"), Some(PathBuf::from("phrases/a.png")));
        assert!(safe_rel("../x").is_none() && safe_rel("phrases/../../x").is_none() && safe_rel("C:/x").is_none());
        assert!(safe_rel("").is_none());
    }

    #[test]
    fn half_extracted_pending_is_discarded() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join(PENDING_DIR)).unwrap();
        fs::write(d.path().join(PENDING_DIR).join("visor.db"), b"x").unwrap();
        assert!(apply_pending_restore(d.path()).is_none());
        assert!(!d.path().join(PENDING_DIR).exists());
    }

    #[test]
    fn auto_backup_every_seven_days_keeps_five() {
        let (d, db) = setup();
        let day = |m, dd| NaiveDate::from_ymd_opt(2026, m, dd).unwrap();
        let dir = d.path().join(AUTO_DIR);
        assert!(auto_backup_if_due(&db, d.path(), day(1, 1)).unwrap().is_some(), "la primera siempre");
        assert!(auto_backup_if_due(&db, d.path(), day(1, 7)).unwrap().is_none(), "6 dias: aun no");
        assert!(auto_backup_if_due(&db, d.path(), day(1, 8)).unwrap().is_some(), "7 dias: toca");
        for (m, dd) in [(1, 15), (1, 22), (1, 29), (2, 5), (2, 12)] {
            assert!(auto_backup_if_due(&db, d.path(), day(m, dd)).unwrap().is_some());
        }
        let l = list_auto(&dir);
        assert_eq!(l.len(), AUTO_KEEP);
        assert_eq!(l[0].0, day(2, 12));
        assert_eq!(l[4].0, day(1, 15), "las mas antiguas se borran");
        // Archivos ajenos no se tocan ni cuentan.
        fs::write(dir.join("notas.txt"), "x").unwrap();
        assert_eq!(rotate_auto(&dir, 1), 4);
        assert!(dir.join("notas.txt").exists());
        // La copia es una base valida.
        assert!(validate_file(&l[0].1).is_ok());
        assert_eq!(Db::open_path(&dir.join("visor-2026-02-12.db")).unwrap().with(favorites_load).unwrap(), vec!["uno.png"]);
    }
}
