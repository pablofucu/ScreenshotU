//! Copias .zip de las partidas de un juego: crear (con huella para no repetir copias iguales), listar,
//! podar las automaticas y restaurar (antes se guarda una copia de lo que hay).
//!
//! Dentro del .zip: `copia.json` (juego, fecha, tipo, huella y ubicaciones), los archivos de la ubicacion `i`
//! bajo `e<i>/` (relativos a su carpeta fija, el «ancla») y las claves del registro como `e<i>.reg`.
use super::{glob, literal_prefix, registry, LocKind, MATCH_CAP};
use crate::backup::safe_rel;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const COPY_JSON: &str = "copia.json";
/// Copias automaticas que se conservan por juego (las manuales y las previas a restaurar no se borran solas).
pub const KEEP_AUTO: usize = 10;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CopyKind {
    /// Al cerrar el juego.
    Auto,
    /// Pedida por el usuario.
    Manual,
    /// Lo que habia justo antes de restaurar otra copia.
    Antes,
}

impl CopyKind {
    fn suffix(self) -> &'static str {
        match self {
            CopyKind::Auto => "auto",
            CopyKind::Manual => "manual",
            CopyKind::Antes => "antes-de-restaurar",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CopyEntry {
    /// Ubicacion tal y como se detecto (puede tener comodines).
    pub path: String,
    pub kind: LocKind,
    /// Carpeta a la que son relativos sus archivos en el .zip.
    pub anchor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopyManifest {
    pub version: u32,
    pub game_id: String,
    pub game_name: String,
    pub created_ms: i64,
    pub kind: CopyKind,
    pub fingerprint: String,
    pub entries: Vec<CopyEntry>,
    pub files: usize,
    pub bytes: u64,
}

/// Una copia en disco, para la lista.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveCopy {
    pub file: String,
    pub created_ms: i64,
    pub kind: CopyKind,
    /// Tamano del .zip.
    pub size: u64,
    pub files: usize,
}

/// Carpeta de las copias de un juego: `<raiz>/<Nombre> [<final del id>]`.
pub fn game_dir(root: &Path, game_id: &str, game_name: &str) -> PathBuf {
    let tail: String = game_id.chars().rev().take(6).collect::<Vec<_>>().into_iter().rev().filter(|c| c.is_ascii_alphanumeric()).collect();
    root.join(format!("{} [{tail}]", crate::games::sanitize_folder_name(game_name)))
}

struct Item {
    abs: PathBuf,
    zip_name: String,
    size: u64,
    mtime: i64,
}

/// Archivos de las ubicaciones y entradas para `copia.json`. Las claves de registro se exportan a `tmp`.
fn collect(locs: &[(String, LocKind)], tmp: &Path) -> Result<(Vec<CopyEntry>, Vec<Item>)> {
    let mut entries = Vec::new();
    let mut items = Vec::new();
    for (path, kind) in locs {
        let i = entries.len();
        match kind {
            LocKind::Registry => {
                if !registry::exists(path) {
                    continue;
                }
                let f = tmp.join(format!("e{i}.reg"));
                registry::export(path, &f)?;
                let md = fs::metadata(&f)?;
                items.push(Item { abs: f, zip_name: format!("e{i}.reg"), size: md.len(), mtime: 0 });
                entries.push(CopyEntry { path: path.clone(), kind: *kind, anchor: String::new() });
            }
            LocKind::Path => {
                let matches = glob::expand(path, MATCH_CAP);
                if matches.is_empty() {
                    continue;
                }
                let anchor = if glob::has_magic(path) {
                    literal_prefix(path)
                } else if Path::new(path).is_dir() {
                    PathBuf::from(path)
                } else {
                    Path::new(path).parent().map(Path::to_path_buf).unwrap_or_default()
                };
                let before = items.len();
                for m in matches {
                    for e in WalkDir::new(&m).into_iter().flatten().filter(|e| e.file_type().is_file()) {
                        let Ok(rel) = e.path().strip_prefix(&anchor) else { continue };
                        let md = e.metadata()?;
                        let rel = rel.to_string_lossy().replace('\\', "/");
                        items.push(Item { abs: e.path().to_path_buf(), zip_name: format!("e{i}/{rel}"), size: md.len(), mtime: crate::scan::mtime_ms(&md) });
                    }
                }
                if items.len() > before {
                    entries.push(CopyEntry { path: path.clone(), kind: *kind, anchor: anchor.to_string_lossy().into_owned() });
                }
            }
        }
    }
    items.sort_by(|a, b| a.zip_name.cmp(&b.zip_name));
    items.dedup_by(|a, b| a.zip_name == b.zip_name);
    Ok((entries, items))
}

/// Huella del contenido: nombres, tamanos y fechas; las claves del registro, por su contenido.
fn fingerprint(items: &[Item]) -> Result<String> {
    let mut h = Sha256::new();
    for it in items {
        h.update(it.zip_name.as_bytes());
        h.update(it.size.to_le_bytes());
        if it.zip_name.ends_with(".reg") && !it.zip_name.contains('/') {
            h.update(fs::read(&it.abs)?);
        } else {
            h.update(it.mtime.to_le_bytes());
        }
    }
    Ok(format!("{:x}", h.finalize()))
}

fn read_manifest(zip: &Path) -> Result<CopyManifest> {
    let mut z = zip::ZipArchive::new(File::open(zip).with_context(|| format!("abriendo {}", zip.display()))?).context("no es un .zip valido")?;
    let mut s = String::new();
    z.by_name(COPY_JSON).context("no es una copia de partidas del visor")?.read_to_string(&mut s)?;
    serde_json::from_str(&s).context("copia.json no se entiende")
}

/// Copias de la carpeta, la mas reciente primero.
pub fn list(dir: &Path) -> Vec<SaveCopy> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<SaveCopy> = rd
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("zip")).unwrap_or(false))
        .filter_map(|e| {
            let m = read_manifest(&e.path()).ok()?;
            Some(SaveCopy { file: e.path().to_string_lossy().into_owned(), created_ms: m.created_ms, kind: m.kind, size: e.metadata().map(|md| md.len()).unwrap_or(0), files: m.files })
        })
        .collect();
    v.sort_by(|a, b| b.created_ms.cmp(&a.created_ms).then(b.file.cmp(&a.file)));
    v
}

/// Lo que se va a copiar.
pub struct CopyReq<'a> {
    pub dir: &'a Path,
    pub game_id: &'a str,
    pub game_name: &'a str,
    pub locs: &'a [(String, LocKind)],
    pub kind: CopyKind,
    pub now_ms: i64,
    /// Nombre del archivo segun la hora local (`AAAA-MM-DD_HHMMSS`).
    pub stamp: &'a str,
    /// No crear la copia si el contenido es igual al de la ultima.
    pub skip_if_same: bool,
}

/// Crea la copia. `Ok(None)` si no hay nada que copiar o (con `skip_if_same`) nada cambio desde la ultima.
pub fn create(req: &CopyReq) -> Result<Option<SaveCopy>> {
    fs::create_dir_all(req.dir).with_context(|| format!("creando {}", req.dir.display()))?;
    let tmp = req.dir.join(format!(".tmp-{}-{}", std::process::id(), req.now_ms));
    fs::create_dir_all(&tmp)?;
    let r = create_in(req, &tmp);
    let _ = fs::remove_dir_all(&tmp);
    r
}

fn create_in(req: &CopyReq, tmp: &Path) -> Result<Option<SaveCopy>> {
    let (entries, items) = collect(req.locs, tmp)?;
    if items.is_empty() {
        return Ok(None);
    }
    let fp = fingerprint(&items)?;
    if req.skip_if_same && list(req.dir).first().and_then(|c| read_manifest(Path::new(&c.file)).ok()).map(|m| m.fingerprint == fp).unwrap_or(false) {
        return Ok(None);
    }
    let man = CopyManifest {
        version: 1,
        game_id: req.game_id.into(),
        game_name: req.game_name.into(),
        created_ms: req.now_ms,
        kind: req.kind,
        fingerprint: fp,
        entries,
        files: items.iter().filter(|i| i.zip_name.contains('/')).count(),
        bytes: items.iter().map(|i| i.size).sum(),
    };
    let mut name = format!("{}-{}.zip", req.stamp, req.kind.suffix());
    let mut n = 2;
    while req.dir.join(&name).exists() {
        name = format!("{}-{}-{n}.zip", req.stamp, req.kind.suffix());
        n += 1;
    }
    let dest = req.dir.join(&name);
    let part = req.dir.join(format!("{name}.part"));
    let r = (|| -> Result<()> {
        let mut z = zip::ZipWriter::new(File::create(&part)?);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
        z.start_file(COPY_JSON, opts)?;
        z.write_all(&serde_json::to_vec_pretty(&man)?)?;
        for it in &items {
            z.start_file(it.zip_name.as_str(), opts)?;
            let mut f = File::open(&it.abs).with_context(|| format!("no se pudo leer {} (¿el juego sigue abierto?)", it.abs.display()))?;
            io::copy(&mut f, &mut z)?;
        }
        z.finish()?;
        fs::rename(&part, &dest)?;
        Ok(())
    })();
    if let Err(e) = r {
        let _ = fs::remove_file(&part);
        return Err(e);
    }
    let size = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    Ok(Some(SaveCopy { file: dest.to_string_lossy().into_owned(), created_ms: man.created_ms, kind: man.kind, size, files: man.files }))
}

/// Borra las copias automaticas mas antiguas por encima de `keep`. Devuelve cuantas borro.
pub fn prune(dir: &Path, keep: usize) -> usize {
    list(dir).into_iter().filter(|c| c.kind == CopyKind::Auto).skip(keep).filter(|c| fs::remove_file(&c.file).is_ok()).count()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub files: usize,
    pub registry: usize,
    /// Copia de lo que habia antes de restaurar (si habia algo).
    pub before: Option<SaveCopy>,
}

/// Restaura `zip` en sus ubicaciones originales: primero guarda en `dir` una copia de lo que hay ahora y despues
/// sobrescribe (los archivos que no estan en la copia no se borran).
pub fn restore(zip: &Path, dir: &Path, now_ms: i64, stamp: &str) -> Result<RestoreReport> {
    let man = read_manifest(zip)?;
    for e in &man.entries {
        if e.kind == LocKind::Path && !Path::new(&e.anchor).is_absolute() {
            bail!("la copia tiene una ruta no valida: {}", e.anchor);
        }
    }
    let locs: Vec<(String, LocKind)> = man.entries.iter().map(|e| (e.path.clone(), e.kind)).collect();
    let before = create(&CopyReq { dir, game_id: &man.game_id, game_name: &man.game_name, locs: &locs, kind: CopyKind::Antes, now_ms, stamp, skip_if_same: false })?;
    let mut z = zip::ZipArchive::new(File::open(zip)?)?;
    let tmp = dir.join(format!(".tmp-restaurar-{}-{now_ms}", std::process::id()));
    let mut rep = RestoreReport { files: 0, registry: 0, before };
    let r = (|| -> Result<()> {
        for k in 0..z.len() {
            let mut f = z.by_index(k)?;
            if f.is_dir() {
                continue;
            }
            let name = f.name().to_string();
            if name == COPY_JSON {
                continue;
            }
            let (head, rest) = name.split_once('/').unwrap_or((name.as_str(), ""));
            if rest.is_empty() {
                // Clave del registro: e<i>.reg
                let Some(i) = head.strip_prefix('e').and_then(|s| s.strip_suffix(".reg")).and_then(|s| s.parse::<usize>().ok()) else { continue };
                if man.entries.get(i).map(|e| e.kind) != Some(LocKind::Registry) {
                    continue;
                }
                fs::create_dir_all(&tmp)?;
                let p = tmp.join(format!("e{i}.reg"));
                io::copy(&mut f, &mut File::create(&p)?)?;
                registry::import(&p)?;
                rep.registry += 1;
                continue;
            }
            let Some(i) = head.strip_prefix('e').and_then(|s| s.parse::<usize>().ok()) else { continue };
            let Some(e) = man.entries.get(i).filter(|e| e.kind == LocKind::Path) else { continue };
            let Some(rel) = safe_rel(rest) else { bail!("la copia tiene una ruta no valida: {name}") };
            let dest = Path::new(&e.anchor).join(rel);
            if let Some(p) = dest.parent() {
                fs::create_dir_all(p).with_context(|| format!("creando {}", p.display()))?;
            }
            let mut out = File::create(&dest).with_context(|| format!("no se pudo escribir {} (¿el juego sigue abierto?)", dest.display()))?;
            io::copy(&mut f, &mut out)?;
            rep.files += 1;
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&tmp);
    r.map(|_| rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(p: &Path, s: &str) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, s).unwrap();
    }

    fn req<'a>(dir: &'a Path, locs: &'a [(String, LocKind)], kind: CopyKind, now: i64, stamp: &'a str, skip: bool) -> CopyReq<'a> {
        CopyReq { dir, game_id: "manual:abc123", game_name: "Mi: Juego", locs, kind, now_ms: now, stamp, skip_if_same: skip }
    }

    #[test]
    fn create_skip_list_prune_and_restore() {
        let d = tempfile::tempdir().unwrap();
        let game = d.path().join("juego");
        put(&game.join("save/slot1.sav"), "uno");
        put(&game.join("save/sub/slot2.sav"), "dos");
        put(&game.join("Save01.rvdata2"), "rv");
        put(&game.join("otra.txt"), "no");
        let s = |p: &Path| p.to_string_lossy().into_owned();
        let locs = vec![
            (s(&game.join("save")), LocKind::Path),
            (format!("{}/Save*.rvdata2", s(&game)), LocKind::Path),
            (s(&game.join("no-existe")), LocKind::Path),
        ];
        let out = game_dir(&d.path().join("copias"), "manual:abc123", "Mi: Juego");
        assert!(out.ends_with("Mi - Juego [abc123]"));
        let c1 = create(&req(&out, &locs, CopyKind::Manual, 1000, "2026-10-19_120000", true)).unwrap().unwrap();
        assert_eq!(c1.files, 3);
        // Sin cambios: no se repite.
        assert!(create(&req(&out, &locs, CopyKind::Auto, 2000, "2026-10-19_120001", true)).unwrap().is_none());
        // Cambia una partida: copia nueva.
        put(&game.join("save/slot1.sav"), "uno-modificado");
        let c2 = create(&req(&out, &locs, CopyKind::Auto, 3000, "2026-10-19_120002", true)).unwrap().unwrap();
        assert_eq!(list(&out).iter().map(|c| c.created_ms).collect::<Vec<_>>(), vec![3000, 1000]);
        // Nada que copiar.
        assert!(create(&req(&out, &[(s(&game.join("vacia")), LocKind::Path)], CopyKind::Manual, 4000, "x", false)).unwrap().is_none());
        // Restaurar la primera: vuelve el contenido y se guarda antes lo que habia.
        put(&game.join("save/sub/slot2.sav"), "estropeado");
        fs::remove_file(game.join("Save01.rvdata2")).unwrap();
        let rep = restore(Path::new(&c1.file), &out, 5000, "2026-10-19_120005").unwrap();
        assert_eq!((rep.files, rep.registry), (3, 0));
        assert_eq!(fs::read_to_string(game.join("save/slot1.sav")).unwrap(), "uno");
        assert_eq!(fs::read_to_string(game.join("save/sub/slot2.sav")).unwrap(), "dos");
        assert_eq!(fs::read_to_string(game.join("Save01.rvdata2")).unwrap(), "rv");
        assert_eq!(rep.before.unwrap().kind, CopyKind::Antes);
        assert!(!fs::read_dir(&out).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with(".tmp")));
        // Podar: solo las automaticas.
        assert_eq!(prune(&out, 0), 1);
        assert!(!Path::new(&c2.file).exists());
        assert_eq!(list(&out).len(), 2);
    }

    #[test]
    fn rejects_foreign_zip() {
        let d = tempfile::tempdir().unwrap();
        let z = d.path().join("x.zip");
        let mut w = zip::ZipWriter::new(File::create(&z).unwrap());
        w.start_file("a.txt", zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(b"x").unwrap();
        w.finish().unwrap();
        assert!(restore(&z, d.path(), 1, "s").unwrap_err().to_string().contains("no es una copia"));
        assert!(list(d.path()).is_empty());
    }
}
