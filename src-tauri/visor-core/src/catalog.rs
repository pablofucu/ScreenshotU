//! Catalogo de capturas (tabla `shots`): una fila por imagen de la biblioteca con su ruta portable
//! (ver [`crate::relpath`]), su titulo, tamano, fechas, dimensiones y su huella SHA-256.
//!
//! Es la base para sincronizar con la nube: la huella identifica el contenido aunque el archivo se
//! mueva o se renombre, y permite saber que hay que subir sin releer todo. El catalogo se pone al dia
//! con cada escaneo ([`sync`], solo las filas que cambian) y sirve de cache de dimensiones para el siguiente ([`load_cache`]);
//! la huella se calcula despues, en segundo plano ([`hash_pending`]): solo la de los archivos nuevos o modificados (cambia el tamano o la fecha de modificacion).
//! Los archivos del usuario solo se leen.
use crate::db::Db;
use crate::relpath;
use crate::scan::{mtime_ms, Library, ShotCache};
use anyhow::Result;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Migracion que crea el catalogo.
pub const SCHEMA: &str = r#"
-- Catalogo de capturas: ruta relativa a la raiz (con '/'), titulo y huella del contenido.
CREATE TABLE shots (
    path     TEXT PRIMARY KEY,
    title_id TEXT NOT NULL,
    size     INTEGER NOT NULL,
    mtime_ms INTEGER NOT NULL,
    date_ms  INTEGER NOT NULL,
    width    INTEGER NOT NULL,
    height   INTEGER NOT NULL,
    -- SHA-256 en hexadecimal; NULL hasta que se calcula (o si el archivo cambio desde entonces).
    sha256   TEXT,
    seen_ms  INTEGER NOT NULL
);
CREATE INDEX shots_title ON shots(title_id);
CREATE INDEX shots_sha256 ON shots(sha256);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SyncReport {
    /// Capturas en el catalogo tras el escaneo.
    pub total: usize,
    /// Filas quitadas porque el archivo ya no esta.
    pub removed: usize,
}

/// Dimensiones ya conocidas del catalogo (ruta absoluta bajo `root`), para que el escaneo no abra de nuevo
/// las imagenes que no cambiaron (ver [`crate::scan::ShotCache`]). Solo lee la base.
pub fn load_cache(c: &Connection, root: &Path) -> Result<ShotCache> {
    let mut st = c.prepare("SELECT path, size, mtime_ms, width, height FROM shots")?;
    let mut cache = ShotCache::default();
    let mut rows = st.query([])?;
    while let Some(r) = rows.next()? {
        let rel: String = r.get(0)?;
        cache.insert(relpath::to_abs(root, &rel), r.get::<_, i64>(1)? as u64, r.get(2)?, r.get(3)?, r.get(4)?);
    }
    Ok(cache)
}

/// Fila del catalogo sin la huella: lo que un escaneo puede cambiar.
type Row = (String, i64, i64, i64, u32, u32);

/// Pone el catalogo al dia con un escaneo **completo** de `root` (con los titulos ocultos incluidos):
/// anade y actualiza las capturas vistas y quita las que ya no estan. La huella se conserva si el
/// archivo no cambio de tamano ni de fecha. Si la raiz no existe (disco desconectado) no se toca nada.
/// Solo se escriben las filas nuevas o distintas (con la biblioteca sin cambios no se escribe nada: antes
/// se reescribian las miles de filas en cada escaneo); `seen_ms` queda como la ultima vez que cambio la fila.
pub fn sync(c: &Connection, root: &Path, lib: &Library, now_ms: i64) -> Result<SyncReport> {
    if !root.is_dir() {
        return Ok(SyncReport::default());
    }
    let mut old: HashMap<String, Row> = HashMap::new();
    {
        let mut st = c.prepare("SELECT path, title_id, size, mtime_ms, date_ms, width, height FROM shots")?;
        let mut rows = st.query([])?;
        while let Some(r) = rows.next()? {
            old.insert(r.get(0)?, (r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?));
        }
    }
    let tx = c.unchecked_transaction()?;
    let mut total = 0;
    {
        let mut up = tx.prepare(
            "INSERT INTO shots(path, title_id, size, mtime_ms, date_ms, width, height, sha256, seen_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)
             ON CONFLICT(path) DO UPDATE SET
                 sha256   = CASE WHEN size = excluded.size AND mtime_ms = excluded.mtime_ms THEN sha256 ELSE NULL END,
                 title_id = excluded.title_id, size = excluded.size, mtime_ms = excluded.mtime_ms,
                 date_ms  = excluded.date_ms, width = excluded.width, height = excluded.height, seen_ms = excluded.seen_ms",
        )?;
        for t in lib.platforms.iter().flat_map(|p| &p.titles) {
            for s in &t.shots {
                let Some(rel) = relpath::to_rel(root, &s.path) else { continue };
                total += 1;
                let row: Row = (t.id.clone(), s.size as i64, s.mtime, s.date, s.width, s.height);
                if old.remove(&rel).as_ref() == Some(&row) {
                    continue; // sin cambios
                }
                up.execute(params![rel, row.0, row.1, row.2, row.3, row.4, row.5, now_ms])?;
            }
        }
    }
    // Lo que queda en `old` ya no esta en la biblioteca.
    let removed = old.len();
    if removed > 0 {
        let mut del = tx.prepare("DELETE FROM shots WHERE path = ?1")?;
        for rel in old.keys() {
            del.execute([rel])?;
        }
    }
    tx.commit()?;
    Ok(SyncReport { total, removed })
}

/// Una captura cambia de ruta (se movio a otro juego): conserva su fila y su huella.
pub fn rename(c: &Connection, old_rel: &str, new_rel: &str) -> Result<()> {
    c.execute("UPDATE OR REPLACE shots SET path = ?2 WHERE path = ?1", [old_rel, new_rel])?;
    Ok(())
}

/// Una captura se ha borrado (a la papelera): se quita su fila del catalogo.
pub fn remove(c: &Connection, rel: &str) -> Result<()> {
    c.execute("DELETE FROM shots WHERE path = ?1", [rel])?;
    Ok(())
}

/// Capturas en el catalogo y cuantas tienen ya huella.
pub fn counts(c: &Connection) -> Result<(usize, usize)> {
    Ok(c.query_row("SELECT count(*), count(sha256) FROM shots", [], |r| Ok((r.get::<_, i64>(0)? as usize, r.get::<_, i64>(1)? as usize)))?)
}

/// Huella de una captura por su ruta portable.
pub fn hash_of(c: &Connection, rel: &str) -> Result<Option<String>> {
    use rusqlite::OptionalExtension;
    Ok(c.query_row("SELECT sha256 FROM shots WHERE path = ?1", [rel], |r| r.get(0)).optional()?.flatten())
}

struct Pending {
    rowid: i64,
    path: String,
    size: i64,
    mtime: i64,
}

fn pending(c: &Connection, after_rowid: i64, limit: usize) -> Result<Vec<Pending>> {
    let mut st = c.prepare("SELECT rowid, path, size, mtime_ms FROM shots WHERE sha256 IS NULL AND rowid > ?1 ORDER BY rowid LIMIT ?2")?;
    let v = st
        .query_map(params![after_rowid, limit as i64], |r| Ok(Pending { rowid: r.get(0)?, path: r.get(1)?, size: r.get(2)?, mtime: r.get(3)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(v)
}

/// SHA-256 de un archivo, en hexadecimal (minusculas).
pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Calcula las huellas que faltan, por lotes y sin tener la base bloqueada mientras se lee del disco.
/// Un archivo que cambio desde el escaneo (o ya no esta) se salta: el proximo escaneo lo pondra al dia.
/// `stop` se consulta entre archivos para poder parar. Devuelve cuantas huellas se guardaron.
pub fn hash_pending(db: &Db, root: &Path, stop: impl Fn() -> bool) -> Result<usize> {
    let mut done = 0;
    let mut cursor = 0;
    loop {
        let batch = db.with(|c| pending(c, cursor, 64))?;
        let Some(last) = batch.last() else { return Ok(done) };
        cursor = last.rowid;
        for p in batch {
            if stop() {
                return Ok(done);
            }
            let abs = relpath::to_abs(root, &p.path);
            let Ok(md) = std::fs::metadata(&abs) else { continue };
            if md.len() as i64 != p.size || mtime_ms(&md) != p.mtime {
                continue;
            }
            let Ok(hash) = hash_file(Path::new(&abs)) else { continue };
            let n = db.with(|c| {
                c.execute(
                    "UPDATE shots SET sha256 = ?4 WHERE path = ?1 AND size = ?2 AND mtime_ms = ?3",
                    params![p.path, p.size, p.mtime, hash],
                )
            })?;
            done += n;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_platforms;
    use crate::games::Stats;
    use crate::scan::{scan_library, scan_library_ex, ScanOpts};
    use crate::userdata::HiddenEntry;
    use std::fs;

    fn png(path: &Path, w: u32) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbImage::new(w, 4).save(path).unwrap();
    }

    fn scan(root: &Path) -> Library {
        scan_library(Some(root), &default_platforms(), &[], &Stats::new())
    }

    fn rows(c: &Connection) -> Vec<(String, String, Option<String>)> {
        let mut st = c.prepare("SELECT path, title_id, sha256 FROM shots ORDER BY path").unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().map(|r| r.unwrap()).collect()
    }

    #[test]
    fn hash_file_is_sha256() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.bin");
        fs::write(&p, b"abc").unwrap();
        assert_eq!(hash_file(&p).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn sync_hash_rename_and_remove() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        png(&r.join("PC/Hades II/1.png"), 8);
        png(&r.join("PC/Hades II/sub/2.png"), 9);
        png(&r.join("Switch/Zelda/3.png"), 10);
        png(&r.join("PC/Hades II/cover.png"), 11); // grafismo: no entra
        let db = Db::open_memory().unwrap();
        let rep = db.with(|c| sync(c, r, &scan(r), 1)).unwrap();
        assert_eq!(rep, SyncReport { total: 3, removed: 0 });
        let v = db.with(rows);
        assert_eq!(v.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), vec!["PC/Hades II/1.png", "PC/Hades II/sub/2.png", "Switch/Zelda/3.png"]);
        assert_eq!(v[0].1, "pc/hades ii");
        assert!(v.iter().all(|x| x.2.is_none()), "sin huella hasta el hilo de fondo");

        assert_eq!(hash_pending(&db, r, || false).unwrap(), 3);
        assert_eq!(db.with(counts).unwrap(), (3, 3));
        let h1 = db.with(|c| hash_of(c, "PC/Hades II/1.png")).unwrap().unwrap();
        assert_eq!(h1, hash_file(&r.join("PC/Hades II/1.png")).unwrap());
        assert_eq!(hash_pending(&db, r, || false).unwrap(), 0, "nada pendiente");

        // Un nuevo escaneo sin cambios conserva las huellas.
        db.with(|c| sync(c, r, &scan(r), 2)).unwrap();
        assert_eq!(db.with(counts).unwrap(), (3, 3));

        // Mover conserva la huella; borrar quita la fila; modificar la invalida.
        fs::create_dir_all(r.join("Switch/Kirby")).unwrap();
        fs::rename(r.join("PC/Hades II/1.png"), r.join("Switch/Kirby/1.png")).unwrap();
        db.with(|c| rename(c, "PC/Hades II/1.png", "Switch/Kirby/1.png")).unwrap();
        fs::remove_file(r.join("Switch/Zelda/3.png")).unwrap();
        png(&r.join("PC/Hades II/sub/2.png"), 20);
        let rep = db.with(|c| sync(c, r, &scan(r), 3)).unwrap();
        assert_eq!(rep, SyncReport { total: 2, removed: 1 });
        assert_eq!(db.with(|c| hash_of(c, "Switch/Kirby/1.png")).unwrap(), Some(h1));
        assert_eq!(db.with(rows).iter().find(|x| x.0 == "Switch/Kirby/1.png").unwrap().1, "switch/kirby");
        assert_eq!(db.with(counts).unwrap(), (2, 1), "la modificada espera nueva huella");

        // Borrar una captura (papelera) quita su fila y no falla si ya no estaba.
        db.with(|c| remove(c, "Switch/Kirby/1.png")).unwrap();
        db.with(|c| remove(c, "Switch/Kirby/1.png")).unwrap();
        assert_eq!(db.with(counts).unwrap(), (1, 0));
        assert_eq!(db.with(|c| hash_of(c, "Switch/Kirby/1.png")).unwrap(), None);
    }

    #[test]
    fn sync_writes_only_what_changed_and_feeds_the_scan_cache() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        png(&r.join("PC/X/1.png"), 8);
        png(&r.join("PC/X/2.png"), 9);
        png(&r.join("PC/Y/3.png"), 10);
        let db = Db::open_memory().unwrap();
        db.with(|c| sync(c, r, &scan(r), 1)).unwrap();
        let seen = |c: &Connection, p: &str| c.query_row("SELECT seen_ms FROM shots WHERE path = ?1", [p], |x| x.get::<_, i64>(0)).unwrap();
        // Sin cambios: no se reescribe ninguna fila.
        let rep = db.with(|c| sync(c, r, &scan(r), 2)).unwrap();
        assert_eq!(rep, SyncReport { total: 3, removed: 0 });
        assert_eq!(db.with(|c| seen(c, "PC/X/1.png")), 1);
        // Un archivo cambiado, uno nuevo y uno borrado: solo esas filas se tocan.
        png(&r.join("PC/X/2.png"), 20);
        png(&r.join("PC/Y/4.png"), 11);
        fs::remove_file(r.join("PC/Y/3.png")).unwrap();
        let rep = db.with(|c| sync(c, r, &scan(r), 3)).unwrap();
        assert_eq!(rep, SyncReport { total: 3, removed: 1 });
        assert_eq!(db.with(|c| seen(c, "PC/X/1.png")), 1, "la que no cambio sigue como estaba");
        assert_eq!(db.with(|c| seen(c, "PC/X/2.png")), 3);
        assert_eq!(db.with(|c| seen(c, "PC/Y/4.png")), 3);
        assert_eq!(db.with(counts).unwrap().0, 3);
        // El catalogo vale de cache de dimensiones para el siguiente escaneo.
        let cache = db.with(|c| load_cache(c, r)).unwrap();
        assert_eq!(cache.len(), 3);
        let lib = scan_library_ex(Some(r), &default_platforms(), &[], &Stats::new(), &ScanOpts { shot_cache: Some(&cache), ..Default::default() });
        let x = lib.platforms[0].titles.iter().find(|t| t.name == "X").unwrap();
        assert!(x.shots.iter().any(|s| (s.width, s.height) == (20, 4)) && x.shots.iter().any(|s| (s.width, s.height) == (8, 4)));
    }

    #[test]
    fn missing_root_keeps_catalog_and_hidden_titles_count() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path().join("lib");
        png(&r.join("PC/Oculto/1.png"), 8);
        let db = Db::open_memory().unwrap();
        let hidden = vec![HiddenEntry { id: "pc/oculto".into(), name: "Oculto".into(), platform: "pc".into() }];
        let lib = scan_library_ex(Some(&r), &default_platforms(), &[], &Stats::new(), &ScanOpts { hidden: &hidden, show_hidden: true, ..Default::default() });
        assert_eq!(db.with(|c| sync(c, &r, &lib, 1)).unwrap().total, 1);
        let gone = d.path().join("desconectado");
        assert_eq!(db.with(|c| sync(c, &gone, &Library::default(), 2)).unwrap(), SyncReport::default());
        assert_eq!(db.with(counts).unwrap().0, 1, "sin raiz no se borra nada");
    }

    #[test]
    fn changed_file_after_scan_is_skipped() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        png(&r.join("PC/X/1.png"), 8);
        let db = Db::open_memory().unwrap();
        db.with(|c| sync(c, r, &scan(r), 1)).unwrap();
        png(&r.join("PC/X/1.png"), 30); // cambia de tamano antes de calcular la huella
        assert_eq!(hash_pending(&db, r, || false).unwrap(), 0);
        assert_eq!(hash_pending(&db, r, || true).unwrap(), 0, "stop corta enseguida");
    }
}
