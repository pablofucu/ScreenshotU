//! Sistema de logros para juegos fuera de Steam:
//! - Lector de esquemas (`steam_settings/achievements.json`) y runtime (`GSE Saves/<appid>/achievements.json`).
//! - Deteccion y clasificacion en grupos 1 a 4 segun presencia de `steam_api(64).dll` y configuraciones.
//! - Persistencia en SQLite (tablas `achievements`, `achievement_unlocks`, `title_achievements_pref`).
//! - Resumen para tarjetas en la ficha (variantes 1, 2, 4, 7 y 8).

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Esquema de la tabla de logros y sus estados de desbloqueo.
pub const SCHEMA: &str = r#"
CREATE TABLE achievements (
    app_id       TEXT NOT NULL,
    api_name     TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    hidden       INTEGER NOT NULL DEFAULT 0,
    icon         TEXT,
    icon_gray    TEXT,
    source       TEXT NOT NULL DEFAULT 'gse',
    PRIMARY KEY (app_id, api_name)
);

CREATE TABLE achievement_unlocks (
    app_id      TEXT NOT NULL,
    api_name    TEXT NOT NULL,
    unlocked_at INTEGER NOT NULL,
    source      TEXT NOT NULL DEFAULT 'gse',
    evidence    TEXT,
    notified    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (app_id, api_name)
);

CREATE INDEX idx_achievement_unlocks_app_id ON achievement_unlocks(app_id);
CREATE INDEX idx_achievement_unlocks_notified ON achievement_unlocks(notified);

CREATE TABLE title_achievements_pref (
    title_id TEXT PRIMARY KEY,
    variant  INTEGER NOT NULL DEFAULT 1
);
"#;

pub const VALID_VARIANTS: &[u8] = &[1, 2, 3, 4, 7, 8];

/// Normaliza la variante de tarjeta elegida a una de las permitidas (1, 2, 3, 4, 7 u 8).
pub fn normalize_variant(v: u8) -> u8 {
    if VALID_VARIANTS.contains(&v) {
        v
    } else {
        1
    }
}

/// Representacion de un logro.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Achievement {
    pub app_id: String,
    pub api_name: String,
    pub display_name: String,
    pub description: String,
    pub hidden: bool,
    pub icon: Option<String>,
    pub icon_gray: Option<String>,
    pub earned: bool,
    pub unlocked_at: Option<i64>,
    pub source: String,
    pub stat_current: Option<u64>,
    pub stat_max: Option<u64>,
    pub rarity_tier: u8, // 0: Comun (>=15%), 1: Raro (5-15%), 2: Ultra raro (<5%)
    pub rarity_pct: Option<f32>,
}

/// Resumen agregado de los logros de un juego para la interfaz y las tarjetas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AchievementSummary {
    pub app_id: String,
    pub total: usize,
    pub unlocked: usize,
    pub pct: u32,
    pub last_unlocked: Option<Achievement>,
    pub recent: Vec<Achievement>,
    pub week_unlocked: [u32; 7],
    pub rarity_counts: (u32, u32, u32), // (comun, raro, ultra_raro)
    pub next_target: Option<Achievement>,
    pub variant: u8,
}

/// Clasificacion de compatibilidad Steamworks de un juego.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameDetection {
    pub group: u8, // 1..4
    pub dll_state: String,
    pub calls_steam: bool,
    pub has_settings: bool,
    pub app_id: Option<String>,
}

/* ------------------------------------------------------------------ */
/* Parsers de schema y runtime                                        */
/* ------------------------------------------------------------------ */

/// Parsea el fichero `achievements.json` de definiciones (schema).
/// Soporta arrays simples u objetos `{ "achievements": [...] }`.
pub fn parse_schema_json(
    app_id: &str,
    json_str: &str,
    base_dir: Option<&Path>,
) -> Result<Vec<Achievement>> {
    let root: serde_json::Value =
        serde_json::from_str(json_str).context("formato JSON invalido en schema")?;

    let items = if let Some(arr) = root.as_array() {
        arr
    } else if let Some(arr) = root.get("achievements").and_then(|v| v.as_array()) {
        arr
    } else {
        return Ok(Vec::new());
    };

    let mut list = Vec::new();

    for item in items {
        let api_name = item
            .get("name")
            .or_else(|| item.get("api_name"))
            .or_else(|| item.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        if api_name.is_empty() {
            continue;
        }

        let display_name = item
            .get("displayName")
            .or_else(|| item.get("display_name"))
            .or_else(|| item.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or(api_name)
            .to_string();

        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let hidden = match item.get("hidden") {
            Some(v) if v.is_boolean() => v.as_bool().unwrap_or(false),
            Some(v) if v.is_number() => v.as_i64().unwrap_or(0) == 1,
            Some(v) if v.is_string() => {
                let s = v.as_str().unwrap_or("0");
                s == "1" || s.eq_ignore_ascii_case("true")
            }
            _ => false,
        };

        let resolve_img = |key: &str| -> Option<String> {
            let path_str = item.get(key).and_then(|v| v.as_str())?.trim();
            if path_str.is_empty() {
                return None;
            }
            if let Some(base) = base_dir {
                let p = Path::new(path_str);
                if p.is_absolute() {
                    Some(path_str.to_string())
                } else {
                    let full = base.join(p);
                    Some(full.to_string_lossy().to_string())
                }
            } else {
                Some(path_str.to_string())
            }
        };

        let icon = resolve_img("icon");
        let icon_gray = resolve_img("icongray").or_else(|| resolve_img("icon_gray"));

        list.push(Achievement {
            app_id: app_id.to_string(),
            api_name: api_name.to_string(),
            display_name,
            description,
            hidden,
            icon,
            icon_gray,
            earned: false,
            unlocked_at: None,
            source: "gse".to_string(),
            stat_current: None,
            stat_max: None,
            rarity_tier: 0,
            rarity_pct: None,
        });
    }

    Ok(list)
}

/// Parsea el runtime `achievements.json` (progreso).
/// Devuelve un mapa de `api_name` -> (earned, Option<unlocked_at_sec>).
pub fn parse_runtime_json(json_str: &str) -> Result<HashMap<String, (bool, Option<i64>)>> {
    let root: serde_json::Value =
        serde_json::from_str(json_str).context("formato JSON invalido en runtime")?;
    let mut map = HashMap::new();

    let obj = match root.as_object() {
        Some(o) => o,
        None => return Ok(map),
    };

    for (k, v) in obj {
        if let Some(b) = v.as_bool() {
            if b {
                map.insert(k.clone(), (true, None));
            }
            continue;
        }

        if let Some(item_obj) = v.as_object() {
            let earned = item_obj
                .get("earned")
                .or_else(|| item_obj.get("achieved"))
                .and_then(|val| {
                    if let Some(b) = val.as_bool() {
                        Some(b)
                    } else if let Some(n) = val.as_i64() {
                        Some(n > 0)
                    } else {
                        None
                    }
                })
                .unwrap_or(false);

            let time = item_obj
                .get("earned_time")
                .or_else(|| item_obj.get("unlock_time"))
                .or_else(|| item_obj.get("time"))
                .and_then(|val| val.as_i64());

            if earned {
                map.insert(k.clone(), (true, time));
            }
        }
    }

    Ok(map)
}

/// Parsea `stats.json` para extraer valores de contadores.
pub fn parse_stats_json(json_str: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    let Ok(root) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return map;
    };

    if let Some(obj) = root.as_object() {
        for (k, v) in obj {
            if let Some(n) = v.as_u64() {
                map.insert(k.clone(), n);
            } else if let Some(item_obj) = v.as_object() {
                if let Some(n) = item_obj.get("value").and_then(|x| x.as_u64()) {
                    map.insert(k.clone(), n);
                }
            }
        }
    }
    map
}

/* ------------------------------------------------------------------ */
/* Localizacion de ficheros en disco                                  */
/* ------------------------------------------------------------------ */

/// Localiza el AppID de un juego buscando `steam_appid.txt` o en subcarpetas.
pub fn find_app_id(game_dir: &Path, exe_path: Option<&Path>) -> Option<String> {
    // 1. Junto al ejecutable
    if let Some(exe) = exe_path {
        if let Some(parent) = exe.parent() {
            let f = parent.join("steam_appid.txt");
            if let Ok(txt) = fs::read_to_string(f) {
                if let Some(id) = parse_appid_txt(&txt) {
                    return Some(id);
                }
            }
        }
    }

    // 2. En la raiz del juego
    let f = game_dir.join("steam_appid.txt");
    if let Ok(txt) = fs::read_to_string(f) {
        if let Some(id) = parse_appid_txt(&txt) {
            return Some(id);
        }
    }

    // 3. En steam_settings/steam_appid.txt
    let f = game_dir.join("steam_settings").join("steam_appid.txt");
    if let Ok(txt) = fs::read_to_string(f) {
        if let Some(id) = parse_appid_txt(&txt) {
            return Some(id);
        }
    }

    None
}

fn parse_appid_txt(s: &str) -> Option<String> {
    for line in s.lines() {
        let t = line.trim();
        if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
            return Some(t.to_string());
        }
    }
    None
}

/// Busca el fichero de definiciones de schema (`steam_settings/achievements.json`).
pub fn find_schema_file(game_dir: &Path) -> Option<PathBuf> {
    let direct = game_dir.join("steam_settings").join("achievements.json");
    if direct.is_file() {
        return Some(direct);
    }

    // En Unreal Engine o subcarpetas de binaries
    let entries = walkdir::WalkDir::new(game_dir)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in entries {
        if entry.file_type().is_dir() && entry.file_name() == "steam_settings" {
            let ach = entry.path().join("achievements.json");
            if ach.is_file() {
                return Some(ach);
            }
        }
    }

    None
}

/// Busca ficheros de progreso (runtime) conocidos para un AppID.
pub fn find_runtime_files(app_id: &str, game_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();

    // 1. local_save_path en steam_settings/configs.user.ini
    let ini_path = game_dir.join("steam_settings").join("configs.user.ini");
    if let Ok(content) = fs::read_to_string(&ini_path) {
        for line in content.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("local_save_path=") {
                let custom_rel = rest.trim();
                let target = game_dir.join(custom_rel).join(app_id).join("achievements.json");
                if target.is_file() {
                    out.push(target);
                }
            }
        }
    }

    // 2. Carpetas locales dentro del juego
    let local_gse = game_dir.join("saves").join(app_id).join("achievements.json");
    if local_gse.is_file() {
        out.push(local_gse);
    }
    let local_gse2 = game_dir.join("GSE Saves").join(app_id).join("achievements.json");
    if local_gse2.is_file() {
        out.push(local_gse2);
    }

    // 3. %APPDATA%\GSE Saves\<appid>\achievements.json
    if let Ok(appdata) = std::env::var("APPDATA") {
        let appdata_path = PathBuf::from(appdata);
        let gse1 = appdata_path.join("GSE Saves").join(app_id).join("achievements.json");
        if gse1.is_file() {
            out.push(gse1);
        }
        let gse2 = appdata_path.join("Goldberg SteamEmu Saves").join(app_id).join("achievements.json");
        if gse2.is_file() {
            out.push(gse2);
        }
    }

    out
}

/// Clasifica un juego en uno de los cuatro grupos segun su integracion de Steamworks.
pub fn classify_game(game_dir: &Path, exe_path: Option<&Path>) -> GameClassification {
    let mut dll_state = "missing".to_string();
    let mut has_dll = false;
    let mut is_valve = false;
    let mut is_emu = false;

    // Buscar DLLs de Steamworks
    let entries = walkdir::WalkDir::new(game_dir)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name == "steam_api64.dll" || name == "steam_api.dll" {
            has_dll = true;
            if let Ok(meta) = entry.metadata() {
                let size = meta.len();
                if size >= 4 * 1024 * 1024 {
                    is_emu = true;
                    dll_state = "emulator".to_string();
                } else if size <= 900 * 1024 {
                    is_valve = true;
                    dll_state = "valve".to_string();
                } else {
                    dll_state = "unknown".to_string();
                }
            }
            break;
        }
    }

    let has_settings = game_dir.join("steam_settings").is_dir()
        || walkdir::WalkDir::new(game_dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|e| e.file_type().is_dir() && e.file_name() == "steam_settings");

    let app_id = find_app_id(game_dir, exe_path);

    let group = if has_settings || is_emu {
        2
    } else if is_valve {
        1
    } else if has_dll {
        2
    } else {
        4
    };

    GameClassification {
        group,
        dll_state,
        calls_steam: has_dll || has_settings,
        has_settings,
        app_id,
    }
}

pub type GameClassification = GameDetection;

/* ------------------------------------------------------------------ */
/* Calculo de resumen y metricas                                      */
/* ------------------------------------------------------------------ */

/// Genera el resumen agregado para las tarjetas y vistas.
pub fn calculate_summary(
    app_id: &str,
    achievements: &[Achievement],
    variant: u8,
    now: DateTime<Local>,
) -> AchievementSummary {
    let total = achievements.len();
    let mut unlocked = 0;
    let mut recent_earned = Vec::new();
    let mut common = 0;
    let mut rare = 0;
    let mut ultra = 0;
    let mut next_target = None;

    // Calcular distribución por días (últimos 7 días)
    let today = now.date_naive();
    let midnight = |d: NaiveDate| -> i64 {
        let t = d.and_hms_opt(0, 0, 0).unwrap();
        now.timezone()
            .from_local_datetime(&t)
            .earliest()
            .unwrap_or(now)
            .timestamp()
    };

    let mut week_unlocked = [0u32; 7];

    for a in achievements {
        match a.rarity_tier {
            2 => ultra += 1,
            1 => rare += 1,
            _ => common += 1,
        }

        if a.earned {
            unlocked += 1;
            recent_earned.push(a.clone());

            if let Some(ts) = a.unlocked_at {
                for (i, slot) in week_unlocked.iter_mut().enumerate() {
                    let day = today - Duration::days(6 - i as i64);
                    let (from, to) = (midnight(day), midnight(day + Duration::days(1)));
                    if ts >= from && ts < to {
                        *slot += 1;
                    }
                }
            }
        } else if next_target.is_none() && a.stat_current.is_some() && a.stat_max.is_some() {
            next_target = Some(a.clone());
        }
    }

    // Ordenar desbloqueados recientes por fecha descendente
    recent_earned.sort_by(|x, y| y.unlocked_at.unwrap_or(0).cmp(&x.unlocked_at.unwrap_or(0)));

    let last_unlocked = recent_earned.first().cloned();
    let recent = recent_earned.into_iter().take(5).collect();

    let pct = if total > 0 {
        ((unlocked as f64 / total as f64) * 100.0).round() as u32
    } else {
        0
    };

    AchievementSummary {
        app_id: app_id.to_string(),
        total,
        unlocked,
        pct,
        last_unlocked,
        recent,
        week_unlocked,
        rarity_counts: (common, rare, ultra),
        next_target,
        variant: normalize_variant(variant),
    }
}

/* ------------------------------------------------------------------ */
/* Persistencia en SQLite                                             */
/* ------------------------------------------------------------------ */

pub fn achievements_save(
    c: &Connection,
    app_id: &str,
    list: &[Achievement],
) -> Result<()> {
    c.execute_batch("SAVEPOINT ach_save")?;
    let res = (|| -> Result<()> {
        let mut st = c.prepare(
            "INSERT INTO achievements(app_id, api_name, display_name, description, hidden, icon, icon_gray, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(app_id, api_name) DO UPDATE SET
                display_name = excluded.display_name,
                description = excluded.description,
                hidden = excluded.hidden,
                icon = excluded.icon,
                icon_gray = excluded.icon_gray,
                source = excluded.source",
        )?;

        for a in list {
            st.execute(params![
                app_id,
                a.api_name,
                a.display_name,
                a.description,
                if a.hidden { 1 } else { 0 },
                a.icon,
                a.icon_gray,
                a.source,
            ])?;
        }
        Ok(())
    })();

    if res.is_ok() {
        c.execute_batch("RELEASE ach_save")?;
    } else {
        let _ = c.execute_batch("ROLLBACK TO ach_save; RELEASE ach_save");
    }
    res
}

pub fn achievements_load(c: &Connection, app_id: &str) -> Result<Vec<Achievement>> {
    let mut st = c.prepare(
        "SELECT a.app_id, a.api_name, a.display_name, a.description, a.hidden, a.icon, a.icon_gray, a.source,
                u.unlocked_at
         FROM achievements a
         LEFT JOIN achievement_unlocks u ON a.app_id = u.app_id AND a.api_name = u.api_name
         WHERE a.app_id = ?1
         ORDER BY (u.unlocked_at IS NULL), u.unlocked_at DESC, a.rowid ASC",
    )?;

    let rows = st.query_map([app_id], |r| {
        let unlocked_at: Option<i64> = r.get(8)?;
        Ok(Achievement {
            app_id: r.get(0)?,
            api_name: r.get(1)?,
            display_name: r.get(2)?,
            description: r.get(3)?,
            hidden: r.get::<_, i64>(4)? != 0,
            icon: r.get(5)?,
            icon_gray: r.get(6)?,
            earned: unlocked_at.is_some(),
            unlocked_at,
            source: r.get(7)?,
            stat_current: None,
            stat_max: None,
            rarity_tier: 0,
            rarity_pct: None,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn unlock_record(
    c: &Connection,
    app_id: &str,
    api_name: &str,
    unlocked_at: i64,
    source: &str,
    evidence: Option<&str>,
) -> Result<bool> {
    let inserted = c.execute(
        "INSERT INTO achievement_unlocks(app_id, api_name, unlocked_at, source, evidence, notified)
         VALUES (?1, ?2, ?3, ?4, ?5, 0)
         ON CONFLICT(app_id, api_name) DO NOTHING",
        params![app_id, api_name, unlocked_at, source, evidence],
    )?;
    Ok(inserted > 0)
}

pub fn card_pref_get(c: &Connection, title_id: &str) -> Result<u8> {
    let var: Option<u8> = c
        .query_row(
            "SELECT variant FROM title_achievements_pref WHERE title_id = ?1",
            [title_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(normalize_variant(var.unwrap_or(1)))
}

pub fn card_pref_set(c: &Connection, title_id: &str, variant: u8) -> Result<()> {
    let norm = normalize_variant(variant);
    c.execute(
        "INSERT INTO title_achievements_pref(title_id, variant)
         VALUES (?1, ?2)
         ON CONFLICT(title_id) DO UPDATE SET variant = excluded.variant",
        params![title_id, norm],
    )?;
    Ok(())
}

/* ------------------------------------------------------------------ */
/* Pruebas unitarias                                                  */
/* ------------------------------------------------------------------ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_schema_variants() {
        let json = r#"[
            { "name": "ACH_1", "displayName": "Primero", "description": "Gana", "hidden": "0", "icon": "a.png" },
            { "name": "ACH_2", "displayName": "Secreto", "hidden": 1, "icongray": "b.png" }
        ]"#;

        let res = parse_schema_json("480", json, Some(Path::new("/game/steam_settings"))).unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].api_name, "ACH_1");
        assert_eq!(res[0].display_name, "Primero");
        assert!(!res[0].hidden);
        assert!(res[1].hidden);
    }

    #[test]
    fn parses_runtime_variants() {
        let json = r#"{
            "ACH_1": { "earned": true, "earned_time": 1710000000 },
            "ACH_2": { "achieved": 1, "unlock_time": 1720000000 },
            "ACH_3": { "earned": false }
        }"#;

        let res = parse_runtime_json(json).unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res["ACH_1"], (true, Some(1710000000)));
        assert_eq!(res["ACH_2"], (true, Some(1720000000)));
        assert!(!res.contains_key("ACH_3"));
    }

    #[test]
    fn normalizes_card_variants() {
        assert_eq!(normalize_variant(1), 1);
        assert_eq!(normalize_variant(2), 2);
        assert_eq!(normalize_variant(4), 4);
        assert_eq!(normalize_variant(7), 7);
        assert_eq!(normalize_variant(8), 8);
        assert_eq!(normalize_variant(3), 1);
        assert_eq!(normalize_variant(99), 1);
    }

    #[test]
    fn calculates_summary_metrics() {
        let achs = vec![
            Achievement {
                app_id: "100".into(),
                api_name: "A1".into(),
                display_name: "Uno".into(),
                description: "".into(),
                hidden: false,
                icon: None,
                icon_gray: None,
                earned: true,
                unlocked_at: Some(Local::now().timestamp()),
                source: "gse".into(),
                stat_current: None,
                stat_max: None,
                rarity_tier: 0,
                rarity_pct: None,
            },
            Achievement {
                app_id: "100".into(),
                api_name: "A2".into(),
                display_name: "Dos".into(),
                description: "".into(),
                hidden: false,
                icon: None,
                icon_gray: None,
                earned: false,
                unlocked_at: None,
                source: "gse".into(),
                stat_current: Some(5),
                stat_max: Some(10),
                rarity_tier: 1,
                rarity_pct: None,
            },
        ];

        let sum = calculate_summary("100", &achs, 7, Local::now());
        assert_eq!(sum.total, 2);
        assert_eq!(sum.unlocked, 1);
        assert_eq!(sum.pct, 50);
        assert_eq!(sum.variant, 7);
        assert_eq!(sum.recent.len(), 1);
        assert!(sum.next_target.is_some());
    }

    #[test]
    fn db_achievements_and_unlocks_crud() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();

        let list = vec![Achievement {
            app_id: "123".into(),
            api_name: "WIN".into(),
            display_name: "Ganador".into(),
            description: "Gana".into(),
            hidden: false,
            icon: Some("w.png".into()),
            icon_gray: None,
            earned: false,
            unlocked_at: None,
            source: "gse".into(),
            stat_current: None,
            stat_max: None,
            rarity_tier: 0,
            rarity_pct: None,
        }];

        achievements_save(&conn, "123", &list).unwrap();
        let loaded = achievements_load(&conn, "123").unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(!loaded[0].earned);

        let is_new = unlock_record(&conn, "123", "WIN", 1700000000, "gse", None).unwrap();
        assert!(is_new);
        let loaded_after = achievements_load(&conn, "123").unwrap();
        assert!(loaded_after[0].earned);
        assert_eq!(loaded_after[0].unlocked_at, Some(1700000000));

        // Preferencia de tarjeta
        assert_eq!(card_pref_get(&conn, "pc/mygame").unwrap(), 1);
        card_pref_set(&conn, "pc/mygame", 8).unwrap();
        assert_eq!(card_pref_get(&conn, "pc/mygame").unwrap(), 8);
    }
}
