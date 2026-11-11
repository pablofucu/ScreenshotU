//! Tiempo de consola por juego, listo para sumarlo al del PC al escanear (SW-09).
//!
//! Reglas (ver `docs/archivo/plan-switch-playtime.md`, decisiones 6-8):
//! - Total de un titulo = total del historial + minutos de los dias **posteriores** al dia en que
//!   se leyo ese total (el dia de la lectura ya va dentro del total).
//! - Por dia y titulo cuenta el **maximo** entre el historial y la suma de los usuarios marcados del
//!   Control Parental (las dos fuentes de Nintendo se solapan; no se suman).
//! - Varios titulos de Nintendo enlazados a la misma ficha (p. ej. Switch 1 y Switch 2) se suman.
//! - Solo cuentan los titulos enlazados y no ignorados. Se indexa por `game_id` y por carpeta normalizada.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, TimeZone};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::store::{daily_since, players_load, titles_load, SwitchTitle};
use crate::games::norm_key;

/// Dias recientes que se devuelven para el desglose.
pub const RECENT_DAYS: i64 = 28;

/// Tiempo de consola de una ficha.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleTime {
    pub seconds: u64,
    pub first_played: Option<i64>,
    pub last_played: Option<i64>,
    /// `[inicio_del_dia_ms, minutos]` de los ultimos 28 dias con tiempo, del mas antiguo al mas reciente.
    pub days: Vec<(i64, u32)>,
    /// `"history"` o `"parental"`: de donde sale la mayor parte de los dias recientes.
    pub source: String,
}

/// Tiempos de consola indexados por ficha enlazada.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConsoleMap {
    pub by_game: HashMap<String, ConsoleTime>,
    /// Clave: `norm_key` de la carpeta.
    pub by_folder: HashMap<String, ConsoleTime>,
}

impl ConsoleMap {
    pub fn is_empty(&self) -> bool {
        self.by_game.is_empty() && self.by_folder.is_empty()
    }
    pub fn for_game(&self, game_id: &str) -> Option<&ConsoleTime> {
        self.by_game.get(game_id)
    }
    pub fn for_folder(&self, folder: &str) -> Option<&ConsoleTime> {
        self.by_folder.get(&norm_key(folder))
    }
}

/// Medianoche local de un dia en ms (si no existe por el cambio de hora, la hora siguiente).
fn midnight_ms<Tz: TimeZone>(tz: &Tz, d: NaiveDate) -> i64 {
    let t = d.and_hms_opt(0, 0, 0).unwrap();
    tz.from_local_datetime(&t)
        .earliest()
        .or_else(|| tz.from_local_datetime(&(t + Duration::hours(1))).earliest())
        .map(|x| x.timestamp_millis())
        .unwrap_or(0)
}

fn day_of<Tz: TimeZone>(tz: &Tz, ms: i64) -> Option<NaiveDate> {
    DateTime::from_timestamp_millis(ms).map(|d| d.with_timezone(tz).date_naive())
}

impl ConsoleTime {
    /// Minutos de consola de cada uno de los ultimos 7 dias locales (el ultimo es el de `now`).
    pub fn week_minutes<Tz: TimeZone>(&self, now: DateTime<Tz>) -> [u32; 7] {
        let tz = now.timezone();
        let today = now.date_naive();
        let mut out = [0u32; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            let start = midnight_ms(&tz, today - Duration::days(6 - i as i64));
            *slot = self.days.iter().filter(|(d, _)| *d == start).map(|(_, m)| *m).sum();
        }
        out
    }
}

#[derive(Default)]
struct Acc {
    minutes: i64,
    first: Option<i64>,
    last: Option<i64>,
    days: HashMap<i64, u32>,
    parental_days: u32,
    history_days: u32,
}

impl Acc {
    fn add(&mut self, o: &Acc) {
        self.minutes += o.minutes;
        self.first = match (self.first, o.first) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        self.last = match (self.last, o.last) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        for (d, m) in &o.days {
            *self.days.entry(*d).or_default() += *m;
        }
        self.parental_days += o.parental_days;
        self.history_days += o.history_days;
    }

    fn clone_acc(&self) -> Acc {
        Acc {
            minutes: self.minutes,
            first: self.first,
            last: self.last,
            days: self.days.clone(),
            parental_days: self.parental_days,
            history_days: self.history_days,
        }
    }

    fn into_time(self) -> ConsoleTime {
        let mut days: Vec<(i64, u32)> = self.days.into_iter().collect();
        days.sort();
        ConsoleTime {
            seconds: (self.minutes.max(0) as u64) * 60,
            first_played: self.first,
            last_played: self.last,
            days,
            source: if self.parental_days > self.history_days { "parental" } else { "history" }.to_string(),
        }
    }
}

/// Calcula los tiempos de consola de todas las fichas enlazadas. `now` fija el dia de hoy y la zona.
pub fn console_times<Tz: TimeZone>(conn: &Connection, now: DateTime<Tz>) -> Result<ConsoleMap> {
    let tz = now.timezone();
    let today = now.date_naive();
    let window_start = today - Duration::days(RECENT_DAYS - 1);
    let window_start_ms = midnight_ms(&tz, window_start);

    let titles: Vec<SwitchTitle> = titles_load(conn)?
        .into_iter()
        .filter(|t| !t.ignored && (t.game_id.is_some() || t.folder.is_some()))
        .collect();
    if titles.is_empty() {
        return Ok(ConsoleMap::default());
    }

    // Dia de lectura de cada historial: los dias posteriores se suman al total.
    let read_day = |t: &SwitchTitle| t.history_ms.and_then(|ms| day_of(&tz, ms));
    let mut since = window_start;
    for t in &titles {
        if let Some(d) = read_day(t) {
            since = since.min(d + Duration::days(1));
        }
    }

    let uncounted: HashSet<(String, String)> =
        players_load(conn)?.into_iter().filter(|p| !p.counted).map(|p| (p.device_id, p.player_id)).collect();

    // (nx_id, dia) -> (historial, suma de Moon)
    let mut per_day: HashMap<(String, NaiveDate), (i64, i64)> = HashMap::new();
    for r in daily_since(conn, &since.format("%Y-%m-%d").to_string())? {
        let Ok(day) = NaiveDate::parse_from_str(&r.day, "%Y-%m-%d") else { continue };
        let e = per_day.entry((r.nx_id.clone(), day)).or_default();
        if r.source == "parental" {
            if !uncounted.contains(&(r.device_id.clone(), r.player_id.clone())) {
                e.1 += r.minutes;
            }
        } else {
            e.0 = e.0.max(r.minutes);
        }
    }

    let mut by_title: HashMap<&str, Vec<(NaiveDate, i64, bool)>> = HashMap::new();
    for ((nx, day), (h, p)) in &per_day {
        let m = (*h).max(*p);
        if m > 0 {
            by_title.entry(nx.as_str()).or_default().push((*day, m, p > h));
        }
    }

    let mut game_acc: HashMap<String, Acc> = HashMap::new();
    let mut folder_acc: HashMap<String, Acc> = HashMap::new();
    for t in &titles {
        let mut acc = Acc { minutes: t.total_minutes, first: t.first_played_ms, last: t.last_played_ms, ..Default::default() };
        let read = read_day(t);
        if let Some(days) = by_title.get(t.nx_id.as_str()) {
            for &(day, m, from_parental) in days {
                if read.map_or(true, |r| day > r) {
                    acc.minutes += m;
                }
                let start = midnight_ms(&tz, day);
                if start >= window_start_ms && day <= today {
                    *acc.days.entry(start).or_default() += m as u32;
                    if from_parental {
                        acc.parental_days += 1;
                    } else {
                        acc.history_days += 1;
                    }
                }
                acc.last = Some(acc.last.map_or(start, |l| l.max(start)));
                acc.first = Some(acc.first.map_or(start, |f| f.min(start)));
            }
        }
        if acc.minutes <= 0 && acc.days.is_empty() {
            continue;
        }
        if let Some(g) = &t.game_id {
            game_acc.entry(g.clone()).or_default().add(&acc);
        }
        if let Some(f) = &t.folder {
            folder_acc.entry(norm_key(f)).or_default().add(&acc.clone_acc());
        }
    }
    Ok(ConsoleMap {
        by_game: game_acc.into_iter().map(|(k, a)| (k, a.into_time())).collect(),
        by_folder: folder_acc.into_iter().map(|(k, a)| (k, a.into_time())).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_connection;
    use crate::nintendo::store::*;
    use chrono::{FixedOffset, Utc};

    fn conn() -> (tempfile::TempDir, Connection) {
        let d = tempfile::tempdir().unwrap();
        let c = open_connection(&d.path().join("visor.db")).unwrap();
        (d, c)
    }

    fn utc(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    fn title(c: &Connection, id: &str, total: i64, read: DateTime<Utc>, game: Option<&str>, folder: Option<&str>) {
        upsert_title(c, &TitleUpsert { nx_id: id.into(), name: id.into(), total_minutes: total, ..Default::default() }, read.timestamp_millis()).unwrap();
        title_set_link(c, id, game, folder).unwrap();
    }

    fn day(c: &Connection, d: &str, id: &str, source: &str, player: &str, min: i64) {
        let dev = if source == "parental" { "dev" } else { "" };
        daily_upsert(c, &DailyRow { day: d.into(), nx_id: id.into(), source: source.into(), device_id: dev.into(), player_id: player.into(), minutes: min }, 1).unwrap();
    }

    #[test]
    fn total_is_history_plus_later_days_only() {
        let (_d, c) = conn();
        title(&c, "A", 100, utc(2026, 11, 5, 10), Some("g1"), None);
        day(&c, "2026-11-04", "A", "history", "", 30); // anterior a la lectura: ya esta en el total
        day(&c, "2026-11-05", "A", "history", "", 20); // dia de la lectura: ya esta en el total
        day(&c, "2026-11-06", "A", "history", "", 15);
        day(&c, "2026-11-07", "A", "history", "", 5);
        let m = console_times(&c, utc(2026, 11, 7, 12)).unwrap();
        let t = m.for_game("g1").unwrap();
        assert_eq!(t.seconds, (100 + 15 + 5) * 60);
        assert_eq!(t.days.len(), 4, "el desglose lleva todos los dias recientes");
    }

    #[test]
    fn per_day_takes_max_between_sources_not_the_sum() {
        let (_d, c) = conn();
        title(&c, "A", 0, utc(2026, 11, 1, 0), Some("g1"), None);
        day(&c, "2026-11-06", "A", "history", "", 40);
        day(&c, "2026-11-06", "A", "parental", "p1", 25);
        day(&c, "2026-11-06", "A", "parental", "p2", 30); // Moon suma 55 > 40
        day(&c, "2026-11-07", "A", "history", "", 10);
        day(&c, "2026-11-07", "A", "parental", "p1", 4);
        let t = console_times(&c, utc(2026, 11, 7, 12)).unwrap().by_game["g1"].clone();
        assert_eq!(t.seconds, (55 + 10) * 60);
    }

    #[test]
    fn unmarked_player_does_not_count() {
        let (_d, c) = conn();
        title(&c, "A", 0, utc(2026, 11, 1, 0), Some("g1"), None);
        player_upsert(&c, "dev", "p1", "Pablo", "Switch").unwrap();
        player_upsert(&c, "dev", "p2", "Hermano", "Switch").unwrap();
        player_set_counted(&c, "dev", "p2", false).unwrap();
        day(&c, "2026-11-06", "A", "parental", "p1", 25);
        day(&c, "2026-11-06", "A", "parental", "p2", 90);
        let t = console_times(&c, utc(2026, 11, 7, 12)).unwrap().by_game["g1"].clone();
        assert_eq!(t.seconds, 25 * 60);
        assert_eq!(t.source, "parental");
    }

    #[test]
    fn two_titles_of_one_card_are_summed_and_folder_is_normalized() {
        let (_d, c) = conn();
        title(&c, "A", 60, utc(2026, 11, 7, 1), None, Some("Zelda: BotW"));
        title(&c, "B", 30, utc(2026, 11, 7, 1), None, Some("zelda botw"));
        let m = console_times(&c, utc(2026, 11, 7, 12)).unwrap();
        assert_eq!(m.for_folder("zelda botw").map(|t| t.seconds), Some(90 * 60));
        assert!(m.by_game.is_empty());
    }

    #[test]
    fn unlinked_and_ignored_are_left_out() {
        let (_d, c) = conn();
        title(&c, "A", 60, utc(2026, 11, 7, 1), None, None);
        title(&c, "B", 60, utc(2026, 11, 7, 1), Some("g2"), None);
        title_set_ignored(&c, "B", true).unwrap();
        assert!(console_times(&c, utc(2026, 11, 7, 12)).unwrap().is_empty());
    }

    #[test]
    fn old_days_stay_out_of_the_window_but_count_in_the_total() {
        let (_d, c) = conn();
        // Historial leido hace 40 dias: el hueco sigue sumando aunque este fuera de los 28 dias.
        title(&c, "A", 10, utc(2026, 9, 28, 10), Some("g1"), None);
        day(&c, "2026-10-01", "A", "history", "", 7);
        day(&c, "2026-11-07", "A", "history", "", 3);
        let t = console_times(&c, utc(2026, 11, 7, 12)).unwrap().by_game["g1"].clone();
        assert_eq!(t.seconds, (10 + 7 + 3) * 60);
        assert_eq!(t.days.len(), 1);
    }

    #[test]
    fn week_uses_local_calendar_days() {
        let (_d, c) = conn();
        title(&c, "A", 0, utc(2026, 3, 1, 0), Some("g1"), None);
        day(&c, "2026-03-28", "A", "history", "", 11);
        day(&c, "2026-03-29", "A", "history", "", 22);
        day(&c, "2026-03-30", "A", "history", "", 33);
        for hours in [1, 2] {
            let now = FixedOffset::east_opt(hours * 3600).unwrap().with_ymd_and_hms(2026, 3, 30, 12, 0, 0).unwrap();
            let t = console_times(&c, now).unwrap().by_game["g1"].clone();
            assert_eq!(t.week_minutes(now), [0, 0, 0, 0, 11, 22, 33]);
        }
    }
}
