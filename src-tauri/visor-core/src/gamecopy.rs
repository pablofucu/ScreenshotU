//! Multi-PC (fase 3b): copiar un juego entero del PC principal a un cliente.
//!
//! El host sirve la lista de archivos de la carpeta del juego (`GET /api/v1/install/list?id=`) y cada archivo
//! (`GET /api/v1/install/file?id=&f=`) con `Range`, para poder reanudar. Solo se sirve esa carpeta.
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Archivos como mucho en una copia (una carpeta con mas no es un juego, o es un error).
pub const MAX_FILES: usize = 300_000;

/// Juego del host que se puede copiar.
#[derive(Debug, Clone)]
pub struct InstallSource {
    /// Carpeta del juego en el host.
    pub dir: PathBuf,
    /// Ejecutable relativo a `dir`.
    pub exe: String,
    pub args: String,
    pub name: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopyFile {
    /// Ruta relativa con `/`.
    pub f: String,
    pub size: u64,
    pub mtime: i64,
}

/// Respuesta de `GET /api/v1/install/list`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CopyList {
    pub id: String,
    pub name: String,
    pub platform: String,
    /// Nombre de la carpeta del juego en el host.
    pub folder: String,
    pub exe: String,
    pub args: String,
    pub total: u64,
    pub files: Vec<CopyFile>,
}

/// Archivos de `dir` (sin enlaces simbolicos), ordenados.
pub fn list_files(dir: &Path) -> Result<Vec<CopyFile>> {
    if !dir.is_dir() {
        bail!("La carpeta del juego no existe: {}", dir.display());
    }
    let mut out = Vec::new();
    for e in walkdir::WalkDir::new(dir).follow_links(false) {
        let e = e?;
        if !e.file_type().is_file() {
            continue;
        }
        let rel = e.path().strip_prefix(dir)?.to_string_lossy().replace('\\', "/");
        let md = e.metadata()?;
        out.push(CopyFile { f: rel, size: md.len(), mtime: crate::scan::mtime_ms(&md) });
        if out.len() > MAX_FILES {
            bail!("La carpeta del juego tiene demasiados archivos");
        }
    }
    out.sort_by(|a, b| a.f.cmp(&b.f));
    Ok(out)
}

/// Lista completa de un juego del host.
pub fn copy_list(id: &str, src: &InstallSource) -> Result<CopyList> {
    let files = list_files(&src.dir)?;
    Ok(CopyList {
        id: id.to_string(),
        name: src.name.clone(),
        platform: src.platform.clone(),
        folder: src.dir.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_else(|| "Juego".into()),
        exe: src.exe.clone(),
        args: src.args.clone(),
        total: files.iter().map(|f| f.size).sum(),
        files,
    })
}

/// Ruta relativa de la lista (`data/a.pak`) sin salir de la carpeta.
pub fn safe_rel(f: &str) -> Option<PathBuf> {
    crate::backup::safe_rel(f)
}

/// Archivo `f` dentro de `dir` (None si se sale de la carpeta o no es un archivo).
pub fn resolve(dir: &Path, f: &str) -> Option<PathBuf> {
    let rel = crate::backup::safe_rel(f)?;
    let p = dir.join(rel);
    let md = fs::symlink_metadata(&p).ok()?;
    if !md.is_file() {
        return None;
    }
    // Sin saltos por enlaces o uniones de carpetas fuera del juego.
    let (root, real) = (fs::canonicalize(dir).ok()?, fs::canonicalize(&p).ok()?);
    real.starts_with(&root).then_some(p)
}

/// `Range: bytes=a-` o `bytes=a-b` sobre un archivo de `len` bytes -> `(inicio, longitud)`. None = no valido.
pub fn parse_range(h: &str, len: u64) -> Option<(u64, u64)> {
    let r = h.trim().strip_prefix("bytes=")?;
    if r.contains(',') {
        return None;
    }
    let (a, b) = r.split_once('-')?;
    let (a, b) = (a.trim(), b.trim());
    if a.is_empty() {
        // Sufijo: los ultimos `b` bytes.
        let n: u64 = b.parse().ok()?;
        if n == 0 || len == 0 {
            return None;
        }
        let n = n.min(len);
        return Some((len - n, n));
    }
    let start: u64 = a.parse().ok()?;
    if start >= len {
        return None;
    }
    let end = if b.is_empty() { len - 1 } else { b.parse::<u64>().ok()?.min(len - 1) };
    if end < start {
        return None;
    }
    Some((start, end - start + 1))
}

/* ------------------------------ cliente ------------------------------ */

fn agent() -> ureq::Agent {
    // Sin limite total (un archivo puede tardar mucho), pero sin quedarse colgado si el host deja de responder.
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(5)).timeout_read(Duration::from_secs(30)).build()
}

fn get(addr: &str, token: &str, version: &str, path: &str) -> ureq::Request {
    agent()
        .get(&format!("http://{}{path}", crate::remote_client::normalize_addr(addr)))
        .set("Authorization", &format!("Bearer {token}"))
        .set("X-Visor-Api", &crate::remote::API_REV.to_string())
        .set("X-Visor-Version", version)
}

fn err_msg(e: ureq::Error) -> anyhow::Error {
    match e {
        ureq::Error::Status(code, r) => {
            let msg = r.into_json::<serde_json::Value>().ok().and_then(|v| v["error"].as_str().map(String::from)).unwrap_or_default();
            if msg.is_empty() {
                anyhow!("El PC principal respondió {code}")
            } else {
                anyhow!(msg)
            }
        }
        ureq::Error::Transport(t) => anyhow!("No se pudo hablar con el PC principal: {t}"),
    }
}

fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Lista de archivos de un juego del host.
pub fn fetch_list(addr: &str, token: &str, version: &str, id: &str) -> Result<CopyList> {
    let r = get(addr, token, version, &format!("/api/v1/install/list?id={}", enc(id))).call().map_err(err_msg)?;
    r.into_json().context("lista de archivos del juego")
}

/// Progreso de una copia.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: u64,
    pub total: u64,
    pub files_done: usize,
    pub files: usize,
}

/// Archivo temporal mientras se descarga (se reanuda desde lo que tenga).
fn part_path(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".visor-part");
    PathBuf::from(s)
}

/// Descarga en `dest` los archivos de `list` que falten o tengan otro tamaño. Reanuda los `.visor-part`.
pub fn download(
    addr: &str,
    token: &str,
    version: &str,
    list: &CopyList,
    dest: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(Progress),
) -> Result<()> {
    let mut pr = Progress { total: list.total, files: list.files.len(), ..Default::default() };
    let mut buf = vec![0u8; 256 * 1024];
    for f in &list.files {
        if cancel.load(Ordering::Relaxed) {
            bail!("Copia cancelada");
        }
        let rel = crate::backup::safe_rel(&f.f).ok_or_else(|| anyhow!("ruta no válida en la lista: {}", f.f))?;
        let target = dest.join(rel);
        if fs::metadata(&target).is_ok_and(|m| m.is_file() && m.len() == f.size) {
            pr.done += f.size;
            pr.files_done += 1;
            progress(pr);
            continue;
        }
        if let Some(d) = target.parent() {
            fs::create_dir_all(d).with_context(|| format!("no se pudo crear {}", d.display()))?;
        }
        let part = part_path(&target);
        let mut have = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        if have > f.size {
            have = 0;
        }
        let base = pr.done;
        if have < f.size || f.size == 0 {
            let path = format!("/api/v1/install/file?id={}&f={}", enc(&list.id), enc(&f.f));
            let mut req = get(addr, token, version, &path);
            if have > 0 {
                req = req.set("Range", &format!("bytes={have}-"));
            }
            let r = req.call().map_err(err_msg)?;
            let mut out = if r.status() == 206 && have > 0 {
                fs::OpenOptions::new().append(true).open(&part)?
            } else {
                have = 0;
                fs::File::create(&part)?
            };
            let mut rd = r.into_reader();
            let mut got = have;
            loop {
                if cancel.load(Ordering::Relaxed) {
                    out.flush()?;
                    bail!("Copia cancelada");
                }
                let n = rd.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                out.write_all(&buf[..n])?;
                got += n as u64;
                pr.done = base + got.min(f.size);
                progress(pr);
            }
            out.flush()?;
            drop(out);
            if got != f.size {
                bail!("{} llegó incompleto ({got} de {} bytes)", f.f, f.size);
            }
        }
        if target.exists() {
            fs::remove_file(&target).with_context(|| format!("no se pudo reemplazar {}", target.display()))?;
        }
        fs::rename(&part, &target)?;
        pr.done = base + f.size;
        pr.files_done += 1;
        progress(pr);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-", 10), Some((0, 10)));
        assert_eq!(parse_range("bytes=4-", 10), Some((4, 6)));
        assert_eq!(parse_range("bytes=2-5", 10), Some((2, 4)));
        assert_eq!(parse_range("bytes=2-50", 10), Some((2, 8)));
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 3)));
        assert_eq!(parse_range("bytes=10-", 10), None);
        assert_eq!(parse_range("bytes=5-2", 10), None);
        assert_eq!(parse_range("bytes=0-1,4-5", 10), None);
        assert_eq!(parse_range("items=0-", 10), None);
    }

    #[test]
    fn lists_and_resolves_inside_only() {
        let t = tempfile::tempdir().unwrap();
        let g = t.path().join("Juego");
        fs::create_dir_all(g.join("data")).unwrap();
        fs::write(g.join("game.exe"), b"exe").unwrap();
        fs::write(g.join("data/a.pak"), b"12345").unwrap();
        fs::write(t.path().join("fuera.txt"), b"x").unwrap();
        let src = InstallSource { dir: g.clone(), exe: "game.exe".into(), args: String::new(), name: "Juego".into(), platform: "pc".into() };
        let l = copy_list("manual:1", &src).unwrap();
        assert_eq!(l.folder, "Juego");
        assert_eq!(l.total, 8);
        assert_eq!(l.files.iter().map(|f| f.f.as_str()).collect::<Vec<_>>(), ["data/a.pak", "game.exe"]);
        assert!(resolve(&g, "data/a.pak").is_some());
        assert!(resolve(&g, "data\\a.pak").is_some());
        assert!(resolve(&g, "../fuera.txt").is_none());
        assert!(resolve(&g, "data").is_none());
        assert!(resolve(&g, "C:/Windows/win.ini").is_none());
    }
}
