//! Importar capturas y grafismos a la biblioteca (copiando, nunca moviendo ni modificando los originales),
//! mover capturas entre juegos dentro de la raiz. Todo con rutas del usuario, asi
//! que cada operacion comprueba que el destino queda dentro de la raiz de la biblioteca.
use crate::art::{find_art, ArtKind};
use crate::capture::unique_path;
use crate::dates::parse_date_from_filename;
use crate::games::resolve_folder;
use crate::scan::is_image;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime};
use serde::Serialize;
use std::fs;
use std::io::{self, BufReader};
use std::path::{Component, Path, PathBuf};

/* ------------------------------------------------------------------ */
/* Rutas dentro de la raiz                                              */
/* ------------------------------------------------------------------ */

/// `p` esta dentro de `root` (y no es la propia raiz). Se rechazan `..`, y se resuelven los enlaces
/// simbolicos del tramo que ya existe para que un enlace no pueda sacar la ruta de la biblioteca.
pub fn is_within_root(root: &Path, p: &Path) -> bool {
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    let Ok(root_c) = root.canonicalize() else { return false };
    let mut existing = p.to_path_buf();
    let mut tail = Vec::new();
    while !existing.exists() {
        match existing.file_name() {
            Some(n) => tail.push(n.to_os_string()),
            None => return false,
        }
        if !existing.pop() {
            return false;
        }
    }
    let Ok(mut full) = existing.canonicalize() else { return false };
    for t in tail.iter().rev() {
        full.push(t);
    }
    full != root_c && full.starts_with(&root_c)
}

/// Carpeta de un titulo: `Raiz/<Plataforma>/<Carpeta>`, reutilizando las que ya existan aunque cambien
/// las mayusculas o los signos.
pub fn title_dir(root: &Path, platform_folder: &str, game_folder: &str) -> PathBuf {
    let plat = resolve_folder(root, platform_folder);
    let folder = resolve_folder(&root.join(&plat), game_folder);
    root.join(plat).join(folder)
}

/* ------------------------------------------------------------------ */
/* Fecha de una captura importada                                       */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DateSource {
    Exif,
    Filename,
    Mtime,
}

fn exif_date(path: &Path) -> Option<NaiveDateTime> {
    let f = fs::File::open(path).ok()?;
    let ex = exif::Reader::new().read_from_container(&mut BufReader::new(f)).ok()?;
    let field = ex.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)?;
    if let exif::Value::Ascii(ref v) = field.value {
        let dt = exif::DateTime::from_ascii(v.first()?).ok()?;
        return NaiveDate::from_ymd_opt(dt.year as i32, dt.month as u32, dt.day as u32)?
            .and_hms_opt(dt.hour as u32, dt.minute as u32, dt.second as u32);
    }
    None
}

/// Fecha de una captura: EXIF `DateTimeOriginal`; si no hay, la del nombre del archivo; si no, la de modificacion
/// (en hora local).
pub fn capture_date(path: &Path) -> (NaiveDateTime, DateSource) {
    if let Some(d) = exif_date(path) {
        return (d, DateSource::Exif);
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if let Some(ms) = parse_date_from_filename(&name) {
        if let Some(d) = DateTime::from_timestamp_millis(ms) {
            return (d.naive_utc(), DateSource::Filename);
        }
    }
    let m = fs::metadata(path).and_then(|m| m.modified()).ok();
    let d = m.map(|t| DateTime::<Local>::from(t).naive_local()).unwrap_or_else(|| Local::now().naive_local());
    (d, DateSource::Mtime)
}

/// `Carpeta_AAAA-MM-DD_HH-MM-SS.ext`
pub fn import_filename(folder: &str, at: NaiveDateTime, ext: &str) -> String {
    format!("{folder}_{}.{}", at.format("%Y-%m-%d_%H-%M-%S"), ext.to_ascii_lowercase())
}

/* ------------------------------------------------------------------ */
/* Importar capturas                                                    */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Rutas de las copias creadas.
    pub copied: Vec<String>,
    /// Motivos de lo que se ha saltado o ha fallado (uno por archivo).
    pub skipped: Vec<String>,
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn copy_new(src: &Path, dest: &Path) -> io::Result<()> {
    let mut input = fs::File::open(src)?;
    let mut out = fs::OpenOptions::new().write(true).create_new(true).open(dest)?; // jamas sobrescribe
    io::copy(&mut input, &mut out)?;
    Ok(())
}

/// Copia imagenes a la carpeta de un titulo con el nombre `Carpeta_AAAA-MM-DD_HH-MM-SS.ext`.
/// Los originales no se tocan. `progress(hechas, total)` se llama tras cada archivo.
pub fn import_shots(root: &Path, dir: &Path, sources: &[PathBuf], mut progress: impl FnMut(usize, usize)) -> Result<ImportReport, String> {
    if !is_within_root(root, dir) {
        return Err("Esa carpeta esta fuera de la biblioteca".into());
    }
    fs::create_dir_all(dir).map_err(|e| format!("No se pudo crear la carpeta del juego: {e}"))?;
    let folder = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Captura".into());
    let mut rep = ImportReport::default();
    for (i, src) in sources.iter().enumerate() {
        let label = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !src.is_file() {
            rep.skipped.push(format!("{label}: no existe"));
        } else if !is_image(src) {
            rep.skipped.push(format!("{label}: no es png, jpg, jpeg ni webp"));
        } else if src.parent().map(|p| same_file(p, dir)).unwrap_or(false) {
            rep.skipped.push(format!("{label}: ya esta en la carpeta del juego"));
        } else {
            let (at, _) = capture_date(src);
            let ext = src.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "png".into());
            // El nombre libre se reserva y se copia con create_new: si otro proceso lo ocupa entre medias, se reintenta.
            let mut done = false;
            for _ in 0..8 {
                let dest = unique_path(dir, &import_filename(&folder, at, &ext));
                match copy_new(src, &dest) {
                    Ok(()) => {
                        rep.copied.push(dest.to_string_lossy().into_owned());
                        done = true;
                        break;
                    }
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(e) => {
                        rep.skipped.push(format!("{label}: {e}"));
                        done = true;
                        break;
                    }
                }
            }
            if !done {
                rep.skipped.push(format!("{label}: no se encontro un nombre libre"));
            }
        }
        progress(i + 1, sources.len());
    }
    Ok(rep)
}

/* ------------------------------------------------------------------ */
/* Mover capturas dentro de la biblioteca                               */
/* ------------------------------------------------------------------ */

/// Mueve una captura a la carpeta de otro titulo. Origen y destino deben estar dentro de la raiz;
/// no sobrescribe (anade `_2`, `_3`...). Devuelve la nueva ruta.
pub fn move_shot(root: &Path, src: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    if !is_within_root(root, src) || !is_within_root(root, dest_dir) {
        return Err("Solo se pueden mover archivos dentro de la biblioteca".into());
    }
    if !src.is_file() || !is_image(src) {
        return Err("No es una captura".into());
    }
    if src.parent().map(|p| same_file(p, dest_dir)).unwrap_or(false) {
        return Err("Ya esta en ese juego".into());
    }
    fs::create_dir_all(dest_dir).map_err(|e| format!("No se pudo crear la carpeta destino: {e}"))?;
    let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("Nombre no valido")?;
    let dest = unique_path(dest_dir, &name);
    if fs::rename(src, &dest).is_err() {
        // Otra unidad: copiar y, solo si salio bien, quitar el original.
        copy_new(src, &dest).map_err(|e| format!("No se pudo mover: {e}"))?;
        if let Err(e) = fs::remove_file(src) {
            let _ = fs::remove_file(&dest);
            return Err(format!("No se pudo quitar el original: {e}"));
        }
    }
    Ok(dest)
}

/* ------------------------------------------------------------------ */
/* Grafismos                                                            */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, PartialEq)]
pub enum ArtCopy {
    Copied(PathBuf),
    /// Ya habia un grafismo de ese tipo en la carpeta: no se sobrescribe.
    Exists,
}

/// Copia un grafismo a la carpeta del titulo con el nombre estandar (`hero.ext`, `logo.png|webp`, `cover.ext`).
pub fn copy_art(dir: &Path, kind: ArtKind, src: &Path) -> Result<ArtCopy, String> {
    let ext = src.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let (stem, ok): (&str, &[&str]) = match kind {
        ArtKind::Hero => ("hero", &["png", "jpg", "jpeg", "webp"]),
        ArtKind::Cover => ("cover", &["jpg", "jpeg", "png", "webp"]),
        ArtKind::Logo => ("logo", &["png", "webp"]),
    };
    if !ok.contains(&ext.as_str()) {
        return Err(format!("El {stem} debe ser {}", ok.join(", ")));
    }
    if !src.is_file() {
        return Err("La imagen no existe".into());
    }
    fs::create_dir_all(dir).map_err(|e| format!("No se pudo crear la carpeta del juego: {e}"))?;
    let present = find_art(dir);
    let has = match kind {
        ArtKind::Hero => present.hero.is_some(),
        ArtKind::Cover => present.cover.is_some(),
        ArtKind::Logo => present.logo.is_some(),
    };
    if has {
        return Ok(ArtCopy::Exists);
    }
    let dest = dir.join(format!("{stem}.{ext}"));
    copy_new(src, &dest).map_err(|e| format!("No se pudo copiar: {e}"))?;
    Ok(ArtCopy::Copied(dest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::{Duration, SystemTime};

    /// JPEG minimo con un bloque EXIF que lleva `DateTimeOriginal`.
    fn jpeg_with_exif(date: &str) -> Vec<u8> {
        assert_eq!(date.len(), 19);
        let mut jpg = Vec::new();
        image::DynamicImage::ImageRgb8(image::RgbImage::new(8, 8)).write_to(&mut Cursor::new(&mut jpg), image::ImageFormat::Jpeg).unwrap();
        let mut tiff: Vec<u8> = vec![0x49, 0x49, 0x2A, 0x00, 8, 0, 0, 0];
        tiff.extend([1, 0]); // IFD0: 1 entrada
        tiff.extend([0x69, 0x87, 4, 0, 1, 0, 0, 0, 26, 0, 0, 0]); // puntero al IFD Exif
        tiff.extend([0, 0, 0, 0]); // sin IFD siguiente
        tiff.extend([1, 0]); // IFD Exif: 1 entrada (en el offset 26)
        tiff.extend([0x03, 0x90, 2, 0, 20, 0, 0, 0, 44, 0, 0, 0]); // DateTimeOriginal, ASCII x20 en el offset 44
        tiff.extend([0, 0, 0, 0]);
        tiff.extend(date.bytes());
        tiff.push(0);
        let mut app1 = vec![0xFF, 0xE1];
        let len = (2 + 6 + tiff.len()) as u16;
        app1.extend(len.to_be_bytes());
        app1.extend(b"Exif\0\0");
        app1.extend(tiff);
        let mut out = jpg[..2].to_vec();
        out.extend(app1);
        out.extend(&jpg[2..]);
        out
    }

    fn png(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbImage::new(6, 6).save(path).unwrap();
    }

    fn naive(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, mo, d).unwrap().and_hms_opt(h, mi, s).unwrap()
    }

    #[test]
    fn date_prefers_exif_then_filename_then_mtime() {
        let d = tempfile::tempdir().unwrap();
        // EXIF gana al nombre.
        let a = d.path().join("Foto 2020-01-02 03-04-05.jpg");
        fs::write(&a, jpeg_with_exif("2024:03:09 21:05:44")).unwrap();
        assert_eq!(capture_date(&a), (naive(2024, 3, 9, 21, 5, 44), DateSource::Exif));
        // Sin EXIF: el nombre.
        let b = d.path().join("Elden Ring 2023-05-06 07-08-09.png");
        png(&b);
        assert_eq!(capture_date(&b), (naive(2023, 5, 6, 7, 8, 9), DateSource::Filename));
        let steam = d.path().join("20240115123456_1.png");
        png(&steam);
        assert_eq!(capture_date(&steam), (naive(2024, 1, 15, 12, 34, 56), DateSource::Filename));
        // Sin nada: la fecha de modificacion, en hora local.
        let c = d.path().join("sin fecha.png");
        png(&c);
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        fs::File::options().write(true).open(&c).unwrap().set_modified(t).unwrap();
        let (got, src) = capture_date(&c);
        assert_eq!(src, DateSource::Mtime);
        assert_eq!(got, DateTime::<Local>::from(t).naive_local());
        // Un JPEG sin EXIF cae al nombre; un EXIF roto tambien.
        let e = d.path().join("20220202020202.jpg");
        let mut j = Vec::new();
        image::DynamicImage::ImageRgb8(image::RgbImage::new(4, 4)).write_to(&mut Cursor::new(&mut j), image::ImageFormat::Jpeg).unwrap();
        fs::write(&e, j).unwrap();
        assert_eq!(capture_date(&e).1, DateSource::Filename);
    }

    #[test]
    fn import_names_copies_and_never_overwrites_or_touches_originals() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Capturas");
        let dir = root.join("PC/Hades II");
        let src = d.path().join("Descargas");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&src).unwrap();
        let a = src.join("IMG_1.jpg");
        fs::write(&a, jpeg_with_exif("2024:03:09 21:05:44")).unwrap();
        let b = src.join("Pantalla 2023-05-06 07-08-09.PNG");
        png(&b);
        let c = src.join("Pantalla 2023-05-06 07-08-09 (otra).png"); // misma fecha: _2
        png(&c);
        let note = src.join("notas.txt");
        fs::write(&note, "x").unwrap();
        let before = fs::read(&a).unwrap();
        let mut ticks = Vec::new();
        let rep = import_shots(&root, &dir, &[a.clone(), b.clone(), c.clone(), note, src.join("falta.png")], |h, t| ticks.push((h, t))).unwrap();
        let names: Vec<String> = rep.copied.iter().map(|p| Path::new(p).file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["Hades II_2024-03-09_21-05-44.jpg", "Hades II_2023-05-06_07-08-09.png", "Hades II_2023-05-06_07-08-09_2.png"]);
        assert_eq!(rep.skipped.len(), 2, "{:?}", rep.skipped);
        assert_eq!(ticks.last(), Some(&(5, 5)));
        assert!(a.exists() && b.exists() && c.exists(), "los originales siguen donde estaban");
        assert_eq!(fs::read(&a).unwrap(), before);
        // Importar otra vez el mismo archivo no sobrescribe: crea otra copia con sufijo.
        let rep2 = import_shots(&root, &dir, &[a.clone()], |_, _| {}).unwrap();
        assert!(rep2.copied[0].ends_with("Hades II_2024-03-09_21-05-44_2.jpg"));
        // Un archivo que ya esta en la carpeta del juego se salta.
        let inside = PathBuf::from(&rep.copied[0]);
        let rep3 = import_shots(&root, &dir, &[inside], |_, _| {}).unwrap();
        assert!(rep3.copied.is_empty() && rep3.skipped.len() == 1);
        // Las copias se leen como capturas con esa fecha.
        let lib = crate::scan::scan_library(Some(&root), &crate::config::default_platforms(), &[], &crate::games::Stats::new());
        let t = &lib.platforms[0].titles[0];
        assert_eq!(t.shots.len(), 4);
        assert_eq!(t.shots.last().unwrap().date, parse_date_from_filename("2023-05-06 07-08-09").unwrap());
    }

    #[test]
    fn import_refuses_destinations_outside_the_root() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Capturas");
        fs::create_dir_all(&root).unwrap();
        let a = d.path().join("a.png");
        png(&a);
        assert!(import_shots(&root, &d.path().join("fuera"), &[a.clone()], |_, _| {}).is_err());
        assert!(import_shots(&root, &root.join("../fuera"), &[a.clone()], |_, _| {}).is_err());
        assert!(import_shots(&root, &root, &[a], |_, _| {}).is_err(), "la propia raiz no es un juego");
        assert!(!d.path().join("fuera").exists());
    }

    #[test]
    fn move_stays_inside_the_root() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Capturas");
        let a = root.join("PC/A/uno.png");
        let b_dir = root.join("PC/B");
        png(&a);
        fs::create_dir_all(&b_dir).unwrap();
        // Origen fuera de la raiz: rechazado y el archivo sigue ahi.
        let outside = d.path().join("ajeno.png");
        png(&outside);
        assert!(move_shot(&root, &outside, &b_dir).is_err());
        assert!(outside.exists());
        // Destino fuera de la raiz.
        assert!(move_shot(&root, &a, &d.path().join("otro")).is_err());
        assert!(move_shot(&root, &a, &root.join("PC/../../otro")).is_err());
        assert!(a.exists());
        // Mismo juego: no hace nada.
        assert!(move_shot(&root, &a, &root.join("PC/A")).is_err());
        // Un archivo que no es imagen.
        let txt = root.join("PC/A/notas.txt");
        fs::write(&txt, "x").unwrap();
        assert!(move_shot(&root, &txt, &b_dir).is_err());
        // Mover bien, y sin sobrescribir lo que ya hay.
        png(&b_dir.join("uno.png"));
        let new = move_shot(&root, &a, &b_dir).unwrap();
        assert!(new.ends_with("PC/B/uno_2.png") || new.ends_with("PC\\B\\uno_2.png"));
        assert!(!a.exists() && new.exists() && b_dir.join("uno.png").exists());
        // Destino nuevo (juego sin carpeta aun) dentro de la raiz: se crea.
        let c_dir = root.join("Switch/Zelda");
        png(&root.join("PC/A/dos.png"));
        let n2 = move_shot(&root, &root.join("PC/A/dos.png"), &c_dir).unwrap();
        assert!(n2.exists() && c_dir.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_escape_the_root() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Capturas");
        let out = d.path().join("privado");
        fs::create_dir_all(root.join("PC")).unwrap();
        fs::create_dir_all(&out).unwrap();
        std::os::unix::fs::symlink(&out, root.join("PC/enlace")).unwrap();
        assert!(!is_within_root(&root, &root.join("PC/enlace")));
        assert!(!is_within_root(&root, &root.join("PC/enlace/nuevo")));
        let a = root.join("PC/A/uno.png");
        png(&a);
        assert!(move_shot(&root, &a, &root.join("PC/enlace/X")).is_err());
        assert!(a.exists());
        assert!(is_within_root(&root, &root.join("PC/A/uno.png")));
        assert!(is_within_root(&root, &root.join("PC/Nuevo/Juego")), "rutas que aun no existen pero cuelgan de la raiz");
        assert!(!is_within_root(&root, &root));
    }

    #[test]
    fn art_is_copied_with_standard_names_and_never_overwritten() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("PC/Mi Juego");
        let src = d.path().join("descargas");
        fs::create_dir_all(&src).unwrap();
        let h = src.join("banner grande.JPG");
        let l = src.join("mi-logo.png");
        let c = src.join("portada.webp");
        png(&l);
        fs::write(&h, b"hero").unwrap();
        fs::write(&c, b"cover").unwrap();
        assert!(matches!(copy_art(&dir, ArtKind::Hero, &h), Ok(ArtCopy::Copied(p)) if p.ends_with("hero.jpg")));
        assert!(matches!(copy_art(&dir, ArtKind::Logo, &l), Ok(ArtCopy::Copied(p)) if p.ends_with("logo.png")));
        assert!(matches!(copy_art(&dir, ArtKind::Cover, &c), Ok(ArtCopy::Copied(p)) if p.ends_with("cover.webp")));
        // Ya existia: se conserva y se avisa.
        assert_eq!(copy_art(&dir, ArtKind::Hero, &h), Ok(ArtCopy::Exists));
        assert_eq!(fs::read(dir.join("hero.jpg")).unwrap(), b"hero");
        // Un logo en jpg no es valido; los originales siguen intactos.
        assert!(copy_art(&d.path().join("otra"), ArtKind::Logo, &h).is_err());
        assert!(h.exists() && l.exists() && c.exists());
        let a = find_art(&dir);
        assert!(a.hero.is_some() && a.logo.is_some() && a.cover.is_some());
    }

    #[test]
    fn title_dir_reuses_existing_folders() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("pc/ELDEN RING")).unwrap();
        assert_eq!(title_dir(d.path(), "PC", "Elden Ring"), d.path().join("pc/ELDEN RING"));
        assert_eq!(title_dir(d.path(), "PC", "Nuevo"), d.path().join("pc/Nuevo"));
        assert_eq!(title_dir(d.path(), "Switch", "Zelda"), d.path().join("Switch/Zelda"));
    }
}
