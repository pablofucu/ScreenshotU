//! Juegos instalados en los PCs cliente (fase 2 del multi-PC) y partidas que esos clientes juegan en local.
//!
//! En el host, `installs` apunta qué dispositivo tiene instalado cada titulo (solo metadatos: la ruta es del
//! cliente). El id del titulo es siempre el del host (`Title.id`): el cliente nunca inventa ids, pide al host que
//! busque (`match_title`) o que cree el titulo en una categoria que ya existe.
//!
//! Las partidas locales del cliente suben al host con su `uid` (idempotente): en el host se guardan tal cual, sin
//! fundirlas con otras (el cliente ya las fundio), y si llega otra vez el mismo `uid` con mas tiempo se amplia.
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::data::InputCounts;

/// Migracion 17: instalaciones de los clientes (solo se usa en el host).
pub const SCHEMA: &str = r#"
CREATE TABLE installs (
    game_id     TEXT NOT NULL,
    device_id   TEXT NOT NULL,
    exe_name    TEXT NOT NULL DEFAULT '',
    folder_name TEXT NOT NULL DEFAULT '',
    size_bytes  INTEGER NOT NULL DEFAULT 0,
    added_ms    INTEGER NOT NULL,
    seen_ms     INTEGER NOT NULL,
    PRIMARY KEY (game_id, device_id)
);
CREATE INDEX installs_device ON installs(device_id);
"#;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Install {
    pub game_id: String,
    pub device_id: String,
    pub exe_name: String,
    pub folder_name: String,
    pub size_bytes: i64,
    pub added_ms: i64,
    pub seen_ms: i64,
}

/// Apunta (o refresca) la instalacion de `device_id`. `added_ms` se queda con la primera vez.
pub fn upsert(c: &Connection, i: &Install, now_ms: i64) -> Result<()> {
    c.execute(
        "INSERT INTO installs(game_id, device_id, exe_name, folder_name, size_bytes, added_ms, seen_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
         ON CONFLICT(game_id, device_id) DO UPDATE SET exe_name = excluded.exe_name,
             folder_name = excluded.folder_name, size_bytes = excluded.size_bytes, seen_ms = excluded.seen_ms",
        params![i.game_id, i.device_id, i.exe_name, i.folder_name, i.size_bytes, now_ms],
    )?;
    Ok(())
}

/// Quita la instalacion. Devuelve si habia algo.
pub fn remove(c: &Connection, game_id: &str, device_id: &str) -> Result<bool> {
    Ok(c.execute("DELETE FROM installs WHERE game_id = ?1 AND device_id = ?2", params![game_id, device_id])? > 0)
}

/// Si `device_id` tiene instalado `game_id`.
pub fn has(c: &Connection, game_id: &str, device_id: &str) -> Result<bool> {
    Ok(c.query_row("SELECT 1 FROM installs WHERE game_id = ?1 AND device_id = ?2", params![game_id, device_id], |_| Ok(()))
        .optional()?
        .is_some())
}

/// Ids de los dispositivos que tienen instalado cada titulo (sin los dispositivos quitados).
pub fn by_game(c: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let mut st = c.prepare(
        "SELECT i.game_id, i.device_id FROM installs i JOIN devices d ON d.id = i.device_id
         WHERE d.revoked = 0 ORDER BY i.added_ms",
    )?;
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for row in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (g, d) = row?;
        out.entry(g).or_default().push(d);
    }
    Ok(out)
}

/* ------------------------------------------------------------------ */
/* Buscar el titulo del host que corresponde a un juego del cliente    */
/* ------------------------------------------------------------------ */

/// Un titulo del host de la categoria pedida, con lo que sirve para reconocerlo.
#[derive(Debug, Clone, Default)]
pub struct MatchItem {
    pub id: String,
    pub name: String,
    pub folder: String,
    /// Nombre del ejecutable del lanzador del host, si tiene.
    pub exe_name: String,
    pub steam_app_id: Option<String>,
}

/// Lo que el cliente sabe del juego que añade.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MatchQuery {
    pub name: String,
    pub exe_name: String,
    pub folder_name: String,
    pub steam_app_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: String,
    pub name: String,
    /// "appid" | "exe" | "folder" | "name".
    pub reason: &'static str,
    /// 0-100: 100 seguro, menos = parecido.
    pub score: u32,
}

/// Minusculas y solo letras y numeros (`Hollow Knight: Silksong` -> `hollowknightsilksong`).
pub fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// `C:\x\Juego.exe` -> `juego`.
fn exe_stem(s: &str) -> String {
    let file = s.rsplit(['\\', '/']).next().unwrap_or(s);
    norm(file.rsplit_once('.').map(|(a, _)| a).unwrap_or(file))
}

fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric()).filter(|w| w.chars().count() >= 2).map(|w| w.to_lowercase()).collect()
}

/// Parecido de nombres (0-100): igual normalizado 90; uno contiene al otro 70; si no, palabras en comun.
fn name_score(a: &str, b: &str) -> u32 {
    let (na, nb) = (norm(a), norm(b));
    if na.is_empty() || nb.is_empty() {
        return 0;
    }
    if na == nb {
        return 90;
    }
    if na.len().min(nb.len()) >= 4 && (na.contains(&nb) || nb.contains(&na)) {
        return 70;
    }
    let (wa, wb) = (words(a), words(b));
    if wa.is_empty() || wb.is_empty() {
        return 0;
    }
    let common = wa.iter().filter(|w| wb.contains(w)).count();
    let total = wa.len().max(wb.len());
    (common * 60 / total) as u32
}

/// Candidatos de la categoria (ya filtrada) para `q`, del mas seguro al menos: appid exacto, luego nombre de
/// ejecutable o carpeta, luego parecido de nombre (al menos 40). Como mucho 5.
pub fn match_title(items: &[MatchItem], q: &MatchQuery) -> Vec<Candidate> {
    let exe = exe_stem(&q.exe_name);
    let folder = norm(&q.folder_name);
    let appid = q.steam_app_id.as_deref().map(str::trim).filter(|a| !a.is_empty());
    let mut out: Vec<Candidate> = items
        .iter()
        .filter_map(|it| {
            let (reason, score) = if appid.is_some() && it.steam_app_id.as_deref() == appid {
                ("appid", 100)
            } else if !exe.is_empty() && exe_stem(&it.exe_name) == exe {
                ("exe", 95)
            } else if !folder.is_empty() && (norm(&it.folder) == folder || norm(&it.name) == folder) {
                ("folder", 92)
            } else {
                let s = name_score(&q.name, &it.name).max(name_score(&q.folder_name, &it.name));
                if s < 40 {
                    return None;
                }
                ("name", s)
            };
            Some(Candidate { id: it.id.clone(), name: it.name.clone(), reason, score })
        })
        .collect();
    out.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
    out.truncate(5);
    out
}

/* ------------------------------------------------------------------ */
/* Partidas jugadas en local en un cliente                              */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Input {
    pub clicks: i64,
    pub wheel: i64,
    pub mouse_dist: i64,
    pub keys: i64,
    pub pad: i64,
    pub peak_cpm: i64,
    pub peak_kpm: i64,
    pub peak_ppm: i64,
}

impl From<&Input> for InputCounts {
    fn from(i: &Input) -> Self {
        InputCounts {
            clicks: i.clicks,
            wheel: i.wheel,
            mouse_dist: i.mouse_dist,
            keys: i.keys,
            pad: i.pad,
            peak_cpm: i.peak_cpm,
            peak_kpm: i.peak_kpm,
            peak_ppm: i.peak_ppm,
        }
    }
}

/// Una partida tal como viaja del cliente al host.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteSession {
    pub uid: String,
    pub game_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub active_ms: i64,
    pub input: Option<Input>,
}

/// Partidas propias (`device_id = ''`) aun sin enviar al host, de las mas antiguas a las mas nuevas.
pub fn unsynced(c: &Connection, limit: usize) -> Result<Vec<RemoteSession>> {
    let mut st = c.prepare(
        "SELECT s.uid, s.game_id, s.start_ms, s.end_ms, COALESCE(s.active_ms, 0),
                i.clicks, i.wheel, i.mouse_dist, i.keys, i.pad, i.peak_cpm, i.peak_kpm, i.peak_ppm
         FROM sessions s LEFT JOIN session_input i ON i.session_id = s.id
         WHERE s.device_id = '' AND s.uid IS NOT NULL AND s.synced_ms IS NULL
         ORDER BY s.start_ms LIMIT ?1",
    )?;
    let rows = st.query_map([limit as i64], |r| {
        let clicks: Option<i64> = r.get(5)?;
        let input = match clicks {
            None => None,
            Some(clicks) => Some(Input {
                clicks,
                wheel: r.get(6)?,
                mouse_dist: r.get(7)?,
                keys: r.get(8)?,
                pad: r.get(9)?,
                peak_cpm: r.get(10)?,
                peak_kpm: r.get(11)?,
                peak_ppm: r.get(12)?,
            }),
        };
        Ok(RemoteSession { uid: r.get(0)?, game_id: r.get(1)?, start_ms: r.get(2)?, end_ms: r.get(3)?, active_ms: r.get(4)?, input })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Cuantas partidas propias quedan por enviar.
pub fn unsynced_count(c: &Connection) -> Result<i64> {
    Ok(c.query_row("SELECT count(*) FROM sessions WHERE device_id = '' AND uid IS NOT NULL AND synced_ms IS NULL", [], |r| r.get(0))?)
}

/// Marca como enviadas las partidas con esos `uid`.
pub fn mark_synced(c: &Connection, uids: &[String], now_ms: i64) -> Result<()> {
    let tx = c.unchecked_transaction()?;
    {
        let mut st = tx.prepare("UPDATE sessions SET synced_ms = ?1 WHERE uid = ?2")?;
        for u in uids {
            st.execute(params![now_ms, u])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Guarda en el host una partida subida por `device_id`. Nueva -> se inserta tal cual (sin fundir); ya conocida
/// (mismo `uid` y dispositivo) -> se amplia si trae mas tiempo. Un `uid` de otro dispositivo se ignora.
/// Devuelve si el `uid` queda guardado (para que el cliente lo marque como enviado).
pub fn put_remote_session(c: &Connection, device_id: &str, s: &RemoteSession) -> Result<bool> {
    if s.uid.trim().is_empty() || s.game_id.trim().is_empty() || device_id.is_empty() {
        return Ok(false);
    }
    let end = s.end_ms.max(s.start_ms);
    let active = s.active_ms.clamp(0, end - s.start_ms);
    let input: InputCounts = s.input.as_ref().map(Into::into).unwrap_or_default();
    let tx = c.unchecked_transaction()?;
    let known: Option<(i64, String, i64)> = tx
        .query_row("SELECT id, device_id, end_ms FROM sessions WHERE uid = ?1", [&s.uid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .optional()?;
    let extra_secs = match &known {
        Some((_, dev, _)) if dev != device_id => return Ok(false),
        Some((_, _, old_end)) => (end - old_end).max(0) / 1000,
        None => (end - s.start_ms) / 1000,
    };
    tx.execute(
        "INSERT INTO play_stats(game_id, last_played_ms, seconds) VALUES (?1, ?2, ?3)
         ON CONFLICT(game_id) DO UPDATE SET seconds = seconds + excluded.seconds,
             last_played_ms = MAX(last_played_ms, excluded.last_played_ms)",
        params![s.game_id, end, extra_secs],
    )?;
    let sid = match known {
        Some((sid, _, _)) => {
            tx.execute(
                "UPDATE sessions SET end_ms = MAX(end_ms, ?1), active_ms = MAX(COALESCE(active_ms, 0), ?2) WHERE id = ?3",
                params![end, active, sid],
            )?;
            sid
        }
        None => {
            tx.execute(
                "INSERT INTO sessions(game_id, start_ms, end_ms, active_ms, device_id, uid, synced_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?3)",
                params![s.game_id, s.start_ms, end, active, device_id, s.uid],
            )?;
            tx.last_insert_rowid()
        }
    };
    if !input.is_empty() {
        // Los contadores del cliente ya son el total de la partida: se sustituyen, no se suman.
        tx.execute(
            "INSERT OR REPLACE INTO session_input(session_id, clicks, wheel, mouse_dist, keys, pad, peak_cpm, peak_kpm, peak_ppm)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![sid, input.clicks, input.wheel, input.mouse_dist, input.keys, input.pad, input.peak_cpm, input.peak_kpm, input.peak_ppm],
        )?;
    }
    tx.commit()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::data::{stat_add_session, stats_load};
    use crate::db::Db;

    fn item(id: &str, name: &str, folder: &str, exe: &str, appid: Option<&str>) -> MatchItem {
        MatchItem { id: id.into(), name: name.into(), folder: folder.into(), exe_name: exe.into(), steam_app_id: appid.map(Into::into) }
    }

    #[test]
    fn match_prefers_appid_then_exe_folder_then_name() {
        let items = vec![
            item("steam:367520", "Hollow Knight", "Hollow Knight", "", Some("367520")),
            item("manual:1", "Hollow Knight: Silksong", "Silksong", r"D:\HK\Silksong.exe", None),
            item("pc/Celeste", "Celeste", "Celeste", "", None),
            item("pc/Otro", "Algo distinto", "Otro", "", None),
        ];
        let q = |name: &str, exe: &str, folder: &str, appid: Option<&str>| MatchQuery {
            name: name.into(),
            exe_name: exe.into(),
            folder_name: folder.into(),
            steam_app_id: appid.map(Into::into),
        };
        let r = match_title(&items, &q("HK", "", "", Some("367520")));
        assert_eq!((r[0].id.as_str(), r[0].reason), ("steam:367520", "appid"));
        let r = match_title(&items, &q("lo que sea", "silksong.EXE", "", None));
        assert_eq!((r[0].id.as_str(), r[0].reason), ("manual:1", "exe"));
        let r = match_title(&items, &q("", "", "celeste", None));
        assert_eq!((r[0].id.as_str(), r[0].reason), ("pc/Celeste", "folder"));
        let r = match_title(&items, &q("Hollow Knight Silksong", "", "", None));
        assert_eq!(r[0].id, "manual:1");
        assert!(r.iter().any(|c| c.id == "steam:367520"), "el parecido tambien sale");
        assert!(match_title(&items, &q("Zelda", "", "", None)).is_empty());
    }

    #[test]
    fn installs_by_game_skip_revoked_devices() {
        let db = Db::open_memory().unwrap();
        db.with(|c| {
            let (a, _) = crate::devices::create(c, "Portátil", "pc", "1", 2).unwrap();
            let (b, _) = crate::devices::create(c, "Viejo", "pc", "1", 2).unwrap();
            let i = |g: &str, d: &str| Install { game_id: g.into(), device_id: d.into(), exe_name: "x.exe".into(), ..Default::default() };
            upsert(c, &i("pc/A", &a), 10).unwrap();
            upsert(c, &i("pc/A", &a), 20).unwrap();
            upsert(c, &i("pc/A", &b), 10).unwrap();
            crate::devices::revoke(c, &b).unwrap();
            assert_eq!(by_game(c).unwrap()["pc/A"], vec![a.clone()]);
            assert!(has(c, "pc/A", &a).unwrap());
            let (added, seen): (i64, i64) = c.query_row("SELECT added_ms, seen_ms FROM installs WHERE device_id = ?1", [&a], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
            assert_eq!((added, seen), (10, 20));
            assert!(remove(c, "pc/A", &a).unwrap());
            assert!(!has(c, "pc/A", &a).unwrap());
        });
    }

    #[test]
    fn local_sessions_upload_once_and_grow() {
        // Cliente: partida local, se lee como pendiente y se marca enviada.
        let client = Db::open_memory().unwrap();
        let host = Db::open_memory().unwrap();
        client.with(|c| stat_add_session(c, "pc/A", 1_000, 61_000, 30_000, &InputCounts { clicks: 5, ..Default::default() })).unwrap();
        let pend = client.with(|c| unsynced(c, 100)).unwrap();
        assert_eq!(pend.len(), 1);
        assert_eq!(pend[0].input.as_ref().unwrap().clicks, 5);
        let dev = host.with(|c| crate::devices::create(c, "Portátil", "pc", "1", 2)).unwrap().0;
        assert!(host.with(|c| put_remote_session(c, &dev, &pend[0])).unwrap());
        // Repetir la subida no cuenta dos veces.
        assert!(host.with(|c| put_remote_session(c, &dev, &pend[0])).unwrap());
        assert_eq!(host.with(stats_load).unwrap()["pc/A"].seconds, 60);
        // Otro dispositivo no puede pisar ese uid.
        assert!(!host.with(|c| put_remote_session(c, "otro", &pend[0])).unwrap());
        client.with(|c| mark_synced(c, &[pend[0].uid.clone()], 99)).unwrap();
        assert_eq!(client.with(unsynced_count).unwrap(), 0);
        // La partida se amplia en el cliente (fusion): vuelve a quedar pendiente y el host suma solo lo nuevo.
        client.with(|c| stat_add_session(c, "pc/A", 70_000, 130_000, 0, &InputCounts::default())).unwrap();
        let pend2 = client.with(|c| unsynced(c, 100)).unwrap();
        assert_eq!((pend2.len(), pend2[0].uid.as_str(), pend2[0].end_ms), (1, pend[0].uid.as_str(), 130_000));
        assert!(host.with(|c| put_remote_session(c, &dev, &pend2[0])).unwrap());
        assert_eq!(host.with(stats_load).unwrap()["pc/A"].seconds, 129);
        let (n, d): (i64, String) = host.with(|c| c.query_row("SELECT count(*), max(device_id) FROM sessions", [], |r| Ok((r.get(0)?, r.get(1)?)))).unwrap();
        assert_eq!((n, d), (1, dev));
    }
}
