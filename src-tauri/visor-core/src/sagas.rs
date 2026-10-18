//! Sagas: agrupaciones virtuales de juegos. Solo guardan nombre y ids de juego (tablas `sagas` y
//! `saga_members` de la base); nunca crean, copian ni mueven archivos de capturas.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Saga {
    pub id: String,
    pub name: String,
    /// Ids de titulo (`Title.id`) en el orden de la saga.
    #[serde(default)]
    pub members: Vec<String>,
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
        out.push(Saga { id: s.id.clone(), name, members });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saga(id: &str, name: &str, m: &[&str]) -> Saga {
        Saga { id: id.into(), name: name.into(), members: m.iter().map(|s| s.to_string()).collect() }
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
            saga("d", "Zelda", &["pc/zelda"]),
        ];
        let saved = sanitize(&list);
        db.with(|c| crate::db::data::sagas_replace(c, &saved)).unwrap();
        assert_eq!(saved, vec![saga("a", "Mass Effect", &["steam:1", "steam:2"]), saga("d", "Zelda", &["pc/zelda"])]);
        assert_eq!(db.with(crate::db::data::sagas_load).unwrap(), saved);
        // Reemplazar la lista entera (orden nuevo, una saga menos) deja solo lo ultimo.
        let nueva = vec![saga("d", "Zelda", &["pc/zelda", "pc/botw"]), saga("a", "Mass Effect", &["steam:2"])];
        db.with(|c| crate::db::data::sagas_replace(c, &nueva)).unwrap();
        assert_eq!(db.with(crate::db::data::sagas_load).unwrap(), nueva);
        db.with(|c| crate::db::data::sagas_replace(c, &[])).unwrap();
        assert!(db.with(crate::db::data::sagas_load).unwrap().is_empty());
        let n: i64 = db.with(|c| c.query_row("SELECT count(*) FROM saga_members", [], |r| r.get(0))).unwrap();
        assert_eq!(n, 0, "los miembros caen con su saga");
    }
}
