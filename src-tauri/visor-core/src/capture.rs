//! Parte pura de la captura propia: nombres de archivo, carpeta de destino, guardado PNG, deteccion de
//! fotogramas negros, combinaciones de mando, recortes, deteccion de Magpie y sonido. La captura real (Windows.Graphics.Capture) vive en
//! la aplicacion Tauri porque depende del sistema operativo.
use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use std::fs;
use std::path::{Path, PathBuf};

/// Carpeta (dentro de la plataforma PC) donde van las capturas que no se pueden asignar a un juego.
pub const UNCLASSIFIED_FOLDER: &str = "Sin clasificar";
/// Id de la plataforma usada para las capturas sin juego.
pub const UNCLASSIFIED_PLATFORM: &str = "pc";

/// `Carpeta_AAAA-MM-DD_HH-MM-SS.png`
pub fn capture_filename(folder: &str, at: NaiveDateTime) -> String {
    format!("{folder}_{}.png", at.format("%Y-%m-%d_%H-%M-%S"))
}

/// `Raiz/<Plataforma>/<Carpeta>`
pub fn destination_dir(root: &Path, platform_folder: &str, folder: &str) -> PathBuf {
    root.join(platform_folder).join(folder)
}

/// Ruta libre: si ya existe `nombre.png` (dos capturas en el mismo segundo) usa `nombre_2.png`, etc.
/// Nunca sobrescribe.
pub fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let first = dir.join(file_name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(file_name);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = p.extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    for n in 2.. {
        let c = dir.join(format!("{stem}_{n}.{ext}"));
        if !c.exists() {
            return c;
        }
    }
    unreachable!()
}

/// Convierte BGRA (formato de Windows.Graphics.Capture) a RGBA en el sitio.
pub fn bgra_to_rgba(buf: &mut [u8]) {
    for px in buf.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
}

/// `true` si el fotograma es (casi) todo negro: tipico de la captura de ventana en pantalla completa
/// exclusiva. Muestrea unos pocos miles de pixeles; el negro «real» de un juego (menu oscuro) tiene
/// algo de luz, asi que se exige que practicamente todo sea < 6 en cada canal.
pub fn is_blank(rgba: &[u8]) -> bool {
    let n = rgba.len() / 4;
    if n == 0 {
        return true;
    }
    let step = (n / 4000).max(1);
    let (mut total, mut dark) = (0usize, 0usize);
    let mut i = 0;
    while i < n {
        let p = &rgba[i * 4..i * 4 + 3];
        total += 1;
        if p.iter().all(|&c| c < 6) {
            dark += 1;
        }
        i += step;
    }
    dark * 1000 >= total * 998
}

/// Recorta una imagen RGBA (o BGRA: no mira los canales). El rectangulo se ajusta a la imagen; devuelve
/// `None` si queda vacio.
pub fn crop_rgba(px: &[u8], width: u32, height: u32, x: i64, y: i64, w: i64, h: i64) -> Option<(u32, u32, Vec<u8>)> {
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + w).min(width as i64), (y + h).min(height as i64));
    if x1 <= x0 || y1 <= y0 || px.len() < width as usize * height as usize * 4 {
        return None;
    }
    let (cw, ch) = ((x1 - x0) as usize, (y1 - y0) as usize);
    if cw == width as usize && ch == height as usize {
        return Some((width, height, px[..cw * ch * 4].to_vec()));
    }
    let mut out = Vec::with_capacity(cw * ch * 4);
    for row in y0 as usize..y1 as usize {
        let s = (row * width as usize + x0 as usize) * 4;
        out.extend_from_slice(&px[s..s + cw * 4]);
    }
    Some((cw as u32, ch as u32, out))
}

/// Rectangulo de pantalla (izquierda, arriba, derecha, abajo) en pixeles fisicos.
pub type Rect = (i32, i32, i32, i32);

/// Zona a recortar (x, y, ancho, alto) dentro de una imagen de `frame` (tamano `fw`x`fh`) para quedarse con
/// `inner`. `None` si no hace falta recortar o si los tamanos no cuadran (mejor no recortar que cortar mal).
pub fn crop_for(fw: u32, fh: u32, frame: Rect, inner: Rect) -> Option<(i64, i64, i64, i64)> {
    let (frw, frh) = ((frame.2 - frame.0) as i64, (frame.3 - frame.1) as i64);
    // La imagen debe corresponder al rectangulo de origen (con un par de pixeles de margen por redondeos).
    if (frw - fw as i64).abs() > 2 || (frh - fh as i64).abs() > 2 {
        return None;
    }
    let x = (inner.0 - frame.0) as i64;
    let y = (inner.1 - frame.1) as i64;
    let w = (inner.2 - inner.0) as i64;
    let h = (inner.3 - inner.1) as i64;
    if w < 64 || h < 64 || x < -2 || y < -2 || x + w > fw as i64 + 2 || y + h > fh as i64 + 2 {
        return None;
    }
    if x.abs() <= 1 && y.abs() <= 1 && (w - fw as i64).abs() <= 1 && (h - fh as i64).abs() <= 1 {
        return None;
    }
    Some((x, y, w, h))
}

/// `true` si una ventana es la ventana de escalado de Magpie (no su ventana principal de ajustes).
/// Magpie usa la clase `Window_Magpie_967EB565-...` para la ventana que muestra el juego escalado.
pub fn is_magpie_scaling_window(class: &str, exe_name: &str) -> bool {
    let c = class.to_ascii_lowercase();
    let e = exe_name.to_ascii_lowercase();
    if c.starts_with("window_magpie") {
        return true;
    }
    e.contains("magpie") && c.contains("magpie") && (c.contains("scaling") || c.contains("swapchain"))
}

/// RGBA -> JPEG en memoria, reducido a `max_w` de ancho como mucho (para las capturas de lectura, que van a Anki).
pub fn rgba_to_jpeg(width: u32, height: u32, rgba: &[u8], max_w: u32, quality: u8) -> Result<Vec<u8>> {
    use image::codecs::jpeg::JpegEncoder;
    if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
        anyhow::bail!("tamano de imagen incoherente");
    }
    let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    let img = image::RgbImage::from_raw(width, height, rgb).context("imagen incoherente")?;
    let img = if width > max_w && max_w > 0 {
        let h = ((height as u64 * max_w as u64) / width as u64).max(1) as u32;
        image::imageops::resize(&img, max_w, h, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality).encode_image(&img)?;
    Ok(out)
}

/// Base64 estandar (con relleno), para mandar imagenes pequenas a una webview como `data:` URL.
pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Guarda un PNG creando las carpetas necesarias. Descarta el canal alfa (las capturas son opacas y el
/// archivo ocupa menos) y usa compresion rapida. Escribe primero a `.tmp` y renombra.
pub fn save_rgba_png(path: &Path, width: u32, height: u32, rgba: Vec<u8>) -> Result<()> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    use image::ImageEncoder;
    if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
        anyhow::bail!("tamano de imagen incoherente");
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("creando {}", dir.display()))?;
    }
    let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    drop(rgba);
    let tmp = path.with_extension("png.tmp");
    let res = (|| -> Result<()> {
        let f = fs::File::create(&tmp).with_context(|| format!("creando {}", tmp.display()))?;
        let mut w = std::io::BufWriter::new(f);
        PngEncoder::new_with_quality(&mut w, CompressionType::Fast, FilterType::Adaptive).write_image(
            &rgb,
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )?;
        use std::io::Write;
        w.flush()?;
        drop(w);
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res
}

/* ---------------- mando ---------------- */

pub const XI_UP: u32 = 0x0001;
pub const XI_DOWN: u32 = 0x0002;
pub const XI_LEFT: u32 = 0x0004;
pub const XI_RIGHT: u32 = 0x0008;
pub const XI_START: u32 = 0x0010;
pub const XI_BACK: u32 = 0x0020;
pub const XI_L3: u32 = 0x0040;
pub const XI_R3: u32 = 0x0080;
pub const XI_LB: u32 = 0x0100;
pub const XI_RB: u32 = 0x0200;
/// Boton Xbox: solo lo da `XInputGetStateEx` (no documentada).
pub const XI_GUIDE: u32 = 0x0400;
pub const XI_A: u32 = 0x1000;
pub const XI_B: u32 = 0x2000;
pub const XI_X: u32 = 0x4000;
pub const XI_Y: u32 = 0x8000;
/// Los gatillos son analogicos en XInput: se representan con bits por encima de los 16 de `wButtons`.
pub const XI_LT: u32 = 1 << 16;
pub const XI_RT: u32 = 1 << 17;
/// Valor (0..255) a partir del cual un gatillo cuenta como pulsado.
pub const TRIGGER_THRESHOLD: u8 = 100;

/// Botones en el orden en que se escriben las combinaciones: (nombre en el id, bit).
pub const PAD_BUTTONS: &[(&str, u32)] = &[
    ("back", XI_BACK),
    ("start", XI_START),
    ("guide", XI_GUIDE),
    ("lb", XI_LB),
    ("rb", XI_RB),
    ("lt", XI_LT),
    ("rt", XI_RT),
    ("l3", XI_L3),
    ("r3", XI_R3),
    ("a", XI_A),
    ("b", XI_B),
    ("x", XI_X),
    ("y", XI_Y),
    ("up", XI_UP),
    ("down", XI_DOWN),
    ("left", XI_LEFT),
    ("right", XI_RIGHT),
];

/// Mascara completa del mando a partir de `wButtons` y los gatillos.
pub fn pad_mask(buttons: u16, left_trigger: u8, right_trigger: u8) -> u32 {
    let mut m = buttons as u32;
    if left_trigger >= TRIGGER_THRESHOLD {
        m |= XI_LT;
    }
    if right_trigger >= TRIGGER_THRESHOLD {
        m |= XI_RT;
    }
    m
}

/// Mascara de una combinacion escrita como `boton+boton` (p. ej. `back+rb`). 0 si esta desactivada
/// (`off` o vacia) o si algun nombre es desconocido.
pub fn combo_mask(id: &str) -> u32 {
    let id = id.trim();
    if id.is_empty() || id.eq_ignore_ascii_case("off") {
        return 0;
    }
    let mut m = 0;
    for part in id.split('+') {
        match PAD_BUTTONS.iter().find(|(n, _)| n.eq_ignore_ascii_case(part.trim())) {
            Some((_, b)) => m |= b,
            None => return 0,
        }
    }
    m
}

/// Id canonico (`back+rb`) de una mascara; `off` si es 0.
pub fn combo_id(mask: u32) -> String {
    let parts: Vec<&str> = PAD_BUTTONS.iter().filter(|(_, b)| mask & b != 0).map(|(n, _)| *n).collect();
    if parts.is_empty() {
        "off".into()
    } else {
        parts.join("+")
    }
}

/// Detecta el flanco de subida de una combinacion, con tiempo minimo entre disparos.
#[derive(Debug)]
pub struct ComboDetector {
    pub mask: u32,
    was_down: bool,
    last_fire_ms: Option<u64>,
    cooldown_ms: u64,
}

impl ComboDetector {
    pub fn new(mask: u32, cooldown_ms: u64) -> Self {
        ComboDetector { mask, was_down: false, last_fire_ms: None, cooldown_ms }
    }
    /// `buttons` = mascara de [`pad_mask`]. Devuelve `true` una sola vez al completar la combinacion.
    pub fn update(&mut self, buttons: u32, now_ms: u64) -> bool {
        if self.mask == 0 {
            self.was_down = false;
            return false;
        }
        let down = buttons & self.mask == self.mask;
        let rising = down && !self.was_down;
        self.was_down = down;
        if rising && self.last_fire_ms.map(|t| now_ms.saturating_sub(t) >= self.cooldown_ms).unwrap_or(true) {
            self.last_fire_ms = Some(now_ms);
            return true;
        }
        false
    }
}

/// Aprende una combinacion «esperando al mando»: espera a que se suelte todo (el boton con el que se abrio
/// el dialogo no cuenta), acumula lo que se pulse y la da por buena al soltarlo todo.
#[derive(Debug, Default)]
pub struct ComboLearner {
    armed: bool,
    peak: u32,
}

impl ComboLearner {
    pub fn reset(&mut self) {
        *self = ComboLearner::default();
    }
    /// Devuelve la combinacion completa cuando se sueltan todos los botones.
    pub fn update(&mut self, buttons: u32) -> Option<u32> {
        if !self.armed {
            self.armed = buttons == 0;
            return None;
        }
        if buttons != 0 {
            self.peak |= buttons;
            return None;
        }
        if self.peak != 0 {
            let m = self.peak;
            self.peak = 0;
            return Some(m);
        }
        None
    }
    /// Lo pulsado hasta ahora (para enseñarlo mientras se aprende).
    pub fn current(&self) -> u32 {
        self.peak
    }
}

/* ---------------- raton ---------------- */

/// Botones de raton que pueden capturar ("" = desactivado). Los botones izquierdo y derecho no se ofrecen:
/// capturar con ellos dispararia en cada clic del juego.
pub const MOUSE_BUTTONS: &[&str] = &["middle", "x1", "x2"];

/// Indice (1..) de un boton de raton; 0 si esta desactivado o es desconocido.
pub fn mouse_button_index(id: &str) -> u8 {
    MOUSE_BUTTONS.iter().position(|b| *b == id).map(|i| i as u8 + 1).unwrap_or(0)
}

/* ---------------- teclado ---------------- */

/// Atajos de teclado ofrecidos en Ajustes ("" = desactivado).
pub const HOTKEYS: &[&str] = &["F12", "F10", "F9", "Ctrl+F12", "Alt+F12", "Ctrl+Shift+S", ""];

/* ---------------- sonido ---------------- */

/// WAV PCM 16 bits mono con dos tonos cortos (un «clic de obturador» agradable), generado en memoria.
pub fn shutter_wav() -> Vec<u8> {
    let rate = 22050u32;
    let mut samples: Vec<i16> = Vec::new();
    for (freq, ms) in [(1318.5f32, 55u32), (1760.0, 90)] {
        let n = rate * ms / 1000;
        for i in 0..n {
            let t = i as f32 / rate as f32;
            let env = (1.0 - i as f32 / n as f32).powf(1.6) * (i as f32 / 60.0).min(1.0);
            samples.push(((t * freq * std::f32::consts::TAU).sin() * env * 0.35 * i16::MAX as f32) as i16);
        }
    }
    let data_len = (samples.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}

/// Datos de la ventana en primer plano que decide si se captura.
#[derive(Debug, Clone, Default)]
pub struct ForegroundInfo {
    /// Es una ventana del propio visor (incluidos el overlay y la miniatura).
    pub own: bool,
    /// Nombre del exe (sin ruta), en cualquier caso.
    pub exe_name: String,
    pub class: String,
    /// Ocupa su monitor entero (pantalla completa o sin bordes).
    pub fullscreen: bool,
    /// Su exe esta dentro de la carpeta de un juego conocido.
    pub known_game: bool,
}

/// `true` si `win` cubre por completo `mon` (rectangulos izquierda, arriba, derecha, abajo).
pub fn covers_monitor(win: Rect, mon: Rect) -> bool {
    mon.2 > mon.0 && mon.3 > mon.1 && win.0 <= mon.0 && win.1 <= mon.1 && win.2 >= mon.2 && win.3 >= mon.3
}

/// Ventanas del escritorio y del shell de Windows: nunca cuentan como «un juego», aunque ocupen la pantalla.
pub fn is_shell_window(exe_name: &str, class: &str) -> bool {
    const CLASSES: [&str; 4] = ["progman", "workerw", "shell_traywnd", "shell_secondarytraywnd"];
    const EXES: [&str; 7] = [
        "explorer.exe", "searchhost.exe", "searchapp.exe", "startmenuexperiencehost.exe", "shellexperiencehost.exe",
        "lockapp.exe", "textinputhost.exe",
    ];
    let (e, c) = (exe_name.to_ascii_lowercase(), class.to_ascii_lowercase());
    EXES.contains(&e.as_str()) || CLASSES.contains(&c.as_str())
}

/// Decide si hay que capturar. `only_games` = ajuste `capture_only_games`; `has_active` = hay un juego lanzado
/// desde el visor; `fg` = ventana en primer plano (si no se pudo leer, no se captura).
pub fn should_capture(only_games: bool, has_active: bool, fg: Option<&ForegroundInfo>) -> bool {
    if !only_games || has_active {
        return true;
    }
    match fg {
        None => false,
        Some(f) if f.own => false,
        Some(f) if f.known_game => true,
        Some(f) => f.fullscreen && !is_shell_window(&f.exe_name, &f.class),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fgi(exe: &str, class: &str, fullscreen: bool, known: bool) -> ForegroundInfo {
        ForegroundInfo { own: false, exe_name: exe.into(), class: class.into(), fullscreen, known_game: known }
    }

    #[test]
    fn capture_rule() {
        // Desactivado o juego activo: siempre.
        assert!(should_capture(false, false, None));
        assert!(should_capture(true, true, None));
        // Escritorio, explorador a pantalla completa y ventanas normales: no.
        assert!(!should_capture(true, false, None));
        assert!(!should_capture(true, false, Some(&fgi("explorer.exe", "Progman", true, false))));
        assert!(!should_capture(true, false, Some(&fgi("explorer.exe", "CabinetWClass", true, false))));
        assert!(!should_capture(true, false, Some(&fgi("chrome.exe", "Chrome_WidgetWin_1", false, false))));
        assert!(!should_capture(true, false, Some(&ForegroundInfo { own: true, fullscreen: true, ..Default::default() })));
        // Juego conocido (ventana) o cualquier ventana a pantalla completa que no sea el shell: si.
        assert!(should_capture(true, false, Some(&fgi("game.exe", "UnityWndClass", false, true))));
        assert!(should_capture(true, false, Some(&fgi("emu.exe", "SDL_app", true, false))));
    }

    #[test]
    fn monitor_cover() {
        let m = (0, 0, 1920, 1080);
        assert!(covers_monitor((0, 0, 1920, 1080), m));
        assert!(covers_monitor((-8, -8, 1928, 1088), m));
        assert!(!covers_monitor((0, 0, 1920, 1040), m));
        assert!(!covers_monitor((0, 0, 100, 100), (0, 0, 0, 0)));
    }
    use crate::dates::parse_date_from_filename;
    use chrono::NaiveDate;

    #[test]
    fn jpeg_reducido() {
        let (w, h) = (400u32, 200u32);
        let px: Vec<u8> = (0..w * h).flat_map(|i| [(i % 251) as u8, 90, 200, 255]).collect();
        let j = rgba_to_jpeg(w, h, &px, 100, 80).unwrap();
        assert_eq!(&j[..2], &[0xFF, 0xD8]);
        let img = image::load_from_memory(&j).unwrap();
        assert_eq!((img.width(), img.height()), (100, 50));
        assert!(rgba_to_jpeg(w, h, &px[..10], 100, 80).is_err());
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        let j2 = rgba_to_jpeg(w, h, &px, 960, 80).unwrap();
        assert_eq!(image::load_from_memory(&j2).unwrap().width(), 400); // no se agranda
    }

    fn at() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 3, 9).unwrap().and_hms_opt(21, 5, 4).unwrap()
    }

    #[test]
    fn filename_and_destination() {
        let f = capture_filename("Elden Ring", at());
        assert_eq!(f, "Elden Ring_2026-03-09_21-05-04.png");
        let d = destination_dir(Path::new("D:/Caps"), "PC", "Elden Ring");
        assert_eq!(d, Path::new("D:/Caps").join("PC").join("Elden Ring"));
        // El visor reconoce la fecha del nombre generado.
        assert_eq!(parse_date_from_filename(&f), parse_date_from_filename("2026-03-09 21-05-04"));
        assert!(parse_date_from_filename(&f).is_some());
    }

    #[test]
    fn unique_path_never_overwrites() {
        let d = tempfile::tempdir().unwrap();
        let a = unique_path(d.path(), "x_2026-03-09_21-05-04.png");
        assert_eq!(a, d.path().join("x_2026-03-09_21-05-04.png"));
        fs::write(&a, b"1").unwrap();
        let b = unique_path(d.path(), "x_2026-03-09_21-05-04.png");
        assert_eq!(b, d.path().join("x_2026-03-09_21-05-04_2.png"));
        fs::write(&b, b"2").unwrap();
        assert!(unique_path(d.path(), "x_2026-03-09_21-05-04.png").ends_with("x_2026-03-09_21-05-04_3.png"));
        assert_eq!(fs::read(&a).unwrap(), b"1");
    }

    #[test]
    fn saves_png_creating_folders_and_swaps_channels() {
        let d = tempfile::tempdir().unwrap();
        let mut px = vec![0u8; 4 * 4 * 4];
        for p in px.chunks_exact_mut(4) {
            p.copy_from_slice(&[10, 20, 200, 255]); // BGRA: azul fuerte
        }
        bgra_to_rgba(&mut px);
        assert_eq!(&px[0..4], &[200, 20, 10, 255]);
        px[3] = 7; // el alfa se descarta: el PNG sale opaco
        let path = d.path().join("PC/Juego Nuevo/j.png");
        save_rgba_png(&path, 4, 4, px).unwrap();
        let img = image::open(&path).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (4, 4));
        assert_eq!(img.get_pixel(0, 0).0, [200, 20, 10, 255]);
        assert!(!path.with_extension("png.tmp").exists());
        assert!(save_rgba_png(&d.path().join("mal.png"), 3, 3, vec![0; 5]).is_err());
    }

    #[test]
    fn blank_detection() {
        assert!(is_blank(&vec![0u8; 400 * 4]));
        assert!(is_blank(&[]));
        let mut lit = vec![0u8; 100 * 100 * 4];
        for p in lit.chunks_exact_mut(4) {
            p[0] = 90;
            p[3] = 255;
        }
        assert!(!is_blank(&lit));
        // Un menu oscuro con un logo claro no cuenta como negro.
        let mut dark = vec![0u8; 100 * 100 * 4];
        for p in dark.chunks_exact_mut(4).take(500) {
            p[0] = 255;
            p[1] = 255;
            p[2] = 255;
        }
        assert!(!is_blank(&dark));
    }

    #[test]
    fn combos_parse_and_roundtrip() {
        assert_eq!(combo_mask("back+rb"), XI_BACK | XI_RB);
        assert_eq!(combo_mask("RB + back"), XI_BACK | XI_RB);
        assert_eq!(combo_mask("off"), 0);
        assert_eq!(combo_mask(""), 0);
        assert_eq!(combo_mask("raro"), 0);
        assert_eq!(combo_mask("back+raro"), 0);
        assert_eq!(combo_mask("guide"), XI_GUIDE);
        assert_eq!(combo_id(XI_RB | XI_BACK), "back+rb");
        assert_eq!(combo_id(XI_LT | XI_Y), "lt+y");
        assert_eq!(combo_id(0), "off");
        for id in ["back+rb", "back+lb", "lb+rb", "back+start", "l3+r3", "guide", "lt+rt+a", "up"] {
            assert_eq!(combo_id(combo_mask(id)), id);
        }
        assert_eq!(pad_mask(0x0020, 0, 255), XI_BACK | XI_RT);
        assert_eq!(pad_mask(0, TRIGGER_THRESHOLD - 1, 0), 0);
    }

    #[test]
    fn learner_waits_for_release_and_takes_whole_combo() {
        let mut l = ComboLearner::default();
        assert_eq!(l.update(XI_A), None, "el A que abrio el dialogo no cuenta");
        assert_eq!(l.update(XI_A), None);
        assert_eq!(l.update(0), None);
        assert_eq!(l.update(XI_BACK), None);
        assert_eq!(l.update(XI_BACK | XI_RB), None);
        assert_eq!(l.current(), XI_BACK | XI_RB);
        assert_eq!(l.update(XI_RB), None, "soltar uno a uno no corta la combinacion");
        assert_eq!(l.update(0), Some(XI_BACK | XI_RB));
        assert_eq!(l.update(0), None);
        assert_eq!(l.update(XI_LT), None);
        assert_eq!(l.update(0), Some(XI_LT));
    }

    #[test]
    fn mouse_buttons() {
        assert_eq!(mouse_button_index("middle"), 1);
        assert_eq!(mouse_button_index("x2"), 3);
        assert_eq!(mouse_button_index(""), 0);
        assert_eq!(mouse_button_index("left"), 0);
    }

    #[test]
    fn crops() {
        // 4x3 con el indice del pixel en el primer canal.
        let px: Vec<u8> = (0..12u8).flat_map(|i| [i, 0, 0, 255]).collect();
        let (w, h, c) = crop_rgba(&px, 4, 3, 1, 1, 2, 2).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(c.chunks(4).map(|p| p[0]).collect::<Vec<_>>(), vec![5, 6, 9, 10]);
        let (w, h, _) = crop_rgba(&px, 4, 3, -5, -5, 100, 100).unwrap();
        assert_eq!((w, h), (4, 3), "se ajusta a la imagen");
        assert!(crop_rgba(&px, 4, 3, 10, 0, 2, 2).is_none());
        // Ventana con marco: imagen 1000x800 del marco, zona cliente desplazada por la barra de titulo.
        assert_eq!(crop_for(1000, 800, (100, 100, 1100, 900), (108, 131, 1092, 892)), Some((8, 31, 984, 761)));
        assert_eq!(crop_for(1000, 800, (0, 0, 1000, 800), (0, 0, 1000, 800)), None, "sin marco no se recorta");
        assert_eq!(crop_for(640, 480, (0, 0, 1000, 800), (8, 31, 992, 792)), None, "tamanos que no cuadran");
        // Magpie a pantalla completa en el segundo monitor: recorte de la captura del monitor.
        assert_eq!(crop_for(2560, 1440, (1920, 0, 4480, 1440), (2240, 0, 4160, 1440)), Some((320, 0, 1920, 1440)));
    }

    #[test]
    fn magpie_detection() {
        assert!(is_magpie_scaling_window("Window_Magpie_967EB565-6F73-4E94-AE53-00CC42592A22", "Magpie.exe"));
        assert!(is_magpie_scaling_window("Magpie_ScalingWindow", "Magpie.exe"));
        assert!(!is_magpie_scaling_window("Magpie_967EB565-6F73-4E94-AE53-00CC42592A22", "Magpie.exe"), "ventana principal");
        assert!(!is_magpie_scaling_window("UnityWndClass", "Game.exe"));
    }

    #[test]
    fn combos_fire_once_with_cooldown() {
        let mut c = ComboDetector::new(combo_mask("back+rb"), 1000);
        assert!(!c.update(XI_BACK, 0));
        assert!(c.update(XI_BACK | XI_RB, 16), "dispara al completarse");
        assert!(!c.update(XI_BACK | XI_RB, 32), "mantener pulsado no repite");
        assert!(!c.update(XI_BACK, 48));
        assert!(!c.update(XI_BACK | XI_RB, 64), "dentro del enfriamiento");
        assert!(!c.update(0, 80));
        assert!(c.update(XI_BACK | XI_RB | 0x1000, 2000), "pasado el enfriamiento");
        let mut off = ComboDetector::new(0, 0);
        assert!(!off.update(0xFFFF, 0));
    }

    #[test]
    fn wav_is_well_formed() {
        let w = shutter_wav();
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        let data_len = u32::from_le_bytes(w[40..44].try_into().unwrap()) as usize;
        assert_eq!(w.len(), 44 + data_len);
        assert!(data_len > 4000);
    }
}
