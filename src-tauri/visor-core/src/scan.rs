use crate::art::{art_kind, find_art};
use crate::config::PlatformCfg;
use crate::dates::parse_date_from_filename;
use crate::games::{norm_key, Game, Launch, Source, Stats};
use crate::palette::{AccentInfo, PaletteCache};
use crate::userdata::{title_id, HiddenEntry, TitlePrefs};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

pub const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Shot {
    pub path: String,
    /// Fecha de modificacion (ms desde epoch).
    pub mtime: i64,
    /// Fecha mostrada: la del nombre del archivo si se reconoce, si no la de modificacion.
    pub date: i64,
    pub width: u32,
    pub height: u32,
    /// Tamano en bytes.
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Title {
    pub name: String,
    /// Portada: `cover.jpg|png` de la carpeta, si no la de Steam. Si es `None` la interfaz
    /// usa la primera captura.
    pub cover: Option<String>,
    /// Hero (banner 1920x620) de la carpeta (`hero.*`) o de Steam.
    #[serde(default)]
    pub hero: Option<String>,
    /// Logo transparente (`logo.png|webp`) o el de Steam.
    #[serde(default)]
    pub logo: Option<String>,
    pub shots: Vec<Shot>,
    /// Nombre real de la carpeta bajo la plataforma.
    #[serde(default)]
    pub folder: String,
    /// Ruta completa de la carpeta (puede no existir aun).
    #[serde(default)]
    pub dir: String,
    #[serde(default)]
    pub has_folder: bool,
    /// Juego vinculado (para poder lanzarlo).
    #[serde(default)]
    pub game_id: Option<String>,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub launchable: bool,
    /// Se puede quitar de la lista (entradas manuales y emuladores).
    #[serde(default)]
    pub removable: bool,
    #[serde(default)]
    pub last_played: Option<i64>,
    #[serde(default)]
    pub play_seconds: u64,
    /// Nombre corto del lanzador: el emulador.
    #[serde(default)]
    pub launcher: Option<String>,
    /// Partidas registradas (0 si solo hay tiempo del formato antiguo).
    #[serde(default)]
    pub sessions: u32,
    #[serde(default)]
    pub first_played: Option<i64>,
    /// Minutos jugados en cada uno de los ultimos 7 dias locales (el ultimo es hoy).
    #[serde(default)]
    pub week: Vec<u32>,
    /// Partidas `[inicio, fin]` en ms desde epoch (de la mas antigua a la mas reciente), para las estadisticas.
    #[serde(default)]
    pub plays: Vec<[i64; 2]>,
    /// Id estable (id del lanzador o `plataforma/carpeta`), clave de «oculto» y de las preferencias.
    #[serde(default)]
    pub id: String,
    /// Oculto por el usuario (solo aparece si se pidio mostrar los ocultos).
    #[serde(default)]
    pub hidden: bool,
    /// Acento extraido del hero y la portada, con su paleta.
    #[serde(default)]
    pub accent: Option<AccentInfo>,
    /// Acento manual `#rrggbb`, si el usuario lo fijo.
    #[serde(default)]
    pub accent_override: Option<String>,
    /// Orden de capturas recordado para este juego.
    #[serde(default)]
    pub sort_mode: Option<String>,
    /// Escalado con Magpie: modo elegido ("off" | "auto" | "2x" | "4x") y resumen de lo ultimo aplicado.
    #[serde(default)]
    pub magpie_mode: Option<String>,
    #[serde(default)]
    pub magpie_summary: Option<String>,
    /// Estado del juego ("pending" | "playing" | "finished" | "dropped") y fecha de fin (ms).
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub finished_ms: Option<i64>,
    /// «Mi lista»: nota (1-5) del terminado, sitio entre los pendientes, cuando se fijo el estado y «dejarlo» ante el aviso de parado.
    #[serde(default)]
    pub rating: Option<i64>,
    #[serde(default)]
    pub list_pos: Option<i64>,
    #[serde(default)]
    pub status_ms: Option<i64>,
    #[serde(default)]
    pub snooze_ms: Option<i64>,
    /// Fecha de lanzamiento (a mano o de SteamGridDB).
    #[serde(default)]
    pub release: Option<crate::userdata::ReleaseDate>,
}

impl Title {
    /// Titulo vacio (sin capturas, sin juego vinculado) para completar con `..Title::blank()`.
    fn blank() -> Title {
        Title {
            name: String::new(),
            cover: None,
            hero: None,
            logo: None,
            shots: Vec::new(),
            folder: String::new(),
            dir: String::new(),
            has_folder: false,
            game_id: None,
            source: None,
            launchable: false,
            removable: false,
            last_played: None,
            play_seconds: 0,
            launcher: None,
            sessions: 0,
            first_played: None,
            week: vec![0; 7],
            plays: Vec::new(),
            id: String::new(),
            hidden: false,
            accent: None,
            accent_override: None,
            sort_mode: None,
            magpie_mode: None,
            magpie_summary: None,
            status: None,
            finished_ms: None,
            rating: None,
            list_pos: None,
            status_ms: None,
            snooze_ms: None,
            release: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub id: String,
    pub name: String,
    pub folder: String,
    pub accent: u8,
    pub icon: String,
    pub titles: Vec<Title>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub platforms: Vec<Platform>,
    /// Juegos ocultos (con o sin carpeta) para la lista de Ajustes.
    #[serde(default)]
    pub hidden: Vec<HiddenEntry>,
}

/// Dimensiones ya conocidas de capturas (las del catalogo `shots`), por ruta absoluta, con el tamano y la fecha
/// de modificacion con que se leyeron: si el archivo no cambio, el escaneo no vuelve a abrirlo para leer su
/// cabecera (en un disco mecanico abrir miles de imagenes en cada escaneo tarda minutos).
#[derive(Default)]
pub struct ShotCache {
    map: HashMap<String, (u64, i64, u32, u32)>,
}

impl ShotCache {
    pub fn insert(&mut self, path: String, size: u64, mtime: i64, width: u32, height: u32) {
        self.map.insert(path, (size, mtime, width, height));
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Dimensiones conocidas de `path` si siguen valiendo (mismo tamano y fecha; 0x0 = no se pudo leer, se reintenta).
    fn dims(&self, path: &str, size: u64, mtime: i64) -> Option<(u32, u32)> {
        let &(s, m, w, h) = self.map.get(path)?;
        (s == size && m == mtime && w > 0 && h > 0).then_some((w, h))
    }
}

/// Opciones del escaneo ligadas a los datos del usuario.
#[derive(Default)]
pub struct ScanOpts<'a> {
    /// Dimensiones ya conocidas (catalogo): solo se leen las cabeceras de las imagenes nuevas o modificadas.
    pub shot_cache: Option<&'a ShotCache>,
    pub hidden: &'a [HiddenEntry],
    /// Mantener los ocultos en las listas (marcados `hidden`).
    pub show_hidden: bool,
    pub prefs: Option<&'a TitlePrefs>,
    /// Si se da, se calcula el acento de cada titulo (con cache).
    pub palettes: Option<&'a PaletteCache>,
    /// Cache de grafismos de SteamGridDB: rellena los que falten (la carpeta y Steam ganan).
    pub sgdb_dir: Option<&'a Path>,
}

pub fn is_image(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub(crate) fn mtime_ms(md: &fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Busca `folder` dentro de `root` sin distinguir mayusculas (Windows ya no lo hace; Linux si).
fn find_dir_ci(root: &Path, folder: &str) -> Option<PathBuf> {
    let exact = root.join(folder);
    if exact.is_dir() {
        return Some(exact);
    }
    fs::read_dir(root).ok()?.flatten().find_map(|e| {
        let p = e.path();
        (p.is_dir() && e.file_name().to_string_lossy().to_lowercase() == folder.to_lowercase()).then_some(p)
    })
}

fn scan_title(dir: &Path, cache: Option<&ShotCache>) -> Option<Title> {
    let name = dir.file_name()?.to_string_lossy().into_owned();
    let mut shots = Vec::new();
    for entry in WalkDir::new(dir).follow_links(false).into_iter().flatten() {
        if !entry.file_type().is_file() || !is_image(entry.path()) {
            continue;
        }
        if entry.depth() == 1 && art_kind(entry.path()).is_some() {
            continue; // grafismo (cover/hero/logo), no una captura
        }
        let Ok(md) = entry.metadata() else { continue };
        let mtime = mtime_ms(&md);
        let fname = entry.file_name().to_string_lossy();
        let date = parse_date_from_filename(&fname).unwrap_or(mtime);
        let path = entry.path().to_string_lossy().into_owned();
        let (width, height) = cache.and_then(|c| c.dims(&path, md.len(), mtime)).unwrap_or_else(|| {
            imagesize::size(entry.path()).map(|s| (s.width as u32, s.height as u32)).unwrap_or((0, 0))
        });
        shots.push(Shot { path, mtime, date, width, height, size: md.len() });
    }
    if shots.is_empty() {
        return None;
    }
    let art = find_art(dir);
    shots.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.path.cmp(&b.path)));
    Some(Title {
        folder: name.clone(),
        dir: dir.to_string_lossy().into_owned(),
        has_folder: true,
        name,
        cover: art.cover,
        hero: art.hero,
        logo: art.logo,
        shots,
        ..Title::blank()
    })
}

fn link_game(t: &mut Title, g: &Game, stats: &Stats) {
    t.name = g.name.clone();
    t.game_id = Some(g.id.clone());
    t.source = Some(g.source);
    t.launchable = !matches!(g.launch, Launch::None);
    t.removable = g.removable;
    t.launcher = g.launcher_label();
    // Los grafismos de la carpeta del usuario ganan a los de Steam.
    t.cover = t.cover.take().or_else(|| g.cover.clone());
    t.hero = t.hero.take().or_else(|| g.hero.clone());
    t.logo = t.logo.take().or_else(|| g.logo.clone());
    if let Some(st) = stats.get(&g.id) {
        t.last_played = (st.last_played > 0).then_some(st.last_played);
        t.play_seconds = st.seconds;
        t.sessions = st.session_count();
        t.first_played = st.first_played();
        t.week = st.week_minutes_local();
        t.plays = st.sessions.iter().map(|x| [x.start, x.end]).collect();
    }
}

/// Recorre `Raiz/<Plataforma>/<Titulo>/*.png|jpg|jpeg|webp` y vincula los juegos detectados.
/// La lista de titulos es la union de carpetas con capturas y juegos (aunque aun no tengan capturas).
/// Nunca modifica nada. Con raiz ausente o inexistente devuelve igualmente las plataformas
/// configuradas (con los juegos, si los hay).
pub fn scan_library(root: Option<&Path>, platforms: &[PlatformCfg], games: &[Game], stats: &Stats) -> Library {
    scan_library_ex(root, platforms, games, stats, &ScanOpts::default())
}

/// Como `scan_library`, con juegos ocultos, preferencias por juego y acentos extraidos del arte.
pub fn scan_library_ex(root: Option<&Path>, platforms: &[PlatformCfg], games: &[Game], stats: &Stats, opts: &ScanOpts) -> Library {
    let hidden_ids: HashSet<&str> = opts.hidden.iter().map(|h| h.id.as_str()).collect();
    let platforms = platforms
        .iter()
        .map(|cfg| {
            let pdir = root.and_then(|r| find_dir_ci(r, &cfg.folder));
            let mut titles: Vec<Title> = pdir
                .as_ref()
                .map(|pdir| {
                    let dirs: Vec<PathBuf> = fs::read_dir(pdir)
                        .map(|rd| {
                            // `file_type` sale del listado (sin consulta por carpeta); solo los enlaces se resuelven.
                            rd.flatten().filter(|e| e.file_type().map(|t| t.is_dir() || (t.is_symlink() && e.path().is_dir())).unwrap_or(false)).map(|e| e.path()).collect()
                        })
                        .unwrap_or_default();
                    dirs.par_iter().filter_map(|d| scan_title(d, opts.shot_cache)).collect()
                })
                .unwrap_or_default();
            // Directorio de plataforma donde viviran las carpetas nuevas (aunque no exista todavia).
            let base = pdir.clone().or_else(|| root.map(|r| r.join(&cfg.folder)));
            for g in games.iter().filter(|g| g.platform == cfg.id) {
                let key = norm_key(&g.folder);
                let hit = titles.iter().position(|t| t.game_id.is_none() && norm_key(&t.folder) == key);
                if let Some(i) = hit {
                    link_game(&mut titles[i], g, stats);
                } else if titles.iter().any(|t| t.game_id.is_some() && norm_key(&t.folder) == key) {
                    continue; // dos juegos con la misma carpeta: gana el primero
                } else {
                    let dir = base.as_ref().map(|b| b.join(&g.folder));
                    let art = dir.as_deref().map(find_art).unwrap_or_default();
                    let mut t = Title {
                        name: g.name.clone(),
                        cover: art.cover,
                        hero: art.hero,
                        logo: art.logo,
                        folder: g.folder.clone(),
                        dir: dir.as_ref().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(),
                        has_folder: dir.as_ref().map(|d| d.is_dir()).unwrap_or(false),
                        ..Title::blank()
                    };
                    link_game(&mut t, g, stats);
                    titles.push(t);
                }
            }
            // Jugados recientemente primero; despues la captura mas reciente; desempate por nombre.
            titles.sort_by(|a, b| {
                b.last_played
                    .cmp(&a.last_played)
                    .then_with(|| b.shots.first().map(|s| s.date).cmp(&a.shots.first().map(|s| s.date)))
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            for t in titles.iter_mut() {
                t.id = title_id(t.game_id.as_deref(), &cfg.id, &t.folder);
                t.hidden = hidden_ids.contains(t.id.as_str());
                if let Some(base) = opts.sgdb_dir {
                    if t.cover.is_none() || t.hero.is_none() || t.logo.is_none() {
                        let a = crate::sgdb::find_cached(base, &t.id);
                        t.cover = t.cover.take().or(a.cover);
                        t.hero = t.hero.take().or(a.hero);
                        t.logo = t.logo.take().or(a.logo);
                    }
                }
                if let Some(p) = opts.prefs.and_then(|p| p.get(&t.id)) {
                    t.accent_override = p.accent.clone();
                    t.sort_mode = p.sort.clone();
                    t.status = p.status.clone();
                    t.finished_ms = p.finished_ms;
                    t.rating = p.rating;
                    t.list_pos = p.list_pos;
                    t.status_ms = p.status_ms;
                    t.snooze_ms = p.snooze_ms;
                    t.release = p.release;
                    if let Some(m) = &p.magpie {
                        t.magpie_mode = Some(m.mode.clone());
                        t.magpie_summary = m.summary.clone();
                    }
                }
            }
            if !opts.show_hidden {
                titles.retain(|t| !t.hidden);
            }
            if let Some(cache) = opts.palettes {
                titles.par_iter_mut().for_each(|t| {
                    let cover = t.cover.clone().or_else(|| t.shots.first().map(|s| s.path.clone()));
                    t.accent = cache.accent_for(t.hero.as_deref().map(Path::new), cover.as_deref().map(Path::new));
                });
            }
            Platform {
                id: cfg.id.clone(),
                name: cfg.name.clone(),
                folder: cfg.folder.clone(),
                accent: cfg.accent,
                icon: cfg.icon.clone(),
                titles,
            }
        })
        .collect();
    Library { platforms, hidden: opts.hidden.to_vec() }
}

/// Orden natural sin distinguir mayusculas: `captura2` antes que `captura10`.
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Equal,
            (None, _) => return Less,
            (_, None) => return Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let mut n = String::new();
                while let Some(c) = x.peek().copied().filter(|c| c.is_ascii_digit()) {
                    n.push(c);
                    x.next();
                }
                let mut m = String::new();
                while let Some(c) = y.peek().copied().filter(|c| c.is_ascii_digit()) {
                    m.push(c);
                    y.next();
                }
                let (n, m) = (n.trim_start_matches('0'), m.trim_start_matches('0'));
                match n.len().cmp(&m.len()).then_with(|| n.cmp(m)) {
                    Equal => {}
                    o => return o,
                }
            }
            (Some(c), Some(d)) => match c.cmp(&d) {
                Equal => {
                    x.next();
                    y.next();
                }
                o => return o,
            },
        }
    }
}

/// Ordena capturas: `recent` (mas recientes), `oldest` (mas antiguas), `favs` (favoritas primero, cada
/// grupo por fecha) o `name` (por nombre de archivo). El resto de modos se trata como `recent`.
/// La interfaz aplica estos mismos criterios en TypeScript (`sortShots` en `store.svelte.ts`).
pub fn sort_shots(shots: &mut [Shot], mode: &str, favs: &HashSet<String>) {
    let recent = |a: &Shot, b: &Shot| b.date.cmp(&a.date).then_with(|| a.path.cmp(&b.path));
    match mode {
        "oldest" => shots.sort_by(|a, b| recent(b, a)),
        "favs" => shots.sort_by(|a, b| favs.contains(&b.path).cmp(&favs.contains(&a.path)).then_with(|| recent(a, b))),
        "name" => shots.sort_by(|a, b| {
            let name = |s: &Shot| s.path.rsplit(['/', '\\']).next().unwrap_or(&s.path).to_string();
            natural_cmp(&name(a), &name(b)).then_with(|| a.path.cmp(&b.path))
        }),
        _ => shots.sort_by(recent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_platforms;

    fn png(path: &Path, w: u32, h: u32) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbImage::new(w, h).save(path).unwrap();
    }

    #[test]
    fn missing_root_gives_four_empty_platforms() {
        let lib = scan_library(Some(Path::new("/no/existe")), &default_platforms(), &[], &Stats::new());
        assert_eq!(lib.platforms.len(), 4);
        assert!(lib.platforms.iter().all(|p| p.titles.is_empty()));
        let lib = scan_library(None, &default_platforms(), &[], &Stats::new());
        assert_eq!(lib.platforms[2].name, "Novelas visuales");
    }

    #[test]
    fn scans_titles_sorted_with_dates_dims_and_cover() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        png(&r.join("PC/Hades II/2025.11.02 - 08.15.30.12.png"), 40, 20);
        png(&r.join("PC/Hades II/20250101000000_1.png"), 30, 10);
        png(&r.join("PC/Hades II/cover.png"), 8, 12);
        png(&r.join("Switch/Zelda/sin fecha.png"), 16, 9);
        fs::create_dir_all(r.join("Switch/Vacio")).unwrap();
        fs::write(r.join("Switch/Zelda/notas.txt"), "x").unwrap();

        let lib = scan_library(Some(r), &default_platforms(), &[], &Stats::new());
        let pc = &lib.platforms[0];
        let hades = pc.titles.iter().find(|t| t.name == "Hades II").unwrap();
        assert_eq!(hades.shots.len(), 2);
        assert!(hades.shots[0].path.contains("2025.11.02"), "la mas reciente va primero");
        assert_eq!((hades.shots[0].width, hades.shots[0].height), (40, 20));
        assert!(hades.cover.as_deref().unwrap().ends_with("cover.png"));
        assert!(hades.shots.iter().all(|s| !s.path.ends_with("cover.png")));
        let sw = &lib.platforms[1];
        assert_eq!(sw.titles.len(), 1, "carpetas sin imagenes se omiten");
        let z = &sw.titles[0].shots[0];
        assert_eq!(z.date, z.mtime, "sin fecha en el nombre se usa la de modificacion");
        assert!(sw.titles[0].cover.is_none());
    }

    #[test]
    fn shot_cache_replaces_header_reads_only_while_the_file_is_unchanged() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        png(&r.join("PC/Hades II/a.png"), 40, 20);
        let plats = default_platforms();
        let lib = scan_library(Some(r), &plats, &[], &Stats::new());
        let s = lib.platforms[0].titles[0].shots[0].clone();
        assert_eq!((s.width, s.height), (40, 20));
        let dims_with = |size: u64, mtime: i64, w: u32, h: u32| {
            let mut cache = ShotCache::default();
            cache.insert(s.path.clone(), size, mtime, w, h);
            let lib = scan_library_ex(Some(r), &plats, &[], &Stats::new(), &ScanOpts { shot_cache: Some(&cache), ..Default::default() });
            let t = &lib.platforms[0].titles[0].shots[0];
            (t.width, t.height)
        };
        assert_eq!(dims_with(s.size, s.mtime, 7, 9), (7, 9), "mismo tamano y fecha: se usan las dimensiones guardadas");
        assert_eq!(dims_with(s.size + 1, s.mtime, 7, 9), (40, 20), "cambio de tamano: se lee de nuevo");
        assert_eq!(dims_with(s.size, s.mtime + 1, 7, 9), (40, 20), "cambio de fecha: se lee de nuevo");
        assert_eq!(dims_with(s.size, s.mtime, 0, 0), (40, 20), "una lectura fallida (0x0) se reintenta");
    }

    #[test]
    fn platform_folder_is_case_insensitive() {
        let d = tempfile::tempdir().unwrap();
        png(&d.path().join("switch/Kirby/a.jpg.png"), 4, 4);
        let lib = scan_library(Some(d.path()), &default_platforms(), &[], &Stats::new());
        assert_eq!(lib.platforms[1].titles.len(), 1);
    }

    fn game(id: &str, name: &str, platform: &str, folder: &str) -> Game {
        use crate::games::Launch;
        Game {
            id: id.into(), name: name.into(), source: Source::Manual, platform: platform.into(), folder: folder.into(),
            launch: Launch::Exe { exe: "a.exe".into(), args: String::new() }, install_dir: None, cover: Some("/cache/1.jpg".into()),
            hero: Some("/cache/1_hero.jpg".into()), logo: Some("/cache/1_logo.png".into()), removable: false,
        }
    }

    #[test]
    fn links_games_with_folders_and_lists_the_union() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        png(&r.join("PC/ELDEN RING/20240101000000_1.png"), 8, 8);
        png(&r.join("PC/Carpeta Suelta/20230101000000_1.png"), 8, 8);
        png(&r.join("PC/Hades II/cover.png"), 8, 8); // sin capturas pero con portada propia
        let games = vec![
            game("steam:1", "Elden Ring", "pc", "Elden Ring"),
            game("steam:2", "Hades II", "pc", "Hades II"),
            game("steam:3", "Stardew Valley", "pc", "Stardew Valley"),
            game("manual:4", "Zelda", "switch", "Zelda"),
            game("steam:9", "Duplicado", "pc", "elden ring"),
        ];
        let mut stats = Stats::new();
        stats.insert("steam:3".into(), crate::games::PlayStat { last_played: 5000, seconds: 3600, ..Default::default() });
        let lib = scan_library(Some(r), &default_platforms(), &games, &stats);
        let pc = &lib.platforms[0];
        let names: Vec<_> = pc.titles.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["Stardew Valley", "Elden Ring", "Carpeta Suelta", "Hades II"], "jugado primero, luego capturas, luego nombre");
        let er = &pc.titles[1];
        assert_eq!(er.folder, "ELDEN RING", "se reutiliza la carpeta existente");
        assert_eq!(er.game_id.as_deref(), Some("steam:1"));
        assert!(er.launchable && er.shots.len() == 1 && er.has_folder);
        assert_eq!(er.cover.as_deref(), Some("/cache/1.jpg"), "sin cover.jpg se usa la portada de Steam");
        assert!(!pc.titles[2].launchable, "carpeta sin juego: solo capturas");
        let hades = &pc.titles[3];
        assert!(hades.shots.is_empty() && hades.launchable);
        assert!(hades.cover.as_deref().unwrap().ends_with("cover.png"), "cover.jpg propio gana a la cache de Steam");
        let sv = &pc.titles[0];
        assert!(!sv.has_folder && sv.shots.is_empty());
        assert_eq!((sv.last_played, sv.play_seconds), (Some(5000), 3600));
        assert!(sv.dir.ends_with("Stardew Valley"));
        assert!(!pc.titles.iter().any(|t| t.game_id.as_deref() == Some("steam:9")));
        let sw = &lib.platforms[1];
        assert_eq!(sw.titles.len(), 1);
        assert!(sw.titles[0].dir.ends_with("Switch/Zelda") || sw.titles[0].dir.ends_with("Switch\\Zelda"));
    }

    #[test]
    fn games_show_without_a_root() {
        let games = vec![game("steam:1", "Elden Ring", "pc", "Elden Ring")];
        let lib = scan_library(None, &default_platforms(), &games, &Stats::new());
        assert_eq!(lib.platforms[0].titles.len(), 1);
        assert!(lib.platforms[0].titles[0].dir.is_empty());
    }

    #[test]
    fn title_art_from_folder_beats_steam_cache_and_is_not_a_shot() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        png(&r.join("PC/Elden Ring/20240101000000_1.png"), 8, 8);
        png(&r.join("PC/Elden Ring/Hero.png"), 8, 8);
        png(&r.join("PC/Elden Ring/logo.png"), 8, 8);
        png(&r.join("PC/Hades II/20240101000000_1.png"), 8, 8);
        png(&r.join("PC/Hades II/cover.jpg"), 8, 8);
        png(&r.join("PC/Hades II/sub/hero.png"), 8, 8); // en subcarpeta: es una captura mas
        let games = vec![game("steam:1", "Elden Ring", "pc", "Elden Ring"), game("steam:2", "Hades II", "pc", "Hades II")];
        let lib = scan_library(Some(r), &default_platforms(), &games, &Stats::new());
        let pc = &lib.platforms[0];
        let er = pc.titles.iter().find(|t| t.name == "Elden Ring").unwrap();
        assert!(er.hero.as_deref().unwrap().ends_with("Hero.png"), "hero de la carpeta gana a la cache de Steam");
        assert!(er.logo.as_deref().unwrap().ends_with("logo.png"));
        assert_eq!(er.cover.as_deref(), Some("/cache/1.jpg"), "sin cover propia se usa la de Steam");
        assert_eq!(er.shots.len(), 1, "hero y logo no cuentan como capturas");
        let h = pc.titles.iter().find(|t| t.name == "Hades II").unwrap();
        assert_eq!(h.hero.as_deref(), Some("/cache/1_hero.jpg"), "sin hero propio, el de Steam");
        assert_eq!(h.logo.as_deref(), Some("/cache/1_logo.png"));
        assert!(h.cover.as_deref().unwrap().ends_with("cover.jpg"));
        assert_eq!(h.shots.len(), 2);
        // Sin juego vinculado ni arte: todo vacio.
        png(&r.join("Switch/Zelda/a.png"), 4, 4);
        let lib = scan_library(Some(r), &default_platforms(), &[], &Stats::new());
        let z = &lib.platforms[1].titles[0];
        assert!(z.hero.is_none() && z.logo.is_none() && z.cover.is_none() && z.week == vec![0; 7]);
    }

    #[test]
    fn gameless_folder_art_and_stats_reach_the_title() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        png(&r.join("PC/Nuevo/hero.webp"), 8, 8);
        let mut g = game("steam:5", "Nuevo", "pc", "Nuevo");
        g.hero = None;
        g.logo = None;
        g.source = Source::Emulator;
        g.launch = crate::games::Launch::Emulator { exe: "C:\\Emus\\ryujinx.exe".into(), args: String::new(), rom: "r".into() };
        let now = chrono::Local::now().timestamp_millis();
        let mut stats = Stats::new();
        crate::games::add_session(&mut stats, "steam:5", now - 3_600_000, now - 1_800_000);
        let lib = scan_library(Some(r), &default_platforms(), &[g], &stats);
        let t = &lib.platforms[0].titles[0];
        assert!(t.hero.as_deref().unwrap().ends_with("hero.webp"));
        assert!(t.logo.is_none());
        assert_eq!(t.launcher.as_deref(), Some("Ryujinx"));
        assert_eq!((t.sessions, t.play_seconds), (1, 1800));
        assert!(t.first_played.is_some());
        assert_eq!(t.plays, vec![[now - 3_600_000, now - 1_800_000]]);
        assert_eq!(t.week.len(), 7);
        assert!(t.week.iter().sum::<u32>() >= 29, "la partida de hoy aparece en la semana: {:?}", t.week);
    }

    fn shot(path: &str, date: i64) -> Shot {
        Shot { path: path.into(), mtime: date, date, width: 1, height: 1, size: 1 }
    }

    #[test]
    fn sort_orders() {
        let mk = || vec![shot("/a/Juego_10.png", 300), shot("/a/juego_2.png", 100), shot("/a/Juego_1.png", 200), shot("/a/zeta.png", 200)];
        let names = |v: &[Shot]| v.iter().map(|s| s.path.rsplit('/').next().unwrap().to_string()).collect::<Vec<_>>();
        let favs: HashSet<String> = ["/a/juego_2.png".to_string(), "/a/zeta.png".to_string()].into();
        let mut v = mk();
        sort_shots(&mut v, "recent", &favs);
        assert_eq!(names(&v), vec!["Juego_10.png", "Juego_1.png", "zeta.png", "juego_2.png"]);
        sort_shots(&mut v, "oldest", &favs);
        assert_eq!(names(&v), vec!["juego_2.png", "zeta.png", "Juego_1.png", "Juego_10.png"]);
        sort_shots(&mut v, "favs", &favs);
        assert_eq!(names(&v), vec!["zeta.png", "juego_2.png", "Juego_10.png", "Juego_1.png"], "favoritas primero, cada grupo reciente primero");
        sort_shots(&mut v, "name", &favs);
        assert_eq!(names(&v), vec!["Juego_1.png", "juego_2.png", "Juego_10.png", "zeta.png"], "natural y sin distinguir mayusculas");
        sort_shots(&mut v, "desconocido", &favs);
        assert_eq!(names(&v)[0], "Juego_10.png");
        let mut w = vec![shot(r"D:\x\b.png", 1), shot(r"D:\x\a.png", 2)];
        sort_shots(&mut w, "name", &favs);
        assert!(w[0].path.ends_with("a.png"));
    }

    #[test]
    fn hidden_titles_disappear_unless_shown_and_prefs_attach() {
        use crate::userdata::{TitlePref, TitlePrefs};
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        png(&r.join("PC/Elden Ring/20240101000000_1.png"), 8, 8);
        png(&r.join("PC/Carpeta Suelta/20230101000000_1.png"), 8, 8);
        png(&r.join("Switch/Zelda/a.png"), 8, 8);
        let games = vec![game("steam:1", "Elden Ring", "pc", "Elden Ring")];
        let hid = |id: &str| HiddenEntry { id: id.into(), name: id.into(), platform: "pc".into() };
        let hidden = vec![hid("steam:1"), hid("pc/carpeta suelta")];
        let mut prefs = TitlePrefs::new();
        prefs.insert("switch/zelda".into(), TitlePref { accent: Some("#112233".into()), sort: Some("name".into()), ..Default::default() });
        let plats = default_platforms();
        let lib = scan_library_ex(Some(r), &plats, &games, &Stats::new(), &ScanOpts { hidden: &hidden, prefs: Some(&prefs), ..Default::default() });
        assert!(lib.platforms[0].titles.is_empty(), "ocultos fuera de la lista (el juego vinculado y la carpeta suelta)");
        assert_eq!(lib.hidden.len(), 2, "pero siguen en la lista de ocultos para poder mostrarlos");
        let z = &lib.platforms[1].titles[0];
        assert_eq!((z.id.as_str(), z.hidden), ("switch/zelda", false));
        assert_eq!((z.accent_override.as_deref(), z.sort_mode.as_deref()), (Some("#112233"), Some("name")));
        let lib = scan_library_ex(Some(r), &plats, &games, &Stats::new(), &ScanOpts { hidden: &hidden, show_hidden: true, ..Default::default() });
        let pc = &lib.platforms[0];
        assert_eq!(pc.titles.len(), 2);
        assert!(pc.titles.iter().all(|t| t.hidden));
        assert_eq!(pc.titles.iter().find(|t| t.name == "Elden Ring").unwrap().id, "steam:1");
        assert!(r.join("PC/Elden Ring/20240101000000_1.png").exists(), "los archivos no se tocan");
        assert_eq!(scan_library(Some(r), &plats, &games, &Stats::new()).platforms[0].titles.len(), 2);
    }

    #[test]
    fn accent_is_extracted_from_hero_and_cover_with_fallbacks() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        let solid = |p: &Path, rgb: [u8; 3]| {
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            image::RgbImage::from_pixel(40, 20, image::Rgb(rgb)).save(p).unwrap();
        };
        solid(&r.join("PC/Rojo/hero.png"), [220, 30, 40]);
        solid(&r.join("PC/Rojo/20240101000000_1.png"), [10, 10, 10]);
        solid(&r.join("PC/Gris/20240101000000_1.png"), [120, 120, 120]);
        solid(&r.join("PC/Azul/20240101000000_1.png"), [20, 60, 230]); // sin hero ni portada: la primera captura
        let cache = PaletteCache::default();
        let lib = scan_library_ex(Some(r), &default_platforms(), &[], &Stats::new(), &ScanOpts { palettes: Some(&cache), ..Default::default() });
        let by = |n: &str| lib.platforms[0].titles.iter().find(|t| t.name == n).unwrap();
        assert!(by("Rojo").accent.is_some());
        assert!(by("Gris").accent.is_none(), "sin color: la interfaz usa el arcoiris");
        let blue = by("Azul").accent.as_ref().unwrap();
        assert!(blue.accent.color.starts_with('#') && !blue.palette.is_empty());
        let n = cache.computed.load(std::sync::atomic::Ordering::Relaxed);
        let _ = scan_library_ex(Some(r), &default_platforms(), &[], &Stats::new(), &ScanOpts { palettes: Some(&cache), ..Default::default() });
        assert_eq!(cache.computed.load(std::sync::atomic::Ordering::Relaxed), n, "segunda pasada: de la cache");
    }

    #[test]
    fn sgdb_art_only_fills_gaps() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path().join("lib");
        let c = d.path().join("sgdb");
        png(&r.join("PC/Hades II/20240101000000_1.png"), 8, 8);
        png(&r.join("PC/Hades II/hero.png"), 8, 8);
        crate::sgdb::store_image(&c, "pc/hades ii", crate::art::ArtKind::Hero, 1, "png", b"x").unwrap();
        crate::sgdb::store_image(&c, "pc/hades ii", crate::art::ArtKind::Cover, 2, "png", b"x").unwrap();
        let lib = scan_library_ex(Some(&r), &default_platforms(), &[], &Stats::new(), &ScanOpts { sgdb_dir: Some(&c), ..Default::default() });
        let t = &lib.platforms[0].titles[0];
        assert!(t.hero.as_deref().unwrap().ends_with("hero.png"), "el hero de la carpeta gana");
        assert!(t.cover.as_deref().unwrap().ends_with("cover-2.png"), "sin portada propia, la de SteamGridDB");
        assert!(t.logo.is_none());
    }
}
