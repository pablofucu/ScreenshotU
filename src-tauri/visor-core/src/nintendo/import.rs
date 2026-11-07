//! Vista previa e importacion de la lista de juegos de Nintendo (SW-17).
//!
//! Todo es puro salvo `apply`, que escribe en una transaccion. Las demos se excluyen siempre. El minimo
//! de minutos solo limita **crear fichas**; enlazar con una ficha que ya existe se propone siempre. La
//! importacion nunca crea carpetas en disco ni toca otras categorias que la de Switch.

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::link::{self, Candidate};
use super::store::{self, SwitchTitle};
use super::{is_demo, SWITCH_PLATFORM};
use crate::db::data as dbd;
use crate::games::ManualGame;

/// Un juego de la cuenta Nintendo, tal como lo ve la interfaz.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub nx_id: String,
    pub name: String,
    pub minutes: i64,
    pub last_played: Option<i64>,
    pub image_url: Option<String>,
    pub platform: Option<String>,
}

impl From<&SwitchTitle> for Item {
    fn from(t: &SwitchTitle) -> Self {
        Item {
            nx_id: t.nx_id.clone(),
            name: t.name.clone(),
            minutes: t.total_minutes,
            last_played: t.last_played_ms,
            image_url: t.image_url.clone(),
            platform: t.device_type.clone(),
        }
    }
}

/// Una ficha nueva que saldria de uno o varios juegos de Nintendo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewGroup {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkItem {
    pub item: Item,
    pub target: Candidate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DoubtItem {
    pub item: Item,
    pub options: Vec<Candidate>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub nuevos: Vec<NewGroup>,
    pub enlazados: Vec<LinkItem>,
    pub dudosos: Vec<DoubtItem>,
    /// Sin ficha y por debajo del minimo: se pueden crear a mano con `Choice::Create`.
    pub excluidos: Vec<Item>,
    pub ignorados: Vec<Item>,
    pub demos: usize,
    pub min_minutes: i64,
}

/// Lo que el usuario decide aplicar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum Choice {
    #[serde(rename_all = "camelCase")]
    Create { nx_ids: Vec<String>, name: String },
    #[serde(rename_all = "camelCase")]
    Link { nx_id: String, game_id: Option<String>, folder: Option<String> },
    #[serde(rename_all = "camelCase")]
    Ignore { nx_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Created {
    pub game_id: String,
    pub name: String,
    pub nx_ids: Vec<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub created: Vec<Created>,
    pub linked: usize,
    pub ignored: usize,
    pub skipped: Vec<String>,
}

fn items(titles: &[SwitchTitle], ids: &[String]) -> Vec<Item> {
    ids.iter().filter_map(|id| titles.iter().find(|t| &t.nx_id == id)).map(Item::from).collect()
}

/// Reparte los juegos de la cuenta: nuevos (con minimo), enlazados, dudosos, excluidos e ignorados.
pub fn preview(titles: &[SwitchTitle], cands: &[Candidate], min_minutes: i64) -> Preview {
    let mut out = Preview { min_minutes, ..Default::default() };
    let real: Vec<SwitchTitle> = titles.iter().filter(|t| !is_demo(&t.name)).cloned().collect();
    out.demos = titles.iter().filter(|t| is_demo(&t.name) && t.game_id.is_none() && t.folder.is_none()).count();
    out.ignorados = real.iter().filter(|t| t.ignored).map(Item::from).collect();
    let p = link::propose(&real, cands);
    for l in &p.enlazados {
        if let Some(t) = real.iter().find(|t| t.nx_id == l.nx_id) {
            let target = cands
                .iter()
                .find(|c| c.game_id == l.game_id && c.folder == l.folder)
                .cloned()
                .unwrap_or(Candidate { game_id: l.game_id.clone(), folder: l.folder.clone(), name: l.name.clone() });
            out.enlazados.push(LinkItem { item: Item::from(t), target });
        }
    }
    for d in &p.dudosos {
        if let Some(t) = real.iter().find(|t| t.nx_id == d.nx_id) {
            out.dudosos.push(DoubtItem { item: Item::from(t), options: d.options.clone() });
        }
    }
    for it in items(&real, &p.nuevos) {
        if it.minutes >= min_minutes {
            out.nuevos.push(NewGroup { name: it.name.clone(), items: vec![it] });
        } else {
            out.excluidos.push(it);
        }
    }
    out
}

/// Juegos sin ficha, no ignorados, que no son demo y llegan al minimo («N nuevos sin importar»).
pub fn pending_count(titles: &[SwitchTitle], min_minutes: i64) -> usize {
    titles
        .iter()
        .filter(|t| !t.ignored && t.game_id.is_none() && t.folder.is_none() && !is_demo(&t.name) && t.total_minutes >= min_minutes)
        .count()
}

/// Un id de ficha que no existe ni entre los ya generados. `new_id` puede repetir en un bucle rapido.
fn unique_id(conn: &Connection, used: &[String], new_id: &mut impl FnMut() -> String) -> Result<String> {
    loop {
        let id = new_id();
        let in_db: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM games WHERE id = ?1)", [&id], |r| r.get(0))?;
        if !in_db && !used.contains(&id) {
            return Ok(id);
        }
    }
}

/// Aplica las decisiones en una transaccion. Salta (con motivo) demos, juegos inexistentes o ya enlazados y
/// enlaces a destinos que no son candidatos de la categoria Switch.
pub fn apply(conn: &Connection, choices: &[Choice], cands: &[Candidate], mut new_id: impl FnMut() -> String) -> Result<Report> {
    let tx = conn.unchecked_transaction()?;
    let mut rep = Report::default();
    let mut used: Vec<String> = Vec::new();
    for ch in choices {
        match ch {
            Choice::Create { nx_ids, name } => {
                let mut ok: Vec<SwitchTitle> = Vec::new();
                for id in nx_ids {
                    match store::title_get(&tx, id)? {
                        None => rep.skipped.push(format!("{id}: no existe")),
                        Some(t) if is_demo(&t.name) => rep.skipped.push(format!("{}: es una demo", t.name)),
                        Some(t) if t.game_id.is_some() || t.folder.is_some() => rep.skipped.push(format!("{}: ya tiene ficha", t.name)),
                        Some(t) => ok.push(t),
                    }
                }
                let name = name.trim();
                if ok.is_empty() {
                    continue;
                }
                if name.is_empty() {
                    rep.skipped.push(format!("{}: sin nombre", ok[0].name));
                    continue;
                }
                let id = unique_id(&tx, &used, &mut new_id)?;
                used.push(id.clone());
                dbd::manual_add(
                    &tx,
                    &ManualGame {
                        id: id.clone(),
                        name: name.to_string(),
                        kind: "captures".into(),
                        platform: SWITCH_PLATFORM.into(),
                        folder: None,
                        ..Default::default()
                    },
                )?;
                for t in &ok {
                    store::title_set_link(&tx, &t.nx_id, Some(&id), None)?;
                }
                let image_url = ok.iter().find_map(|t| t.image_url.clone());
                rep.created.push(Created { game_id: id, name: name.to_string(), nx_ids: ok.iter().map(|t| t.nx_id.clone()).collect(), image_url });
            }
            Choice::Link { nx_id, game_id, folder } => {
                let Some(t) = store::title_get(&tx, nx_id)? else {
                    rep.skipped.push(format!("{nx_id}: no existe"));
                    continue;
                };
                let valid = match (game_id, folder) {
                    (Some(g), _) => cands.iter().any(|c| c.game_id.as_deref() == Some(g.as_str())),
                    (None, Some(f)) => cands.iter().any(|c| c.game_id.is_none() && &c.folder == f),
                    (None, None) => false,
                };
                if is_demo(&t.name) || !valid {
                    rep.skipped.push(format!("{}: destino no valido", t.name));
                    continue;
                }
                store::title_set_link(&tx, nx_id, game_id.as_deref(), folder.as_deref())?;
                rep.linked += 1;
            }
            Choice::Ignore { nx_id } => {
                if store::title_set_ignored(&tx, nx_id, true)? {
                    rep.ignored += 1;
                } else {
                    rep.skipped.push(format!("{nx_id}: no existe"));
                }
            }
        }
    }
    tx.commit()?;
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_connection;
    use crate::games::{all_games, ManualGame};
    use crate::nintendo::models::parse_play_histories;

    fn setup() -> (tempfile::TempDir, Connection) {
        let d = tempfile::tempdir().unwrap();
        let c = open_connection(&d.path().join("visor.db")).unwrap();
        let ph = parse_play_histories(include_str!("fixtures/play_histories.json")).unwrap();
        store::save_history(&c, &ph.title_upserts(), &ph.daily_rows(), 1).unwrap();
        (d, c)
    }

    fn id_of(titles: &[SwitchTitle], prefix: &str) -> String {
        titles.iter().find(|t| t.name.starts_with(prefix)).unwrap().nx_id.clone()
    }

    #[test]
    fn preview_filters_demos_and_minimum() {
        let (_d, c) = setup();
        let titles = store::titles_load(&c).unwrap();
        let p = preview(&titles, &[], 60);
        assert_eq!(p.demos, 2);
        assert_eq!(p.nuevos.len(), 5);
        let n: Vec<&str> = p.nuevos.iter().map(|g| g.name.as_str()).collect();
        for want in ["Super Smash", "The Legend of Zelda: Ocarina", "Yoshi and the Mysterious Book", "Mina", "Xenoblade"] {
            assert!(n.iter().any(|x| x.starts_with(want)), "{want} en {n:?}");
        }
        let ex: Vec<&str> = p.excluidos.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(ex.len(), 3);
        assert!(ex.iter().any(|n| n.contains("Super Nintendo")) && ex.iter().any(|n| n.contains("Game Boy")) && ex.contains(&"It Takes Two"));
        assert!(!p.nuevos.iter().flat_map(|g| &g.items).any(|i| is_demo(&i.name)));
        assert_eq!(pending_count(&titles, 60), 5);
        assert_eq!(pending_count(&titles, 0), 8);
    }

    #[test]
    fn create_makes_a_captures_card_without_folder() {
        let (d, c) = setup();
        let titles = store::titles_load(&c).unwrap();
        let smash = id_of(&titles, "Super Smash");
        let ch = Choice::Create { nx_ids: vec![smash.clone()], name: "Super Smash Bros. Ultimate".into() };
        let rep = apply(&c, &[ch], &[], || "manual:1".into()).unwrap();
        assert_eq!(rep.created.len(), 1);
        let m = dbd::manual_load(&c).unwrap();
        assert_eq!((m[0].kind.as_str(), m[0].platform.as_str(), m[0].folder.clone()), ("captures", "switch", None));
        assert_eq!(store::title_get(&c, &smash).unwrap().unwrap().game_id.as_deref(), Some("manual:1"));
        assert_eq!(std::fs::read_dir(d.path()).unwrap().filter(|e| e.as_ref().unwrap().path().is_dir()).count(), 0, "no se crean carpetas");
        let after = preview(&store::titles_load(&c).unwrap(), &[], 60);
        assert_eq!(after.nuevos.len(), 4);
    }

    #[test]
    fn create_of_demo_or_bad_link_is_skipped() {
        let (_d, c) = setup();
        let titles = store::titles_load(&c).unwrap();
        let demo = titles.iter().find(|t| is_demo(&t.name)).unwrap().nx_id.clone();
        let smash = id_of(&titles, "Super Smash");
        let choices = [
            Choice::Create { nx_ids: vec![demo], name: "Demo".into() },
            Choice::Link { nx_id: smash.clone(), game_id: Some("manual:x".into()), folder: None },
        ];
        let rep = apply(&c, &choices, &[], || "manual:1".into()).unwrap();
        assert_eq!((rep.created.len(), rep.linked, rep.skipped.len()), (0, 0, 2));
        assert!(dbd::manual_load(&c).unwrap().is_empty());
        assert!(store::title_get(&c, &smash).unwrap().unwrap().game_id.is_none());
    }

    #[test]
    fn repeated_ids_give_distinct_cards() {
        let (_d, c) = setup();
        let titles = store::titles_load(&c).unwrap();
        let a = id_of(&titles, "Super Smash");
        let b = id_of(&titles, "Mina");
        let mut n = 0;
        let choices = [Choice::Create { nx_ids: vec![a], name: "A".into() }, Choice::Create { nx_ids: vec![b], name: "B".into() }];
        let rep = apply(&c, &choices, &[], || {
            n += 1;
            if n <= 2 { "manual:same".into() } else { format!("manual:{n}") }
        })
        .unwrap();
        assert_eq!(rep.created.len(), 2);
        assert_ne!(rep.created[0].game_id, rep.created[1].game_id);
        assert_eq!(dbd::manual_load(&c).unwrap().len(), 2);
    }

    #[test]
    fn ignore_and_link_to_candidate() {
        let (_d, c) = setup();
        let titles = store::titles_load(&c).unwrap();
        let smash = id_of(&titles, "Super Smash");
        let mina = id_of(&titles, "Mina");
        let cands = vec![Candidate { game_id: None, folder: "Mina".into(), name: "Mina".into() }];
        let choices = [Choice::Ignore { nx_id: smash.clone() }, Choice::Link { nx_id: mina, game_id: None, folder: Some("Mina".into()) }];
        let rep = apply(&c, &choices, &cands, || "manual:1".into()).unwrap();
        assert_eq!((rep.ignored, rep.linked), (1, 1));
        let t = store::titles_load(&c).unwrap();
        assert!(t.iter().find(|x| x.nx_id == smash).unwrap().ignored);
        let p = preview(&t, &cands, 60);
        assert!(!p.nuevos.iter().any(|g| g.name.starts_with("Super Smash")));
        assert_eq!(p.ignorados.len(), 1);
    }

    #[test]
    fn candidates_only_from_switch_and_fold_card_with_folder() {
        let mk = |id: &str, n: &str, plat: &str| ManualGame { id: id.into(), name: n.into(), kind: "captures".into(), platform: plat.into(), ..Default::default() };
        let games = all_games(&[mk("manual:1", "Hades", "switch"), mk("manual:2", "Celeste", "pc")]);
        let cs = link::candidates(&games, &["Hades".into(), "Suelta".into()]);
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].game_id.as_deref(), Some("manual:1"));
        assert_eq!(cs[1].folder, "Suelta");
    }
}
