//! Datos del usuario que no son archivos suyos: juegos ocultos y preferencias por juego
//! (color de acento manual, orden de capturas y escalado con Magpie). Se guardan en la base (`db::data`);
//! aqui estan los tipos y las operaciones sobre la copia en memoria.
use crate::palette::{parse_hex, to_hex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Id estable de un titulo: el del lanzador (`steam:123`, `manual:...`) si lo hay; si no,
/// `plataforma/carpeta` en minusculas (la carpeta real puede cambiar de mayusculas).
pub fn title_id(game_id: Option<&str>, platform_id: &str, folder: &str) -> String {
    match game_id {
        Some(g) if !g.is_empty() => g.to_string(),
        _ => format!("{}/{}", platform_id, folder.to_lowercase()),
    }
}

/* ------------------------------------------------------------------ */
/* Juegos ocultos                                                       */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HiddenEntry {
    pub id: String,
    pub name: String,
    /// Id de la plataforma (para mostrarla en la lista de ocultos).
    pub platform: String,
}

/// Oculta (o muestra) un titulo. No toca ningun archivo del juego.
pub fn set_hidden(list: &mut Vec<HiddenEntry>, e: HiddenEntry, hidden: bool) {
    list.retain(|x| x.id != e.id);
    if hidden {
        list.push(e);
    }
}

pub fn hidden_ids(list: &[HiddenEntry]) -> HashSet<String> {
    list.iter().map(|e| e.id.clone()).collect()
}

/* ------------------------------------------------------------------ */
/* Preferencias por juego                                               */
/* ------------------------------------------------------------------ */

/// Ordenes de capturas.
pub const SORT_MODES: &[&str] = &["recent", "oldest", "favs", "name"];

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TitlePref {
    /// Color de acento manual `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    /// Orden de capturas recordado.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Estado del juego ("pending" | "playing" | "finished" | "dropped"); sin valor = sin estado.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Cuando se marco como terminado (ms desde epoch). Solo con `status == "finished"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_ms: Option<i64>,
    /// Escalado con Magpie al lanzar el juego.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magpie: Option<MagpiePref>,
}

/// Escalado con Magpie de un juego: el modo elegido y lo medido de su ventana la ultima vez que se lanzo.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MagpiePref {
    /// Ver `magpie::MODES` ("off" | "auto" | "2x" | "4x").
    pub mode: String,
    /// Tamano nativo de la ventana del juego (zona cliente).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native: Option<(u32, u32)>,
    /// Tamano del monitor donde estaba la ventana.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<(u32, u32)>,
    /// Clase de la ventana (regla de Magpie).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// Ejecutable real de la ventana (regla de Magpie; puede no ser el que se lanza).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exe: Option<String>,
    /// Resumen del ultimo escalado aplicado («1280×720 → 2x · CuNNy 8x32»).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// El visor ha escrito un perfil de Magpie para este juego (para desactivarlo si se apaga).
    #[serde(default)]
    pub managed: bool,
}

impl MagpiePref {
    pub fn enabled(&self) -> bool {
        self.mode != "off" && !self.mode.is_empty()
    }
}

pub type TitlePrefs = BTreeMap<String, TitlePref>;

fn prune(p: &mut TitlePrefs, id: &str) {
    if p.get(id).map(|x| x.accent.is_none() && x.sort.is_none() && x.magpie.is_none() && x.status.is_none()).unwrap_or(false) {
        p.remove(id);
    }
}

/// Fija o quita (`None`) el acento manual. Devuelve error si el color no es `#rrggbb`.
pub fn set_accent(p: &mut TitlePrefs, id: &str, color: Option<&str>) -> Result<(), String> {
    match color {
        Some(c) => {
            let rgb = parse_hex(c).ok_or("Color no valido (se espera #rrggbb)")?;
            p.entry(id.to_string()).or_default().accent = Some(to_hex(rgb));
        }
        None => {
            if let Some(x) = p.get_mut(id) {
                x.accent = None;
            }
            prune(p, id);
        }
    }
    Ok(())
}

pub fn set_sort(p: &mut TitlePrefs, id: &str, mode: &str) -> Result<(), String> {
    if !SORT_MODES.contains(&mode) {
        return Err(format!("Orden desconocido: {mode}"));
    }
    if mode == "recent" {
        // Es el valor por defecto: no hace falta guardarlo.
        if let Some(x) = p.get_mut(id) {
            x.sort = None;
        }
        prune(p, id);
    } else {
        p.entry(id.to_string()).or_default().sort = Some(mode.to_string());
    }
    Ok(())
}

/// Estados de juego validos (ademas de «sin estado», que es quitarlo).
pub const STATUSES: &[&str] = &["pending", "playing", "finished", "dropped"];

/// Fija el estado de un titulo ("none" o vacio = sin estado). Al pasar a "finished" se guarda la fecha
/// (`now_ms`); si ya estaba terminado se conserva la anterior. Con otro estado la fecha se borra.
pub fn set_status(p: &mut TitlePrefs, id: &str, status: &str, now_ms: i64) -> Result<(), String> {
    if status.is_empty() || status == "none" {
        if let Some(x) = p.get_mut(id) {
            x.status = None;
            x.finished_ms = None;
        }
        prune(p, id);
        return Ok(());
    }
    if !STATUSES.contains(&status) {
        return Err(format!("Estado desconocido: {status}"));
    }
    let e = p.entry(id.to_string()).or_default();
    if status == "finished" {
        if e.status.as_deref() != Some("finished") || e.finished_ms.is_none() {
            e.finished_ms = Some(now_ms);
        }
    } else {
        e.finished_ms = None;
    }
    e.status = Some(status.to_string());
    Ok(())
}

/// Cambia el modo de Magpie de un titulo. Lo medido se conserva para no tener que volver a medir.
pub fn set_magpie_mode(p: &mut TitlePrefs, id: &str, mode: &str) -> Result<(), String> {
    if !crate::magpie::MODES.contains(&mode) {
        return Err(format!("Modo de Magpie desconocido: {mode}"));
    }
    let e = p.entry(id.to_string()).or_default();
    match e.magpie.as_mut() {
        Some(m) => m.mode = mode.to_string(),
        None if mode == "off" => {}
        None => e.magpie = Some(MagpiePref { mode: mode.to_string(), ..Default::default() }),
    }
    prune(p, id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_rules() {
        let mut p = TitlePrefs::new();
        assert!(set_status(&mut p, "a", "raro", 1).is_err());
        set_status(&mut p, "a", "playing", 10).unwrap();
        assert_eq!((p["a"].status.as_deref(), p["a"].finished_ms), (Some("playing"), None));
        set_status(&mut p, "a", "finished", 20).unwrap();
        assert_eq!(p["a"].finished_ms, Some(20));
        set_status(&mut p, "a", "finished", 99).unwrap();
        assert_eq!(p["a"].finished_ms, Some(20), "la fecha de fin se conserva");
        set_status(&mut p, "a", "dropped", 30).unwrap();
        assert_eq!(p["a"].finished_ms, None);
        set_accent(&mut p, "a", Some("#112233")).unwrap();
        set_status(&mut p, "a", "none", 40).unwrap();
        assert!(p.contains_key("a"), "conserva el acento");
        set_accent(&mut p, "a", None).unwrap();
        assert!(p.is_empty(), "sin nada que guardar se poda");
    }

    #[test]
    fn title_id_prefers_launcher_id_else_platform_folder() {
        assert_eq!(title_id(Some("steam:1245620"), "pc", "Elden Ring"), "steam:1245620");
        assert_eq!(title_id(None, "pc", "Carpeta Suelta"), "pc/carpeta suelta");
        assert_eq!(title_id(Some(""), "switch", "Zelda"), "switch/zelda");
    }

    #[test]
    fn hidden_list_persists_and_toggles() {
        let db = crate::db::Db::open_memory().unwrap();
        assert!(db.with(crate::db::data::hidden_load).unwrap().is_empty(), "sin datos, lista vacia");
        let mut l = Vec::new();
        let e = |id: &str| HiddenEntry { id: id.into(), name: format!("Juego {id}"), platform: "pc".into() };
        set_hidden(&mut l, e("steam:1"), true);
        set_hidden(&mut l, e("pc/x"), true);
        set_hidden(&mut l, e("steam:1"), true); // sin duplicados
        assert_eq!(l.len(), 2);
        // Se escribe fila a fila, igual que la lista en memoria.
        for x in &l {
            db.with(|c| crate::db::data::hidden_set(c, x, true)).unwrap();
        }
        db.with(|c| crate::db::data::hidden_set(c, &l[0], true)).unwrap(); // sin duplicados
        let back = db.with(crate::db::data::hidden_load).unwrap();
        assert_eq!(back, vec![l[1].clone(), l[0].clone()], "al ocultar de nuevo pasa al final");
        assert!(hidden_ids(&back).contains("pc/x"));
        let mut back = back;
        set_hidden(&mut back, e("steam:1"), false);
        db.with(|c| crate::db::data::hidden_set(c, &e("steam:1"), false)).unwrap();
        assert_eq!(db.with(crate::db::data::hidden_load).unwrap().iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["pc/x"]);
        assert_eq!(db.with(crate::db::data::hidden_load).unwrap(), back);
    }

    #[test]
    fn accent_override_validates_and_clears() {
        let db = crate::db::Db::open_memory().unwrap();
        let mut p = TitlePrefs::new();
        set_accent(&mut p, "steam:1", Some("#00A0D8")).unwrap();
        assert_eq!(p["steam:1"].accent.as_deref(), Some("#00a0d8"), "se guarda en minusculas");
        assert!(set_accent(&mut p, "steam:1", Some("azul")).is_err());
        set_sort(&mut p, "steam:1", "name").unwrap();
        db.with(|c| crate::db::data::pref_save(c, "steam:1", p.get("steam:1"))).unwrap();
        let back = db.with(crate::db::data::prefs_load).unwrap();
        assert_eq!(back, p);
        let mut back = back;
        set_accent(&mut back, "steam:1", None).unwrap();
        assert_eq!(back["steam:1"].accent, None);
        assert_eq!(back["steam:1"].sort.as_deref(), Some("name"), "borrar el acento no borra el orden");
        set_sort(&mut back, "steam:1", "recent").unwrap();
        assert!(!back.contains_key("steam:1"), "sin preferencias, la entrada desaparece");
        db.with(|c| crate::db::data::pref_save(c, "steam:1", back.get("steam:1"))).unwrap();
        assert!(db.with(crate::db::data::prefs_load).unwrap().is_empty());
        assert!(set_sort(&mut back, "x", "aleatorio").is_err());
    }

    #[test]
    fn magpie_mode_keeps_measurements() {
        let mut p = TitlePrefs::new();
        set_magpie_mode(&mut p, "manual:1", "off").unwrap();
        assert!(!p.contains_key("manual:1"), "apagar sin nada guardado no crea entrada");
        set_magpie_mode(&mut p, "manual:1", "auto").unwrap();
        p.get_mut("manual:1").unwrap().magpie.as_mut().unwrap().native = Some((1280, 720));
        set_magpie_mode(&mut p, "manual:1", "off").unwrap();
        let m = p["manual:1"].magpie.clone().unwrap();
        assert_eq!((m.mode.as_str(), m.native, m.enabled()), ("off", Some((1280, 720)), false));
        assert!(set_magpie_mode(&mut p, "manual:1", "8k").is_err());
        let s = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<TitlePrefs>(&s).unwrap(), p);
        // Ida y vuelta por la base, con lo medido y sin ello.
        let db = crate::db::Db::open_memory().unwrap();
        db.with(|c| crate::db::data::pref_save(c, "manual:1", p.get("manual:1"))).unwrap();
        assert_eq!(db.with(crate::db::data::prefs_load).unwrap(), p);
        let mut q = p.clone();
        q.get_mut("manual:1").unwrap().magpie.as_mut().unwrap().native = None;
        db.with(|c| crate::db::data::pref_save(c, "manual:1", q.get("manual:1"))).unwrap();
        assert_eq!(db.with(crate::db::data::prefs_load).unwrap(), q);
    }
}
