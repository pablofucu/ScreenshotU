//! Reglas por motor: muchas novelas visuales y juegos independientes no estan en ninguna base de datos, pero
//! su motor guarda siempre en el mismo sitio. Se reconoce el motor por los archivos junto al ejecutable.
use super::{LocKind, SaveEnv};
use crate::games::norm_key;
use std::fs;
use std::path::{Path, PathBuf};

/// Carpetas que suelen ser de partidas en cualquier juego (sin distinguir mayusculas).
const SAVE_DIR_NAMES: &[&str] = &["save", "saves", "savedata", "save_data", "savegame", "savegames", "saved games", "userdata_save"];

/// Una ruta candidata (puede tener comodines) y el motor que la propone.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLoc {
    pub pattern: String,
    pub kind: LocKind,
    pub engine: &'static str,
}

fn s(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn has(dir: &Path, name: &str) -> bool {
    dir.join(name).exists()
}

fn any_ext(dir: &Path, exts: &[&str]) -> bool {
    fs::read_dir(dir)
        .map(|rd| rd.flatten().any(|e| e.path().extension().map(|x| exts.iter().any(|w| x.eq_ignore_ascii_case(w))).unwrap_or(false)))
        .unwrap_or(false)
}

/// Subcarpetas de `dir` cuyo nombre (sin el `-123456` final de Ren'Py) coincide con alguna clave.
fn dirs_named(dir: &Path, keys: &[String]) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            let stem = n.rsplit_once('-').filter(|(_, d)| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit())).map(|(a, _)| a).unwrap_or(&n);
            let k = norm_key(stem);
            !k.is_empty() && keys.iter().any(|w| *w == k)
        })
        .map(|e| e.path())
        .collect();
    v.sort();
    v
}

/// `name` de un `package.json` (NW.js, Electron).
fn package_name(p: &Path) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(&fs::read(p).ok()?).ok()?;
    v.get("name")?.as_str().map(str::to_string).filter(|n| !n.trim().is_empty())
}

/// Empresa y producto de un juego de Unity (`<Juego>_Data/app.info`).
fn unity_info(root: &Path) -> Option<(String, String)> {
    let rd = fs::read_dir(root).ok()?;
    for e in rd.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if !e.path().is_dir() || !n.to_lowercase().ends_with("_data") {
            continue;
        }
        if let Ok(txt) = fs::read_to_string(e.path().join("app.info")) {
            let mut l = txt.lines().map(str::trim);
            if let (Some(c), Some(p)) = (l.next(), l.next()) {
                if !c.is_empty() && !p.is_empty() {
                    return Some((c.to_string(), p.to_string()));
                }
            }
        }
    }
    None
}

fn renpy_save_dir(root: &Path) -> Option<String> {
    let re = regex::Regex::new(r#"config\.save_directory\s*=\s*["']([^"']+)["']"#).ok()?;
    let txt = fs::read_to_string(root.join("game").join("options.rpy")).ok()?;
    re.captures(&txt).map(|c| c[1].to_string())
}

/// Motor reconocido junto al ejecutable y sus rutas de partidas. `name` es el nombre del juego.
pub fn detect(exe: &Path, name: &str, env: &SaveEnv) -> (Option<&'static str>, Vec<EngineLoc>) {
    let Some(root) = exe.parent() else { return (None, Vec::new()) };
    let stem = exe.file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
    let keys: Vec<String> = [stem.as_str(), name, &root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()]
        .iter()
        .map(|k| norm_key(k))
        .filter(|k| !k.is_empty())
        .collect();
    let mut out: Vec<EngineLoc> = Vec::new();
    let mut add = |engine: &'static str, pattern: String, kind: LocKind| out.push(EngineLoc { pattern, kind, engine });
    let r = s(root);
    let engine: Option<&'static str>;

    if has(root, "renpy") && has(root, "game") {
        engine = Some("Ren'Py");
        add("Ren'Py", format!("{r}/game/saves"), LocKind::Path);
        let base = env.appdata.join("RenPy");
        match renpy_save_dir(root) {
            Some(d) => add("Ren'Py", s(&base.join(d)), LocKind::Path),
            None => {
                for d in dirs_named(&base, &keys) {
                    add("Ren'Py", s(&d), LocKind::Path);
                }
            }
        }
    } else if has(root, "www/js/rpg_core.js") || has(root, "www/js/rpg_managers.js") {
        engine = Some("RPG Maker MV");
        add("RPG Maker MV", format!("{r}/www/save"), LocKind::Path);
    } else if has(root, "js/rmmz_core.js") {
        engine = Some("RPG Maker MZ");
        add("RPG Maker MZ", format!("{r}/save"), LocKind::Path);
    } else if has(root, "RPG_RT.ldb") || has(root, "RPG_RT.lmt") {
        engine = Some("RPG Maker 2000/2003");
        add("RPG Maker 2000/2003", format!("{r}/Save*.lsd"), LocKind::Path);
    } else if has(root, "Game.ini") && (any_ext(root, &["rgss3a", "rgss2a", "rgssad"]) || has(root, "Data")) {
        engine = Some("RPG Maker XP/VX/VX Ace");
        for ext in ["rvdata2", "rvdata", "rxdata"] {
            add("RPG Maker XP/VX/VX Ace", format!("{r}/Save*.{ext}"), LocKind::Path);
        }
    } else if has(root, "Data.wolf") || has(root, "Data/BasicData") || has(root, "Data/BasicData.wolf") {
        engine = Some("Wolf RPG Editor");
        add("Wolf RPG Editor", format!("{r}/Save"), LocKind::Path);
        add("Wolf RPG Editor", format!("{r}/Data/Save"), LocKind::Path);
    } else if has(root, "data.xp3") {
        engine = Some("KiriKiri");
        add("KiriKiri", format!("{r}/savedata"), LocKind::Path);
    } else if let Some((company, product)) = unity_info(root) {
        engine = Some("Unity");
        add("Unity", s(&env.home.join("AppData/LocalLow").join(&company).join(&product)), LocKind::Path);
        add("Unity", format!(r"HKEY_CURRENT_USER\Software\{company}\{product}"), LocKind::Registry);
    } else if let Some((ue_root, project)) = unreal_project(root) {
        engine = Some("Unreal Engine");
        add("Unreal Engine", s(&env.local_appdata.join(&project).join("Saved/SaveGames")), LocKind::Path);
        add("Unreal Engine", s(&ue_root.join(&project).join("Saved/SaveGames")), LocKind::Path);
    } else if has(root, &format!("{stem}.pck")) {
        engine = Some("Godot");
        for d in dirs_named(&env.appdata.join("Godot/app_userdata"), &keys) {
            add("Godot", s(&d), LocKind::Path);
        }
        for d in dirs_named(&env.appdata, &keys) {
            add("Godot", s(&d), LocKind::Path);
        }
    } else if has(root, "nw.dll") || has(root, "nw_elf.dll") {
        engine = Some("NW.js");
        if let Some(n) = package_name(&root.join("package.json")) {
            let ud = env.local_appdata.join(&n).join("User Data/Default");
            add("NW.js", s(&ud.join("Local Storage")), LocKind::Path);
            add("NW.js", s(&ud.join("IndexedDB")), LocKind::Path);
        }
    } else if has(root, "resources/app.asar") || has(root, "resources/app") {
        engine = Some("Electron");
        let mut names: Vec<String> = package_name(&root.join("resources/app/package.json")).into_iter().collect();
        names.push(stem.clone());
        for n in names {
            let d = env.appdata.join(&n);
            add("Electron", s(&d.join("Local Storage")), LocKind::Path);
            add("Electron", s(&d.join("IndexedDB")), LocKind::Path);
        }
    } else {
        engine = None;
    }

    // Carpetas con nombre de partidas junto al juego (hasta dos niveles), con o sin motor reconocido.
    for e in walkdir::WalkDir::new(root).min_depth(1).max_depth(2).into_iter().flatten() {
        if e.file_type().is_dir() && SAVE_DIR_NAMES.contains(&e.file_name().to_string_lossy().to_lowercase().as_str()) {
            out.push(EngineLoc { pattern: s(e.path()), kind: LocKind::Path, engine: "Carpeta de partidas" });
        }
    }
    (engine, out)
}

/// Unreal: la carpeta con `Engine/` (el ejecutable puede estar en `<Proyecto>/Binaries/Win64`) y el proyecto.
fn unreal_project(exe_dir: &Path) -> Option<(PathBuf, String)> {
    for a in exe_dir.ancestors().take(4) {
        if !a.join("Engine").is_dir() {
            continue;
        }
        let rd = fs::read_dir(a).ok()?;
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n != "Engine" && e.path().join("Binaries").is_dir() {
                return Some((a.to_path_buf(), n));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"x").unwrap();
    }

    fn env(home: &Path) -> SaveEnv {
        SaveEnv {
            home: home.to_path_buf(),
            appdata: home.join("AppData/Roaming"),
            local_appdata: home.join("AppData/Local"),
            documents: home.join("Documents"),
            public: home.join("Public"),
            program_data: home.join("ProgramData"),
            win_dir: home.join("Windows"),
            user_name: "yo".into(),
            steam: None,
        }
    }

    fn pats(v: &[EngineLoc]) -> Vec<String> {
        v.iter().map(|l| l.pattern.rsplit('/').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/")).collect()
    }

    #[test]
    fn renpy_uses_options_or_matching_appdata_dir() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("DDLC");
        touch(&g.join("renpy/a.py"));
        touch(&g.join("DDLC.exe"));
        fs::create_dir_all(g.join("game")).unwrap();
        let e = env(&d.path().join("home"));
        fs::create_dir_all(e.appdata.join("RenPy/DDLC-1454445547")).unwrap();
        fs::create_dir_all(e.appdata.join("RenPy/Otro-1")).unwrap();
        let (eng, locs) = detect(&g.join("DDLC.exe"), "Doki Doki", &e);
        assert_eq!(eng, Some("Ren'Py"));
        assert_eq!(pats(&locs), vec!["game/saves", "RenPy/DDLC-1454445547"]);
        fs::write(g.join("game/options.rpy"), "define config.save_directory = \"Mio-123\"\n").unwrap();
        assert_eq!(pats(&detect(&g.join("DDLC.exe"), "x", &e).1)[1], "RenPy/Mio-123");
    }

    #[test]
    fn other_engines() {
        let d = tempfile::tempdir().unwrap();
        let e = env(&d.path().join("home"));
        let mv = d.path().join("mv");
        touch(&mv.join("www/js/rpg_core.js"));
        assert_eq!(detect(&mv.join("Game.exe"), "x", &e).0, Some("RPG Maker MV"));
        let kk = d.path().join("kk");
        touch(&kk.join("data.xp3"));
        fs::create_dir_all(kk.join("savedata")).unwrap();
        let (eng, locs) = detect(&kk.join("game.exe"), "x", &e);
        assert_eq!(eng, Some("KiriKiri"));
        // La carpeta savedata sale por el motor y por el nombre: se deduplica mas tarde.
        assert_eq!(pats(&locs), vec!["kk/savedata", "kk/savedata"]);
        let un = d.path().join("un");
        fs::create_dir_all(un.join("Juego_Data")).unwrap();
        fs::write(un.join("Juego_Data/app.info"), "Empresa\nProducto").unwrap();
        let (eng, locs) = detect(&un.join("Juego.exe"), "x", &e);
        assert_eq!(eng, Some("Unity"));
        assert!(locs[0].pattern.ends_with("AppData/LocalLow/Empresa/Producto"));
        assert_eq!((locs[1].kind, locs[1].pattern.as_str()), (LocKind::Registry, r"HKEY_CURRENT_USER\Software\Empresa\Producto"));
        let ue = d.path().join("ue");
        fs::create_dir_all(ue.join("Engine")).unwrap();
        touch(&ue.join("Proy/Binaries/Win64/Proy-Win64-Shipping.exe"));
        let (eng, locs) = detect(&ue.join("Proy/Binaries/Win64/Proy-Win64-Shipping.exe"), "x", &e);
        assert_eq!(eng, Some("Unreal Engine"));
        assert!(locs[0].pattern.ends_with("AppData/Local/Proy/Saved/SaveGames"));
        let plain = d.path().join("plain");
        fs::create_dir_all(plain.join("Saves")).unwrap();
        let (eng, locs) = detect(&plain.join("p.exe"), "x", &e);
        assert_eq!((eng, locs.len(), locs[0].engine), (None, 1, "Carpeta de partidas"));
    }
}
