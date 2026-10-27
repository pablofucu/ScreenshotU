//! Juegos anadidos a mano (ejecutables, emuladores, solo capturas), vinculacion con las
//! carpetas de capturas y estadisticas de juego. Todo es puro y se prueba sin Tauri.
use chrono::{DateTime, Duration, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Manual,
    Emulator,
}

/// Como se lanza un juego.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Launch {
    /// Un ejecutable con argumentos opcionales (cadena tal y como la escribe el usuario).
    /// `workdir` vacio = la carpeta del ejecutable.
    Exe {
        exe: String,
        args: String,
        #[serde(default)]
        workdir: String,
    },
    /// Emulador + plantilla de argumentos con `{rom}` + ruta de la ROM.
    Emulator { exe: String, args: String, rom: String },
    /// Sin lanzador: solo una carpeta de capturas que se quiere tener en la lista.
    None,
}

impl Launch {
    /// Para `Exe` y `Emulator`: programa y argumentos ya separados.
    pub fn command(&self) -> Option<(String, Vec<String>)> {
        match self {
            Launch::None => None,
            Launch::Exe { exe, args, .. } => Some((exe.clone(), split_args(args))),
            Launch::Emulator { exe, args, rom } => Some((exe.clone(), build_emulator_args(args, rom))),
        }
    }

    /// Carpeta en la que arranca el programa, si se eligió una distinta a la de su ejecutable.
    pub fn workdir(&self) -> Option<&str> {
        match self {
            Launch::Exe { workdir, .. } if !workdir.is_empty() => Some(workdir),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    /// `manual:<id>`.
    pub id: String,
    pub name: String,
    pub source: Source,
    /// Id de plataforma de la configuracion ("pc", "switch"...).
    pub platform: String,
    /// Nombre de carpeta bajo `Raiz/<Plataforma>/`.
    pub folder: String,
    pub launch: Launch,
    /// Carpeta de instalacion, usada para reconocer el proceso del juego.
    pub install_dir: Option<String>,
    /// Portada local (cache de Steam), si existe.
    pub cover: Option<String>,
    /// Hero 1920x620 de la cache de Steam, si existe.
    #[serde(default)]
    pub hero: Option<String>,
    /// Logo transparente de la cache de Steam, si existe.
    #[serde(default)]
    pub logo: Option<String>,
    /// Viene de `games.json` (anadido a mano): se puede quitar de la lista.
    #[serde(default)]
    pub removable: bool,
}

impl Game {
    /// Nombre corto del lanzador para la interfaz: el emulador (por su ejecutable).
    pub fn launcher_label(&self) -> Option<String> {
        match self.source {
            Source::Emulator => match &self.launch {
                Launch::Emulator { exe, .. } => Some(emulator_label(exe)),
                _ => None,
            },
            Source::Manual => None,
        }
    }
}

/// `C:\Emus\ryujinx.exe` -> «Ryujinx»; `yuzu-early-access.exe` -> «yuzu-early-access».
pub fn emulator_label(exe: &str) -> String {
    let file = exe.rsplit(['\\', '/']).next().unwrap_or(exe);
    let stem = file.rsplit_once('.').map(|(a, _)| a).unwrap_or(file).trim();
    if stem.is_empty() {
        return "emulador".into();
    }
    if stem.chars().any(|c| c.is_uppercase()) {
        return stem.to_string();
    }
    let mut ch = stem.chars();
    ch.next().map(|f| f.to_uppercase().collect::<String>() + ch.as_str()).unwrap_or_default()
}

/* ------------------------------------------------------------------ */
/* Nombres de carpeta                                                   */
/* ------------------------------------------------------------------ */

const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1", "lpt2",
    "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Convierte el nombre de un juego en un nombre de carpeta valido en Windows.
/// `Zelda: Tears` -> `Zelda - Tears`; se quitan `<>"/\|?*`, simbolos de marca y espacios/puntos finales.
pub fn sanitize_folder_name(name: &str) -> String {
    let mut s = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            ':' => s.push_str(" - "),
            '<' | '>' | '"' | '/' | '\\' | '|' | '?' | '*' | '™' | '®' | '©' => {}
            c if c.is_control() => {}
            c => s.push(c),
        }
    }
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s = s.trim_matches(|c: char| c == '.' || c.is_whitespace()).to_string();
    if s.is_empty() {
        return "Juego".into();
    }
    if RESERVED.contains(&s.to_lowercase().as_str()) {
        return format!("{s}_");
    }
    s
}

/// Clave de comparacion tolerante: minusculas y solo letras/digitos.
pub fn norm_key(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// Nombre real de carpeta para un juego: si ya hay una carpeta equivalente en `platform_dir`
/// (mismo nombre sin distinguir mayusculas/signos) se reutiliza; si no, el deseado.
pub fn resolve_folder(platform_dir: &Path, desired: &str) -> String {
    let want = norm_key(desired);
    if let Ok(rd) = fs::read_dir(platform_dir) {
        let mut exact: Option<String> = None;
        let mut loose: Option<String> = None;
        for e in rd.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let n = e.file_name().to_string_lossy().into_owned();
            if n == desired {
                exact = Some(n);
            } else if n.to_lowercase() == desired.to_lowercase() {
                loose = loose.or(Some(n));
            } else if !want.is_empty() && norm_key(&n) == want {
                loose = loose.or(Some(n));
            }
        }
        if let Some(n) = exact.or(loose) {
            return n;
        }
    }
    desired.to_string()
}

/* ------------------------------------------------------------------ */
/* Argumentos                                                           */
/* ------------------------------------------------------------------ */

/// Separa una linea de argumentos respetando comillas dobles. La barra invertida es literal
/// (rutas de Windows). `"a b" c` -> `["a b", "c"]`.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut has = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_q = !in_q;
                has = true;
            }
            c if c.is_whitespace() && !in_q => {
                if has {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            c => {
                cur.push(c);
                has = true;
            }
        }
    }
    if has {
        out.push(cur);
    }
    out
}

/// Aplica la plantilla del emulador: `{rom}` se sustituye por la ruta (un solo argumento aunque tenga
/// espacios). Sin plantilla, o si no contiene `{rom}`, la ROM se anade al final.
pub fn build_emulator_args(template: &str, rom: &str) -> Vec<String> {
    let mut toks = split_args(template);
    let mut used = false;
    for t in toks.iter_mut() {
        if t.contains("{rom}") {
            *t = t.replace("{rom}", rom);
            used = true;
        }
    }
    if !used && !rom.is_empty() {
        toks.push(rom.to_string());
    }
    toks
}

/* ------------------------------------------------------------------ */
/* Rutas                                                                */
/* ------------------------------------------------------------------ */

fn norm_path(p: &str) -> String {
    let mut s = p.replace('\\', "/").to_lowercase();
    while s.ends_with('/') {
        s.pop();
    }
    s
}

/// `path` esta dentro de `dir` (comparacion sin mayusculas y con `/` o `\`).
pub fn path_inside(path: &str, dir: &str) -> bool {
    let d = norm_path(dir);
    if d.is_empty() {
        return false;
    }
    norm_path(path).starts_with(&format!("{d}/"))
}

/// Juego (instalado, no emulador) al que pertenece un ejecutable; gana la carpeta mas especifica.
pub fn game_for_exe<'a>(games: &'a [Game], exe: &str) -> Option<&'a Game> {
    games
        .iter()
        .filter(|g| g.source != Source::Emulator)
        .filter_map(|g| g.install_dir.as_deref().filter(|d| path_inside(exe, d)).map(|d| (d.len(), g)))
        .max_by_key(|(l, _)| *l)
        .map(|(_, g)| g)
}

/* ------------------------------------------------------------------ */
/* Entradas manuales (tabla `games`; formato antiguo: games.json)      */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ManualGame {
    pub id: String,
    pub name: String,
    /// "exe" | "emulator"
    pub kind: String,
    #[serde(default)]
    pub exe: String,
    #[serde(default)]
    pub args: String,
    #[serde(default)]
    pub rom: String,
    pub platform: String,
    #[serde(default)]
    pub folder: Option<String>,
    /// Carpeta de trabajo del `exe` (vacia = la de su ejecutable).
    #[serde(default)]
    pub workdir: String,
}

/// Formato del antiguo `games.json` (solo lo lee la importacion y los tests).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GamesFile {
    #[serde(default)]
    pub games: Vec<ManualGame>,
}

/// Id nuevo para una entrada manual (marca de tiempo en hexadecimal).
pub fn new_manual_id() -> String {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("manual:{n:x}")
}

/// Tipos de entrada de `games.json`.
pub const MANUAL_KINDS: &[&str] = &["exe", "emulator", "captures"];

/// Convierte una entrada de `games.json` en juego. Devuelve `None` para tipos desconocidos (p. ej. las
/// entradas de Steam de versiones anteriores).
pub fn manual_to_game(m: &ManualGame) -> Option<Game> {
    let emu = m.kind == "emulator";
    let exe_dir = Path::new(&m.exe).parent().map(|p| p.to_string_lossy().into_owned()).filter(|p| !p.is_empty());
    let workdir = m.workdir.trim().to_string();
    let exe_dir = if workdir.is_empty() || m.kind != "exe" { exe_dir } else { Some(workdir.clone()) };
    let folder = m.folder.clone().filter(|f| !f.trim().is_empty()).unwrap_or_else(|| sanitize_folder_name(&m.name));
    let base = |source: Source, launch: Launch, install_dir: Option<String>| Game {
        id: m.id.clone(),
        name: m.name.clone(),
        source,
        platform: m.platform.clone(),
        folder: folder.clone(),
        launch,
        install_dir,
        cover: None,
        hero: None,
        logo: None,
        removable: true,
    };
    Some(match m.kind.as_str() {
        "captures" => base(Source::Manual, Launch::None, None),
        "emulator" if emu => base(Source::Emulator, Launch::Emulator { exe: m.exe.clone(), args: m.args.clone(), rom: m.rom.clone() }, exe_dir),
        "exe" => base(Source::Manual, Launch::Exe { exe: m.exe.clone(), args: m.args.clone(), workdir }, exe_dir),
        _ => return None,
    })
}

/// Todos los juegos: las entradas de `games.json`.
pub fn all_games(manual: &[ManualGame]) -> Vec<Game> {
    let mut out: Vec<Game> = Vec::new();
    for m in manual {
        let Some(g) = manual_to_game(m) else { continue };
        if out.iter().any(|o| o.id == g.id) {
            continue;
        }
        out.push(g);
    }
    out
}

/* ------------------------------------------------------------------ */
/* Estadisticas de juego (tablas play_stats y sessions)                 */
/* ------------------------------------------------------------------ */

/// Una partida: inicio y fin en ms desde epoch.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub start: i64,
    pub end: i64,
}

/// Maximo de partidas guardadas por juego (las mas antiguas se descartan; el total no se pierde).
pub(crate) const MAX_SESSIONS: usize = 5000;

/// Estadisticas de un juego. Formato antiguo (`lastPlayed` + `seconds`) compatible: `sessions`
/// es opcional y `seconds` sigue siendo el total, incluido el tiempo anterior a guardar partidas.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayStat {
    /// ms desde epoch del ultimo inicio.
    pub last_played: i64,
    /// Total de segundos jugados.
    pub seconds: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<Session>,
}
pub type Stats = BTreeMap<String, PlayStat>;

impl PlayStat {
    pub fn session_count(&self) -> u32 {
        self.sessions.len() as u32
    }
    /// Inicio de la primera partida registrada (las migradas del formato antiguo no la tienen).
    pub fn first_played(&self) -> Option<i64> {
        self.sessions.iter().map(|s| s.start).min()
    }

    /// Segundos jugados en cada uno de los ultimos 7 dias locales (el ultimo es el dia de `now`).
    /// Las partidas que cruzan la medianoche se reparten entre los dos dias.
    pub fn week_seconds<Tz: TimeZone>(&self, now: DateTime<Tz>) -> [u64; 7] {
        let tz = now.timezone();
        let today = now.date_naive();
        let midnight = |d: NaiveDate| -> i64 {
            let t = d.and_hms_opt(0, 0, 0).unwrap();
            tz.from_local_datetime(&t)
                .earliest()
                .or_else(|| tz.from_local_datetime(&(t + Duration::hours(1))).earliest())
                .map(|x| x.timestamp_millis())
                .unwrap_or(0)
        };
        let mut out = [0u64; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            let day = today - Duration::days(6 - i as i64);
            let (from, to) = (midnight(day), midnight(day + Duration::days(1)));
            let ms: i64 = self.sessions.iter().map(|s| (s.end.min(to) - s.start.max(from)).max(0)).sum();
            *slot = (ms / 1000) as u64;
        }
        out
    }
    /// Minutos (redondeados) por dia de la ultima semana, del mas antiguo al de hoy.
    pub fn week_minutes<Tz: TimeZone>(&self, now: DateTime<Tz>) -> Vec<u32> {
        self.week_seconds(now).iter().map(|s| ((s + 30) / 60) as u32).collect()
    }
    pub fn week_minutes_local(&self) -> Vec<u32> {
        self.week_minutes(chrono::Local::now())
    }
}

/// Marca el inicio de una partida (actualiza `last_played`).
pub fn mark_started(st: &mut Stats, id: &str, now_ms: i64) {
    st.entry(id.to_string()).or_default().last_played = now_ms;
}
/// Si una partida nueva empieza menos de esto (ms) despues de terminar la anterior del mismo juego, es la misma:
/// se amplia la anterior y el hueco cuenta como tiempo jugado (reinicios del juego, cortes del vigilante...).
pub const MERGE_GAP_MS: i64 = 120_000;

/// Registra una partida terminada: suma su duracion al total y la anade a la lista. Si empieza menos de
/// `MERGE_GAP_MS` despues del fin de la ultima de ese juego (o dentro de ella), la amplia en vez de crear otra.
pub fn add_session(st: &mut Stats, id: &str, start_ms: i64, end_ms: i64) {
    let e = st.entry(id.to_string()).or_default();
    let end = end_ms.max(start_ms);
    if let Some(last) = e.sessions.last_mut() {
        if start_ms >= last.start && start_ms - last.end < MERGE_GAP_MS {
            if end > last.end {
                e.seconds += ((end - last.end) / 1000) as u64;
                last.end = end;
            }
            return;
        }
    }
    e.seconds += ((end - start_ms) / 1000) as u64;
    e.sessions.push(Session { start: start_ms, end });
    if e.sessions.len() > MAX_SESSIONS {
        let extra = e.sessions.len() - MAX_SESSIONS;
        e.sessions.drain(..extra);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_folder_name("Zelda: Tears of the Kingdom"), "Zelda - Tears of the Kingdom");
        assert_eq!(sanitize_folder_name("Hades II™"), "Hades II");
        assert_eq!(sanitize_folder_name("  What/If? *  "), "WhatIf");
        assert_eq!(sanitize_folder_name("Fin..."), "Fin");
        assert_eq!(sanitize_folder_name("CON"), "CON_");
        assert_eq!(sanitize_folder_name("???"), "Juego");
        assert_eq!(sanitize_folder_name("Steins;Gate"), "Steins;Gate");
    }

    #[test]
    fn splits_args_with_quotes_and_backslashes() {
        assert_eq!(split_args(r#"-f "C:\Mis Juegos\a.nsp" --x"#), vec!["-f", r"C:\Mis Juegos\a.nsp", "--x"]);
        assert!(split_args("   ").is_empty());
        assert_eq!(split_args(r#""" a"#), vec!["", "a"]);
    }

    #[test]
    fn emulator_args_template() {
        let rom = r"D:\ROMs\Super Mario.nsp";
        assert_eq!(build_emulator_args(r#"-g "{rom}" -f"#, rom), vec!["-g", rom, "-f"]);
        assert_eq!(build_emulator_args("--rom={rom}", rom), vec![format!("--rom={rom}")]);
        assert_eq!(build_emulator_args("", rom), vec![rom]);
        assert_eq!(build_emulator_args("-f", rom), vec!["-f", rom]);
    }

    #[test]
    fn paths_inside() {
        assert!(path_inside(r"C:\Steam\steamapps\common\Hades II\bin\Hades2.exe", r"c:\steam\steamapps\common\hades ii"));
        assert!(path_inside("/a/b/c.exe", "/a/b/"));
        assert!(!path_inside(r"C:\Steam\steamapps\common\Hades II2\x.exe", r"C:\Steam\steamapps\common\Hades II"));
        assert!(!path_inside("x", ""));
    }

    #[test]
    fn emulator_labels() {
        assert_eq!(emulator_label("C:\\Emus\\ryujinx.exe"), "Ryujinx");
        assert_eq!(emulator_label("/opt/Yuzu.AppImage"), "Yuzu");
        assert_eq!(emulator_label("D:\\yuzu-early-access.exe"), "Yuzu-early-access");
        assert_eq!(emulator_label("D:\\RetroArch\\retroarch.exe"), "Retroarch");
    }

    #[test]
    fn manual_games_roundtrip_and_conversion() {
        let db = crate::db::Db::open_memory().unwrap();
        let list = vec![
            ManualGame { id: "manual:1".into(), name: "Mi: Juego".into(), kind: "exe".into(), exe: r"D:\Juegos\Mi\mi.exe".into(), args: "-w".into(), rom: String::new(), platform: "pc".into(), folder: None, ..Default::default() },
            ManualGame { id: "manual:2".into(), name: "Zelda".into(), kind: "emulator".into(), exe: r"D:\Emu\emu.exe".into(), args: "-g {rom}".into(), rom: r"D:\r\z.nsp".into(), platform: "switch".into(), folder: Some("Zelda TOTK".into()), ..Default::default() },
        ];
        assert!(db.with(crate::db::data::manual_load).unwrap().is_empty());
        for g in &list {
            assert!(db.with(|c| crate::db::data::manual_add(c, g)).unwrap());
        }
        assert!(!db.with(|c| crate::db::data::manual_add(c, &list[0])).unwrap(), "un id repetido se ignora");
        assert_eq!(db.with(crate::db::data::manual_load).unwrap(), list);
        assert!(db.with(|c| crate::db::data::manual_remove(c, "manual:1")).unwrap());
        assert!(!db.with(|c| crate::db::data::manual_remove(c, "manual:1")).unwrap());
        assert_eq!(db.with(crate::db::data::manual_load).unwrap(), vec![list[1].clone()]);
        let g = manual_to_game(&list[0]).unwrap();
        assert_eq!(g.folder, "Mi - Juego");
        assert_eq!(g.source, Source::Manual);
        let (exe, args) = g.launch.command().unwrap();
        assert_eq!((exe.as_str(), args), (r"D:\Juegos\Mi\mi.exe", vec!["-w".to_string()]));
        let e = manual_to_game(&list[1]).unwrap();
        assert_eq!(e.folder, "Zelda TOTK");
        assert_eq!(e.source, Source::Emulator);
        assert_eq!(e.launch.command().unwrap().1, vec!["-g".to_string(), r"D:\r\z.nsp".to_string()]);
        assert_ne!(new_manual_id(), "");
        assert!(g.removable && e.removable);
    }

    #[test]
    fn manual_kinds_captures_and_legacy() {
        let cap = ManualGame { id: "manual:c".into(), name: "Zelda: Cap".into(), kind: "captures".into(), platform: "switch".into(), ..Default::default() };
        let c = manual_to_game(&cap).unwrap();
        assert_eq!((c.launch.clone(), c.folder.as_str()), (Launch::None, "Zelda - Cap"));
        assert!(c.launch.command().is_none());
        // Las entradas de Steam de versiones anteriores se ignoran, y un games.json antiguo se sigue leyendo.
        let old: GamesFile = serde_json::from_str(r#"{"games":[{"id":"steam:1","name":"X","kind":"steam","appid":"1","platform":"pc"},{"id":"manual:1","name":"Y","kind":"exe","exe":"a.exe","platform":"pc"}]}"#).unwrap();
        let all = all_games(&old.games);
        assert_eq!(all.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(), vec!["manual:1"]);
    }

    #[test]
    fn game_for_exe_prefers_deepest_dir_and_ignores_emulators() {
        let mk = |id: &str, src: Source, dir: &str| Game {
            id: id.into(), name: id.into(), source: src, platform: "pc".into(), folder: id.into(),
            launch: Launch::None, install_dir: Some(dir.into()), cover: None, hero: None, logo: None, removable: false,
        };
        let games = vec![mk("a", Source::Manual, r"C:\G"), mk("b", Source::Manual, r"C:\G\B"), mk("e", Source::Emulator, r"C:\G\B\emu")];
        assert_eq!(game_for_exe(&games, r"C:\G\B\x.exe").unwrap().id, "b");
        assert_eq!(game_for_exe(&games, r"C:\G\B\emu\y.exe").unwrap().id, "b");
        assert_eq!(game_for_exe(&games, r"C:\G\z.exe").unwrap().id, "a");
        assert!(game_for_exe(&games, r"D:\z.exe").is_none());
    }

    #[test]
    fn resolve_folder_reuses_equivalent_existing_dir() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("ELDEN RING")).unwrap();
        fs::create_dir_all(d.path().join("Zelda Tears of the Kingdom")).unwrap();
        assert_eq!(resolve_folder(d.path(), "Elden Ring"), "ELDEN RING");
        assert_eq!(resolve_folder(d.path(), "Zelda - Tears of the Kingdom"), "Zelda Tears of the Kingdom");
        assert_eq!(resolve_folder(d.path(), "Hades II"), "Hades II");
        assert_eq!(resolve_folder(Path::new("/no/existe"), "X"), "X");
    }

    #[test]
    fn stats_accumulate_and_persist() {
        use crate::db::data::*;
        let db = crate::db::Db::open_memory().unwrap();
        // Lo mismo en memoria y en la base: al recargar deben coincidir.
        let mut st = Stats::new();
        mark_started(&mut st, "steam:1", 1000);
        db.with(|c| stat_mark_started(c, "steam:1", 1000)).unwrap();
        // Separadas por mas de 2 minutos: son dos partidas.
        for (a, b) in [(1000, 61_000), (300_000, 330_000)] {
            add_session(&mut st, "steam:1", a, b);
            db.with(|c| stat_add_session(c, "steam:1", a, b, 0)).unwrap();
        }
        let st2 = db.with(stats_load).unwrap();
        assert_eq!(st2, st);
        let e = &st2["steam:1"];
        assert_eq!((e.last_played, e.seconds, e.session_count()), (1000, 90, 2));
        assert_eq!(e.first_played(), Some(1000));
        assert_eq!(e.sessions[1], Session { start: 300_000, end: 330_000 });
        // Una partida con fin anterior al inicio dura cero y no rompe nada.
        db.with(|c| stat_add_session(c, "steam:1", 500_000, 400_000, 0)).unwrap();
        assert_eq!(db.with(stats_load).unwrap()["steam:1"].sessions[2], Session { start: 500_000, end: 500_000 });
    }

    #[test]
    fn close_sessions_of_same_game_are_merged() {
        use crate::db::data::*;
        let db = crate::db::Db::open_memory().unwrap();
        let mut st = Stats::new();
        // 03:03:30-03:12:44 y 03:13:09-04:03:46 (corte de 25 s) + otro juego intercalado + una a mas de 2 min.
        let (a1, b1) = (1_000_000, 1_554_000);
        let (a2, b2) = (b1 + 25_000, b1 + 25_000 + 3_037_000);
        let steps = [("g", a1, b1), ("otro", a2, b2), ("g", a2, b2), ("g", b2 + 10_000_000, b2 + 10_060_000)];
        for (id, a, b) in steps {
            add_session(&mut st, id, a, b);
            db.with(|c| stat_add_session(c, id, a, b, 0)).unwrap();
        }
        let e = &st["g"];
        assert_eq!(e.sessions, vec![Session { start: a1, end: b2 }, Session { start: b2 + 10_000_000, end: b2 + 10_060_000 }]);
        assert_eq!(e.seconds, ((b2 - a1) / 1000 + 60) as u64); // el hueco de 25 s cuenta
        assert_eq!(st["otro"].sessions.len(), 1); // el otro juego no se mezcla
        // La base queda igual que la memoria.
        assert_eq!(db.with(stats_load).unwrap(), st);
    }

    #[test]
    fn stats_without_sessions_and_new_session_on_top() {
        use crate::db::data::*;
        let db = crate::db::Db::open_memory().unwrap();
        // Juego migrado del formato antiguo: total sin partidas.
        db.with(|c| stat_import(c, "steam:7", &PlayStat { last_played: 5000, seconds: 7200, sessions: vec![] })).unwrap();
        let mut st = db.with(stats_load).unwrap();
        let e = &st["steam:7"];
        assert_eq!((e.last_played, e.seconds, e.session_count(), e.first_played()), (5000, 7200, 0, None));
        // Una partida nueva se suma al total antiguo.
        add_session(&mut st, "steam:7", 10_000, 70_000);
        db.with(|c| stat_add_session(c, "steam:7", 10_000, 70_000, 0)).unwrap();
        assert_eq!(db.with(stats_load).unwrap(), st);
        assert_eq!((st["steam:7"].seconds, st["steam:7"].session_count()), (7260, 1));
        // Iniciar un juego nuevo sin partidas no escribe la lista vacia en JSON.
        let mut st3 = Stats::new();
        mark_started(&mut st3, "a", 1);
        assert!(!serde_json::to_string(&st3).unwrap().contains("sessions"));
        // Borrar el juego arrastra sus partidas (clave foranea con cascada).
        db.with(|c| c.execute("DELETE FROM play_stats WHERE game_id = 'steam:7'", [])).unwrap();
        let n: i64 = db.with(|c| c.query_row("SELECT count(*) FROM sessions", [], |r| r.get(0))).unwrap();
        assert_eq!(n, 0);
    }

    fn at(off_h: i32, y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<chrono::FixedOffset> {
        chrono::FixedOffset::east_opt(off_h * 3600).unwrap().with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }
    fn ses(a: DateTime<chrono::FixedOffset>, b: DateTime<chrono::FixedOffset>) -> Session {
        Session { start: a.timestamp_millis(), end: b.timestamp_millis() }
    }

    #[test]
    fn week_aggregates_local_days() {
        let now = at(2, 2026, 10, 15, 15, 0); // jueves 15, UTC+2
        let st = PlayStat {
            sessions: vec![
                ses(at(2, 2026, 10, 15, 10, 0), at(2, 2026, 10, 15, 11, 30)), // hoy: 90 min
                ses(at(2, 2026, 10, 14, 23, 30), at(2, 2026, 10, 15, 0, 45)), // cruza medianoche: 30 + 45
                ses(at(2, 2026, 10, 9, 20, 0), at(2, 2026, 10, 9, 21, 0)),    // dia 9 = hace 6 dias (primero)
                ses(at(2, 2026, 10, 8, 20, 0), at(2, 2026, 10, 8, 23, 0)),    // dia 8: fuera de la ventana
                ses(at(2, 2026, 10, 13, 9, 0), at(2, 2026, 10, 13, 9, 0)),    // duracion cero
            ],
            ..Default::default()
        };
        assert_eq!(st.week_minutes(now), vec![60, 0, 0, 0, 0, 30, 135]);
        // El mismo instante visto desde UTC-8 cae en otro dia local: 13:30 UTC del 15 = 05:30 del 15.
        let early = at(-8, 2026, 10, 15, 6, 0);
        let st2 = PlayStat { sessions: vec![ses(at(2, 2026, 10, 15, 0, 30), at(2, 2026, 10, 15, 1, 30))], ..Default::default() };
        // 00:30-01:30 (UTC+2) = 14 oct 22:30-23:30 UTC = 14 oct 14:30-15:30 en UTC-8 -> el dia anterior.
        assert_eq!(st2.week_minutes(early), vec![0, 0, 0, 0, 0, 60, 0]);
        assert_eq!(PlayStat::default().week_minutes(now), vec![0; 7]);
    }
}
