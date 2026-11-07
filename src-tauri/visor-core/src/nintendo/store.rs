//! Lectura y escritura de las tablas `switch_*` (migracion 15). Todo idempotente: volver a
//! sincronizar el mismo dia no duplica nada.

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};

/// Un juego de la cuenta Nintendo, tal como esta guardado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchTitle {
    pub nx_id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub device_type: Option<String>,
    pub first_played_ms: Option<i64>,
    pub last_played_ms: Option<i64>,
    pub total_minutes: i64,
    pub history_ms: Option<i64>,
    pub game_id: Option<String>,
    pub folder: Option<String>,
    pub ignored: bool,
    pub seen_ms: i64,
}

/// Lo que trae el historial para un juego (sin el enlace ni las marcas del usuario).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TitleUpsert {
    pub nx_id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub device_type: Option<String>,
    pub first_played_ms: Option<i64>,
    pub last_played_ms: Option<i64>,
    pub total_minutes: i64,
}

/// Minutos de un juego en un dia, de una fuente (`history` o `parental`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyRow {
    pub day: String,
    pub nx_id: String,
    pub source: String,
    pub device_id: String,
    pub player_id: String,
    pub minutes: i64,
}

/// Usuario de una consola vinculada al Control Parental.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchPlayer {
    pub device_id: String,
    pub player_id: String,
    pub nickname: String,
    pub device_name: String,
    pub counted: bool,
}

const TITLE_COLS: &str = "nx_id, name, image_url, device_type, first_played_ms, last_played_ms, total_minutes, history_ms, game_id, folder, ignored, seen_ms";

fn title_from_row(r: &Row) -> rusqlite::Result<SwitchTitle> {
    Ok(SwitchTitle {
        nx_id: r.get(0)?,
        name: r.get(1)?,
        image_url: r.get(2)?,
        device_type: r.get(3)?,
        first_played_ms: r.get(4)?,
        last_played_ms: r.get(5)?,
        total_minutes: r.get(6)?,
        history_ms: r.get(7)?,
        game_id: r.get(8)?,
        folder: r.get(9)?,
        ignored: r.get::<_, i64>(10)? != 0,
        seen_ms: r.get(11)?,
    })
}

/// Inserta o actualiza un juego del historial. `now_ms` es el momento de la lectura: se guarda
/// como `history_ms` y, la primera vez, como `seen_ms`. No toca el enlace ni `ignored`.
pub fn upsert_title(conn: &Connection, t: &TitleUpsert, now_ms: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO switch_titles (nx_id, name, image_url, device_type, first_played_ms, last_played_ms, total_minutes, history_ms, seen_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(nx_id) DO UPDATE SET
            name = excluded.name,
            image_url = COALESCE(excluded.image_url, image_url),
            device_type = COALESCE(excluded.device_type, device_type),
            first_played_ms = COALESCE(excluded.first_played_ms, first_played_ms),
            last_played_ms = COALESCE(excluded.last_played_ms, last_played_ms),
            total_minutes = excluded.total_minutes,
            history_ms = excluded.history_ms",
        params![t.nx_id, t.name, t.image_url, t.device_type, t.first_played_ms, t.last_played_ms, t.total_minutes, now_ms],
    )?;
    Ok(())
}

pub fn title_get(conn: &Connection, nx_id: &str) -> Result<Option<SwitchTitle>> {
    Ok(conn
        .query_row(&format!("SELECT {TITLE_COLS} FROM switch_titles WHERE nx_id = ?1"), [nx_id], title_from_row)
        .optional()?)
}

/// Todos los juegos, por nombre.
pub fn titles_load(conn: &Connection) -> Result<Vec<SwitchTitle>> {
    let mut st = conn.prepare(&format!("SELECT {TITLE_COLS} FROM switch_titles ORDER BY name COLLATE NOCASE, nx_id"))?;
    let rows = st.query_map([], title_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Juegos sin ficha ni carpeta y no descartados: los candidatos a importar o enlazar.
pub fn titles_unlinked(conn: &Connection) -> Result<Vec<SwitchTitle>> {
    let mut st = conn.prepare(&format!(
        "SELECT {TITLE_COLS} FROM switch_titles WHERE game_id IS NULL AND folder IS NULL AND ignored = 0 ORDER BY name COLLATE NOCASE, nx_id"
    ))?;
    let rows = st.query_map([], title_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Enlaza un juego con una ficha (`game_id`) o con una carpeta suelta; el otro campo se vacia.
/// `(None, None)` quita el enlace. Devuelve si el juego existia.
pub fn title_set_link(conn: &Connection, nx_id: &str, game_id: Option<&str>, folder: Option<&str>) -> Result<bool> {
    let (game_id, folder) = if game_id.is_some() { (game_id, None) } else { (None, folder) };
    let n = conn.execute("UPDATE switch_titles SET game_id = ?2, folder = ?3 WHERE nx_id = ?1", params![nx_id, game_id, folder])?;
    Ok(n > 0)
}

pub fn title_set_ignored(conn: &Connection, nx_id: &str, ignored: bool) -> Result<bool> {
    let n = conn.execute("UPDATE switch_titles SET ignored = ?2 WHERE nx_id = ?1", params![nx_id, ignored as i64])?;
    Ok(n > 0)
}

/// Guarda los minutos de un dia. La clave completa es unica: repetir la llamada sustituye el valor.
pub fn daily_upsert(conn: &Connection, row: &DailyRow, now_ms: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO switch_daily (day, nx_id, source, device_id, player_id, minutes, updated_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(day, nx_id, source, device_id, player_id) DO UPDATE SET
            minutes = excluded.minutes, updated_ms = excluded.updated_ms",
        params![row.day, row.nx_id, row.source, row.device_id, row.player_id, row.minutes, now_ms],
    )?;
    Ok(())
}

/// Dias guardados desde `since_day` (inclusive, `YYYY-MM-DD`), por dia y juego.
pub fn daily_since(conn: &Connection, since_day: &str) -> Result<Vec<DailyRow>> {
    let mut st = conn.prepare(
        "SELECT day, nx_id, source, device_id, player_id, minutes FROM switch_daily WHERE day >= ?1
         ORDER BY day, nx_id, source, device_id, player_id",
    )?;
    let rows = st
        .query_map([since_day], |r| {
            Ok(DailyRow { day: r.get(0)?, nx_id: r.get(1)?, source: r.get(2)?, device_id: r.get(3)?, player_id: r.get(4)?, minutes: r.get(5)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Resultado de guardar una descarga del historial.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HistorySaved {
    pub titles: usize,
    pub days: usize,
    /// Juegos que no estaban guardados.
    pub new: Vec<String>,
    /// Juegos cuyo total, ultima vez o minutos de algun dia han cambiado (incluye los nuevos con tiempo en dias).
    pub changed: Vec<String>,
}

/// Guarda el historial en una transaccion. Solo escribe `switch_*`: nunca crea fichas ni toca `games`.
pub fn save_history(conn: &Connection, titles: &[TitleUpsert], days: &[DailyRow], now_ms: i64) -> Result<HistorySaved> {
    let tx = conn.unchecked_transaction()?;
    let mut out = HistorySaved { titles: titles.len(), days: days.len(), ..Default::default() };
    let mark = |list: &mut Vec<String>, id: &str| {
        if !list.iter().any(|x| x == id) {
            list.push(id.to_string());
        }
    };
    for t in titles {
        match title_get(&tx, &t.nx_id)? {
            None => {
                mark(&mut out.new, &t.nx_id);
                mark(&mut out.changed, &t.nx_id);
            }
            Some(old) => {
                if old.total_minutes != t.total_minutes || (t.last_played_ms.is_some() && old.last_played_ms != t.last_played_ms) {
                    mark(&mut out.changed, &t.nx_id);
                }
            }
        }
        // Siempre se escribe: asi `history_ms` refleja cuando se leyo el total.
        upsert_title(&tx, t, now_ms)?;
    }
    for d in days {
        let old: Option<i64> = tx
            .query_row(
                "SELECT minutes FROM switch_daily WHERE day = ?1 AND nx_id = ?2 AND source = ?3 AND device_id = ?4 AND player_id = ?5",
                params![d.day, d.nx_id, d.source, d.device_id, d.player_id],
                |r| r.get(0),
            )
            .optional()?;
        if old != Some(d.minutes) {
            daily_upsert(&tx, d, now_ms)?;
            mark(&mut out.changed, &d.nx_id);
        }
    }
    tx.commit()?;
    Ok(out)
}

/// Guarda un usuario del Control Parental. Si ya existia se actualizan los nombres y se
/// **respeta** `counted` (lo decide el usuario).
pub fn player_upsert(conn: &Connection, device_id: &str, player_id: &str, nickname: &str, device_name: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO switch_players (device_id, player_id, nickname, device_name) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(device_id, player_id) DO UPDATE SET nickname = excluded.nickname, device_name = excluded.device_name",
        params![device_id, player_id, nickname, device_name],
    )?;
    Ok(())
}

pub fn player_set_counted(conn: &Connection, device_id: &str, player_id: &str, counted: bool) -> Result<bool> {
    let n = conn.execute(
        "UPDATE switch_players SET counted = ?3 WHERE device_id = ?1 AND player_id = ?2",
        params![device_id, player_id, counted as i64],
    )?;
    Ok(n > 0)
}

pub fn players_load(conn: &Connection) -> Result<Vec<SwitchPlayer>> {
    let mut st = conn.prepare("SELECT device_id, player_id, nickname, device_name, counted FROM switch_players ORDER BY device_id, player_id")?;
    let rows = st
        .query_map([], |r| {
            Ok(SwitchPlayer { device_id: r.get(0)?, player_id: r.get(1)?, nickname: r.get(2)?, device_name: r.get(3)?, counted: r.get::<_, i64>(4)? != 0 })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_connection;

    fn conn() -> (tempfile::TempDir, Connection) {
        let d = tempfile::tempdir().unwrap();
        let c = open_connection(&d.path().join("visor.db")).unwrap();
        (d, c)
    }

    fn up(id: &str, name: &str, total: i64) -> TitleUpsert {
        TitleUpsert { nx_id: id.into(), name: name.into(), total_minutes: total, ..Default::default() }
    }

    #[test]
    fn title_upsert_updates_but_keeps_link_and_flags() {
        let (_d, c) = conn();
        upsert_title(&c, &TitleUpsert { image_url: Some("http://i".into()), ..up("A", "Zelda", 60) }, 1000).unwrap();
        assert!(title_set_link(&c, "A", Some("manual:3"), None).unwrap());
        assert!(title_set_ignored(&c, "A", true).unwrap());
        // Segunda lectura: sin imagen nueva, mas minutos.
        upsert_title(&c, &up("A", "Zelda BotW", 90), 5000).unwrap();
        let t = title_get(&c, "A").unwrap().unwrap();
        assert_eq!((t.name.as_str(), t.total_minutes, t.history_ms, t.seen_ms), ("Zelda BotW", 90, Some(5000), 1000));
        assert_eq!(t.image_url.as_deref(), Some("http://i"), "no se pierde la imagen");
        assert_eq!(t.game_id.as_deref(), Some("manual:3"));
        assert!(t.ignored);
        assert_eq!(title_get(&c, "no").unwrap(), None);
    }

    #[test]
    fn unlinked_excludes_linked_and_ignored() {
        let (_d, c) = conn();
        for (id, n) in [("A", "Alpha"), ("B", "Beta"), ("C", "Gamma"), ("D", "Delta")] {
            upsert_title(&c, &up(id, n, 1), 1).unwrap();
        }
        title_set_link(&c, "A", Some("manual:1"), None).unwrap();
        title_set_link(&c, "B", None, Some("carpeta")).unwrap();
        title_set_ignored(&c, "C", true).unwrap();
        let ids: Vec<_> = titles_unlinked(&c).unwrap().into_iter().map(|t| t.nx_id).collect();
        assert_eq!(ids, vec!["D"]);
        // Quitar el enlace lo devuelve a la lista.
        title_set_link(&c, "A", None, None).unwrap();
        assert_eq!(titles_unlinked(&c).unwrap().len(), 2);
        // Un enlace con ficha y carpeta se queda solo con la ficha.
        title_set_link(&c, "B", Some("manual:2"), Some("x")).unwrap();
        let b = title_get(&c, "B").unwrap().unwrap();
        assert_eq!((b.game_id.as_deref(), b.folder.as_deref()), (Some("manual:2"), None));
        assert_eq!(titles_load(&c).unwrap().len(), 4);
    }

    #[test]
    fn daily_is_idempotent_and_filtered_by_day() {
        let (_d, c) = conn();
        let row = |day: &str, m: i64| DailyRow { day: day.into(), nx_id: "A".into(), source: "history".into(), device_id: String::new(), player_id: String::new(), minutes: m };
        daily_upsert(&c, &row("2026-11-01", 10), 1).unwrap();
        daily_upsert(&c, &row("2026-11-01", 25), 2).unwrap(); // sustituye
        daily_upsert(&c, &row("2026-11-03", 5), 2).unwrap();
        let all = daily_since(&c, "2026-11-01").unwrap();
        assert_eq!(all.iter().map(|r| (r.day.as_str(), r.minutes)).collect::<Vec<_>>(), vec![("2026-11-01", 25), ("2026-11-03", 5)]);
        assert_eq!(daily_since(&c, "2026-11-02").unwrap().len(), 1);
    }

    #[test]
    fn save_history_is_idempotent_and_reports_changes() {
        use crate::nintendo::models::parse_play_histories;
        let (_d, c) = conn();
        let ph = parse_play_histories(include_str!("fixtures/play_histories.json")).unwrap();
        let (titles, days) = (ph.title_upserts(), ph.daily_rows());
        assert!(!titles.is_empty());
        let games_before: i64 = c.query_row("SELECT count(*) FROM games", [], |r| r.get(0)).unwrap();

        let first = save_history(&c, &titles, &days, 1000).unwrap();
        assert_eq!(first.new.len(), titles.len());
        assert_eq!((first.titles, first.days), (titles.len(), days.len()));
        let rows = |c: &Connection| -> (i64, i64) {
            (
                c.query_row("SELECT count(*) FROM switch_titles", [], |r| r.get(0)).unwrap(),
                c.query_row("SELECT count(*) FROM switch_daily", [], |r| r.get(0)).unwrap(),
            )
        };
        let counts = rows(&c);
        assert_eq!(counts, (titles.len() as i64, days.len() as i64));

        // Repetir con los mismos datos: nada nuevo ni cambiado, mismas filas.
        let again = save_history(&c, &titles, &days, 2000).unwrap();
        assert!(again.new.is_empty() && again.changed.is_empty(), "{again:?}");
        assert_eq!(rows(&c), counts);

        // Un enlace y un descarte sobreviven a otra descarga y subir un total solo marca ese juego.
        let id = titles[0].nx_id.clone();
        title_set_link(&c, &id, Some("manual:9"), None).unwrap();
        title_set_ignored(&c, &id, true).unwrap();
        let mut more = titles.clone();
        more[0].total_minutes += 15;
        let third = save_history(&c, &more, &days, 3000).unwrap();
        assert_eq!(third.changed, vec![id.clone()]);
        assert!(third.new.is_empty());
        let t = title_get(&c, &id).unwrap().unwrap();
        assert_eq!((t.game_id.as_deref(), t.ignored, t.history_ms), (Some("manual:9"), true, Some(3000)));

        // Un dia con minutos distintos tambien marca el juego.
        let mut days2 = days.clone();
        if let Some(d) = days2.first_mut() {
            d.minutes += 5;
            let nx = d.nx_id.clone();
            assert_eq!(save_history(&c, &more, &days2, 4000).unwrap().changed, vec![nx]);
        }
        // Nunca crea fichas.
        assert_eq!(c.query_row::<i64, _, _>("SELECT count(*) FROM games", [], |r| r.get(0)).unwrap(), games_before);
    }

    #[test]
    fn player_upsert_keeps_counted_choice() {
        let (_d, c) = conn();
        player_upsert(&c, "d1", "p1", "Pablo", "Switch").unwrap();
        assert!(players_load(&c).unwrap()[0].counted, "por defecto cuenta");
        assert!(player_set_counted(&c, "d1", "p1", false).unwrap());
        player_upsert(&c, "d1", "p1", "Pablo V", "Switch 2").unwrap();
        let p = &players_load(&c).unwrap()[0];
        assert_eq!((p.nickname.as_str(), p.device_name.as_str(), p.counted), ("Pablo V", "Switch 2", false));
        assert!(!player_set_counted(&c, "d1", "otro", true).unwrap());
    }
}
