//! Perfiles de Magpie (v0.12) para escalar solos los juegos lanzados desde el visor. Todo es puro:
//! se lee y se escribe su `config.json` como JSON generico para no perder ningun campo que no conozcamos.
//! Magpie solo lee el archivo al arrancar, asi que quien llame tiene que reiniciarlo despues de guardar.
use crate::config::write_atomic;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Prefijo de los modos de escalado que crea el visor (los demas son del usuario y no se tocan).
pub const MODE_PREFIX: &str = "Visor · ";

/// Valores de `autoScale` en Magpie.
pub const AUTO_OFF: i64 = 0;
pub const AUTO_FULLSCREEN: i64 = 1;
pub const AUTO_WINDOWED: i64 = 2;

/// Modos que elige el usuario por juego.
pub const MODES: &[&str] = &["off", "auto", "2x", "4x"];

/// Escalado elegido para un juego.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// Cuanto hay que ampliar la ventana para llenar el monitor (manteniendo la proporcion).
    pub factor: f64,
    /// 0 = no hace falta escalar (ya es nativa), 2 o 4 = pasos de ampliacion internos.
    pub scale: u8,
    /// Modelo principal (nombre corto, p. ej. "8x32").
    pub model: String,
    /// Nombre del modo de escalado en Magpie.
    pub mode_name: String,
    /// Cadena de efectos del modo (formato de `scalingModes[].effects`).
    pub effects: Value,
}

impl Plan {
    /// Resumen para la interfaz: «1280×720 → 2x · CuNNy 8x32».
    pub fn summary(&self, native: (u32, u32)) -> String {
        if self.scale == 0 {
            return format!("{}×{} · ya es nativa", native.0, native.1);
        }
        format!("{}×{} → {}x · {}", native.0, native.1, self.scale, self.model)
    }
}

/// Modelo CuNNy (entrenado para novelas visuales) segun la altura nativa, para que vaya fluido.
fn cunny_for(height: u32) -> &'static str {
    if height <= 800 {
        "8x32"
    } else if height <= 1100 {
        "4x16"
    } else {
        "fast"
    }
}

/// Archivos de efecto que necesita la seleccion automatica con CuNNy.
pub const CUNNY_FILES: &[&str] = &["CuNNy-8x32-NVL", "CuNNy-4x16-NVL", "CuNNy-fast-NVL", "CuNNy-3x12-NVL"];

fn lanczos_fit() -> Value {
    json!({ "name": "Lanczos", "scalingType": 1, "scale": { "x": 1.0, "y": 1.0 } })
}

fn rcas() -> Value {
    json!({ "name": "FSR\\FSR_RCAS", "parameters": { "sharpness": 0.45 } })
}

/// Decide el escalado. `forced`: `Some(2|4)` para imponerlo; `None` = automatico segun el factor.
/// `has_cunny`: estan los efectos CuNNy2 (si no, se usa Anime4K).
pub fn choose(native: (u32, u32), monitor: (u32, u32), forced: Option<u8>, has_cunny: bool) -> Plan {
    let (w, h) = (native.0.max(1) as f64, native.1.max(1) as f64);
    let factor = (monitor.0 as f64 / w).min(monitor.1 as f64 / h);
    let scale = match forced {
        Some(s @ (2 | 4)) => s,
        _ if factor <= 1.05 => 0,
        _ if factor <= 2.2 => 2,
        _ => 4,
    };
    let (model, first, second) = if has_cunny {
        let m = cunny_for(native.1);
        (format!("CuNNy {m}"), format!("CuNNy2\\CuNNy-{m}-NVL"), "CuNNy2\\CuNNy-3x12-NVL".to_string())
    } else {
        ("Anime4K".to_string(), "Anime4K\\Anime4K_Upscale_Denoise_L".to_string(), "Anime4K\\Anime4K_Upscale_S".to_string())
    };
    let mut effects = vec![json!({ "name": first })];
    if scale == 4 {
        effects.push(json!({ "name": second }));
    }
    effects.extend([lanczos_fit(), rcas()]);
    let mode_name = if scale == 0 { String::new() } else { format!("{MODE_PREFIX}{scale}x · {model}") };
    Plan { factor, scale, model, mode_name, effects: Value::Array(effects) }
}

/* ------------------------------------------------------------------ */
/* Archivo de configuracion                                             */
/* ------------------------------------------------------------------ */

/// `config.json` que usa una instalacion de Magpie: el portable (`<carpeta>\config\config.json`) si existe;
/// si no, el de `%LOCALAPPDATA%\Magpie\config\vN\` con la version mas alta.
pub fn config_path(magpie_exe: &Path, local_appdata: &Path) -> Option<PathBuf> {
    let portable = magpie_exe.parent()?.join("config").join("config.json");
    if portable.is_file() {
        return Some(portable);
    }
    let base = local_appdata.join("Magpie").join("config");
    let best = fs::read_dir(&base)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            let v: u32 = n.strip_prefix('v')?.parse().ok()?;
            e.path().join("config.json").is_file().then_some((v, e.path().join("config.json")))
        })
        .max_by_key(|(v, _)| *v)
        .map(|(_, p)| p);
    best.or_else(|| Some(base.join("config.json")).filter(|p| p.is_file()))
}

/// Estan los modelos CuNNy2 en la carpeta `effects` de Magpie.
pub fn has_cunny(magpie_exe: &Path) -> bool {
    let Some(dir) = magpie_exe.parent().map(|d| d.join("effects").join("CuNNy2")) else { return false };
    CUNNY_FILES.iter().all(|f| dir.join(format!("{f}.hlsl")).is_file())
}

pub fn load(path: &Path) -> Result<Value> {
    let s = fs::read_to_string(path).with_context(|| format!("leyendo {}", path.display()))?;
    let v: Value = serde_json::from_str(s.trim_start_matches('\u{feff}')).with_context(|| format!("{} no es JSON valido", path.display()))?;
    if !v.get("profiles").map(Value::is_array).unwrap_or(false) {
        anyhow::bail!("{} no parece una configuracion de Magpie", path.display());
    }
    Ok(v)
}

/// Guarda la configuracion. La primera vez deja una copia `config.json.bak-visor` del original.
pub fn save(path: &Path, cfg: &Value) -> Result<()> {
    let bak = path.with_file_name("config.json.bak-visor");
    if path.is_file() && !bak.exists() {
        fs::copy(path, &bak).with_context(|| format!("copiando {}", bak.display()))?;
    }
    write_atomic(path, serde_json::to_string_pretty(cfg)?.as_bytes())
}

/* ------------------------------------------------------------------ */
/* Modos y perfiles                                                     */
/* ------------------------------------------------------------------ */

fn arr_mut<'a>(cfg: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !cfg.get(key).map(Value::is_array).unwrap_or(false) {
        cfg[key] = Value::Array(Vec::new());
    }
    cfg[key].as_array_mut().unwrap()
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

/// Indice del modo de escalado con ese nombre; lo crea (o corrige sus efectos) si hace falta.
pub fn ensure_mode(cfg: &mut Value, name: &str, effects: &Value) -> usize {
    let modes = arr_mut(cfg, "scalingModes");
    if let Some(i) = modes.iter().position(|m| str_of(m, "name") == name) {
        if modes[i].get("effects") != Some(effects) {
            modes[i]["effects"] = effects.clone();
        }
        return i;
    }
    modes.push(json!({ "name": name, "effects": effects }));
    modes.len() - 1
}

/// Perfil de Magpie para ese ejecutable (sin distinguir mayusculas). Prefiere el que tenga la misma clase de
/// ventana si hay varios. El perfil 0 son los valores por defecto y nunca cuenta.
pub fn find_profile(cfg: &Value, exe: &str, class: &str) -> Option<usize> {
    let profiles = cfg.get("profiles")?.as_array()?;
    let same_exe = |p: &Value| str_of(p, "pathRule").eq_ignore_ascii_case(exe) && !p.get("packaged").and_then(Value::as_bool).unwrap_or(false);
    let found = profiles.iter().enumerate().skip(1).filter(|(_, p)| same_exe(p));
    let all: Vec<usize> = found.map(|(i, _)| i).collect();
    all.iter().copied().find(|&i| str_of(&profiles[i], "classNameRule") == class).or_else(|| all.first().copied())
}

/// Ajusta (o crea) el perfil del juego para que Magpie lo escale solo con el plan dado. Si el usuario ya tenia
/// un perfil, solo se cambian la clase de ventana, `autoScale` y `scalingMode`; el resto (cursor, recorte...) se
/// respeta. Si el plan dice que no hace falta escalar, se desactiva el escalado automatico. Devuelve si cambio algo.
pub fn apply(cfg: &mut Value, exe: &str, class: &str, name: &str, plan: &Plan) -> bool {
    let before = cfg.clone();
    let mode = (plan.scale != 0).then(|| ensure_mode(cfg, &plan.mode_name, &plan.effects));
    let idx = match find_profile(cfg, exe, class) {
        Some(i) => i,
        None if mode.is_none() => return *cfg != before, // nada que escalar ni perfil que desactivar
        None => {
            let profiles = arr_mut(cfg, "profiles");
            let mut p = profiles.first().cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
            let obj = p.as_object_mut().unwrap();
            obj.insert("name".into(), json!(name));
            obj.insert("packaged".into(), json!(false));
            obj.insert("pathRule".into(), json!(exe));
            obj.insert("classNameRule".into(), json!(class));
            obj.insert("launcherPath".into(), json!(""));
            obj.insert("launchParameters".into(), json!(""));
            profiles.push(p);
            profiles.len() - 1
        }
    };
    let p = &mut cfg["profiles"][idx];
    p["classNameRule"] = json!(class);
    match mode {
        Some(m) => {
            // Si el usuario lo tenia en modo ventana, se respeta; si no, pantalla completa.
            let auto = if p.get("autoScale").and_then(Value::as_i64) == Some(AUTO_WINDOWED) { AUTO_WINDOWED } else { AUTO_FULLSCREEN };
            p["autoScale"] = json!(auto);
            p["scalingMode"] = json!(m);
        }
        None => p["autoScale"] = json!(AUTO_OFF),
    }
    *cfg != before
}

/// Quita el escalado automatico del perfil de ese ejecutable (el perfil se conserva). Devuelve si cambio algo.
pub fn disable(cfg: &mut Value, exe: &str) -> bool {
    let Some(profiles) = cfg.get_mut("profiles").and_then(Value::as_array_mut) else { return false };
    let mut changed = false;
    for p in profiles.iter_mut().skip(1) {
        if str_of(p, "pathRule").eq_ignore_ascii_case(exe) && p.get("autoScale").and_then(Value::as_i64).unwrap_or(0) != AUTO_OFF {
            p["autoScale"] = json!(AUTO_OFF);
            changed = true;
        }
    }
    changed
}

/// Modo forzado a partir del que elige el usuario ("2x", "4x"; el resto es automatico).
pub fn forced_scale(mode: &str) -> Option<u8> {
    match mode {
        "2x" => Some(2),
        "4x" => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fragmento de una configuracion real de Magpie v0.12.1.
    fn sample() -> Value {
        serde_json::from_str(
            r#"{
            "language": "", "theme": 2, "overlay": { "x": 1 },
            "scalingModes": [
                { "name": "ACNet", "effects": [ { "name": "ACNet" } ] },
                { "name": "VN", "effects": [ { "name": "CuNNy2\\CuNNy-8x32-NVL" } ] }
            ],
            "profiles": [
                { "scalingMode": 1, "captureMethod": 0, "cursorScaling": 2, "croppingEnabled": true },
                { "name": "GINKA", "packaged": false, "pathRule": "P:\\Visual Novels\\Ginka\\GINKA.exe",
                  "classNameRule": "TVPMainWindow", "autoScale": 1, "scalingMode": 0, "cursorScaling": 5, "3DGameMode": false },
                { "name": "Gore", "pathRule": "P:\\VN\\gss.exe", "classNameRule": "bcyc", "autoScale": 2, "scalingMode": 0 }
            ]
        }"#,
        )
        .unwrap()
    }

    #[test]
    fn decision_table_follows_native_resolution() {
        let mon = (2560, 1440);
        let p = choose((1280, 720), mon, None, true);
        assert_eq!((p.scale, p.model.as_str()), (2, "CuNNy 8x32"));
        assert!((p.factor - 2.0).abs() < 1e-9);
        assert_eq!(p.mode_name, "Visor · 2x · CuNNy 8x32");
        assert_eq!(p.effects.as_array().unwrap().len(), 3);
        // 800x600 en 1440p = 2,4: sobremuestreo 4x con dos pasos.
        let p = choose((800, 600), mon, None, true);
        assert_eq!(p.scale, 4);
        let names: Vec<&str> = p.effects.as_array().unwrap().iter().map(|e| e["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["CuNNy2\\CuNNy-8x32-NVL", "CuNNy2\\CuNNy-3x12-NVL", "Lanczos", "FSR\\FSR_RCAS"]);
        // 1080p: modelo mas ligero; 1440p: ya es nativa.
        assert_eq!(choose((1920, 1080), mon, None, true).model, "CuNNy 4x16");
        assert_eq!(choose((2560, 1440), mon, None, true).scale, 0);
        // En un monitor 4K una 720p pasa a 4x; forzar manda sobre la tabla.
        assert_eq!(choose((1280, 720), (3840, 2160), None, true).scale, 4);
        assert_eq!(choose((1280, 720), mon, Some(4), true).scale, 4);
        assert_eq!(choose((2560, 1440), mon, Some(2), true).scale, 2);
        // Sin CuNNy2, Anime4K.
        assert_eq!(choose((1280, 720), mon, None, false).model, "Anime4K");
        assert_eq!(choose((1280, 720), mon, None, true).summary((1280, 720)), "1280×720 → 2x · CuNNy 8x32");
    }

    #[test]
    fn new_profile_copies_defaults_and_keeps_everything_else() {
        let mut c = sample();
        let plan = choose((1280, 720), (2560, 1440), None, true);
        assert!(apply(&mut c, "P:\\VN\\Nueva\\nueva.exe", "TVPMainWindow", "Nueva", &plan));
        let modes = c["scalingModes"].as_array().unwrap();
        assert_eq!(modes.len(), 3, "se anade el modo del visor sin tocar los del usuario");
        assert_eq!(modes[0]["name"], "ACNet");
        let p = c["profiles"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(p["name"], "Nueva");
        assert_eq!(p["pathRule"], "P:\\VN\\Nueva\\nueva.exe");
        assert_eq!((p["autoScale"].as_i64(), p["scalingMode"].as_i64()), (Some(1), Some(2)));
        assert_eq!(p["cursorScaling"], 2, "copia los valores por defecto del perfil 0");
        assert_eq!(c["overlay"]["x"], 1, "campos desconocidos intactos");
        // Aplicar otra vez lo mismo no cambia nada (no hace falta reiniciar Magpie).
        assert!(!apply(&mut c, "p:\\vn\\nueva\\NUEVA.exe", "TVPMainWindow", "Nueva", &plan));
        assert_eq!(c["scalingModes"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn existing_profile_only_changes_scaling_fields() {
        let mut c = sample();
        let plan = choose((800, 600), (2560, 1440), None, true);
        assert!(apply(&mut c, "p:\\visual novels\\ginka\\ginka.exe", "TVPMainWindow", "otro nombre", &plan));
        let p = &c["profiles"][1];
        assert_eq!(p["name"], "GINKA", "el nombre del usuario se respeta");
        assert_eq!(p["cursorScaling"], 5);
        assert_eq!(p["scalingMode"], 2);
        assert_eq!(c["profiles"].as_array().unwrap().len(), 3, "no se duplica el perfil");
        // Un perfil en modo ventana sigue en modo ventana.
        let plan2 = choose((1280, 720), (2560, 1440), None, true);
        apply(&mut c, "P:\\VN\\gss.exe", "bcyc", "Gore", &plan2);
        assert_eq!((c["profiles"][2]["autoScale"].as_i64(), c["profiles"][2]["scalingMode"].as_i64()), (Some(2), Some(3)));
    }

    #[test]
    fn native_resolution_and_disable_turn_autoscale_off() {
        let mut c = sample();
        let native = choose((2560, 1440), (2560, 1440), None, true);
        assert!(apply(&mut c, "P:\\Visual Novels\\Ginka\\GINKA.exe", "TVPMainWindow", "GINKA", &native));
        assert_eq!(c["profiles"][1]["autoScale"], 0);
        assert!(!apply(&mut c, "P:\\otra\\sin_perfil.exe", "X", "X", &native), "sin perfil y sin escalar: nada");
        let mut c = sample();
        assert!(disable(&mut c, "p:\\vn\\GSS.exe"));
        assert_eq!(c["profiles"][2]["autoScale"], 0);
        assert!(!disable(&mut c, "p:\\vn\\GSS.exe"));
    }

    #[test]
    fn config_file_roundtrip_with_backup_and_paths() {
        let d = tempfile::tempdir().unwrap();
        let la = d.path().join("la");
        for v in ["v3", "v4"] {
            fs::create_dir_all(la.join("Magpie/config").join(v)).unwrap();
            fs::write(la.join("Magpie/config").join(v).join("config.json"), "\u{feff}".to_string() + &sample().to_string()).unwrap();
        }
        let exe = d.path().join("Magpie v.2/Magpie.exe");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        let p = config_path(&exe, &la).unwrap();
        assert!(p.ends_with("v4/config.json"), "la version mas alta: {}", p.display());
        let mut c = load(&p).unwrap();
        disable(&mut c, "P:\\VN\\gss.exe");
        save(&p, &c).unwrap();
        assert!(p.with_file_name("config.json.bak-visor").is_file());
        assert_eq!(load(&p).unwrap()["profiles"][2]["autoScale"], 0);
        // La copia es del original y no se pisa en guardados posteriores.
        save(&p, &sample()).unwrap();
        assert_eq!(load(&p.with_file_name("config.json.bak-visor")).unwrap()["profiles"][2]["autoScale"], 2);
        // Portable: config junto al exe.
        fs::create_dir_all(exe.parent().unwrap().join("config")).unwrap();
        fs::write(exe.parent().unwrap().join("config/config.json"), "{}").unwrap();
        assert!(config_path(&exe, &la).unwrap().starts_with(exe.parent().unwrap()));
        assert!(!has_cunny(&exe));
        fs::write(d.path().join("x.json"), "{\"a\":1}").unwrap();
        assert!(load(&d.path().join("x.json")).is_err(), "un JSON que no es de Magpie no se acepta");
    }
}
