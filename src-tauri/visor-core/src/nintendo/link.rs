//! Emparejado automatico de los titulos de Nintendo con las fichas de la categoria Switch (SW-10).
//!
//! Solo propone: no escribe nada ni cambia ids de fichas. Quien llama decide que aplicar con
//! `store::title_set_link`. Los candidatos son unicamente fichas/carpetas de la categoria Switch.

use serde::{Deserialize, Serialize};

use super::store::SwitchTitle;
use super::{base_key, SWITCH_PLATFORM};
use crate::games::{norm_key, Game};

/// Ficha de la categoria Switch con la que se puede enlazar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub game_id: Option<String>,
    pub folder: String,
    pub name: String,
}

/// Enlace propuesto para un titulo de Nintendo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Linked {
    pub nx_id: String,
    pub name: String,
    pub game_id: Option<String>,
    pub folder: String,
}

/// Titulo con varias fichas posibles: lo resuelve el usuario.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Doubtful {
    pub nx_id: String,
    pub name: String,
    pub options: Vec<Candidate>,
}

/// Resultado del emparejado: `nuevos` no tienen ficha (candidatos a importar).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub nuevos: Vec<String>,
    pub enlazados: Vec<Linked>,
    pub dudosos: Vec<Doubtful>,
}

/// Candidatos de la categoria Switch: sus fichas (`platform == switch`) y las carpetas sueltas de
/// `Raiz/<carpeta Switch>` que no son la de ninguna ficha. Nunca mira otras plataformas.
pub fn candidates(games: &[Game], folders: &[String]) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = games
        .iter()
        .filter(|g| g.platform == SWITCH_PLATFORM)
        .map(|g| Candidate { game_id: Some(g.id.clone()), folder: g.folder.clone(), name: g.name.clone() })
        .collect();
    let taken: Vec<String> = out.iter().map(|c| norm_key(&c.folder)).collect();
    for f in folders {
        let k = norm_key(f);
        if !k.is_empty() && !taken.contains(&k) && !out.iter().any(|c| c.game_id.is_none() && norm_key(&c.folder) == k) {
            out.push(Candidate { game_id: None, folder: f.clone(), name: f.clone() });
        }
    }
    out
}

/// Empareja los titulos sin enlazar y no ignorados por nombre normalizado (contra el nombre de la
/// ficha y contra su carpeta). Una sola ficha => enlazado; varias distintas => dudoso; ninguna => nuevo.
pub fn propose(titles: &[SwitchTitle], candidates: &[Candidate]) -> Proposal {
    let keyed: Vec<(String, String, &Candidate)> =
        candidates.iter().map(|c| (norm_key(&c.name), norm_key(&c.folder), c)).collect();
    let mut out = Proposal::default();
    for t in titles.iter().filter(|t| !t.ignored && t.game_id.is_none() && t.folder.is_none()) {
        let k = norm_key(&t.name);
        let mut hits: Vec<&Candidate> = Vec::new();
        if !k.is_empty() {
            for (n, f, c) in &keyed {
                if (*n == k || *f == k) && !hits.contains(&c) {
                    hits.push(c);
                }
            }
        }
        // Sin coincidencia exacta: se mira el nombre base (p. ej. «… Switch 2 Edition»); lo confirma el usuario.
        let mut by_base = false;
        if hits.is_empty() {
            let bk = base_key(&t.name);
            if !bk.is_empty() {
                for c in candidates {
                    if (base_key(&c.name) == bk || base_key(&c.folder) == bk) && !hits.contains(&c) {
                        hits.push(c);
                    }
                }
                by_base = !hits.is_empty();
            }
        }
        if hits.len() > 1 || by_base {
            hits.sort_by(|a, b| b.game_id.is_some().cmp(&a.game_id.is_some()).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        }
        match hits.as_slice() {
            _ if by_base => out.dudosos.push(Doubtful {
                nx_id: t.nx_id.clone(),
                name: t.name.clone(),
                options: hits.iter().map(|c| (*c).clone()).collect(),
            }),
            [] => out.nuevos.push(t.nx_id.clone()),
            [c] => out.enlazados.push(Linked {
                nx_id: t.nx_id.clone(),
                name: t.name.clone(),
                game_id: c.game_id.clone(),
                folder: c.folder.clone(),
            }),
            many => out.dudosos.push(Doubtful {
                nx_id: t.nx_id.clone(),
                name: t.name.clone(),
                options: many.iter().map(|c| (*c).clone()).collect(),
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: &str, name: &str) -> SwitchTitle {
        SwitchTitle {
            nx_id: id.into(),
            name: name.into(),
            image_url: None,
            device_type: None,
            first_played_ms: None,
            last_played_ms: None,
            total_minutes: 10,
            history_ms: None,
            game_id: None,
            folder: None,
            ignored: false,
            seen_ms: 0,
        }
    }
    fn c(id: Option<&str>, folder: &str, name: &str) -> Candidate {
        Candidate { game_id: id.map(Into::into), folder: folder.into(), name: name.into() }
    }

    #[test]
    fn matches_by_normalized_name_or_folder() {
        let p = propose(
            &[t("A", "Super Mario Odyssey™"), t("B", "Zelda: Tears of the Kingdom")],
            &[c(Some("g1"), "SuperMarioOdyssey", "Super Mario Odyssey"), c(None, "zelda tears of the kingdom", "TotK")],
        );
        assert_eq!(p.enlazados.len(), 2);
        assert_eq!(p.enlazados[0].game_id.as_deref(), Some("g1"));
        assert_eq!(p.enlazados[1].folder, "zelda tears of the kingdom");
        assert!(p.nuevos.is_empty() && p.dudosos.is_empty());
    }

    #[test]
    fn duplicates_are_doubtful_and_unknown_are_new() {
        let p = propose(
            &[t("A", "Hades"), t("B", "Celeste")],
            &[c(Some("g1"), "Hades", "Hades"), c(Some("g2"), "Hades (2)", "Hades")],
        );
        assert_eq!(p.dudosos.len(), 1);
        assert_eq!(p.dudosos[0].options.len(), 2);
        assert_eq!(p.nuevos, vec!["B".to_string()]);
    }

    #[test]
    fn same_card_matching_name_and_folder_is_not_doubtful() {
        let p = propose(&[t("A", "Hades")], &[c(Some("g1"), "Hades", "Hades")]);
        assert_eq!(p.enlazados.len(), 1);
    }

    #[test]
    fn already_linked_and_ignored_are_skipped() {
        let mut linked = t("A", "Hades");
        linked.game_id = Some("g1".into());
        let mut ign = t("B", "Celeste");
        ign.ignored = true;
        let p = propose(&[linked, ign], &[c(Some("g1"), "Hades", "Hades")]);
        assert_eq!(p, Proposal::default());
    }
}
