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
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const COPY_JSON: &str = "copia.json";
/// Notas y copias fijadas de un juego, junto a los .zip (por nombre de archivo).
pub const META_FILE: &str = "copias.json";
/// Copias automaticas que se conservan por juego (las manuales, las fijadas y las previas a restaurar no se borran solas).
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
    /// Nota del usuario ("" = sin nota).
    pub note: String,
    /// Fijada: la poda no la borra.
    pub pinned: bool,
}

/// Nota y fijado de una copia, tal y como se guardan en `copias.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CopyMeta {
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub pinned: bool,
}
type MetaMap = BTreeMap<String, CopyMeta>;

fn meta_load(dir: &Path) -> MetaMap {
    fs::read_to_string(dir.join(META_FILE)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn meta_save(dir: &Path, m: &MetaMap) -> Result<()> {
    let part = dir.join(format!("{META_FILE}.part"));
    fs::write(&part, serde_json::to_vec_pretty(m)?)?;
    fs::rename(&part, dir.join(META_FILE))?;
    Ok(())
}

fn file_key(zip: &Path) -> String {
    zip.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Cambia la nota o el fijado de una copia (el que venga con valor).
pub fn set_meta(dir: &Path, zip: &Path, note: Option<&str>, pinned: Option<bool>) -> Result<()> {
    if zip.parent() != Some(dir) || !zip.is_file() {
        bail!("esa copia ya no existe");
    }
    let mut m = meta_load(dir);
    let e = m.entry(file_key(zip)).or_default();
    if let Some(n) = note {
        e.note = n.trim().to_string();
    }
    if let Some(p) = pinned {
        e.pinned = p;
    }
    if *e == CopyMeta::default() {
        m.remove(&file_key(zip));
    }
    meta_save(dir, &m)
}

/// Borra una copia y sus datos. Solo archivos .zip de la carpeta del juego.
pub fn delete_copy(dir: &Path, zip: &Path) -> Result<()> {
    if zip.parent() != Some(dir) || !zip.extension().map(|x| x.eq_ignore_ascii_case("zip")).unwrap_or(false) {
        bail!("esa copia no es de este juego");
    }
    fs::remove_file(zip).with_context(|| format!("no se pudo borrar {}", zip.display()))?;
    let mut m = meta_load(dir);
    if m.remove(&file_key(zip)).is_some() {
        meta_save(dir, &m)?;
    }
    Ok(())
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

/// Fecha local de un archivo como fecha de entrada del .zip (None si no cabe en el formato).
fn zip_time(ms: i64) -> Option<zip::DateTime> {
    use chrono::{Datelike, TimeZone, Timelike};
    if ms <= 0 {
        return None;
    }
    let d = chrono::Local.timestamp_millis_opt(ms).single()?;
    zip::DateTime::from_date_and_time(u16::try_from(d.year()).ok()?, d.month() as u8, d.day() as u8, d.hour() as u8, d.minute() as u8, d.second() as u8).ok()
}

/// Fecha de una entrada del .zip en ms (0 si no tiene una real: las copias antiguas guardan 1980).
fn entry_ms(t: Option<zip::DateTime>) -> i64 {
    use chrono::TimeZone;
    let Some(t) = t.filter(|t| t.year() > 1980) else { return 0 };
    chrono::Local
        .with_ymd_and_hms(t.year() as i32, t.month() as u32, t.day() as u32, t.hour() as u32, t.minute() as u32, t.second() as u32)
        .single()
        .map(|d| d.timestamp_millis())
        .unwrap_or(0)
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
    let meta = meta_load(dir);
    let mut v: Vec<SaveCopy> = rd
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("zip")).unwrap_or(false))
        .filter_map(|e| {
            let m = read_manifest(&e.path()).ok()?;
            let mt = meta.get(&file_key(&e.path())).cloned().unwrap_or_default();
            Some(SaveCopy { file: e.path().to_string_lossy().into_owned(), created_ms: m.created_ms, kind: m.kind, size: e.metadata().map(|md| md.len()).unwrap_or(0), files: m.files, note: mt.note, pinned: mt.pinned })
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
            // La fecha del archivo original queda en el .zip (para compararla al restaurar).
            let o = match zip_time(it.mtime) {
                Some(t) => opts.last_modified_time(t),
                None => opts,
            };
            z.start_file(it.zip_name.as_str(), o)?;
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
    Ok(Some(SaveCopy { file: dest.to_string_lossy().into_owned(), created_ms: man.created_ms, kind: man.kind, size, files: man.files, note: String::new(), pinned: false }))
}

/// Borra las copias automaticas mas antiguas por encima de `keep` (las fijadas no cuentan ni se borran). Devuelve cuantas borro.
pub fn prune(dir: &Path, keep: usize) -> usize {
    list(dir).into_iter().filter(|c| c.kind == CopyKind::Auto && !c.pinned).skip(keep).filter(|c| fs::remove_file(&c.file).is_ok()).count()
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
    restore_some(zip, dir, now_ms, stamp, None)
}

/// Como `restore`, pero solo un archivo de la copia (`only` = su nombre en el .zip, como `e0/slot1.sav`). Tambien
/// se guarda antes lo que hay.
pub fn restore_one(zip: &Path, dir: &Path, now_ms: i64, stamp: &str, only: &str) -> Result<RestoreReport> {
    restore_some(zip, dir, now_ms, stamp, Some(only))
}

fn restore_some(zip: &Path, dir: &Path, now_ms: i64, stamp: &str, only: Option<&str>) -> Result<RestoreReport> {
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
    if let Some(o) = only {
        if z.by_name(o).is_err() {
            bail!("ese archivo no está en la copia");
        }
    }
    let r = (|| -> Result<()> {
        for k in 0..z.len() {
            let mut f = z.by_index(k)?;
            if f.is_dir() {
                continue;
            }
            let name = f.name().to_string();
            if name == COPY_JSON || only.map(|o| o != name).unwrap_or(false) {
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

/// Estado de un archivo de la copia frente a lo que hay ahora.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileState {
    Same,
    Changed,
    /// Esta en la copia y ahora no existe.
    OnlyCopy,
    /// Existe ahora y no esta en la copia.
    NewNow,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopyFile {
    /// Nombre dentro del .zip (`e0/slot1.sav`, `e1.reg`): identifica el archivo para restaurarlo.
    pub entry: String,
    /// Ubicacion (indice en `CopyFiles::locs`).
    pub loc: usize,
    /// Ruta relativa a la ubicacion.
    pub name: String,
    /// Tamano y fecha en la copia (0 = no estan).
    pub size: u64,
    pub copy_ms: i64,
    /// Tamano y fecha ahora (0 = no existe).
    pub now_size: u64,
    pub now_ms: i64,
    pub state: FileState,
    pub registry: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopyLoc {
    pub path: String,
    pub kind: LocKind,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopyFiles {
    pub locs: Vec<CopyLoc>,
    pub files: Vec<CopyFile>,
}

/// Lee hasta llenar `buf` o hasta el final. Devuelve los bytes leidos.
fn fill(r: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..])? {
            0 => break,
            k => n += k,
        }
    }
    Ok(n)
}

/// ¿El contenido de `a` es igual al del archivo `p`?
fn same_content(a: &mut impl Read, p: &Path) -> bool {
    let Ok(mut b) = File::open(p) else { return false };
    let (mut x, mut y) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
    loop {
        let (Ok(n), Ok(m)) = (fill(a, &mut x), fill(&mut b, &mut y)) else { return false };
        if n != m || x[..n] != y[..m] {
            return false;
        }
        if n == 0 {
            return true;
        }
    }
}

/// Archivos que hay ahora en una ubicacion de la copia: (ruta relativa al ancla, tamano, fecha).
fn current_files(e: &CopyEntry) -> Vec<(String, u64, i64)> {
    let mut out = Vec::new();
    for m in glob::expand(&e.path, MATCH_CAP) {
        for w in WalkDir::new(&m).into_iter().flatten().filter(|w| w.file_type().is_file()) {
            let Ok(rel) = w.path().strip_prefix(&e.anchor) else { continue };
            let Ok(md) = w.metadata() else { continue };
            out.push((rel.to_string_lossy().replace('\\', "/"), md.len(), crate::scan::mtime_ms(&md)));
        }
    }
    out
}

/// Compara los archivos de una copia con los de ahora: tamano y, si coincide, contenido.
pub fn compare(zip: &Path) -> Result<CopyFiles> {
    let man = read_manifest(zip)?;
    let mut z = zip::ZipArchive::new(File::open(zip)?)?;
    let tmp = zip.parent().unwrap_or(Path::new(".")).join(format!(".tmp-comparar-{}", std::process::id()));
    let mut files: Vec<CopyFile> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let r = (|| -> Result<()> {
        for k in 0..z.len() {
            let mut f = z.by_index(k)?;
            if f.is_dir() || f.name() == COPY_JSON {
                continue;
            }
            let entry = f.name().to_string();
            let (head, rest) = entry.split_once('/').unwrap_or((entry.as_str(), ""));
            let (size, copy_ms) = (f.size(), entry_ms(f.last_modified()));
            if rest.is_empty() {
                // Clave del registro: se exporta la de ahora y se compara el texto.
                let Some(i) = head.strip_prefix('e').and_then(|s| s.strip_suffix(".reg")).and_then(|s| s.parse::<usize>().ok()) else { continue };
                let Some(e) = man.entries.get(i).filter(|e| e.kind == LocKind::Registry) else { continue };
                let state = if !registry::exists(&e.path) {
                    FileState::OnlyCopy
                } else {
                    fs::create_dir_all(&tmp)?;
                    let p = tmp.join(&entry);
                    if registry::export(&e.path, &p).is_ok() && same_content(&mut f, &p) {
                        FileState::Same
                    } else {
                        FileState::Changed
                    }
                };
                let now_size = fs::metadata(tmp.join(&entry)).map(|m| m.len()).unwrap_or(0);
                files.push(CopyFile { entry, loc: i, name: "Clave del registro".into(), size, copy_ms, now_size, now_ms: 0, state, registry: true });
                continue;
            }
            let Some(i) = head.strip_prefix('e').and_then(|s| s.parse::<usize>().ok()) else { continue };
            let Some(e) = man.entries.get(i).filter(|e| e.kind == LocKind::Path) else { continue };
            let Some(rel) = safe_rel(rest) else { continue };
            let p = Path::new(&e.anchor).join(rel);
            let (state, now_size, now_ms) = match fs::metadata(&p) {
                Ok(md) if md.is_file() => {
                    let st = if md.len() == size && same_content(&mut f, &p) { FileState::Same } else { FileState::Changed };
                    (st, md.len(), crate::scan::mtime_ms(&md))
                }
                _ => (FileState::OnlyCopy, 0, 0),
            };
            seen.insert(entry.clone());
            let name = rest.to_string();
            files.push(CopyFile { entry, loc: i, name, size, copy_ms, now_size, now_ms, state, registry: false });
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&tmp);
    r?;
    // Lo que hay ahora en esas ubicaciones y la copia no tiene.
    for (i, e) in man.entries.iter().enumerate().filter(|(_, e)| e.kind == LocKind::Path) {
        for (rel, sz, ms) in current_files(e) {
            let entry = format!("e{i}/{rel}");
            if !seen.contains(&entry) {
                files.push(CopyFile { entry, loc: i, name: rel, size: 0, copy_ms: 0, now_size: sz, now_ms: ms, state: FileState::NewNow, registry: false });
            }
        }
    }
    files.sort_by(|a, b| a.loc.cmp(&b.loc).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(CopyFiles { locs: man.entries.iter().map(|e| CopyLoc { path: e.path.clone(), kind: e.kind }).collect(), files })
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
        // Comparar con lo de ahora: todo igual tras restaurar, salvo lo que se toque.
        let cmp = compare(Path::new(&c1.file)).unwrap();
        assert!(cmp.files.iter().all(|f| f.state == FileState::Same));
        put(&game.join("save/slot1.sav"), "otra-vez");
        put(&game.join("save/nuevo.sav"), "n");
        fs::remove_file(game.join("Save01.rvdata2")).unwrap();
        let st = |n: &str| compare(Path::new(&c1.file)).unwrap().files.into_iter().find(|f| f.name == n).unwrap();
        assert_eq!(st("slot1.sav").state, FileState::Changed);
        assert_eq!(st("nuevo.sav").state, FileState::NewNow);
        assert_eq!(st("Save01.rvdata2").state, FileState::OnlyCopy);
        // Restaurar un solo archivo.
        let one = restore_one(Path::new(&c1.file), &out, 5500, "2026-10-19_120006", "e0/slot1.sav").unwrap();
        assert_eq!(one.files, 1);
        assert_eq!(fs::read_to_string(game.join("save/slot1.sav")).unwrap(), "uno");
        assert!(!game.join("Save01.rvdata2").exists());
        // Notas y fijado.
        set_meta(&out, Path::new(&c2.file), Some(" mi nota "), Some(true)).unwrap();
        let l = list(&out);
        let m = l.iter().find(|c| c.file == c2.file).unwrap();
        assert_eq!((m.note.as_str(), m.pinned), ("mi nota", true));
        // Podar: solo las automaticas que no estan fijadas.
        assert_eq!(prune(&out, 0), 0);
        set_meta(&out, Path::new(&c2.file), Some(""), Some(false)).unwrap();
        assert_eq!(prune(&out, 0), 1);
        assert!(!Path::new(&c2.file).exists());
        delete_copy(&out, Path::new(&c1.file)).unwrap();
        assert!(!Path::new(&c1.file).exists());
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
