//! Busca juegos dentro de un directorio con muchas subcarpetas (una por juego) para el asistente de
//! «Anadir carpeta de juegos». Solo lee: no modifica nada en disco.
use crate::games::{norm_key, ManualGame};
use serde::Serialize;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Profundidad maxima (desde la subcarpeta del juego) a la que se buscan ejecutables.
const MAX_DEPTH: usize = 4;
/// Ejecutables propuestos por carpeta como maximo.
const MAX_EXES: usize = 40;

const SKIP_STEMS: &[&str] = &[
    "unins", "uninstall", "setup", "install", "vcredist", "vc_redist", "dxsetup", "dotnet", "ndp", "oalinst",
    "crashhandler", "crashreport", "crashpad", "unitycrashhandler", "notification_helper", "updater", "launcherupdater",
    "ue4prereq", "dxwebsetup", "physx", "quicksfv", "7z", "bootstrapper", "helper",
];
const SKIP_DIRS: &[&str] = &["redist", "_redist", "_commonredist", "commonredist", "directx", "crashpad", "__installer", "support", "prerequisites"];

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DirCandidate {
    /// Nombre de la subcarpeta.
    pub folder: String,
    /// Nombre propuesto (el de la carpeta).
    pub name: String,
    /// Ejecutables encontrados (rutas completas), el mas probable primero.
    pub exes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DirScan {
    pub candidates: Vec<DirCandidate>,
    /// Subcarpetas sin ningun ejecutable.
    pub empty: usize,
    /// Subcarpetas cuyo ejecutable ya esta en la lista de juegos.
    pub known: usize,
}

fn skipped_exe(stem: &str) -> bool {
    let s = stem.to_lowercase();
    SKIP_STEMS.iter().any(|k| s.starts_with(k))
}

fn same_path(a: &str, b: &str) -> bool {
    a.replace('/', "\\").to_lowercase() == b.replace('/', "\\").to_lowercase()
}

/// Ejecutables de una carpeta de juego, ordenados por probabilidad: el que se llama como la carpeta, luego
/// los menos profundos y mas grandes.
fn exes_of(dir: &Path, folder: &str) -> Vec<String> {
    let want = norm_key(folder);
    let mut found: Vec<(bool, usize, u64, String)> = Vec::new();
    let walker = WalkDir::new(dir).max_depth(MAX_DEPTH).into_iter().filter_entry(|e| {
        !(e.file_type().is_dir() && e.depth() > 0 && SKIP_DIRS.contains(&e.file_name().to_string_lossy().to_lowercase().as_str()))
    });
    for e in walker.flatten() {
        if !e.file_type().is_file() {
            continue;
        }
        let p = e.path();
        let is_exe = p.extension().map(|x| x.eq_ignore_ascii_case("exe")).unwrap_or(false);
        let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if !is_exe || skipped_exe(&stem) {
            continue;
        }
        let k = norm_key(&stem);
        let named = !want.is_empty() && !k.is_empty() && (want == k || want.contains(&k) || k.contains(&want));
        let size = e.metadata().map(|m| m.len()).unwrap_or(0);
        found.push((named, e.depth(), size, p.to_string_lossy().into_owned()));
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(b.2.cmp(&a.2)).then(a.3.cmp(&b.3)));
    found.into_iter().take(MAX_EXES).map(|f| f.3).collect()
}

/// Recorre las subcarpetas directas de `root`. Las que ya tienen un juego anadido (por su ejecutable) se omiten.
pub fn scan_game_dirs(root: &Path, existing: &[ManualGame]) -> DirScan {
    let mut out = DirScan::default();
    let Ok(rd) = fs::read_dir(root) else { return out };
    let mut dirs: Vec<_> = rd.flatten().filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)).collect();
    dirs.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    for d in dirs {
        let folder = d.file_name().to_string_lossy().into_owned();
        if folder.starts_with('.') {
            continue;
        }
        let exes = exes_of(&d.path(), &folder);
        if exes.is_empty() {
            out.empty += 1;
        } else if exes.iter().any(|x| existing.iter().any(|m| !m.exe.is_empty() && same_path(&m.exe, x))) {
            out.known += 1;
        } else {
            out.candidates.push(DirCandidate { name: folder.clone(), folder, exes });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path, n: usize) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, vec![0u8; n]).unwrap();
    }

    #[test]
    fn proposes_exes_skipping_helpers_and_known_games() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        touch(&r.join("Hades II/Ship/Hades2.exe"), 10);
        touch(&r.join("Hades II/unins000.exe"), 99);
        touch(&r.join("Hades II/_redist/vcredist_x64.exe"), 99);
        touch(&r.join("Celeste/Launcher.exe"), 500);
        touch(&r.join("Celeste/Celeste.exe"), 5);
        touch(&r.join("Vacia/readme.txt"), 1);
        touch(&r.join("Conocido/c.exe"), 1);
        let known = ManualGame { exe: r.join("Conocido/c.exe").to_string_lossy().into_owned(), ..Default::default() };
        let s = scan_game_dirs(r, &[known]);
        assert_eq!((s.empty, s.known), (1, 1));
        assert_eq!(s.candidates.iter().map(|c| c.folder.as_str()).collect::<Vec<_>>(), vec!["Celeste", "Hades II"]);
        assert!(s.candidates[0].exes[0].ends_with("Celeste.exe"), "el que se llama como la carpeta va primero");
        assert_eq!(s.candidates[1].exes.len(), 1);
        assert!(s.candidates[1].exes[0].ends_with("Hades2.exe"));
        assert!(scan_game_dirs(Path::new("/no/existe"), &[]).candidates.is_empty());
    }
}
