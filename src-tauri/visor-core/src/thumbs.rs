use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::UNIX_EPOCH;

pub const THUMB_WIDTH: u32 = 480;
const JPEG_QUALITY: u8 = 82;

/// FNV-1a de 64 bits: estable entre versiones y plataformas (a diferencia de DefaultHasher).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Clave de miniatura a partir de ruta + mtime (+ tamano): si el archivo cambia, cambia la clave.
pub fn thumb_key(path: &str, mtime_ms: i64, len: u64) -> String {
    let s = format!("{path}\u{0}{mtime_ms}\u{0}{len}");
    format!("{:016x}", fnv1a64(s.as_bytes()))
}

fn file_stamp(src: &Path) -> Result<(i64, u64)> {
    let md = fs::metadata(src).with_context(|| format!("leyendo {}", src.display()))?;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok((mtime, md.len()))
}

/// Ruta donde viviria la miniatura de `src` (exista o no todavia).
pub fn thumb_path(cache_dir: &Path, src: &Path) -> Result<PathBuf> {
    let (mtime, len) = file_stamp(src)?;
    Ok(cache_dir.join(format!("{}.jpg", thumb_key(&src.to_string_lossy(), mtime, len))))
}

/// Pool de rayon propio (no el global) para no saturar la CPU mientras se navega.
pub fn pool() -> &'static ThreadPool {
    static POOL: OnceLock<ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        ThreadPoolBuilder::new()
            .num_threads((n / 2).clamp(2, 6))
            .thread_name(|i| format!("thumbs-{i}"))
            .build()
            .expect("pool de miniaturas")
    })
}

/// Genera (si hace falta) la miniatura JPEG de 480 px de ancho y devuelve su ruta.
/// Solo lee `src`; escribe unicamente dentro de `cache_dir`.
pub fn ensure_thumb(cache_dir: &Path, src: &Path) -> Result<PathBuf> {
    let out = thumb_path(cache_dir, src)?;
    if out.exists() {
        return Ok(out);
    }
    fs::create_dir_all(cache_dir)?;
    let img = image::open(src).with_context(|| format!("decodificando {}", src.display()))?;
    let (w, h) = (img.width(), img.height());
    let img = if w > THUMB_WIDTH {
        let nh = ((h as f64) * (THUMB_WIDTH as f64) / (w as f64)).round().max(1.0) as u32;
        img.resize_exact(THUMB_WIDTH, nh, FilterType::Triangle)
    } else {
        img
    };
    let rgb = img.to_rgb8();
    let tmp = out.with_extension(format!("tmp{}", std::process::id()));
    {
        let f = fs::File::create(&tmp)?;
        let mut enc = JpegEncoder::new_with_quality(std::io::BufWriter::new(f), JPEG_QUALITY);
        enc.encode_image(&rgb)?;
    }
    fs::rename(&tmp, &out)?;
    Ok(out)
}

/// Igual que `ensure_thumb` pero ejecutandose en el pool acotado.
pub fn ensure_thumb_pooled(cache_dir: &Path, src: &Path) -> Result<PathBuf> {
    pool().install(|| ensure_thumb(cache_dir, src))
}

/// Lanza en segundo plano la generacion de varias miniaturas; `on_ready(origen, miniatura)`
/// se llama desde un hilo del pool por cada una terminada.
pub fn prefetch<F>(cache_dir: PathBuf, paths: Vec<String>, on_ready: F)
where
    F: Fn(&str, &Path) + Send + Sync + 'static,
{
    let on_ready = Arc::new(on_ready);
    let cache_dir = Arc::new(cache_dir);
    for p in paths {
        let (cb, cd) = (on_ready.clone(), cache_dir.clone());
        pool().spawn(move || {
            if let Ok(t) = ensure_thumb(&cd, Path::new(&p)) {
                cb(&p, &t);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn make_png(path: &Path, w: u32, h: u32) {
        image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]))
            .save(path)
            .unwrap();
    }

    #[test]
    fn key_is_stable_and_sensitive() {
        let a = thumb_key("C:\\x\\a.png", 1000, 5);
        assert_eq!(a, thumb_key("C:\\x\\a.png", 1000, 5));
        assert_ne!(a, thumb_key("C:\\x\\a.png", 1001, 5));
        assert_ne!(a, thumb_key("C:\\x\\b.png", 1000, 5));
        assert_eq!(a.len(), 16);
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
    }

    #[test]
    fn generates_480_wide_jpeg_and_reuses_it() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("big.png");
        make_png(&src, 1920, 1080);
        let cache = d.path().join("cache");
        let t = ensure_thumb(&cache, &src).unwrap();
        let (w, h) = imagesize::size(&t).map(|s| (s.width, s.height)).unwrap();
        assert_eq!((w, h), (480, 270));
        let m1 = fs::metadata(&t).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let t2 = ensure_thumb_pooled(&cache, &src).unwrap();
        assert_eq!(t, t2);
        assert_eq!(m1, fs::metadata(&t2).unwrap().modified().unwrap(), "no se regenera");
        // el origen no se toca
        assert_eq!(imagesize::size(&src).unwrap().width, 1920);
    }

    #[test]
    fn small_images_are_not_upscaled_and_changes_invalidate() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("s.png");
        make_png(&src, 100, 50);
        let cache = d.path().join("c");
        let t1 = ensure_thumb(&cache, &src).unwrap();
        assert_eq!(imagesize::size(&t1).unwrap().width, 100);
        make_png(&src, 120, 50); // cambia el tamano => cambia la clave
        let t2 = ensure_thumb(&cache, &src).unwrap();
        assert_ne!(t1, t2);
    }

    #[test]
    fn prefetch_reports_each_thumb() {
        let d = tempfile::tempdir().unwrap();
        let mut paths = vec![];
        for i in 0..4 {
            let p = d.path().join(format!("{i}.png"));
            make_png(&p, 600, 300);
            paths.push(p.to_string_lossy().into_owned());
        }
        let (tx, rx) = mpsc::channel();
        let tx = std::sync::Mutex::new(tx);
        prefetch(d.path().join("cache"), paths, move |src, th| {
            tx.lock().unwrap().send((src.to_string(), th.to_path_buf())).unwrap();
        });
        for _ in 0..4 {
            let (_, th) = rx.recv_timeout(Duration::from_secs(20)).unwrap();
            assert!(th.exists());
        }
    }

    #[test]
    fn missing_source_is_an_error() {
        let d = tempfile::tempdir().unwrap();
        assert!(ensure_thumb(d.path(), &d.path().join("nada.png")).is_err());
    }
}
