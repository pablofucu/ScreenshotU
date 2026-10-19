//! Comodines de las rutas de partidas (`*`, `?` y `**`), como los usa el manifiesto de Ludusavi. Solo lee.
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Profundidad maxima que recorre `**`.
const DEEP: usize = 10;

pub fn has_magic(s: &str) -> bool {
    s.contains(['*', '?'])
}

/// `pat` casa con `name` sin distinguir mayusculas (`*` cualquier tramo, `?` un caracter).
pub fn wild_match(pat: &str, name: &str) -> bool {
    let p: Vec<char> = pat.chars().flat_map(|c| c.to_lowercase()).collect();
    let s: Vec<char> = name.chars().flat_map(|c| c.to_lowercase()).collect();
    let (mut pi, mut si) = (0, 0);
    let (mut star, mut mark) = (None::<usize>, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn start_of(first: &str) -> Option<PathBuf> {
    if first.is_empty() {
        return Some(PathBuf::from("/"));
    }
    if first.len() == 2 && first.ends_with(':') {
        return Some(PathBuf::from(format!("{first}\\")));
    }
    None
}

/// Rutas existentes que casan con `pattern` (absoluta, con `/` o `\`), como mucho `limit`.
pub fn expand(pattern: &str, limit: usize) -> Vec<PathBuf> {
    let norm = pattern.replace('\\', "/");
    let mut comps = norm.split('/');
    let Some(start) = comps.next().and_then(start_of) else { return Vec::new() };
    let mut cur = vec![start];
    for comp in comps {
        if comp.is_empty() || comp == "." {
            continue;
        }
        let mut next: Vec<PathBuf> = Vec::new();
        for p in &cur {
            if comp == "**" {
                for e in WalkDir::new(p).max_depth(DEEP).into_iter().flatten() {
                    if e.file_type().is_dir() {
                        next.push(e.into_path());
                    }
                }
            } else if has_magic(comp) {
                let Ok(rd) = fs::read_dir(p) else { continue };
                let mut hits: Vec<PathBuf> = rd.flatten().filter(|e| wild_match(comp, &e.file_name().to_string_lossy())).map(|e| e.path()).collect();
                hits.sort();
                next.extend(hits);
            } else {
                let q = p.join(comp);
                if q.symlink_metadata().is_ok() {
                    next.push(q);
                }
            }
            if next.len() > limit * 4 {
                break;
            }
        }
        cur = next;
        if cur.is_empty() {
            return cur;
        }
    }
    let mut seen = HashSet::new();
    cur.retain(|p| p.exists() && seen.insert(p.to_string_lossy().to_lowercase()));
    cur.truncate(limit);
    cur
}

/// `path` esta dentro de `dir` o es el mismo (sin mayusculas, `/` o `\`).
pub fn inside_or_same(path: &Path, dir: &Path) -> bool {
    let n = |p: &Path| {
        let mut s = p.to_string_lossy().replace('\\', "/").to_lowercase();
        while s.ends_with('/') {
            s.pop();
        }
        s
    };
    let (p, d) = (n(path), n(dir));
    p == d || p.starts_with(&format!("{d}/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards() {
        assert!(wild_match("*.sav", "Slot1.SAV"));
        assert!(wild_match("Save??.rvdata2", "Save01.rvdata2"));
        assert!(!wild_match("Save??.rvdata2", "Save1.rvdata2"));
        assert!(wild_match("*", ""));
        assert!(wild_match("a*b*c", "aXXbYYc"));
        assert!(!wild_match("a*b", "aXXc"));
    }

    #[test]
    fn expands_stars_and_double_stars() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        for f in ["Saved/SaveGames/a/1.sav", "Saved/SaveGames/2.sav", "Saved/SaveGames/x.txt", "Saved/Config/c.ini"] {
            let p = r.join(f);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"x").unwrap();
        }
        let base = r.to_string_lossy().replace('\\', "/");
        let mut got: Vec<String> = expand(&format!("{base}/Saved/SaveGames/**/*.sav"), 100)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        got.sort();
        assert_eq!(got, vec!["1.sav", "2.sav"]);
        assert_eq!(expand(&format!("{base}/Saved/*/c.ini"), 100).len(), 1);
        assert_eq!(expand(&format!("{base}/Saved"), 100).len(), 1);
        assert!(expand(&format!("{base}/No/*"), 100).is_empty());
        assert!(expand("relativa/a", 100).is_empty());
        assert!(inside_or_same(&r.join("Saved/SaveGames"), &r.join("saved")));
        assert!(!inside_or_same(&r.join("Saved2"), &r.join("Saved")));
    }
}
