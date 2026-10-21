//! Grafismos de un titulo (estilo SteamGridDB): `hero`, `logo` y `cover` en la raiz de su carpeta.
//! Se buscan sin distinguir mayusculas; si hay varios formatos gana el orden de preferencia.
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtKind {
    Cover,
    Hero,
    Logo,
}

/// Nombres (sin extension) y extensiones aceptados, en orden de preferencia.
/// `cover.jpg` sigue funcionando como siempre; `grid` y `library_*` son los nombres de SteamGridDB/Steam.
const COVER: (&[&str], &[&str]) = (&["cover", "library_600x900", "grid"], &["jpg", "jpeg", "png", "webp"]);
const HERO: (&[&str], &[&str]) = (&["hero", "library_hero"], &["png", "jpg", "jpeg", "webp"]);
const LOGO: (&[&str], &[&str]) = (&["logo", "library_logo"], &["png", "webp"]);

fn spec(k: ArtKind) -> (&'static [&'static str], &'static [&'static str]) {
    match k {
        ArtKind::Cover => COVER,
        ArtKind::Hero => HERO,
        ArtKind::Logo => LOGO,
    }
}

/// Rango de preferencia de un archivo para un tipo de arte (`None` si no encaja).
fn rank(k: ArtKind, p: &Path) -> Option<(usize, usize)> {
    let (stems, exts) = spec(k);
    let stem = p.file_stem()?.to_string_lossy().to_lowercase();
    let ext = p.extension()?.to_string_lossy().to_lowercase();
    Some((stems.iter().position(|s| *s == stem)?, exts.iter().position(|e| *e == ext)?))
}

/// Si el archivo es un grafismo (y por tanto no una captura).
pub fn art_kind(p: &Path) -> Option<ArtKind> {
    [ArtKind::Cover, ArtKind::Hero, ArtKind::Logo].into_iter().find(|k| rank(*k, p).is_some())
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Art {
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub logo: Option<String>,
}

/// Busca los grafismos en la raiz de la carpeta (sin recorrerla entera).
pub fn find_art(dir: &Path) -> Art {
    let mut best: [Option<((usize, usize), String)>; 3] = [None, None, None];
    let Ok(rd) = fs::read_dir(dir) else { return Art::default() };
    for e in rd.flatten() {
        // `file_type` sale del propio listado (sin tocar el disco); `Path::is_file` hacia una consulta por archivo.
        let p = e.path();
        if !e.file_type().map(|t| t.is_file() || (t.is_symlink() && p.is_file())).unwrap_or(false) {
            continue;
        }
        for (i, k) in [ArtKind::Cover, ArtKind::Hero, ArtKind::Logo].into_iter().enumerate() {
            if let Some(r) = rank(k, &p) {
                let s = p.to_string_lossy().into_owned();
                if best[i].as_ref().map(|(b, _)| r < *b).unwrap_or(true) {
                    best[i] = Some((r, s));
                }
            }
        }
    }
    let [c, h, l] = best;
    Art { cover: c.map(|x| x.1), hero: h.map(|x| x.1), logo: l.map(|x| x.1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(d: &Path, n: &str) {
        fs::write(d.join(n), b"x").unwrap();
    }

    #[test]
    fn finds_standard_names_in_any_case() {
        let d = tempfile::tempdir().unwrap();
        for n in ["Hero.PNG", "LOGO.webp", "Cover.JPG", "2024-01-01.png", "notas.txt"] {
            touch(d.path(), n);
        }
        let a = find_art(d.path());
        assert!(a.hero.unwrap().ends_with("Hero.PNG"));
        assert!(a.logo.unwrap().ends_with("LOGO.webp"));
        assert!(a.cover.unwrap().ends_with("Cover.JPG"));
    }

    #[test]
    fn prefers_name_then_format_and_accepts_steamgriddb_names() {
        let d = tempfile::tempdir().unwrap();
        for n in ["hero.jpg", "hero.png", "library_hero.png", "logo.jpg", "grid.png", "cover.png", "cover.jpeg"] {
            touch(d.path(), n);
        }
        let a = find_art(d.path());
        assert!(a.hero.unwrap().ends_with("hero.png"), "hero.png gana a hero.jpg y a library_hero");
        assert!(a.logo.is_none(), "un logo.jpg no es un logo aceptado");
        assert!(a.cover.unwrap().ends_with("cover.jpeg"), "cover: jpg > jpeg > png, y cover gana a grid");
        let d2 = tempfile::tempdir().unwrap();
        touch(d2.path(), "Grid.png");
        touch(d2.path(), "library_hero.JPG");
        let a2 = find_art(d2.path());
        assert!(a2.cover.unwrap().ends_with("Grid.png") && a2.hero.unwrap().ends_with("library_hero.JPG"));
    }

    #[test]
    fn art_is_not_a_screenshot_and_missing_dir_is_empty() {
        assert_eq!(art_kind(Path::new("/x/HERO.webp")), Some(ArtKind::Hero));
        assert_eq!(art_kind(Path::new("/x/logo.jpg")), None);
        assert_eq!(art_kind(Path::new("/x/captura.png")), None);
        assert_eq!(find_art(Path::new("/no/existe")), Art::default());
    }
}
