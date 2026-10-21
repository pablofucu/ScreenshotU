//! Cronometra el escaneo de la biblioteca real, SOLO LECTURA (no escribe nada en la biblioteca ni en AppData).
//!
//! ```text
//! cargo run -p visor-core --example bench_scan --release -- <raiz de Capturas> [copia de visor.db]
//! ```
//! Con la copia de la base se prueba el escaneo reutilizando el catalogo (`shots`) como cache de
//! dimensiones; sin ella, el escaneo completo leyendo la cabecera de cada imagen. La base se abre
//! en solo lectura y se debe pasar una COPIA (nunca la de la biblioteca).
use std::path::PathBuf;
use std::time::Instant;
use visor_core::catalog;
use visor_core::config::default_platforms;
use visor_core::games::Stats;
use visor_core::scan::{scan_library_ex, ScanOpts};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = PathBuf::from(args.first().expect("falta la raiz de Capturas"));
    let platforms = default_platforms();

    // Fase 1: solo listar carpetas y archivos con su metadata (lo que cuesta el recorrido sin abrir imagenes).
    let t = Instant::now();
    let mut files = 0usize;
    let mut bytes = 0u64;
    for e in walkdir::WalkDir::new(&root).follow_links(false).into_iter().flatten() {
        if e.file_type().is_file() {
            files += 1;
            bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    eprintln!("recorrido + metadata: {files} archivos, {:.1} GB en {:?}", bytes as f64 / 1e9, t.elapsed());

    // Cabeceras "en frio": lee 4 KB de una muestra repartida de imagenes saltandose la cache de Windows
    // (FILE_FLAG_NO_BUFFERING), que es lo que costaria `imagesize` con la cache vacia; se extrapola al total.
    #[cfg(windows)]
    {
        use std::io::Read;
        use std::os::windows::fs::OpenOptionsExt;
        let imgs: Vec<PathBuf> = walkdir::WalkDir::new(&root)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file() && visor_core::scan::is_image(e.path()))
            .map(|e| e.into_path())
            .collect();
        let step = (imgs.len() / 300).max(1);
        let mut buf = vec![0u8; 16384];
        let off = buf.as_ptr().align_offset(4096);
        let mut n = 0usize;
        let t = Instant::now();
        for p in imgs.iter().step_by(step) {
            if let Ok(mut f) = std::fs::OpenOptions::new().read(true).custom_flags(0x2000_0000).open(p) {
                let _ = f.read(&mut buf[off..off + 4096]);
                n += 1;
            }
        }
        let per = t.elapsed().as_secs_f64() / n as f64;
        eprintln!("cabecera en frio: {n} muestras, {:.1} ms cada una => {:.0} s para {} imagenes", per * 1e3, per * imgs.len() as f64, imgs.len());
    }

    // Fase 2: escaneo completo como lo hace la app (sin acentos).
    for pasada in 1..=2 {
        let t = Instant::now();
        let lib = scan_library_ex(Some(&root), &platforms, &[], &Stats::new(), &ScanOpts { show_hidden: true, ..Default::default() });
        let shots: usize = lib.platforms.iter().flat_map(|p| &p.titles).map(|t| t.shots.len()).sum();
        let titles: usize = lib.platforms.iter().map(|p| p.titles.len()).sum();
        eprintln!("scan sin cache (pasada {pasada}): {titles} titulos, {shots} capturas en {:?}", t.elapsed());
    }

    // Fase 2a: coste de una miniatura (decodificar + reducir) de capturas reales; la cache va a una carpeta temporal.
    if let Some(out) = args.get(2) {
        let imgs: Vec<PathBuf> = walkdir::WalkDir::new(&root)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file() && visor_core::scan::is_image(e.path()))
            .map(|e| e.into_path())
            .collect();
        let step = (imgs.len() / 24).max(1);
        let out = PathBuf::from(out);
        let t = Instant::now();
        let mut n = 0;
        let mut mb = 0.0;
        for p in imgs.iter().step_by(step).take(24) {
            mb += std::fs::metadata(p).map(|m| m.len() as f64 / 1e6).unwrap_or(0.0);
            if visor_core::thumbs::ensure_thumb(&out, p).is_ok() {
                n += 1;
            }
        }
        eprintln!("miniaturas: {n} en {:?} ({:.0} ms cada una, {:.1} MB de media por imagen)", t.elapsed(), t.elapsed().as_secs_f64() * 1e3 / n as f64, mb / n as f64);
    }

    // Fase 2b: con acentos (cache real de AppData, solo lectura: no se guarda) y grafismos de SteamGridDB.
    {
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default().join("app.visorcapturas.desktop");
        let pal = visor_core::palette::PaletteCache::load(&local.join("palettes.json"));
        let art = root.parent().unwrap().join("Grafismos");
        for pasada in 1..=2 {
            let t = Instant::now();
            let opts = ScanOpts { show_hidden: true, palettes: Some(&pal), sgdb_dir: Some(&art), ..Default::default() };
            let _ = scan_library_ex(Some(&root), &platforms, &[], &Stats::new(), &opts);
            eprintln!("scan con acentos y grafismos sgdb (pasada {pasada}): {:?} (acentos calculados: {})", t.elapsed(), pal.computed.load(std::sync::atomic::Ordering::Relaxed));
        }
    }

    // Fase 3: coste de mandar la biblioteca a la interfaz (JSON del IPC).
    {
        let lib = scan_library_ex(Some(&root), &platforms, &[], &Stats::new(), &ScanOpts { show_hidden: true, ..Default::default() });
        let t = Instant::now();
        let json = serde_json::to_string(&lib)?;
        eprintln!("JSON de la biblioteca: {:.2} MB en {:?}", json.len() as f64 / 1e6, t.elapsed());
    }

    // Fase 4: con el catalogo como cache (y sincronizacion del catalogo) sobre una COPIA de la base.
    if let Some(db) = args.get(1) {
        let c = rusqlite::Connection::open(db)?;
        c.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        let t = Instant::now();
        let cache = catalog::load_cache(&c, &root)?;
        eprintln!("catalogo cargado: {} filas en {:?}", cache.len(), t.elapsed());
        let mut lib = None;
        for pasada in 1..=2 {
            let t = Instant::now();
            let l = scan_library_ex(
                Some(&root),
                &platforms,
                &[],
                &Stats::new(),
                &ScanOpts { show_hidden: true, shot_cache: Some(&cache), ..Default::default() },
            );
            let shots: usize = l.platforms.iter().flat_map(|p| &p.titles).map(|t| t.shots.len()).sum();
            eprintln!("scan con catalogo (pasada {pasada}): {shots} capturas en {:?}", t.elapsed());
            lib = Some(l);
        }
        let lib = lib.unwrap();
        // Sincronizacion nueva (solo filas distintas).
        for pasada in 1..=2 {
            let t = Instant::now();
            let rep = catalog::sync(&c, &root, &lib, 5)?;
            eprintln!("sync nuevo (pasada {pasada}): {rep:?} en {:?}", t.elapsed());
        }
        // Sincronizacion de antes: upsert de todas las filas.
        let t = Instant::now();
        {
            let tx = c.unchecked_transaction()?;
            {
                let mut up = tx.prepare(
                    "INSERT INTO shots(path, title_id, size, mtime_ms, date_ms, width, height, sha256, seen_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)
                     ON CONFLICT(path) DO UPDATE SET
                         sha256   = CASE WHEN size = excluded.size AND mtime_ms = excluded.mtime_ms THEN sha256 ELSE NULL END,
                         title_id = excluded.title_id, size = excluded.size, mtime_ms = excluded.mtime_ms,
                         date_ms  = excluded.date_ms, width = excluded.width, height = excluded.height, seen_ms = excluded.seen_ms",
                )?;
                for tl in lib.platforms.iter().flat_map(|p| &p.titles) {
                    for s in &tl.shots {
                        let Some(rel) = visor_core::relpath::to_rel(&root, &s.path) else { continue };
                        up.execute(rusqlite::params![rel, tl.id, s.size as i64, s.mtime, s.date, s.width, s.height, 9])?;
                    }
                }
            }
            tx.execute("DELETE FROM shots WHERE seen_ms <> ?1", [9])?;
            tx.commit()?;
        }
        eprintln!("sync de antes (todas las filas): {:?}", t.elapsed());
    }
    Ok(())
}
