//! Colecciones (antes «sagas»): agrupaciones virtuales de juegos dentro de una categoria. Solo guardan
//! nombre, categoria, color y ids de juego (tablas `sagas` y `saga_members` de la base); nunca crean,
//! copian ni mueven archivos de capturas.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cuantos colores manuales hay (los de `RAINBOW` de la interfaz).
pub const COLORS: u8 = 7;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Saga {
    pub id: String,
    pub name: String,
    /// Ids de titulo (`Title.id`) en el orden de la coleccion.
    #[serde(default)]
    pub members: Vec<String>,
    /// Categoria (id de plataforma) a la que pertenece. `None` = saga antigua aun sin convertir.
    #[serde(default)]
    pub platform: Option<String>,
    /// Color manual (0..7, indice de `RAINBOW`). `None` = automatico, sacado de las portadas.
    #[serde(default)]
    pub color: Option<u8>,
}

/// Descarta sagas sin nombre o sin miembros, ids de saga repetidos y miembros repetidos o vacios.
pub fn sanitize(list: &[Saga]) -> Vec<Saga> {
    let mut ids = std::collections::HashSet::new();
    let mut out = Vec::new();
    for s in list {
        let name = s.name.trim().to_string();
        let mut seen = std::collections::HashSet::new();
        let members: Vec<String> = s.members.iter().filter(|m| !m.is_empty() && seen.insert((*m).clone())).cloned().collect();
        if name.is_empty() || members.is_empty() || s.id.is_empty() || !ids.insert(s.id.clone()) {
            continue;
        }
        let platform = s.platform.as_ref().map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
        let color = s.color.filter(|c| *c < COLORS);
        out.push(Saga { id: s.id.clone(), name, members, platform, color });
    }
    out
}

/// Convierte las sagas antiguas (sin categoria) en colecciones de una categoria. `platform_of` dice la
/// categoria de cada juego de la biblioteca cargada. Cada saga pasa a la categoria donde tiene mas juegos
/// (a igualdad, la primera que aparece); si mezcla varias se divide en una coleccion por categoria con el
/// mismo nombre (las nuevas van justo detras). Los juegos que no estan en la biblioteca se quedan en la
/// coleccion principal (no se pierden datos). Una saga sin ningun juego conocido se deja como esta, para
/// intentarlo cuando la biblioteca este completa. Devuelve la lista nueva y si ha cambiado algo.
pub fn convert_legacy(list: &[Saga], platform_of: &HashMap<String, String>) -> (Vec<Saga>, bool) {
    let mut used: std::collections::HashSet<String> = list.iter().map(|s| s.id.clone()).collect();
    let mut out = Vec::with_capacity(list.len());
    let mut changed = false;
    for s in list {
        if s.platform.is_some() {
            out.push(s.clone());
            continue;
        }
        // Categorias presentes, en orden de aparicion, con su numero de juegos.
        let mut cats: Vec<(String, usize)> = Vec::new();
        for m in &s.members {
            if let Some(p) = platform_of.get(m) {
                match cats.iter_mut().find(|c| &c.0 == p) {
                    Some(c) => c.1 += 1,
                    None => cats.push((p.clone(), 1)),
                }
            }
        }
        if cats.is_empty() {
            out.push(s.clone());
            continue;
        }
        changed = true;
        // Orden estable: mas juegos primero; a igualdad, la que aparecio antes.
        cats.sort_by(|a, b| b.1.cmp(&a.1));
        let main = cats[0].0.clone();
        out.push(Saga {
            members: s.members.iter().filter(|m| platform_of.get(*m).map_or(true, |p| *p == main)).cloned().collect(),
            platform: Some(main),
            ..s.clone()
        });
        for (n, (plat, _)) in cats.iter().enumerate().skip(1) {
            let mut id = format!("{}-{}", s.id, n + 1);
            while !used.insert(id.clone()) {
                id.push('x');
            }
            let members = s.members.iter().filter(|m| platform_of.get(*m) == Some(plat)).cloned().collect();
            out.push(Saga { id, name: s.name.clone(), members, platform: Some(plat.clone()), color: s.color });
        }
    }
    (out, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saga(id: &str, name: &str, m: &[&str]) -> Saga {
        Saga { id: id.into(), name: name.into(), members: m.iter().map(|s| s.to_string()).collect(), platform: None, color: None }
    }

    fn col(id: &str, name: &str, plat: &str, m: &[&str]) -> Saga {
        Saga { platform: Some(plat.into()), ..saga(id, name, m) }
    }

    fn plats(l: &[(&str, &str)]) -> HashMap<String, String> {
        l.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn legacy_saga_goes_to_its_main_platform() {
        let map = plats(&[("a", "pc"), ("b", "pc"), ("c", "switch")]);
        let (out, changed) = convert_legacy(&[saga("s", "Saga", &["a", "b", "c"])], &map);
        assert!(changed);
        // Mezcla: una coleccion por categoria, la de mas juegos conserva el id.
        assert_eq!(out, vec![col("s", "Saga", "pc", &["a", "b"]), col("s-2", "Saga", "switch", &["c"])]);
    }

    #[test]
    fn legacy_split_keeps_unknown_games_and_is_idempotent() {
        let map = plats(&[("a", "pc"), ("b", "ps"), ("c", "ps")]);
        // «x» no esta en la biblioteca: se queda en la principal (ps) sin perderse.
        let (out, _) = convert_legacy(&[saga("s", "S", &["a", "x", "b", "c"]), saga("t", "Sin juegos", &["y"]), col("u", "Ya", "pc", &["a"])], &map);
        assert_eq!(out[0], col("s", "S", "ps", &["x", "b", "c"]));
        assert_eq!(out[1], col("s-2", "S", "pc", &["a"]));
        assert_eq!(out[2], saga("t", "Sin juegos", &["y"]), "sin ningun juego conocido se deja para mas tarde");
        assert_eq!(out[3], col("u", "Ya", "pc", &["a"]));
        let (again, changed) = convert_legacy(&out, &map);
        assert!(!changed);
        assert_eq!(again, out);
        // Empate: gana la primera categoria que aparece; el id de la division no choca con uno existente.
        let map = plats(&[("a", "pc"), ("b", "ps")]);
        let (out, _) = convert_legacy(&[saga("s", "S", &["a", "b"]), saga("s-2", "Otra", &["a"])], &map);
        assert_eq!(out[0], col("s", "S", "pc", &["a"]));
        assert_eq!(out[1].id, "s-2x");
    }

    #[test]
    fn sanitize_keeps_platform_and_valid_color() {
        let mut a = col("a", "A", " pc ", &["x"]);
        a.color = Some(3);
        let mut b = col("b", "B", "", &["x"]);
        b.color = Some(9);
        let out = sanitize(&[a, b]);
        assert_eq!((out[0].platform.as_deref(), out[0].color), (Some("pc"), Some(3)));
        assert_eq!((out[1].platform.as_deref(), out[1].color), (None, None));
    }

    #[test]
    fn roundtrip_and_sanitize() {
        let db = crate::db::Db::open_memory().unwrap();
        assert!(db.with(crate::db::data::sagas_load).unwrap().is_empty());
        let list = vec![
            saga("a", " Mass Effect ", &["steam:1", "steam:2", "steam:1", ""]),
            saga("a", "Repetida", &["x"]),
            saga("b", "  ", &["x"]),
            saga("c", "Vacia", &[]),
            col("d", "Zelda", "pc", &["pc/zelda"]),
        ];
        let saved = sanitize(&list);
        db.with(|c| crate::db::data::sagas_replace(c, &saved)).unwrap();
        assert_eq!(saved, vec![saga("a", "Mass Effect", &["steam:1", "steam:2"]), col("d", "Zelda", "pc", &["pc/zelda"])]);
        assert_eq!(db.with(crate::db::data::sagas_load).unwrap(), saved);
        // Reemplazar la lista entera (orden nuevo, una saga menos) deja solo lo ultimo.
        let mut nueva = vec![col("d", "Zelda", "pc", &["pc/zelda", "pc/botw"]), saga("a", "Mass Effect", &["steam:2"])];
        nueva[0].color = Some(4);
        db.with(|c| crate::db::data::sagas_replace(c, &nueva)).unwrap();
        assert_eq!(db.with(crate::db::data::sagas_load).unwrap(), nueva);
        db.with(|c| crate::db::data::sagas_replace(c, &[])).unwrap();
        assert!(db.with(crate::db::data::sagas_load).unwrap().is_empty());
        let n: i64 = db.with(|c| c.query_row("SELECT count(*) FROM saga_members", [], |r| r.get(0))).unwrap();
        assert_eq!(n, 0, "los miembros caen con su saga");
    }
}
