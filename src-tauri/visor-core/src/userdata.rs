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
    /// Nota (1 a 5) de un juego terminado; sin valor = sin nota.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating: Option<i64>,
    /// Posicion en «Mi lista › Pendientes» (menor = antes). Solo con `status == "pending"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_pos: Option<i64>,
    /// «Dejarlo» ante el aviso de juego parado: cuando se dijo (no se vuelve a preguntar hasta que se juegue de nuevo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snooze_ms: Option<i64>,
    /// Cuando se fijo el estado actual (ms desde epoch); lo rellena la base al cargar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_ms: Option<i64>,
    /// Escalado con Magpie al lanzar el juego.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magpie: Option<MagpiePref>,
    /// Fecha de lanzamiento del juego (puesta a mano o sacada de SteamGridDB).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<ReleaseDate>,
}

/// Fecha de lanzamiento de un juego: el año siempre; mes y dia solo si se conocen (SteamGridDB solo da el año
/// fiable y mucha gente solo recuerda el año). `auto` = la puso el visor (SteamGridDB) y se puede sustituir sola;
/// una fecha puesta a mano nunca se pisa.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseDate {
    pub year: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<u32>,
    #[serde(default)]
    pub auto: bool,
}

/// Años de lanzamiento que se aceptan.
pub const RELEASE_YEARS: std::ops::RangeInclusive<i32> = 1950..=2100;

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

impl ReleaseDate {
    /// Comprueba que la fecha existe (el dia necesita mes).
    pub fn validate(&self) -> Result<(), String> {
        if !RELEASE_YEARS.contains(&self.year) {
            return Err(format!("Año de lanzamiento fuera de rango: {}", self.year));
        }
        match (self.month, self.day) {
            (None, Some(_)) => Err("El dia necesita el mes".into()),
            (Some(m), _) if !(1..=12).contains(&m) => Err(format!("Mes no valido: {m}")),
            (Some(m), Some(d)) if d < 1 || d > days_in_month(self.year, m) => Err(format!("Dia no valido: {d}")),
            _ => Ok(()),
        }
    }
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
    if p.get(id).map(|x| x.accent.is_none() && x.sort.is_none() && x.magpie.is_none() && x.status.is_none() && x.release.is_none()).unwrap_or(false) {
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
            x.rating = None;
            x.list_pos = None;
            x.snooze_ms = None;
            x.status_ms = None;
        }
        prune(p, id);
        return Ok(());
    }
    if !STATUSES.contains(&status) {
        return Err(format!("Estado desconocido: {status}"));
    }
    // Un pendiente nuevo va al final de la lista de pendientes.
    let next_pos = p.values().filter_map(|x| x.list_pos).max().unwrap_or(0) + 1;
    let e = p.entry(id.to_string()).or_default();
    if status == "finished" {
        if e.status.as_deref() != Some("finished") || e.finished_ms.is_none() {
            e.finished_ms = Some(now_ms);
        }
    } else {
        e.finished_ms = None;
        e.rating = None;
    }
    if status == "pending" {
        e.list_pos.get_or_insert(next_pos);
    } else {
        e.list_pos = None;
    }
    if e.status.as_deref() != Some(status) {
        e.status_ms = Some(now_ms);
    }
    e.snooze_ms = None;
    e.status = Some(status.to_string());
    Ok(())
}

/// Fecha de fin y nota de un juego ya marcado como terminado (la fecha viene editada por el usuario).
/// La nota va de 1 a 5; 0 o `None` = sin nota.
pub fn set_finish(p: &mut TitlePrefs, id: &str, finished_ms: Option<i64>, rating: Option<i64>) {
    let Some(e) = p.get_mut(id) else { return };
    if e.status.as_deref() != Some("finished") {
        return;
    }
    if let Some(f) = finished_ms {
        e.finished_ms = Some(f);
    }
    e.rating = rating.filter(|r| (1..=5).contains(r));
}

/// Orden de los pendientes: a cada id se le da la posicion que indica su lugar en la lista (1, 2, 3…).
/// Devuelve las parejas `(id, posicion)` que se han fijado (solo juegos pendientes).
pub fn set_pending_order(p: &mut TitlePrefs, ids: &[String]) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        if let Some(e) = p.get_mut(id).filter(|e| e.status.as_deref() == Some("pending")) {
            e.list_pos = Some(i as i64 + 1);
            out.push((id.clone(), i as i64 + 1));
        }
    }
    out
}

/// «Dejarlo» ante el aviso de juego parado: se recuerda cuando, hasta que se vuelva a jugar.
pub fn set_snooze(p: &mut TitlePrefs, ids: &[String], now_ms: i64) -> Vec<String> {
    let mut out = Vec::new();
    for id in ids {
        if let Some(e) = p.get_mut(id).filter(|e| e.status.is_some()) {
            e.snooze_ms = Some(now_ms);
            out.push(id.clone());
        }
    }
    out
}

/// Fecha de lanzamiento puesta a mano (`None` = quitarla). Se guarda como manual aunque venga marcada como automatica.
pub fn set_release(p: &mut TitlePrefs, id: &str, date: Option<ReleaseDate>) -> Result<(), String> {
    match date {
        Some(d) => {
            d.validate()?;
            p.entry(id.to_string()).or_default().release = Some(ReleaseDate { auto: false, ..d });
        }
        None => {
            if let Some(x) = p.get_mut(id) {
                x.release = None;
            }
            prune(p, id);
        }
    }
    Ok(())
}

/// Año de lanzamiento sacado de SteamGridDB: solo se guarda si el juego no tiene fecha o la que tiene tambien es
/// automatica (nunca pisa una puesta a mano). Devuelve la fecha nueva si ha cambiado algo.
pub fn set_release_auto(p: &mut TitlePrefs, id: &str, year: i32) -> Option<ReleaseDate> {
    if !RELEASE_YEARS.contains(&year) {
        return None;
    }
    let cur = p.get(id).and_then(|x| x.release);
    if cur.is_some_and(|r| !r.auto || r.year == year) {
        return None;
    }
    let d = ReleaseDate { year, month: None, day: None, auto: true };
    p.entry(id.to_string()).or_default().release = Some(d);
    Some(d)
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
        assert_eq!(p["a"].status_ms, Some(30));
        set_accent(&mut p, "a", Some("#112233")).unwrap();
        set_status(&mut p, "a", "none", 40).unwrap();
        assert!(p.contains_key("a"), "conserva el acento");
        set_accent(&mut p, "a", None).unwrap();
        assert!(p.is_empty(), "sin nada que guardar se poda");
    }

    #[test]
    fn list_rules_rating_order_and_snooze() {
        let mut p = TitlePrefs::new();
        for (i, id) in ["a", "b", "c"].iter().enumerate() {
            set_status(&mut p, id, "pending", 10 + i as i64).unwrap();
        }
        assert_eq!((p["a"].list_pos, p["b"].list_pos, p["c"].list_pos), (Some(1), Some(2), Some(3)), "cada pendiente nuevo va al final");
        set_status(&mut p, "a", "pending", 99).unwrap();
        assert_eq!((p["a"].list_pos, p["a"].status_ms), (Some(1), Some(10)), "repetir el estado no cambia ni el orden ni la fecha");
        // Reordenar solo afecta a los pendientes.
        set_status(&mut p, "z", "playing", 5).unwrap();
        let done = set_pending_order(&mut p, &["c".into(), "a".into(), "z".into(), "b".into(), "nada".into()]);
        assert_eq!(done, vec![("c".to_string(), 1), ("a".to_string(), 2), ("b".to_string(), 4)]);
        assert_eq!(p["z"].list_pos, None);
        // Nota y fecha: solo en terminados; 0 = sin nota; fuera de 1..5 se ignora.
        set_finish(&mut p, "b", Some(1), Some(3));
        assert_eq!((p["b"].finished_ms, p["b"].rating), (None, None), "b no esta terminado");
        set_status(&mut p, "b", "finished", 50).unwrap();
        assert_eq!(p["b"].list_pos, None, "al salir de pendientes pierde su sitio");
        set_finish(&mut p, "b", Some(777), Some(4));
        assert_eq!((p["b"].finished_ms, p["b"].rating), (Some(777), Some(4)));
        set_finish(&mut p, "b", None, Some(0));
        assert_eq!((p["b"].finished_ms, p["b"].rating), (Some(777), None));
        set_finish(&mut p, "b", None, Some(4));
        set_status(&mut p, "b", "dropped", 60).unwrap();
        assert_eq!(p["b"].rating, None, "la nota solo vale con terminado");
        // «Dejarlo»: se guarda y se borra al cambiar de estado.
        assert_eq!(set_snooze(&mut p, &["z".into(), "ninguno".into()], 123), vec!["z".to_string()]);
        assert_eq!(p["z"].snooze_ms, Some(123));
        set_status(&mut p, "z", "playing", 200).unwrap();
        assert_eq!(p["z"].snooze_ms, None);
    }

    #[test]
    fn release_date_rules() {
        let mut p = TitlePrefs::new();
        let d = |year, month, day| ReleaseDate { year, month, day, auto: false };
        assert!(set_release(&mut p, "a", Some(d(1800, None, None))).is_err());
        assert!(set_release(&mut p, "a", Some(d(2017, None, Some(3)))).is_err(), "dia sin mes");
        assert!(set_release(&mut p, "a", Some(d(2017, Some(13), None))).is_err());
        assert!(set_release(&mut p, "a", Some(d(2017, Some(2), Some(29)))).is_err());
        set_release(&mut p, "a", Some(d(2016, Some(2), Some(29)))).unwrap();
        assert!(p.contains_key("a"));
        // Automatica: rellena huecos y sustituye otra automatica, pero nunca una puesta a mano.
        assert_eq!(set_release_auto(&mut p, "a", 2020), None, "a mano no se pisa");
        assert_eq!(p["a"].release, Some(d(2016, Some(2), Some(29))));
        let auto = set_release_auto(&mut p, "b", 2019).unwrap();
        assert_eq!((auto.year, auto.auto), (2019, true));
        assert_eq!(set_release_auto(&mut p, "b", 2019), None, "sin cambios");
        assert_eq!(set_release_auto(&mut p, "b", 2021).map(|r| r.year), Some(2021), "otra automatica si se sustituye");
        assert_eq!(set_release_auto(&mut p, "c", 0), None);
        // Ponerla a mano la deja como manual aunque llegue marcada como automatica.
        set_release(&mut p, "b", Some(ReleaseDate { auto: true, ..d(2018, Some(5), None) })).unwrap();
        assert!(!p["b"].release.unwrap().auto);
        // Quitarla poda la entrada si no queda nada.
        set_release(&mut p, "b", None).unwrap();
        assert!(!p.contains_key("b"));
        set_status(&mut p, "a", "playing", 1).unwrap();
        set_release(&mut p, "a", None).unwrap();
        assert_eq!(p["a"].status.as_deref(), Some("playing"), "quitar la fecha no toca el estado");
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
