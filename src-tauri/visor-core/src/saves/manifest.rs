//! Base de partidas de Ludusavi (<https://github.com/mtkennerly/ludusavi-manifest>, datos de PCGamingWiki).
//!
//! El `manifest.yaml` original pesa unos 18 MB: al descargarlo se resume en un indice JSON con solo lo que
//! sirve en Windows (rutas de archivos y registro, carpetas de instalacion, ejecutables e ids de Steam) y es
//! ese indice el que se carga. Para reconocer un juego se prueba, por este orden: el id de Steam
//! (`steam_appid.txt` junto al juego), el nombre de la carpeta de instalacion, el ejecutable y el nombre.
use super::{LocKind, SaveEnv};
use crate::games::norm_key;
use anyhow::{Context, Result};
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Direccion del manifiesto.
pub const MANIFEST_URL: &str = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";
/// Indice resumido y sus datos de descarga, dentro de la carpeta de cache.
pub const INDEX_FILE: &str = "ludusavi-index.json";
pub const META_FILE: &str = "ludusavi-meta.json";

/// Ejecutables con nombres tan comunes que no identifican un juego por si solos.
const GENERIC_EXES: &[&str] = &[
    "game.exe", "launcher.exe", "start.exe", "play.exe", "rpg_rt.exe", "nw.exe", "main.exe", "app.exe", "run.exe",
    "setup.exe", "client.exe", "game-win64-shipping.exe", "unitycrashhandler64.exe", "renpy.exe", "lib.exe",
];

/* ---------- formato del manifiesto (solo lo que se usa) ---------- */

#[derive(Deserialize, Default)]
struct RawWhen {
    #[serde(default)]
    os: Option<String>,
}
#[derive(Deserialize, Default)]
struct RawPath {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    when: Vec<RawWhen>,
}
#[derive(Deserialize, Default)]
struct RawLaunch {
    #[serde(default)]
    when: Vec<RawWhen>,
}
#[derive(Deserialize, Default)]
struct RawSteam {
    #[serde(default)]
    id: Option<u32>,
}
#[derive(Deserialize, Default)]
struct RawId {
    #[serde(default, rename = "steamExtra")]
    steam_extra: Vec<u32>,
}
#[derive(Deserialize, Default)]
struct RawGame {
    #[serde(default)]
    files: BTreeMap<String, Option<RawPath>>,
    #[serde(default)]
    registry: BTreeMap<String, Option<RawPath>>,
    #[serde(default, rename = "installDir")]
    install_dir: BTreeMap<String, IgnoredAny>,
    #[serde(default)]
    launch: BTreeMap<String, Option<Vec<RawLaunch>>>,
    #[serde(default)]
    steam: Option<RawSteam>,
    #[serde(default)]
    id: Option<RawId>,
}

fn for_windows(when: &[RawWhen]) -> bool {
    when.is_empty() || when.iter().any(|w| w.os.as_deref().map(|o| o == "windows").unwrap_or(true))
}

/* ---------- indice resumido ---------- */

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MPath {
    #[serde(rename = "p")]
    pub path: String,
    #[serde(rename = "t", default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MEntry {
    #[serde(rename = "n")]
    pub title: String,
    /// Nombres de la carpeta de instalacion.
    #[serde(rename = "i", default, skip_serializing_if = "Vec::is_empty")]
    pub install: Vec<String>,
    /// Ejecutables relativos a la carpeta de instalacion (`bin/juego.exe`), en minusculas.
    #[serde(rename = "l", default, skip_serializing_if = "Vec::is_empty")]
    pub launch: Vec<String>,
    #[serde(rename = "s", default, skip_serializing_if = "Vec::is_empty")]
    pub steam: Vec<u32>,
    #[serde(rename = "f", default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<MPath>,
    #[serde(rename = "r", default, skip_serializing_if = "Vec::is_empty")]
    pub registry: Vec<MPath>,
}

/// Resume el `manifest.yaml`: solo juegos con rutas de Windows (archivos o registro).
pub fn build_index(yaml: &str) -> Result<Vec<MEntry>> {
    let raw: BTreeMap<String, Option<RawGame>> = serde_yaml::from_str(yaml).context("el manifiesto de Ludusavi no se entiende")?;
    let mut out = Vec::new();
    for (title, g) in raw {
        let Some(g) = g else { continue };
        let take = |m: BTreeMap<String, Option<RawPath>>| -> Vec<MPath> {
            m.into_iter()
                .filter_map(|(p, v)| {
                    let v = v.unwrap_or_default();
                    for_windows(&v.when).then_some(MPath { path: p, tags: v.tags })
                })
                .filter(|m| !m.path.contains("<xdg") && !m.path.contains("/Library/") && !m.path.contains("<home>/."))
                .collect()
        };
        let files = take(g.files);
        let registry: Vec<MPath> = take(g.registry).into_iter().filter(|r| r.path.to_uppercase().starts_with("HKEY_CURRENT_USER")).collect();
        if files.is_empty() && registry.is_empty() {
            continue;
        }
        let launch = g
            .launch
            .into_iter()
            .filter(|(_, w)| w.as_ref().map(|v| v.is_empty() || v.iter().any(|l| for_windows(&l.when))).unwrap_or(true))
            .filter_map(|(p, _)| p.strip_prefix("<base>/").map(|s| s.to_lowercase()))
            .filter(|p| p.ends_with(".exe"))
            .collect();
        let mut steam: Vec<u32> = g.steam.and_then(|s| s.id).into_iter().collect();
        steam.extend(g.id.map(|i| i.steam_extra).unwrap_or_default());
        out.push(MEntry { title, install: g.install_dir.into_keys().collect(), launch, steam, files, registry });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IndexMeta {
    pub etag: Option<String>,
    pub fetched_ms: i64,
    pub games: usize,
}

pub fn save_index(cache_dir: &Path, entries: &[MEntry], meta: &IndexMeta) -> Result<()> {
    fs::create_dir_all(cache_dir)?;
    let tmp = cache_dir.join(format!("{INDEX_FILE}.part"));
    fs::write(&tmp, serde_json::to_vec(entries)?)?;
    fs::rename(&tmp, cache_dir.join(INDEX_FILE))?;
    save_meta(cache_dir, meta)
}

pub fn save_meta(cache_dir: &Path, meta: &IndexMeta) -> Result<()> {
    fs::create_dir_all(cache_dir)?;
    fs::write(cache_dir.join(META_FILE), serde_json::to_vec_pretty(meta)?)?;
    Ok(())
}

pub fn load_meta(cache_dir: &Path) -> Option<IndexMeta> {
    serde_json::from_slice(&fs::read(cache_dir.join(META_FILE)).ok()?).ok()
}

pub fn load_index(cache_dir: &Path) -> Option<Manifest> {
    let bytes = fs::read(cache_dir.join(INDEX_FILE)).ok()?;
    let entries: Vec<MEntry> = serde_json::from_slice(&bytes).ok()?;
    Some(Manifest::new(entries))
}

/* ---------- busqueda ---------- */

/// Juego reconocido: entrada, carpeta de instalacion (`<base>`) y como se encontro.
#[derive(Debug, Clone, PartialEq)]
pub struct Match<'a> {
    pub entry: &'a MEntry,
    pub base: PathBuf,
    pub how: &'static str,
    pub steam_id: Option<u32>,
}

pub struct Manifest {
    entries: Vec<MEntry>,
    by_install: HashMap<String, Vec<usize>>,
    by_exe: HashMap<String, Vec<usize>>,
    by_title: HashMap<String, Vec<usize>>,
    by_steam: HashMap<u32, usize>,
}

fn file_of(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}

impl Manifest {
    pub fn new(entries: Vec<MEntry>) -> Manifest {
        let (mut by_install, mut by_exe, mut by_title, mut by_steam) = (HashMap::new(), HashMap::<String, Vec<usize>>::new(), HashMap::new(), HashMap::new());
        for (i, e) in entries.iter().enumerate() {
            for d in &e.install {
                by_install.entry(d.to_lowercase()).or_insert_with(Vec::new).push(i);
            }
            for l in &e.launch {
                let v = by_exe.entry(file_of(l).to_string()).or_default();
                if !v.contains(&i) {
                    v.push(i);
                }
            }
            by_title.entry(norm_key(&e.title)).or_insert_with(Vec::new).push(i);
            for s in &e.steam {
                by_steam.entry(*s).or_insert(i);
            }
        }
        Manifest { entries, by_install, by_exe, by_title, by_steam }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Reconoce un juego por su ejecutable (si lo tiene) y su nombre.
    pub fn find(&self, name: &str, exe: Option<&Path>) -> Option<Match<'_>> {
        let exe_dir = exe.and_then(|e| e.parent());
        let ancestors: Vec<&Path> = exe_dir.map(|d| d.ancestors().take(5).filter(|a| a.file_name().is_some()).collect()).unwrap_or_default();
        let base_for = |e: &MEntry| -> PathBuf {
            ancestors
                .iter()
                .find(|a| e.install.iter().any(|i| a.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case(i)).unwrap_or(false)))
                .map(|a| a.to_path_buf())
                .or_else(|| exe.and_then(|x| self.base_by_launch(e, x)))
                .or_else(|| exe_dir.map(Path::to_path_buf))
                .unwrap_or_default()
        };
        let name_key = norm_key(name);
        // 1. Id de Steam.
        for a in ancestors.iter().take(3) {
            let Ok(txt) = fs::read_to_string(a.join("steam_appid.txt")) else { continue };
            if let Some(&i) = txt.trim().parse::<u32>().ok().and_then(|id| self.by_steam.get(&id)) {
                let e = &self.entries[i];
                return Some(Match { entry: e, base: base_for(e), how: "id de Steam", steam_id: txt.trim().parse().ok() });
            }
        }
        // 2. Carpeta de instalacion (la mas cercana al ejecutable primero).
        for a in &ancestors {
            let Some(dir) = a.file_name().map(|n| n.to_string_lossy().to_lowercase()) else { continue };
            let Some(cands) = self.by_install.get(&dir) else { continue };
            let pick = if cands.len() == 1 {
                Some(cands[0])
            } else {
                cands
                    .iter()
                    .copied()
                    .find(|&i| exe.map(|x| self.base_by_launch(&self.entries[i], x).is_some()).unwrap_or(false))
                    .or_else(|| cands.iter().copied().find(|&i| norm_key(&self.entries[i].title) == name_key))
            };
            if let Some(i) = pick {
                let e = &self.entries[i];
                return Some(Match { entry: e, base: a.to_path_buf(), how: "carpeta del juego", steam_id: e.steam.first().copied() });
            }
        }
        // 3. Ejecutable (ruta relativa a la instalacion), si el nombre no es generico.
        if let Some(x) = exe {
            let file = x.file_name().map(|f| f.to_string_lossy().to_lowercase()).unwrap_or_default();
            if let Some(cands) = self.by_exe.get(&file) {
                let hits: Vec<(usize, PathBuf)> = cands.iter().filter_map(|&i| self.base_by_launch(&self.entries[i], x).map(|b| (i, b))).collect();
                let chosen = hits
                    .iter()
                    .find(|(i, _)| norm_key(&self.entries[*i].title) == name_key)
                    .or_else(|| (hits.len() == 1 && !GENERIC_EXES.contains(&file.as_str())).then(|| &hits[0]));
                if let Some((i, b)) = chosen {
                    let e = &self.entries[*i];
                    return Some(Match { entry: e, base: b.clone(), how: "ejecutable", steam_id: e.steam.first().copied() });
                }
            }
        }
        // 4. Nombre del juego.
        if !name_key.is_empty() {
            if let Some(&i) = self.by_title.get(&name_key).and_then(|v| v.first()) {
                let e = &self.entries[i];
                return Some(Match { entry: e, base: base_for(e), how: "nombre", steam_id: e.steam.first().copied() });
            }
        }
        None
    }

    /// Si `exe` acaba con uno de los ejecutables de la entrada, la carpeta de instalacion que queda.
    fn base_by_launch(&self, e: &MEntry, exe: &Path) -> Option<PathBuf> {
        let full = exe.to_string_lossy().replace('\\', "/").to_lowercase();
        e.launch.iter().find_map(|l| {
            let tail = format!("/{l}");
            // Se sube un nivel por cada componente del ejecutable relativo (sin cortar la cadena: las
            // minusculas pueden cambiar su longitud).
            full.ends_with(&tail).then(|| exe.ancestors().nth(l.split('/').count())).flatten().map(Path::to_path_buf)
        })
    }
}

/* ---------- rutas ---------- */

/// Una ruta del manifiesto con sus marcadores resueltos (puede tener comodines).
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub pattern: String,
    pub kind: LocKind,
    pub tags: Vec<String>,
}

/// Sustituye los marcadores (`<base>`, `<winAppData>`...). `None` si queda alguno sin resolver.
pub fn fill(p: &str, m: &Match, env: &SaveEnv) -> Option<String> {
    let s = |p: &Path| p.to_string_lossy().replace('\\', "/");
    let game = m.base.file_name().map(|n| n.to_string_lossy().into_owned()).or_else(|| m.entry.install.first().cloned()).unwrap_or_default();
    let root = if p.contains("<root>/userdata") { env.steam.as_deref().map(s) } else { m.base.parent().map(s) };
    let pairs: [(&str, Option<String>); 14] = [
        ("<base>", (!m.base.as_os_str().is_empty()).then(|| s(&m.base))),
        ("<root>", root),
        ("<game>", Some(game)),
        ("<home>", Some(s(&env.home))),
        ("<winAppData>", Some(s(&env.appdata))),
        ("<winLocalAppDataLow>", Some(s(&env.home.join("AppData").join("LocalLow")))),
        ("<winLocalAppData>", Some(s(&env.local_appdata))),
        ("<winDocuments>", Some(s(&env.documents))),
        ("<winPublic>", Some(s(&env.public))),
        ("<winProgramData>", Some(s(&env.program_data))),
        ("<winDir>", Some(s(&env.win_dir))),
        ("<osUserName>", Some(env.user_name.clone())),
        ("<storeUserId>", Some("*".into())),
        ("<storeGameId>", Some(m.steam_id.map(|i| i.to_string()).unwrap_or_else(|| "*".into()))),
    ];
    let mut out = p.to_string();
    for (k, v) in pairs {
        if out.contains(k) {
            out = out.replace(k, &v?);
        }
    }
    (!out.contains('<')).then_some(out)
}

/// Todas las rutas de la entrada reconocida, resueltas para este equipo.
pub fn resolve(m: &Match, env: &SaveEnv) -> Vec<Resolved> {
    let mut out: Vec<Resolved> = m
        .entry
        .files
        .iter()
        .filter_map(|f| fill(&f.path, m, env).map(|p| Resolved { pattern: p, kind: LocKind::Path, tags: f.tags.clone() }))
        .collect();
    out.extend(m.entry.registry.iter().filter(|r| !super::glob::has_magic(&r.path) && !r.path.contains('<')).map(|r| Resolved {
        pattern: r.path.replace('/', "\\"),
        kind: LocKind::Registry,
        tags: r.tags.clone(),
    }));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r#"
---
"Juego Uno":
  files:
    "<winLocalAppData>/Uno/Saved/SaveGames/*.sav":
      tags: [save]
      when:
        - os: windows
    "<home>/.local/share/uno":
      when:
        - os: linux
  installDir:
    Juego Uno: {}
  launch:
    "<base>/Binaries/Uno.exe":
      - when:
          - os: windows
  steam:
    id: 111
"Solo Linux":
  files:
    "<xdgData>/x": {}
"Registro":
  registry:
    HKEY_CURRENT_USER/Software/Empresa/Registro:
      tags: [save]
  launch:
    "<base>/registro.exe": []
"Sin rutas":
  steam:
    id: 5
"Alias": ~
"#;

    fn env() -> SaveEnv {
        SaveEnv {
            home: PathBuf::from("C:/Users/yo"),
            appdata: PathBuf::from("C:/Users/yo/AppData/Roaming"),
            local_appdata: PathBuf::from("C:/Users/yo/AppData/Local"),
            documents: PathBuf::from("D:/Docs"),
            public: PathBuf::from("C:/Users/Public"),
            program_data: PathBuf::from("C:/ProgramData"),
            win_dir: PathBuf::from("C:/Windows"),
            user_name: "yo".into(),
            steam: Some(PathBuf::from("C:/Steam")),
        }
    }

    #[test]
    fn index_keeps_only_windows_paths() {
        let idx = build_index(YAML).unwrap();
        assert_eq!(idx.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), vec!["Juego Uno", "Registro"]);
        let uno = &idx[0];
        assert_eq!(uno.files.len(), 1);
        assert_eq!((uno.launch.clone(), uno.steam.clone(), uno.install.clone()), (vec!["binaries/uno.exe".to_string()], vec![111], vec!["Juego Uno".to_string()]));
        assert_eq!(idx[1].registry[0].tags, vec!["save"]);
        // El indice se guarda y se vuelve a leer igual.
        let d = tempfile::tempdir().unwrap();
        save_index(d.path(), &idx, &IndexMeta { etag: Some("e".into()), fetched_ms: 1, games: 2 }).unwrap();
        assert_eq!(load_index(d.path()).unwrap().len(), 2);
        assert_eq!(load_meta(d.path()).unwrap().etag.as_deref(), Some("e"));
    }

    #[test]
    fn finds_by_folder_exe_steam_and_name() {
        let m = Manifest::new(build_index(YAML).unwrap());
        let d = tempfile::tempdir().unwrap();
        // Carpeta de instalacion: el ejecutable esta dos niveles por debajo.
        let base = d.path().join("Juego Uno");
        let exe = base.join("Binaries").join("Uno.exe");
        let hit = m.find("Otro nombre", Some(&exe)).unwrap();
        assert_eq!((hit.entry.title.as_str(), hit.how, hit.base.clone()), ("Juego Uno", "carpeta del juego", base.clone()));
        // Por ejecutable aunque la carpeta se llame distinto.
        let other = d.path().join("uno-v2");
        let hit = m.find("x", Some(&other.join("Binaries/Uno.exe"))).unwrap();
        assert_eq!((hit.how, hit.base.clone()), ("ejecutable", other.clone()));
        // Por id de Steam.
        let s = d.path().join("cualquiera");
        fs::create_dir_all(&s).unwrap();
        fs::write(s.join("steam_appid.txt"), "111\n").unwrap();
        assert_eq!(m.find("x", Some(&s.join("a.exe"))).unwrap().how, "id de Steam");
        // Por nombre, sin ejecutable.
        assert_eq!(m.find("registro", None).unwrap().entry.title, "Registro");
        assert!(m.find("Nada que ver", Some(&d.path().join("z/z.exe"))).is_none());
    }

    /// Con el manifiesto real: `LUDUSAVI_YAML=ruta cargo test -p visor-core --release real_manifest -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_manifest() {
        let p = std::env::var("LUDUSAVI_YAML").expect("LUDUSAVI_YAML");
        let t = std::time::Instant::now();
        let idx = build_index(&fs::read_to_string(p).unwrap()).unwrap();
        let json = serde_json::to_vec(&idx).unwrap().len();
        println!("{} juegos en {:?}, indice de {} KB", idx.len(), t.elapsed(), json / 1024);
        let t = std::time::Instant::now();
        let m = Manifest::new(idx);
        println!("indice en memoria en {:?}", t.elapsed());
        for (name, exe) in [("Hades II", r"D:\Juegos\Hades II\Ship\Hades2.exe"), ("Celeste", r"D:\Juegos\Celeste\Celeste.exe"), ("Doki Doki Literature Club Plus", r"D:\x\DDLC Plus\Doki Doki Literature Club Plus.exe"), ("Hollow Knight", r"E:\HK\hollow_knight.exe")] {
            let hit = m.find(name, Some(Path::new(exe)));
            println!("{name}: {:?}", hit.map(|h| (h.entry.title.clone(), h.how, h.base, h.entry.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>())));
        }
    }

    #[test]
    fn fills_placeholders() {
        let m = Manifest::new(build_index(YAML).unwrap());
        let hit = m.find("Juego Uno", None).unwrap();
        let r = resolve(&hit, &env());
        assert_eq!(r[0].pattern, "C:/Users/yo/AppData/Local/Uno/Saved/SaveGames/*.sav");
        let hit = Match { base: PathBuf::from("E:/Juegos/Juego Uno"), ..hit };
        assert_eq!(fill("<base>/save", &hit, &env()).unwrap(), "E:/Juegos/Juego Uno/save");
        assert_eq!(fill("<root>/userdata/<storeUserId>/<storeGameId>/remote", &hit, &env()).unwrap(), "C:/Steam/userdata/*/111/remote");
        assert_eq!(fill("<home>/AppData/LocalLow/A", &hit, &env()).unwrap(), "C:/Users/yo/AppData/LocalLow/A");
        assert!(fill("<xdgConfig>/a", &hit, &env()).is_none());
        let reg = m.find("Registro", None).unwrap();
        assert_eq!(resolve(&reg, &env())[0], Resolved { pattern: r"HKEY_CURRENT_USER\Software\Empresa\Registro".into(), kind: LocKind::Registry, tags: vec!["save".into()] });
    }
}
