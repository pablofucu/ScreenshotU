//! Color de acento de un juego a partir de su hero y su portada.
//!
//! 1. Se reduce cada imagen a ~64 px y se pasa a OKLab.
//! 2. Se descartan casi negros, casi blancos y grises (poca croma).
//! 3. Se agrupan los pixeles con k-means en OKLab; cada pixel pesa por su croma al cuadrado
//!    (saturacion) y por el area (cada imagen aporta lo mismo, sea cual sea su tamano).
//! 4. Se ordenan los grupos por peso y cada uno se «normaliza» al estilo de los siete acentos del
//!    arcoiris: luminosidad OKLCH 0,62-0,72 y croma >= 0,13 (si el gamut sRGB lo permite), conservando el tono.
//! 5. El texto sobre el acento es blanco u oscuro segun el contraste.
//!
//! Todo es puro y se prueba sin Tauri. Los resultados se guardan en una cache por ruta + fecha de modificacion.
use crate::config::write_atomic;
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

/// Lado maximo al que se reduce cada imagen antes de analizarla.
pub const SAMPLE: u32 = 64;
/// Luminosidad OKLCH permitida para un acento.
pub const L_MIN: f32 = 0.62;
pub const L_MAX: f32 = 0.72;
/// Croma minima (si el gamut sRGB lo permite) y maxima.
pub const C_MIN: f32 = 0.13;
pub const C_MAX: f32 = 0.20;
/// Por debajo de este contraste con el blanco el texto sobre el acento pasa a ser oscuro
/// (el amarillo del arcoiris queda por debajo; su verde y su naranja, por encima).
pub const WHITE_MIN_CONTRAST: f32 = 2.55;
const K: usize = 6;
const MAX_SWATCHES: usize = 5;

/// Un color ya normalizado: `#rrggbb` y si el texto encima debe ser oscuro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Swatch {
    pub color: String,
    pub dark: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccentInfo {
    /// El acento elegido (el primer elemento de `palette`).
    pub accent: Swatch,
    /// Hasta 5 muestras (la primera es el acento), tambien normalizadas y con tonos distintos.
    pub palette: Vec<Swatch>,
}

/* ------------------------------------------------------------------ */
/* Espacios de color                                                    */
/* ------------------------------------------------------------------ */

fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}
fn from_linear(c: f32) -> f32 {
    if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

/// sRGB 0..255 -> OKLab (L, a, b).
pub fn srgb_to_oklab(rgb: [u8; 3]) -> [f32; 3] {
    let (r, g, b) = (to_linear(rgb[0] as f32 / 255.0), to_linear(rgb[1] as f32 / 255.0), to_linear(rgb[2] as f32 / 255.0));
    let l = (0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// OKLab -> sRGB lineal sin recortar (puede salirse de 0..1).
fn oklab_to_linear(lab: [f32; 3]) -> [f32; 3] {
    let l = lab[0] + 0.396_337_78 * lab[1] + 0.215_803_76 * lab[2];
    let m = lab[0] - 0.105_561_346 * lab[1] - 0.063_854_17 * lab[2];
    let s = lab[0] - 0.089_484_18 * lab[1] - 1.291_485_5 * lab[2];
    let (l, m, s) = (l * l * l, m * m * m, s * s * s);
    [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s,
        -0.004_196_086 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ]
}

fn in_gamut(lin: [f32; 3]) -> bool {
    lin.iter().all(|c| (-0.000_5..=1.000_5).contains(c))
}

pub fn oklch_to_lab(l: f32, c: f32, h_deg: f32) -> [f32; 3] {
    let h = h_deg.to_radians();
    [l, c * h.cos(), c * h.sin()]
}

/// OKLab -> (L, C, h en grados 0..360).
pub fn lab_to_lch(lab: [f32; 3]) -> (f32, f32, f32) {
    let c = (lab[1] * lab[1] + lab[2] * lab[2]).sqrt();
    let h = lab[2].atan2(lab[1]).to_degrees();
    (lab[0], c, if h < 0.0 { h + 360.0 } else { h })
}

/// Croma maxima dentro de sRGB para una luminosidad y un tono dados (biseccion).
pub fn max_chroma(l: f32, h_deg: f32) -> f32 {
    let (mut lo, mut hi) = (0.0f32, 0.4f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if in_gamut(oklab_to_linear(oklch_to_lab(l, mid, h_deg))) { lo = mid } else { hi = mid }
    }
    lo
}

pub fn lch_to_rgb(l: f32, c: f32, h_deg: f32) -> [u8; 3] {
    let c = c.min(max_chroma(l, h_deg)).max(0.0);
    let lin = oklab_to_linear(oklch_to_lab(l, c, h_deg));
    let q = |v: f32| (from_linear(v.clamp(0.0, 1.0)) * 255.0).round() as u8;
    [q(lin[0]), q(lin[1]), q(lin[2])]
}

pub fn to_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().strip_prefix('#')?;
    if s.len() != 6 || !s.is_ascii() {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// Luminancia relativa (WCAG) de un color sRGB.
pub fn luminance(rgb: [u8; 3]) -> f32 {
    0.2126 * to_linear(rgb[0] as f32 / 255.0) + 0.7152 * to_linear(rgb[1] as f32 / 255.0) + 0.0722 * to_linear(rgb[2] as f32 / 255.0)
}

/// Contraste WCAG del color con el blanco.
pub fn contrast_with_white(rgb: [u8; 3]) -> f32 {
    1.05 / (luminance(rgb) + 0.05)
}

/// Si el texto sobre este color debe ser oscuro (poco contraste con el blanco).
pub fn needs_dark_ink(rgb: [u8; 3]) -> bool {
    contrast_with_white(rgb) < WHITE_MIN_CONTRAST
}

pub fn swatch(rgb: [u8; 3]) -> Swatch {
    Swatch { color: to_hex(rgb), dark: needs_dark_ink(rgb) }
}

/* ------------------------------------------------------------------ */
/* Normalizacion al estilo de los acentos                               */
/* ------------------------------------------------------------------ */

/// Lleva un color OKLCH al «peso» de los acentos del arcoiris: L en [0,62; 0,72], croma >= 0,13
/// (limitada por el gamut sRGB) y <= 0,20, sin cambiar el tono. Si a la luminosidad pedida no cabe la
/// croma minima, se busca la mas cercana dentro del rango que si la admite (o la que mas croma deja).
pub fn normalize_lch(l: f32, c: f32, h: f32) -> [u8; 3] {
    let l0 = l.clamp(L_MIN, L_MAX);
    let want_c = c.clamp(C_MIN, C_MAX);
    let steps: Vec<f32> = (0..=10).map(|i| L_MIN + (L_MAX - L_MIN) * i as f32 / 10.0).collect();
    let mut best: Option<(f32, f32)> = None; // (distancia a l0, L) entre los que admiten la croma
    let mut fallback = (0.0f32, l0);
    for &ls in &steps {
        let mc = max_chroma(ls, h);
        if mc >= want_c - 0.002 {
            let d = (ls - l0).abs();
            if best.map(|b| d < b.0).unwrap_or(true) {
                best = Some((d, ls));
            }
        }
        if mc > fallback.0 {
            fallback = (mc, ls);
        }
    }
    let lf = best.map(|b| b.1).unwrap_or(fallback.1);
    lch_to_rgb(lf, want_c.min(max_chroma(lf, h)), h)
}

/// Normaliza un color sRGB cualquiera (por ejemplo, el elegido a mano) al estilo de los acentos.
pub fn normalize_rgb(rgb: [u8; 3]) -> Swatch {
    let (l, c, h) = lab_to_lch(srgb_to_oklab(rgb));
    swatch(normalize_lch(l, c, h))
}

/// Acento para un tono dado (selector de tono personalizado).
pub fn accent_for_hue(h: f32) -> Swatch {
    swatch(normalize_lch(0.68, 0.16, h.rem_euclid(360.0)))
}

/* ------------------------------------------------------------------ */
/* Extraccion                                                           */
/* ------------------------------------------------------------------ */

#[derive(Clone, Copy)]
struct Px {
    lab: [f32; 3],
    w: f32,
}

fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

/// Pixeles utiles de una imagen (reducida): sin transparentes, casi negros, casi blancos ni grises.
fn useful_pixels(img: &RgbaImage) -> Vec<Px> {
    let m = img.width().max(img.height()).max(1);
    let small = if m > SAMPLE {
        image::imageops::thumbnail(img, (img.width() * SAMPLE / m).max(1), (img.height() * SAMPLE / m).max(1))
    } else {
        img.clone()
    };
    let total = (small.width() * small.height()).max(1) as f32;
    let mut out = Vec::new();
    for p in small.pixels() {
        if p[3] < 128 {
            continue;
        }
        let lab = srgb_to_oklab([p[0], p[1], p[2]]);
        let (l, c, _) = lab_to_lch(lab);
        if !(0.2..=0.93).contains(&l) || c < 0.045 {
            continue;
        }
        out.push(Px { lab, w: c * c / total });
    }
    out
}

/// k-means ponderado en OKLab con arranque determinista (el mas pesado y luego el mas lejano).
fn kmeans(px: &[Px], k: usize) -> Vec<([f32; 3], f32)> {
    let k = k.min(px.len());
    let mut centers: Vec<[f32; 3]> = Vec::with_capacity(k);
    let first = px.iter().max_by(|a, b| a.w.partial_cmp(&b.w).unwrap_or(std::cmp::Ordering::Equal)).map(|p| p.lab);
    let Some(first) = first else { return Vec::new() };
    centers.push(first);
    while centers.len() < k {
        let next = px
            .iter()
            .map(|p| (centers.iter().map(|c| dist2(p.lab, *c)).fold(f32::MAX, f32::min) * p.w.sqrt(), p.lab))
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        match next {
            Some((d, lab)) if d > 1e-9 => centers.push(lab),
            _ => break,
        }
    }
    let mut weight = vec![0.0f32; centers.len()];
    for _ in 0..14 {
        let mut sum = vec![[0.0f32; 3]; centers.len()];
        weight.iter_mut().for_each(|w| *w = 0.0);
        for p in px {
            let i = (0..centers.len()).min_by(|&a, &b| dist2(p.lab, centers[a]).partial_cmp(&dist2(p.lab, centers[b])).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(0);
            for d in 0..3 {
                sum[i][d] += p.lab[d] * p.w;
            }
            weight[i] += p.w;
        }
        for i in 0..centers.len() {
            if weight[i] > 0.0 {
                centers[i] = [sum[i][0] / weight[i], sum[i][1] / weight[i], sum[i][2] / weight[i]];
            }
        }
    }
    centers.into_iter().zip(weight).filter(|(_, w)| *w > 0.0).collect()
}

fn hue_dist(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    d.min(360.0 - d)
}

/// Extrae el acento y la paleta de una o varias imagenes (hero y portada). `None` si no hay ningun
/// pixel con color (todo gris, negro o blanco, o sin imagenes).
pub fn extract_palette(images: &[RgbaImage]) -> Option<AccentInfo> {
    let mut px: Vec<Px> = Vec::new();
    for im in images {
        let mut v = useful_pixels(im);
        // Cada imagen aporta lo mismo aunque una sea mucho mayor que la otra.
        let sum: f32 = v.iter().map(|p| p.w).sum();
        if sum > 0.0 {
            v.iter_mut().for_each(|p| p.w /= sum);
        }
        px.extend(v);
    }
    if px.is_empty() {
        return None;
    }
    // Muy poco color en total (p. ej. una sola mota perdida en una imagen gris): no es fiable.
    if px.len() < 6 {
        return None;
    }
    let mut groups = kmeans(&px, K);
    groups.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut palette: Vec<Swatch> = Vec::new();
    let mut hues: Vec<f32> = Vec::new();
    for (lab, _) in groups {
        let (l, c, h) = lab_to_lch(lab);
        if hues.iter().any(|x| hue_dist(*x, h) < 24.0) {
            continue;
        }
        hues.push(h);
        palette.push(swatch(normalize_lch(l, c, h)));
        if palette.len() == MAX_SWATCHES {
            break;
        }
    }
    let accent = palette.first()?.clone();
    Some(AccentInfo { accent, palette })
}

/// Abre una imagen para analizarla (JPEG/PNG/WebP) ya reducida: no se decodifica mas de lo necesario.
pub fn load_for_palette(path: &Path) -> Option<RgbaImage> {
    let img = image::open(path).ok()?;
    let img = img.thumbnail(SAMPLE, SAMPLE);
    Some(img.to_rgba8())
}

/* ------------------------------------------------------------------ */
/* Cache por ruta + fecha de modificacion                               */
/* ------------------------------------------------------------------ */

const CACHE_VERSION: &str = "v1";

fn mtime_ms(p: &Path) -> Option<i64> {
    let md = fs::metadata(p).ok()?;
    let t = md.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(t.as_millis() as i64)
}

/// Clave de cache: `v1|ruta@mtime|ruta@mtime`. `None` si no hay ninguna imagen legible.
pub fn cache_key(paths: &[&Path]) -> Option<String> {
    let parts: Vec<String> = paths.iter().filter_map(|p| mtime_ms(p).map(|m| format!("{}@{m}", p.display()))).collect();
    if parts.is_empty() {
        return None;
    }
    Some(format!("{CACHE_VERSION}|{}", parts.join("|")))
}

#[derive(Default)]
pub struct PaletteCache {
    map: Mutex<HashMap<String, Option<AccentInfo>>>,
    used: Mutex<HashSet<String>>,
    dirty: std::sync::atomic::AtomicBool,
    /// Cuantas veces se ha calculado de verdad (para las pruebas).
    pub computed: std::sync::atomic::AtomicUsize,
}

impl PaletteCache {
    pub fn load(path: &Path) -> PaletteCache {
        let map: HashMap<String, Option<AccentInfo>> =
            fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        PaletteCache { map: Mutex::new(map), ..Default::default() }
    }

    pub fn save(&self, path: &Path) {
        use std::sync::atomic::Ordering::Relaxed;
        if !self.dirty.swap(false, Relaxed) {
            return;
        }
        let mut map = self.map.lock().unwrap().clone();
        if map.len() > 1500 {
            let used = self.used.lock().unwrap();
            map.retain(|k, _| used.contains(k));
        }
        if let Ok(s) = serde_json::to_string(&map) {
            let _ = write_atomic(path, s.as_bytes());
        }
    }

    /// Acento de un juego a partir de su hero y su portada (o primera captura). No relee ni recalcula
    /// si las rutas y fechas de modificacion no han cambiado.
    pub fn accent_for(&self, hero: Option<&Path>, cover: Option<&Path>) -> Option<AccentInfo> {
        use std::sync::atomic::Ordering::Relaxed;
        let paths: Vec<&Path> = hero.into_iter().chain(cover).collect();
        let key = cache_key(&paths)?;
        self.used.lock().unwrap().insert(key.clone());
        if let Some(hit) = self.map.lock().unwrap().get(&key) {
            return hit.clone();
        }
        let imgs: Vec<RgbaImage> = paths.iter().filter_map(|p| load_for_palette(p)).collect();
        self.computed.fetch_add(1, Relaxed);
        let res = extract_palette(&imgs);
        self.map.lock().unwrap().insert(key, res.clone());
        self.dirty.store(true, Relaxed);
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn solid(w: u32, h: u32, rgb: [u8; 3]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba([rgb[0], rgb[1], rgb[2], 255]))
    }

    /// Imagen con una franja de color (`frac` del ancho) sobre un fondo.
    fn striped(w: u32, h: u32, bg: [u8; 3], fg: [u8; 3], frac: f32) -> RgbaImage {
        let mut im = solid(w, h, bg);
        let cut = (w as f32 * frac) as u32;
        for y in 0..h {
            for x in 0..cut {
                im.put_pixel(x, y, Rgba([fg[0], fg[1], fg[2], 255]));
            }
        }
        im
    }

    fn lch_of(hex: &str) -> (f32, f32, f32) {
        lab_to_lch(srgb_to_oklab(parse_hex(hex).unwrap()))
    }

    #[test]
    fn oklab_roundtrip_is_stable() {
        for rgb in [[0u8, 160, 216], [232, 56, 79], [242, 184, 7], [128, 128, 128], [255, 255, 255]] {
            let (l, c, h) = lab_to_lch(srgb_to_oklab(rgb));
            let back = lch_to_rgb(l, c, h);
            for i in 0..3 {
                assert!((rgb[i] as i32 - back[i] as i32).abs() <= 2, "{rgb:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn picks_the_vivid_colour_over_a_bigger_dull_area() {
        // 70 % marron apagado, 30 % azul vivo: gana el azul (hue ~ 250).
        let im = striped(200, 60, [120, 100, 90], [20, 90, 230], 0.3);
        let info = extract_palette(&[im]).unwrap();
        let (_, _, h) = lch_of(&info.accent.color);
        assert!((220.0..285.0).contains(&h), "tono {h} de {}", info.accent.color);
    }

    #[test]
    fn dominant_vivid_area_wins_and_palette_has_distinct_hues() {
        let mut im = solid(128, 64, [200, 40, 30]); // rojo dominante
        for y in 0..64 {
            for x in 0..30 {
                im.put_pixel(x, y, Rgba([30, 170, 60, 255])); // verde
            }
            for x in 30..44 {
                im.put_pixel(x, y, Rgba([240, 200, 20, 255])); // amarillo
            }
        }
        let info = extract_palette(&[im]).unwrap();
        let (_, _, h) = lch_of(&info.accent.color);
        assert!(h < 45.0 || h > 340.0, "el rojo manda: tono {h}");
        assert!(info.palette.len() >= 3 && info.palette.len() <= 5);
        assert_eq!(info.palette[0], info.accent);
        for (i, a) in info.palette.iter().enumerate() {
            for b in info.palette.iter().skip(i + 1) {
                assert!(hue_dist(lch_of(&a.color).2, lch_of(&b.color).2) >= 20.0, "tonos repetidos: {} {}", a.color, b.color);
            }
        }
    }

    #[test]
    fn hero_and_cover_both_contribute() {
        // El hero es rojo, la portada azul y de otro tamano: ambos salen en la paleta.
        let hero = solid(400, 130, [220, 30, 40]);
        let cover = solid(60, 90, [30, 60, 220]);
        let info = extract_palette(&[hero, cover]).unwrap();
        let hues: Vec<f32> = info.palette.iter().map(|s| lch_of(&s.color).2).collect();
        assert!(hues.iter().any(|h| *h < 40.0 || *h > 340.0), "{hues:?}");
        assert!(hues.iter().any(|h| (230.0..285.0).contains(h)), "{hues:?}");
    }

    #[test]
    fn grey_black_and_white_images_give_nothing() {
        assert!(extract_palette(&[solid(80, 40, [128, 128, 128])]).is_none());
        assert!(extract_palette(&[solid(80, 40, [2, 2, 3])]).is_none());
        assert!(extract_palette(&[solid(80, 40, [250, 250, 250])]).is_none());
        assert!(extract_palette(&[]).is_none());
        // Gris con degradado: sigue sin haber color.
        let mut g = solid(64, 64, [0, 0, 0]);
        for (x, _y, p) in g.enumerate_pixels_mut() {
            let v = (x * 4) as u8;
            *p = Rgba([v, v, v, 255]);
        }
        assert!(extract_palette(&[g]).is_none());
    }

    #[test]
    fn black_heavy_image_still_finds_its_small_colour() {
        // 90 % negro, 10 % naranja: el negro se ignora y sale el naranja.
        let im = striped(200, 60, [4, 4, 6], [240, 120, 20], 0.1);
        let info = extract_palette(&[im]).unwrap();
        let (_, _, h) = lch_of(&info.accent.color);
        assert!((40.0..80.0).contains(&h), "tono {h} de {}", info.accent.color);
        // Una imagen casi negra con 3 pixeles de color (ruido) no es fiable.
        let mut dark = solid(100, 50, [3, 3, 3]);
        for x in 0..3 {
            dark.put_pixel(x, 0, Rgba([255, 0, 0, 255]));
        }
        assert!(extract_palette(&[dark]).is_none());
    }

    #[test]
    fn transparent_pixels_are_ignored() {
        let mut im = RgbaImage::from_pixel(64, 64, Rgba([255, 0, 0, 0]));
        for x in 0..20 {
            for y in 0..64 {
                im.put_pixel(x, y, Rgba([20, 120, 220, 255]));
            }
        }
        let info = extract_palette(&[im]).unwrap();
        assert!((200.0..285.0).contains(&lch_of(&info.accent.color).2));
    }

    #[test]
    fn normalization_keeps_hue_and_enters_the_accent_range() {
        for h in (0..360).step_by(10) {
            // Una croma bajisima y otra altisima, con luminosidades extremas.
            for (l, c) in [(0.25, 0.05), (0.95, 0.30), (0.5, 0.02), (0.7, 0.16)] {
                let out = to_hex(normalize_lch(l, c, h as f32));
                let (ol, oc, oh) = lch_of(&out);
                assert!((L_MIN - 0.02..=L_MAX + 0.02).contains(&ol), "h{h} l{l}: L={ol} {out}");
                let reach = max_chroma(ol.clamp(L_MIN, L_MAX), h as f32).min(C_MIN);
                assert!(oc >= reach - 0.02, "h{h}: croma {oc} < {reach} ({out})");
                assert!(oc <= C_MAX + 0.02);
                // El tono se conserva (con la cuantizacion a 8 bits el error es pequeno salvo con poca croma).
                assert!(hue_dist(oh, h as f32) < 6.0, "h{h}: tono {oh} ({out})");
            }
        }
        // Un azul oscuro sube de luminosidad; un amarillo pastel baja.
        assert!(lch_of(&normalize_rgb([10, 10, 80]).color).0 >= L_MIN - 0.02);
        assert!(lch_of(&normalize_rgb([255, 250, 160]).color).0 <= L_MAX + 0.02);
    }

    #[test]
    fn ink_choice_matches_the_rainbow_accents() {
        // Los siete acentos de la interfaz: solo el amarillo lleva texto oscuro.
        let rainbow = [
            ("#E8384F", false), ("#F27A1A", false), ("#F2B807", true), ("#3BB54A", false),
            ("#00A0D8", false), ("#3F63E0", false), ("#9A4BD8", false),
        ];
        for (hex, dark) in rainbow {
            assert_eq!(needs_dark_ink(parse_hex(hex).unwrap()), dark, "{hex}");
        }
        assert!(contrast_with_white([255, 255, 255]) < 1.1);
        assert!(contrast_with_white([0, 0, 0]) > 20.0);
        // Un tono amarillo-verdoso normalizado es claro: texto oscuro; un violeta, blanco.
        assert!(accent_for_hue(90.0).dark && accent_for_hue(200.0).dark);
        assert!(!accent_for_hue(300.0).dark);
    }

    #[test]
    fn accent_for_hue_wraps_and_is_valid_hex() {
        assert_eq!(accent_for_hue(-60.0), accent_for_hue(300.0));
        for h in [0.0, 90.0, 200.0, 359.0] {
            let s = accent_for_hue(h);
            assert!(parse_hex(&s.color).is_some());
        }
        assert!(parse_hex("#12ab9Z").is_none() && parse_hex("12ab9f").is_none() && parse_hex("#12ab9").is_none());
        assert_eq!(parse_hex("#0A141e"), Some([10, 20, 30]));
    }

    #[test]
    fn cache_is_keyed_by_path_and_mtime() {
        let d = tempfile::tempdir().unwrap();
        let hero = d.path().join("hero.png");
        solid(64, 20, [230, 40, 60]).save(&hero).unwrap();
        let cache = PaletteCache::default();
        let a = cache.accent_for(Some(&hero), None).unwrap();
        let b = cache.accent_for(Some(&hero), None).unwrap();
        assert_eq!(a, b);
        assert_eq!(cache.computed.load(std::sync::atomic::Ordering::Relaxed), 1, "la segunda vez sale de la cache");
        // Persistencia y recarga: no se recalcula.
        let file = d.path().join("palettes.json");
        cache.save(&file);
        let again = PaletteCache::load(&file);
        assert_eq!(again.accent_for(Some(&hero), None).unwrap(), a);
        assert_eq!(again.computed.load(std::sync::atomic::Ordering::Relaxed), 0);
        // Cambiar el archivo (otra fecha de modificacion) invalida la entrada.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(120);
        solid(64, 20, [20, 60, 230]).save(&hero).unwrap();
        fs::File::options().write(true).open(&hero).unwrap().set_modified(later).unwrap();
        let c = again.accent_for(Some(&hero), None).unwrap();
        assert_eq!(again.computed.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_ne!(c, a);
        // Sin ninguna imagen legible no hay clave ni acento.
        assert!(cache.accent_for(Some(Path::new("/no/existe.png")), None).is_none());
    }
}
