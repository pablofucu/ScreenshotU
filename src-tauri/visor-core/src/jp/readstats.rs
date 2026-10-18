//! Estadisticas de lectura de las novelas: conteo de caracteres japoneses, tiempo activo con pausas,
//! agregados (por dia, juego y semana) y racha. La logica es pura; el acceso a SQLite esta al final
//! (`SCHEMA` + funciones que reciben una `Connection`).
//!
//! Los dias son "dias locales desde 1970-01-01" (`i64`); el desplazamiento horario lo pone quien llama.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// Hueco maximo entre dos lineas para que cuente como tiempo activo (5 minutos).
pub const PAUSE_MS: i64 = 5 * 60 * 1000;
const DAY_MS: i64 = 86_400_000;

/// Esquema (idempotente). Se anade a `MIGRATIONS` de `db.rs` como una migracion nueva.
pub const SCHEMA: &str = r#"
-- Sesiones de lectura: de que se lanza a que se cierra un juego de novela.
CREATE TABLE IF NOT EXISTS read_sessions (
    id        INTEGER PRIMARY KEY,
    game_id   TEXT NOT NULL,
    game_name TEXT NOT NULL DEFAULT '',
    start_ms  INTEGER NOT NULL,
    end_ms    INTEGER NOT NULL,
    lines     INTEGER NOT NULL DEFAULT 0,
    chars     INTEGER NOT NULL DEFAULT 0,
    active_ms INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS read_sessions_game ON read_sessions(game_id, start_ms);

-- Totales por juego y dia local (dias desde 1970-01-01): para graficos, caracteres/dia y racha.
CREATE TABLE IF NOT EXISTS read_days (
    game_id   TEXT NOT NULL,
    day       INTEGER NOT NULL,
    lines     INTEGER NOT NULL DEFAULT 0,
    chars     INTEGER NOT NULL DEFAULT 0,
    active_ms INTEGER NOT NULL DEFAULT 0,
    phrases   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (game_id, day)
);

-- Cuando paso una palabra a conocida (las que ya lo eran antes de existir esta tabla no tienen fecha).
CREATE TABLE IF NOT EXISTS known_word_dates (
    word  TEXT PRIMARY KEY,
    at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS known_word_dates_at ON known_word_dates(at_ms);
"#;

/* ------------------------------------------------------------------ */
/* Texto y fechas                                                       */
/* ------------------------------------------------------------------ */

fn is_jp_char(c: char) -> bool {
    matches!(c as u32,
        0x3041..=0x3096 | 0x309D..=0x309F   // hiragana
        | 0x30A1..=0x30FA | 0x30FC..=0x30FF // katakana (sin el punto medio U+30FB)
        | 0x31F0..=0x31FF                   // katakana fonetico
        | 0xFF66..=0xFF9F                   // katakana de media anchura
        | 0x3005 | 0x3006                   // 々 〆
        | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2FA1F) // kanji
}

/// Caracteres japoneses (kana y kanji) de un texto; signos, espacios, latinos y numeros no cuentan.
pub fn count_jp_chars(text: &str) -> u32 {
    text.chars().filter(|&c| is_jp_char(c)).count() as u32
}

/// Dia local (dias desde 1970-01-01) de un instante, con el desplazamiento horario en minutos.
pub fn day_of(ms: i64, offset_min: i32) -> i64 {
    (ms + offset_min as i64 * 60_000).div_euclid(DAY_MS)
}

/// Desplazamiento horario local (minutos) en un instante.
pub fn local_offset_min(ms: i64) -> i32 {
    use chrono::{Local, Offset, TimeZone};
    Local.timestamp_millis_opt(ms).single().map(|d| d.offset().fix().local_minus_utc() / 60).unwrap_or(0)
}

/// Dia de la semana (0 = lunes) de un dia desde 1970 (el 1970-01-01 fue jueves).
fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// Primer dia (lunes) de la semana que contiene `day`.
pub fn week_start(day: i64) -> i64 {
    day - weekday(day)
}

/* ------------------------------------------------------------------ */
/* Sesion en curso                                                      */
/* ------------------------------------------------------------------ */

/// Lo que aporta una linea nueva.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineDelta {
    /// `false` si la linea ya se habia leido en esta sesion (no suma nada).
    pub counted: bool,
    pub chars: u32,
    /// Tiempo activo anadido (hueco desde la linea anterior si no pasa de la pausa).
    pub active_ms: i64,
}

/// Instantanea para mostrar en la pagina de lectura.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Live {
    pub start_ms: i64,
    pub lines: u32,
    pub chars: u32,
    pub active_ms: i64,
    /// Caracteres por hora de tiempo activo (0 si aun no hay tiempo).
    pub cph: u32,
    /// Instante de la ultima linea contada (0 si ninguna) y pausa maxima: la pagina los usa para seguir el reloj.
    pub last_ms: i64,
    pub pause_ms: i64,
}

/// Seguimiento de una sesion de lectura.
#[derive(Debug, Clone)]
pub struct Tracker {
    pub start_ms: i64,
    pub pause_ms: i64,
    seen: HashSet<String>,
    last_ms: Option<i64>,
    pub lines: u32,
    pub chars: u32,
    pub active_ms: i64,
}

impl Tracker {
    pub fn new(start_ms: i64) -> Tracker {
        Tracker { start_ms, pause_ms: PAUSE_MS, seen: HashSet::new(), last_ms: None, lines: 0, chars: 0, active_ms: 0 }
    }

    /// Registra una linea leida en `at_ms`. Las lineas repetidas y las que no tienen japones no suman
    /// caracteres ni lineas, pero tampoco rompen el tiempo activo (si el lector vuelve atras sigue leyendo).
    pub fn push(&mut self, text: &str, at_ms: i64) -> LineDelta {
        let key = text.trim();
        let chars = count_jp_chars(key);
        if chars == 0 {
            return LineDelta::default();
        }
        if !self.seen.insert(key.to_string()) {
            return LineDelta::default();
        }
        let gap = self.last_ms.map(|l| at_ms - l).unwrap_or(0);
        let active = if gap > 0 && gap <= self.pause_ms { gap } else { 0 };
        self.last_ms = Some(at_ms);
        self.lines += 1;
        self.chars += chars;
        self.active_ms += active;
        LineDelta { counted: true, chars, active_ms: active }
    }

    /// Instantanea; `now_ms` sirve para sumar el tramo en curso si la ultima linea es reciente.
    pub fn live(&self, now_ms: i64) -> Live {
        let tail = match self.last_ms {
            Some(l) if now_ms > l && now_ms - l <= self.pause_ms => now_ms - l,
            _ => 0,
        };
        let active = self.active_ms + tail;
        Live {
            start_ms: self.start_ms,
            lines: self.lines,
            chars: self.chars,
            active_ms: active,
            cph: cph(self.chars as i64, active),
            last_ms: self.last_ms.unwrap_or(0),
            pause_ms: self.pause_ms,
        }
    }
}

/// Caracteres por hora (redondeado); 0 si no hay tiempo suficiente (< 30 s) para que la cifra tenga sentido.
pub fn cph(chars: i64, active_ms: i64) -> u32 {
    if active_ms < 30_000 || chars <= 0 {
        return 0;
    }
    (chars as f64 * 3_600_000.0 / active_ms as f64).round() as u32
}

/* ------------------------------------------------------------------ */
/* Agregados                                                            */
/* ------------------------------------------------------------------ */

/// Fila de `read_days`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DayRow {
    pub game_id: String,
    pub day: i64,
    pub lines: u32,
    pub chars: u32,
    pub active_ms: i64,
    pub phrases: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub lines: u32,
    pub chars: u32,
    pub active_ms: i64,
    pub phrases: u32,
    pub cph: u32,
    pub days: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameTotals {
    pub game_id: String,
    pub game_name: String,
    #[serde(flatten)]
    pub totals: Totals,
    pub last_day: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayPoint {
    pub day: i64,
    pub chars: u32,
    pub active_ms: i64,
    pub new_known: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekPoint {
    /// Lunes de la semana (dias desde 1970).
    pub start: i64,
    pub new_known: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub today: i64,
    pub total: Totals,
    pub games: Vec<GameTotals>,
    /// Ultimos 30 dias (el ultimo es hoy), uno por dia aunque no haya lectura.
    pub days: Vec<DayPoint>,
    /// Ultimas 8 semanas (la ultima es la actual).
    pub weeks: Vec<WeekPoint>,
    pub streak: u32,
    pub best_streak: u32,
    /// Media de caracteres por dia de lectura en los ultimos 30 dias.
    pub chars_per_day: u32,
    pub known_total: u32,
    pub known_dated: u32,
}

fn add(t: &mut Totals, r: &DayRow) {
    t.lines += r.lines;
    t.chars += r.chars;
    t.active_ms += r.active_ms;
    t.phrases += r.phrases;
}

fn finish(t: &mut Totals) {
    t.cph = cph(t.chars as i64, t.active_ms);
}

/// Racha de dias seguidos leyendo (con caracteres): `(actual, mejor)`. La actual sigue viva si el
/// ultimo dia leido es hoy o ayer.
pub fn streaks(days: &BTreeSet<i64>, today: i64) -> (u32, u32) {
    let mut best = 0u32;
    let mut run = 0u32;
    let mut prev: Option<i64> = None;
    for &d in days {
        run = if prev == Some(d - 1) { run + 1 } else { 1 };
        best = best.max(run);
        prev = Some(d);
    }
    let cur = match prev {
        Some(last) if last == today || last == today - 1 => run,
        _ => 0,
    };
    (cur, best)
}

/// Calcula todo. `names`: nombre de cada juego; `known_ms`: instantes en que palabras pasaron a conocidas.
pub fn compute(rows: &[DayRow], names: &BTreeMap<String, String>, known_ms: &[i64], known_total: u32, today: i64, offset_min: i32) -> Stats {
    let mut st = Stats { today, known_total, known_dated: known_ms.len() as u32, ..Default::default() };
    let mut per_game: BTreeMap<&str, GameTotals> = BTreeMap::new();
    let mut per_day: BTreeMap<i64, (u32, i64)> = BTreeMap::new();
    let mut read_days: BTreeSet<i64> = BTreeSet::new();
    for r in rows {
        add(&mut st.total, r);
        let g = per_game.entry(&r.game_id).or_insert_with(|| GameTotals {
            game_id: r.game_id.clone(),
            game_name: names.get(&r.game_id).cloned().unwrap_or_else(|| r.game_id.clone()),
            totals: Totals::default(),
            last_day: i64::MIN,
        });
        add(&mut g.totals, r);
        if r.chars > 0 {
            g.totals.days += 1;
            g.last_day = g.last_day.max(r.day);
            read_days.insert(r.day);
            let e = per_day.entry(r.day).or_default();
            e.0 += r.chars;
            e.1 += r.active_ms;
        }
    }
    st.total.days = read_days.len() as u32;
    finish(&mut st.total);
    let mut games: Vec<GameTotals> = per_game.into_values().collect();
    for g in &mut games {
        finish(&mut g.totals);
    }
    games.sort_by(|a, b| b.last_day.cmp(&a.last_day).then(b.totals.chars.cmp(&a.totals.chars)));
    st.games = games;

    let mut known_by_day: BTreeMap<i64, u32> = BTreeMap::new();
    let mut known_by_week: BTreeMap<i64, u32> = BTreeMap::new();
    for &ms in known_ms {
        let d = day_of(ms, offset_min);
        *known_by_day.entry(d).or_default() += 1;
        *known_by_week.entry(week_start(d)).or_default() += 1;
    }
    st.days = (today - 29..=today)
        .map(|d| {
            let (chars, active_ms) = per_day.get(&d).copied().unwrap_or((0, 0));
            DayPoint { day: d, chars, active_ms, new_known: known_by_day.get(&d).copied().unwrap_or(0) }
        })
        .collect();
    let this_week = week_start(today);
    st.weeks = (0..8).rev().map(|i| this_week - 7 * i).map(|s| WeekPoint { start: s, new_known: known_by_week.get(&s).copied().unwrap_or(0) }).collect();
    let (cur, best) = streaks(&read_days, today);
    st.streak = cur;
    st.best_streak = best;
    let (sum, n) = st.days.iter().filter(|d| d.chars > 0).fold((0u64, 0u64), |(s, n), d| (s + d.chars as u64, n + 1));
    st.chars_per_day = if n > 0 { (sum / n) as u32 } else { 0 };
    st
}

/* ------------------------------------------------------------------ */
/* SQLite                                                               */
/* ------------------------------------------------------------------ */

/// Crea la fila de la sesion y devuelve su id.
pub fn session_start(c: &Connection, game_id: &str, game_name: &str, start_ms: i64) -> rusqlite::Result<i64> {
    c.execute(
        "INSERT INTO read_sessions(game_id, game_name, start_ms, end_ms) VALUES (?1, ?2, ?3, ?3)",
        params![game_id, game_name, start_ms],
    )?;
    Ok(c.last_insert_rowid())
}

/// Actualiza los totales de la sesion (`end_ms` = instante de la ultima actividad).
pub fn session_update(c: &Connection, id: i64, end_ms: i64, t: &Tracker) -> rusqlite::Result<()> {
    c.execute(
        "UPDATE read_sessions SET end_ms = ?2, lines = ?3, chars = ?4, active_ms = ?5 WHERE id = ?1",
        params![id, end_ms, t.lines, t.chars, t.active_ms],
    )?;
    Ok(())
}

/// Cierra la sesion; si no leyo nada se borra.
pub fn session_finish(c: &Connection, id: i64, end_ms: i64, t: &Tracker) -> rusqlite::Result<()> {
    if t.lines == 0 {
        c.execute("DELETE FROM read_sessions WHERE id = ?1", [id])?;
        return Ok(());
    }
    session_update(c, id, end_ms, t)
}

/// Suma a un dia de un juego.
pub fn day_add(c: &Connection, game_id: &str, day: i64, lines: u32, chars: u32, active_ms: i64, phrases: u32) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO read_days(game_id, day, lines, chars, active_ms, phrases) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(game_id, day) DO UPDATE SET lines = lines + ?3, chars = chars + ?4, active_ms = active_ms + ?5, phrases = phrases + ?6",
        params![game_id, day, lines, chars, active_ms, phrases],
    )?;
    Ok(())
}

/// Anota (o quita) la fecha en que una palabra paso a conocida. Si ya tenia fecha se conserva.
pub fn known_dated(c: &Connection, word: &str, on: bool, at_ms: i64) -> rusqlite::Result<()> {
    let w = word.trim();
    if w.is_empty() {
        return Ok(());
    }
    if on {
        c.execute("INSERT OR IGNORE INTO known_word_dates(word, at_ms) VALUES (?1, ?2)", params![w, at_ms])?;
    } else {
        c.execute("DELETE FROM known_word_dates WHERE word = ?1", [w])?;
    }
    Ok(())
}

/// Calcula las estadisticas leyendo la base. `game` = solo ese juego (los globales y la racha siguen siendo de todos
/// los juegos si es `None`; con `Some` todo se limita al juego, salvo las palabras conocidas, que son globales).
pub fn load_stats(c: &Connection, game: Option<&str>, now_ms: i64) -> rusqlite::Result<Stats> {
    let off = local_offset_min(now_ms);
    let mut rows = Vec::new();
    {
        let mut st = c.prepare("SELECT game_id, day, lines, chars, active_ms, phrases FROM read_days ORDER BY day")?;
        let it = st.query_map([], |r| {
            Ok(DayRow { game_id: r.get(0)?, day: r.get(1)?, lines: r.get(2)?, chars: r.get(3)?, active_ms: r.get(4)?, phrases: r.get(5)? })
        })?;
        for r in it {
            let r = r?;
            if game.map(|g| g == r.game_id).unwrap_or(true) {
                rows.push(r);
            }
        }
    }
    let mut names = BTreeMap::new();
    {
        let mut st = c.prepare("SELECT game_id, game_name FROM read_sessions ORDER BY start_ms")?;
        for r in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (id, n) = r?;
            if !n.is_empty() {
                names.insert(id, n);
            }
        }
    }
    let known_ms: Vec<i64> = {
        let mut st = c.prepare("SELECT at_ms FROM known_word_dates ORDER BY at_ms")?;
        let v = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<Vec<i64>>>()?;
        v
    };
    let known_total: u32 = c.query_row("SELECT count(*) FROM known_words", [], |r| r.get(0)).unwrap_or(0);
    Ok(compute(&rows, &names, &known_ms, known_total, day_of(now_ms, off), off))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    #[test]
    fn cuenta_solo_kana_y_kanji() {
        assert_eq!(count_jp_chars("窓の外では、雪が降る。"), 9);
        assert_eq!(count_jp_chars("「ああ・・・」 abc 123 ！？"), 2);
        assert_eq!(count_jp_chars("コーヒー"), 4);
        assert_eq!(count_jp_chars("ﾊﾝｶｸ"), 4);
        assert_eq!(count_jp_chars("人々"), 2);
        assert_eq!(count_jp_chars(""), 0);
    }

    #[test]
    fn lineas_repetidas_no_suman() {
        let mut t = Tracker::new(0);
        assert!(t.push("おはよう。", 1000).counted);
        assert!(!t.push("おはよう。", 2000).counted);
        assert!(!t.push(" おはよう。 ", 3000).counted);
        assert!(!t.push("……！", 4000).counted);
        assert_eq!((t.lines, t.chars), (1, 4));
    }

    #[test]
    fn tiempo_activo_excluye_pausas() {
        let mut t = Tracker::new(0);
        t.push("一行目", 0);
        t.push("二行目", 10 * 1000); // +10 s
        t.push("三行目", 10 * 1000 + 4 * MIN); // +4 min: cuenta
        t.push("四行目", 10 * 1000 + 4 * MIN + 20 * MIN); // 20 min: pausa
        t.push("五行目", 10 * 1000 + 4 * MIN + 20 * MIN + 30 * 1000); // +30 s
        assert_eq!(t.active_ms, 10 * 1000 + 4 * MIN + 30 * 1000);
        assert_eq!(t.lines, 5);
        // Justo en el limite cuenta; un ms mas, no.
        let mut u = Tracker::new(0);
        u.push("あ", 0);
        u.push("い", PAUSE_MS);
        u.push("う", PAUSE_MS * 2 + 1);
        assert_eq!(u.active_ms, PAUSE_MS);
    }

    #[test]
    fn live_suma_tramo_reciente_y_velocidad() {
        let mut t = Tracker::new(0);
        t.push("あいうえお", 0);
        t.push("かきくけこ", 60_000);
        let l = t.live(90_000);
        assert_eq!(l.active_ms, 90_000);
        assert_eq!(l.chars, 10);
        assert_eq!(l.cph, 400); // 10 car en 1,5 min
        // Con pausa larga el tramo en curso no cuenta.
        assert_eq!(t.live(60_000 + 10 * MIN).active_ms, 60_000);
        assert_eq!(cph(100, 10_000), 0);
    }

    #[test]
    fn dias_y_semanas() {
        assert_eq!(day_of(0, 0), 0);
        assert_eq!(day_of(-1, 0), -1);
        assert_eq!(day_of(23 * 3_600_000, 120), 1); // 23:00 UTC con +2 h = dia siguiente
        assert_eq!(weekday(4), 0); // 5 de enero de 1970, lunes
        assert_eq!(week_start(4), 4);
        assert_eq!(week_start(10), 4);
        assert_eq!(week_start(11), 11);
        assert_eq!(week_start(0), -3);
    }

    #[test]
    fn rachas() {
        let s: BTreeSet<i64> = [1, 2, 3, 7, 8].into_iter().collect();
        assert_eq!(streaks(&s, 8), (2, 3));
        assert_eq!(streaks(&s, 9), (2, 3)); // ayer fue el ultimo dia: sigue viva
        assert_eq!(streaks(&s, 10), (0, 3));
        assert_eq!(streaks(&BTreeSet::new(), 10), (0, 0));
    }

    fn row(g: &str, day: i64, chars: u32, active_ms: i64) -> DayRow {
        DayRow { game_id: g.into(), day, lines: chars / 10, chars, active_ms, phrases: 0 }
    }

    #[test]
    fn agregados() {
        let today = 20_000;
        let rows = vec![
            row("a", today - 2, 3000, 3_600_000),
            row("a", today - 1, 1000, 1_800_000),
            row("b", today - 1, 500, 0),
            row("b", today, 2000, 3_600_000),
            row("a", today - 100, 400, 600_000),
        ];
        let mut names = BTreeMap::new();
        names.insert("a".to_string(), "Juego A".to_string());
        let known = [today * DAY_MS + 5, today * DAY_MS + 9, (today - 9) * DAY_MS];
        let s = compute(&rows, &names, &known, 50, today, 0);
        assert_eq!(s.total.chars, 6900);
        assert_eq!(s.total.days, 4);
        assert_eq!(s.games.len(), 2);
        assert_eq!(s.games[0].game_id, "b"); // el mas reciente primero
        assert_eq!(s.games[1].game_name, "Juego A");
        assert_eq!(s.games[1].totals.chars, 4400);
        assert_eq!(s.games[1].totals.cph, cph(4400, 3_600_000 + 1_800_000 + 600_000));
        assert_eq!(s.days.len(), 30);
        assert_eq!(s.days[29].chars, 2000);
        assert_eq!(s.days[29].new_known, 2);
        assert_eq!(s.days[28].chars, 1500);
        assert_eq!(s.streak, 3);
        assert_eq!(s.chars_per_day, (3000 + 1500 + 2000) / 3);
        assert_eq!(s.weeks.len(), 8);
        let total_week: u32 = s.weeks.iter().map(|w| w.new_known).sum();
        assert_eq!(total_week, 3);
        assert_eq!(s.known_total, 50);
        assert_eq!(s.known_dated, 3);
    }

    #[test]
    fn base_de_datos() {
        let db = crate::db::Db::open_memory().unwrap();
        db.with(|c| c.execute_batch(SCHEMA)).unwrap();
        db.with(|c| c.execute_batch(SCHEMA)).unwrap(); // idempotente
        let now = 1_700_000_000_000i64;
        let off = local_offset_min(now);
        let today = day_of(now, off);
        db.with(|c| -> rusqlite::Result<()> {
            let mut t = Tracker::new(now);
            let id = session_start(c, "g1", "Mi novela", now)?;
            let d = t.push("雪が降っている。", now + 1000);
            day_add(c, "g1", today, 1, d.chars, d.active_ms, 0)?;
            let d = t.push("明日も降るだろう。", now + 11_000);
            day_add(c, "g1", today, 1, d.chars, d.active_ms, 0)?;
            session_update(c, id, now + 11_000, &t)?;
            day_add(c, "g1", today, 0, 0, 0, 1)?;
            known_dated(c, "雪", true, now)?;
            known_dated(c, "降る", true, now)?;
            known_dated(c, "降る", false, now)?;
            let empty = session_start(c, "g1", "Mi novela", now + 50_000)?;
            session_finish(c, empty, now + 60_000, &Tracker::new(0))?;
            Ok(())
        })
        .unwrap();
        let (n, chars): (i64, i64) = db.with(|c| c.query_row("SELECT count(*), sum(chars) FROM read_sessions", [], |r| Ok((r.get(0)?, r.get(1)?)))).unwrap();
        assert_eq!((n, chars), (1, 15));
        let s = db.with(|c| load_stats(c, None, now)).unwrap();
        assert_eq!(s.total.chars, 15);
        assert_eq!(s.total.phrases, 1);
        assert_eq!(s.total.active_ms, 10_000);
        assert_eq!(s.games[0].game_name, "Mi novela");
        assert_eq!(s.days[29].new_known, 1);
        assert_eq!(s.streak, 1);
        let other = db.with(|c| load_stats(c, Some("otro"), now)).unwrap();
        assert_eq!(other.total.chars, 0);
    }
}
