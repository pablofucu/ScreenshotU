//! Grafismos descargados de SteamGridDB: se guardan en la cache de la aplicacion (nunca en la carpeta
//! del usuario), una subcarpeta por titulo con `cover-<id>.ext`, `hero-<id>.ext` y `logo-<id>.ext`.
//! El id de la imagen va en el nombre para que cada eleccion tenga una ruta nueva (y las miniaturas
//! y la cache de la interfaz no enseñen la anterior). Solo rellenan huecos: la carpeta del juego gana.
use crate::art::{Art, ArtKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Juego de SteamGridDB asociado a un titulo y si ya se intento la descarga automatica.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SgdbEntry {
    /// Id del juego en SteamGridDB (`None` si la busqueda no encontro nada).
    pub game_id: Option<u64>,
    pub game_name: String,
    /// La descarga automatica ya paso por aqui: no se repite en cada arranque.
    pub auto_done: bool,
    /// Tipos de grafismo (`cover,hero,logo`) que SteamGridDB no tiene para este titulo: la busqueda
    /// automatica ya los intento y no los vuelve a contar como pendientes (el selector manual sigue abierto).
    pub tried: String,
}

impl SgdbEntry {
    /// SteamGridDB ya dijo que no tiene este tipo de grafismo.
    pub fn has_tried(&self, k: ArtKind) -> bool {
        self.tried.split(',').any(|t| t == kind_name(k))
    }

    /// Anota un tipo sin resultado (sin repetirlo).
    pub fn add_tried(&mut self, k: ArtKind) {
        if !self.has_tried(k) {
            if !self.tried.is_empty() {
                self.tried.push(',');
            }
            self.tried.push_str(kind_name(k));
        }
    }
}

/// Id de titulo -> entrada (tabla `sgdb` de la base; antes `sgdb.json`).
pub type SgdbStore = BTreeMap<String, SgdbEntry>;

pub fn kind_name(k: ArtKind) -> &'static str {
    match k {
        ArtKind::Cover => "cover",
        ArtKind::Hero => "hero",
        ArtKind::Logo => "logo",
    }
}

pub fn parse_kind(s: &str) -> Option<ArtKind> {
    match s {
        "cover" => Some(ArtKind::Cover),
        "hero" => Some(ArtKind::Hero),
        "logo" => Some(ArtKind::Logo),
        _ => None,
    }
}

/// Carpeta de un titulo dentro de la cache (el id puede llevar `/` o `:`).
pub fn title_dir(base: &Path, title_id: &str) -> PathBuf {
    let safe: String = title_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect();
    // Varios ids pueden sanearse igual («pc/a b» y «pc/a_b»): se añade un hash corto del original.
    let h = title_id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    base.join(format!("{}-{:08x}", safe.trim_matches('_'), h as u32))
}

fn files_of(dir: &Path, k: ArtKind) -> Vec<PathBuf> {
    let pre = format!("{}-", kind_name(k));
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.file_name().map(|n| n.to_string_lossy().starts_with(&pre)).unwrap_or(false))
        .collect()
}

/// Grafismos de SteamGridDB guardados para un titulo.
pub fn find_cached(base: &Path, title_id: &str) -> Art {
    let dir = title_dir(base, title_id);
    let one = |k| files_of(&dir, k).into_iter().max().map(|p| p.to_string_lossy().into_owned());
    Art { cover: one(ArtKind::Cover), hero: one(ArtKind::Hero), logo: one(ArtKind::Logo) }
}

/// Extension de archivo para un tipo MIME o una URL de SteamGridDB.
pub fn ext_for(mime: &str, url: &str) -> &'static str {
    let m = mime.to_ascii_lowercase();
    let u = url.to_ascii_lowercase();
    if m.contains("png") || u.ends_with(".png") {
        "png"
    } else if m.contains("webp") || u.ends_with(".webp") {
        "webp"
    } else {
        "jpg"
    }
}

/// Guarda un grafismo descargado sustituyendo el anterior del mismo tipo. Devuelve la ruta nueva.
pub fn store_image(base: &Path, title_id: &str, k: ArtKind, image_id: u64, ext: &str, bytes: &[u8]) -> anyhow::Result<PathBuf> {
    let dir = title_dir(base, title_id);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}-{image_id}.{ext}", kind_name(k)));
    let tmp = dir.join(format!("_{}-{image_id}.part", kind_name(k)));
    fs::write(&tmp, bytes)?;
    for old in files_of(&dir, k) {
        if old != path {
            let _ = fs::remove_file(old);
        }
    }
    fs::rename(&tmp, &path)?;
    Ok(path)
}

/// Borra el grafismo de SteamGridDB de un tipo (vuelve a verse el de reserva).
pub fn clear_image(base: &Path, title_id: &str, k: ArtKind) {
    for p in files_of(&title_dir(base, title_id), k) {
        let _ = fs::remove_file(p);
    }
}

/// Codifica un texto para ir en un segmento de ruta de una URL.
pub fn url_segment(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Limpia un nombre de juego para buscarlo: quita marcas (™ ®) y lo que va entre corchetes.
pub fn search_term(name: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in name.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = (depth - 1).max(0),
            '™' | '®' | '©' => {}
            _ if depth == 0 => out.push(if c == '_' { ' ' } else { c }),
            _ => {}
        }
    }
    let t = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() { name.trim().to_string() } else { t }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_replaces_previous_and_find_returns_it() {
        let d = tempfile::tempdir().unwrap();
        let b = d.path();
        assert_eq!(find_cached(b, "pc/hades"), Art::default());
        let p1 = store_image(b, "pc/hades", ArtKind::Cover, 1, "png", b"a").unwrap();
        let p2 = store_image(b, "pc/hades", ArtKind::Cover, 2, "jpg", b"b").unwrap();
        store_image(b, "pc/hades", ArtKind::Logo, 3, "png", b"c").unwrap();
        assert!(!p1.exists() && p2.exists(), "solo queda la ultima portada");
        let a = find_cached(b, "pc/hades");
        assert!(a.cover.unwrap().ends_with("cover-2.jpg"));
        assert!(a.logo.unwrap().ends_with("logo-3.png"));
        assert!(a.hero.is_none());
        clear_image(b, "pc/hades", ArtKind::Cover);
        assert!(find_cached(b, "pc/hades").cover.is_none());
        assert!(find_cached(b, "pc/otro").logo.is_none(), "cada titulo tiene su carpeta");
    }

    #[test]
    fn title_dirs_are_safe_and_distinct() {
        let b = Path::new("/c");
        let a = title_dir(b, "pc/a b");
        let c = title_dir(b, "pc/a_b");
        assert_ne!(a, c);
        assert!(!a.file_name().unwrap().to_string_lossy().contains('/'));
        assert!(title_dir(b, "manual:12").file_name().unwrap().to_string_lossy().starts_with("manual_12-"));
    }

    #[test]
    fn tried_kinds_are_remembered_once() {
        let mut e = SgdbEntry::default();
        assert!(!e.has_tried(ArtKind::Logo));
        e.add_tried(ArtKind::Logo);
        e.add_tried(ArtKind::Logo);
        e.add_tried(ArtKind::Cover);
        assert_eq!(e.tried, "logo,cover");
        assert!(e.has_tried(ArtKind::Cover) && e.has_tried(ArtKind::Logo) && !e.has_tried(ArtKind::Hero));
    }

    #[test]
    fn helpers() {
        assert_eq!(url_segment("Hades II: ¡ya!"), "Hades%20II%3A%20%C2%A1ya%21");
        assert_eq!(search_term("DOOM™ Eternal [v1.2] (GOG)"), "DOOM Eternal");
        assert_eq!(search_term("Final_Fantasy X"), "Final Fantasy X");
        assert_eq!(ext_for("image/webp", "x"), "webp");
        assert_eq!(ext_for("", "https://cdn/x.PNG"), "png");
        assert_eq!(ext_for("image/jpeg", "x.jpg"), "jpg");
        assert_eq!(parse_kind("hero"), Some(ArtKind::Hero));
        let db = crate::db::Db::open_memory().unwrap();
        let mut s = SgdbStore::new();
        s.insert("pc/x".into(), SgdbEntry { game_id: Some(5), game_name: "X".into(), auto_done: true, tried: "logo".into() });
        s.insert("pc/y".into(), SgdbEntry { game_id: None, game_name: String::new(), auto_done: false, tried: String::new() });
        for (k, e) in &s {
            db.with(|c| crate::db::data::sgdb_put(c, k, e)).unwrap();
        }
        assert_eq!(db.with(crate::db::data::sgdb_load).unwrap(), s);
        // Actualizar una entrada la reemplaza.
        let e = SgdbEntry { game_id: Some(9), game_name: "Y".into(), auto_done: true, tried: "cover,hero".into() };
        db.with(|c| crate::db::data::sgdb_put(c, "pc/y", &e)).unwrap();
        assert_eq!(db.with(crate::db::data::sgdb_load).unwrap()["pc/y"], e);
    }
}
