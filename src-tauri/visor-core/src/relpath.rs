//! Rutas portables de la biblioteca: relativas a la carpeta raiz y con `/` como separador
//! (`PC/Elden Ring/2024-03-09 21-05-44.png`). Son las que se guardan en la base para que los datos
//! sigan valiendo si la raiz cambia de sitio o se usan desde otro equipo; la interfaz sigue
//! trabajando con rutas absolutas y la conversion se hace en el borde.
use std::path::{Component, Path, PathBuf};

fn same_component(a: Component, b: Component) -> bool {
    a.as_os_str().to_string_lossy().to_lowercase() == b.as_os_str().to_string_lossy().to_lowercase()
}

/// Ruta portable de `abs` respecto a `root`, o `None` si no esta dentro (ni es la propia raiz).
/// Se compara sin distinguir mayusculas y se rechazan `.` y `..`.
pub fn to_rel(root: &Path, abs: &str) -> Option<String> {
    let mut rest = Path::new(abs).components();
    for rc in root.components() {
        if !same_component(rc, rest.next()?) {
            return None;
        }
    }
    let parts = rest
        .map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Ruta absoluta de una ruta portable, con los separadores del sistema.
pub fn to_abs(root: &Path, rel: &str) -> String {
    root.join(rel.split('/').collect::<PathBuf>()).to_string_lossy().into_owned()
}

/// `s` es una ruta portable (no absoluta).
pub fn is_rel(s: &str) -> bool {
    let p = Path::new(s);
    !(p.has_root() || p.is_absolute() || s.contains(':'))
}

/// Forma que se guarda: la portable si esta dentro de la raiz; si no (o no hay raiz), la absoluta tal cual.
pub fn stored(root: Option<&Path>, abs: &str) -> String {
    root.and_then(|r| to_rel(r, abs)).unwrap_or_else(|| abs.to_string())
}

/// Ruta absoluta de una ruta guardada: las portables se resuelven contra la raiz; las absolutas no cambian.
pub fn resolved(root: Option<&Path>, s: &str) -> String {
    match root {
        Some(r) if is_rel(s) => to_abs(r, s),
        _ => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_inside_root() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Capturas");
        let abs = root.join("PC").join("Elden Ring").join("a b.png").to_string_lossy().into_owned();
        let rel = to_rel(&root, &abs).unwrap();
        assert_eq!(rel, "PC/Elden Ring/a b.png");
        assert!(is_rel(&rel));
        assert_eq!(to_abs(&root, &rel), abs);
        assert_eq!(resolved(Some(&root), &stored(Some(&root), &abs)), abs);
    }

    #[test]
    fn outside_root_stays_absolute() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Capturas");
        let other = d.path().join("Otra").join("x.png").to_string_lossy().into_owned();
        assert_eq!(to_rel(&root, &other), None);
        assert_eq!(to_rel(&root, &root.to_string_lossy()), None, "la propia raiz no es una captura");
        assert_eq!(to_rel(&root, &root.join("..").join("x.png").to_string_lossy()), None);
        assert_eq!(stored(Some(&root), &other), other);
        assert_eq!(stored(None, &other), other);
        assert!(!is_rel(&other));
        assert_eq!(resolved(Some(&root), &other), other);
    }

    #[test]
    fn prefix_is_case_insensitive_and_trailing_separator_ok() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Capturas");
        let abs = d.path().join("CAPTURAS").join("Switch").join("z.jpg").to_string_lossy().into_owned();
        assert_eq!(to_rel(&root, &abs).as_deref(), Some("Switch/z.jpg"));
        let with_sep = PathBuf::from(format!("{}{}", root.display(), std::path::MAIN_SEPARATOR));
        assert_eq!(to_rel(&with_sep, &abs).as_deref(), Some("Switch/z.jpg"));
        assert!(!to_rel(&d.path().join("Captura"), &abs).is_some(), "un prefijo de nombre no basta");
    }

    #[test]
    #[cfg(windows)]
    fn windows_paths() {
        let root = Path::new(r"D:\Capturas");
        assert_eq!(to_rel(root, r"d:\capturas\PC\Hades II\1.png").as_deref(), Some("PC/Hades II/1.png"));
        assert_eq!(to_abs(root, "PC/Hades II/1.png"), r"D:\Capturas\PC\Hades II\1.png");
        assert!(!is_rel(r"D:\x.png") && !is_rel(r"\\nas\x.png") && is_rel("PC/x.png"));
    }
}
