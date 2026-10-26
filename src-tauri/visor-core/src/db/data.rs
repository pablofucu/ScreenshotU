//! Lectura y escritura de cada tipo de dato del usuario en la base. Todas las funciones reciben la
//! conexion (`Db::with`) y hacen escrituras puntuales: una fila (o las de un juego), no el conjunto.
//! Las operaciones de varias sentencias van en un `SAVEPOINT`, asi que valen tambien dentro de la
//! transaccion de la importacion.
use crate::games::{ManualGame, PlayStat, Session, Stats};
use crate::jp::phrases::{Phrase, Review};
use crate::jp::NovelPrefs;
use crate::sagas::Saga;
use crate::sgdb::{SgdbEntry, SgdbStore};
use crate::userdata::{HiddenEntry, MagpiePref, ReleaseDate, TitlePref, TitlePrefs};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeMap, HashMap};

/// Ejecuta `f` dentro de un savepoint: si falla, se deshace todo lo que hizo.
fn atomic<T>(c: &Connection, f: impl FnOnce() -> Result<T>) -> Result<T> {
    c.execute_batch("SAVEPOINT visor_op")?;
    match f() {
        Ok(v) => {
            c.execute_batch("RELEASE visor_op")?;
            Ok(v)
        }
        Err(e) => {
            let _ = c.execute_batch("ROLLBACK TO visor_op; RELEASE visor_op");
            Err(e)
        }
    }
}

/* ------------------------------------------------------------------ */
/* meta                                                                 */
/* ------------------------------------------------------------------ */

pub fn meta_get(c: &Connection, key: &str) -> Result<Option<String>> {
    Ok(c.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()?)
}

pub fn meta_set(c: &Connection, key: &str, value: &str) -> Result<()> {
    c.execute("INSERT INTO meta(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [key, value])?;
    Ok(())
}

/* ------------------------------------------------------------------ */
/* Favoritas                                                            */
/* ------------------------------------------------------------------ */

/// Rutas favoritas en el orden en que se marcaron.
pub fn favorites_load(c: &Connection) -> Result<Vec<String>> {
    let mut st = c.prepare("SELECT path FROM favorites ORDER BY rowid")?;
    let v = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(v)
}

/// Marca o desmarca una captura (al marcarla de nuevo pasa al final, como en la lista en memoria).
pub fn favorite_set(c: &Connection, path: &str, on: bool) -> Result<()> {
    atomic(c, || {
        c.execute("DELETE FROM favorites WHERE path = ?1", [path])?;
        if on {
            c.execute("INSERT INTO favorites(path) VALUES (?1)", [path])?;
        }
        Ok(())
    })
}

/// La favorita sigue a la captura cuando se mueve de carpeta (conserva su posicion).
pub fn favorite_rename(c: &Connection, old: &str, new: &str) -> Result<()> {
    c.execute("UPDATE OR REPLACE favorites SET path = ?2 WHERE path = ?1", [old, new])?;
    Ok(())
}

/// Pasa a ruta portable (relativa a `root`, ver [`crate::relpath`]) las favoritas guardadas con ruta
/// absoluta dentro de la raiz. Las de fuera se quedan como estan. Se puede repetir sin efecto.
pub fn favorites_relativize(c: &Connection, root: &std::path::Path) -> Result<usize> {
    atomic(c, || {
        let mut n = 0;
        for p in favorites_load(c)? {
            if let Some(rel) = crate::relpath::to_rel(root, &p) {
                favorite_rename(c, &p, &rel)?;
                n += 1;
            }
        }
        Ok(n)
    })
}

/* ------------------------------------------------------------------ */
/* Juegos anadidos a mano                                               */
/* ------------------------------------------------------------------ */

pub fn manual_load(c: &Connection) -> Result<Vec<ManualGame>> {
    let mut st = c.prepare("SELECT id, name, kind, exe, args, rom, platform, folder, workdir FROM games ORDER BY rowid")?;
    let v = st
        .query_map([], |r| {
            Ok(ManualGame { id: r.get(0)?, name: r.get(1)?, kind: r.get(2)?, exe: r.get(3)?, args: r.get(4)?, rom: r.get(5)?, platform: r.get(6)?, folder: r.get(7)?, workdir: r.get(8)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(v)
}

/// Anade un juego; si el id ya existe se ignora (devuelve `false`).
pub fn manual_add(c: &Connection, g: &ManualGame) -> Result<bool> {
    let n = c.execute(
        "INSERT OR IGNORE INTO games(id, name, kind, exe, args, rom, platform, folder, workdir) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![g.id, g.name, g.kind, g.exe, g.args, g.rom, g.platform, g.folder, g.workdir],
    )?;
    Ok(n > 0)
}

/// Cambia el ejecutable y/o la carpeta de trabajo de un juego añadido a mano (`None` = no se toca). `false` si no existe.
pub fn manual_set_paths(c: &Connection, id: &str, exe: Option<&str>, workdir: Option<&str>) -> Result<bool> {
    let mut n = 0;
    if let Some(exe) = exe {
        n += c.execute("UPDATE games SET exe = ?2 WHERE id = ?1", [id, exe])?;
    }
    if let Some(workdir) = workdir {
        n += c.execute("UPDATE games SET workdir = ?2 WHERE id = ?1", [id, workdir])?;
    }
    Ok(n > 0)
}

/// Quita un juego; `false` si no existia.
pub fn manual_remove(c: &Connection, id: &str) -> Result<bool> {
    Ok(c.execute("DELETE FROM games WHERE id = ?1", [id])? > 0)
}

/// Cambia de categoria un juego anadido a mano (y su entrada de «oculto», si la tiene). `false` si no existe.
pub fn manual_set_platform(c: &Connection, id: &str, platform: &str) -> Result<bool> {
    atomic(c, || {
        let n = c.execute("UPDATE games SET platform = ?2 WHERE id = ?1", [id, platform])?;
        c.execute("UPDATE hidden SET platform = ?2 WHERE id = ?1", [id, platform])?;
        Ok(n > 0)
    })
}

/// Una carpeta de titulo cambia de ruta: las favoritas y las filas del catalogo de su interior la siguen.
/// Las rutas son las portables (`PC/Juego`), sin barra final.
pub fn dir_moved(c: &Connection, old: &str, new: &str) -> Result<()> {
    atomic(c, || {
        for sql in [
            "UPDATE OR REPLACE favorites SET path = ?2 || substr(path, length(?1) + 1) WHERE substr(path, 1, length(?1) + 1) = ?1 || '/'",
            "UPDATE OR REPLACE shots SET path = ?2 || substr(path, length(?1) + 1) WHERE substr(path, 1, length(?1) + 1) = ?1 || '/'",
        ] {
            c.execute(sql, [old, new])?;
        }
        Ok(())
    })
}

/* ------------------------------------------------------------------ */
/* Estadisticas y partidas                                              */
/* ------------------------------------------------------------------ */

/// Todas las estadisticas. Por juego se cargan las ultimas `MAX_SESSIONS` partidas (como en memoria).
pub fn stats_load(c: &Connection) -> Result<Stats> {
    let mut out = Stats::new();
    {
        let mut st = c.prepare("SELECT game_id, last_played_ms, seconds FROM play_stats")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?;
        for row in rows {
            let (id, last, secs) = row?;
            out.insert(id, PlayStat { last_played: last, seconds: secs.max(0) as u64, sessions: Vec::new() });
        }
    }
    {
        let mut st = c.prepare("SELECT game_id, start_ms, end_ms FROM sessions ORDER BY id")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, Session { start: r.get(1)?, end: r.get(2)? })))?;
        for row in rows {
            let (id, s) = row?;
            if let Some(e) = out.get_mut(&id) {
                e.sessions.push(s);
            }
        }
    }
    for e in out.values_mut() {
        if e.sessions.len() > crate::games::MAX_SESSIONS {
            let extra = e.sessions.len() - crate::games::MAX_SESSIONS;
            e.sessions.drain(..extra);
        }
    }
    Ok(out)
}

/// Marca el inicio de una partida (actualiza `last_played`).
pub fn stat_mark_started(c: &Connection, id: &str, now_ms: i64) -> Result<()> {
    c.execute(
        "INSERT INTO play_stats(game_id, last_played_ms, seconds) VALUES (?1, ?2, 0)
         ON CONFLICT(game_id) DO UPDATE SET last_played_ms = excluded.last_played_ms",
        params![id, now_ms],
    )?;
    Ok(())
}

/// Registra una partida terminada: suma su duracion al total y guarda la partida. Si empieza menos de
/// `MERGE_GAP_MS` despues del fin de la ultima de ese juego (o dentro de ella), la amplia (el hueco cuenta).
pub fn stat_add_session(c: &Connection, id: &str, start_ms: i64, end_ms: i64) -> Result<()> {
    let end = end_ms.max(start_ms);
    let secs = (end - start_ms) / 1000;
    let last: Option<(i64, i64, i64)> = c
        .query_row("SELECT id, start_ms, end_ms FROM sessions WHERE game_id = ?1 ORDER BY id DESC LIMIT 1", [id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .optional()?;
    if let Some((sid, s, e)) = last {
        if start_ms >= s && start_ms - e < crate::games::MERGE_GAP_MS {
            let extra = if end > e { (end - e) / 1000 } else { 0 };
            return atomic(c, || {
                if end > e {
                    c.execute("UPDATE sessions SET end_ms = ?1 WHERE id = ?2", params![end, sid])?;
                }
                c.execute(
                    "INSERT INTO play_stats(game_id, last_played_ms, seconds) VALUES (?1, 0, ?2)
                     ON CONFLICT(game_id) DO UPDATE SET seconds = seconds + excluded.seconds",
                    params![id, extra],
                )?;
                Ok(())
            });
        }
    }
    atomic(c, || {
        c.execute(
            "INSERT INTO play_stats(game_id, last_played_ms, seconds) VALUES (?1, 0, ?2)
             ON CONFLICT(game_id) DO UPDATE SET seconds = seconds + excluded.seconds",
            params![id, secs],
        )?;
        c.execute("INSERT INTO sessions(game_id, start_ms, end_ms) VALUES (?1, ?2, ?3)", params![id, start_ms, end])?;
        Ok(())
    })
}

/// Inserta las estadisticas completas de un juego (importacion).
pub fn stat_import(c: &Connection, id: &str, s: &PlayStat) -> Result<()> {
    atomic(c, || {
        c.execute(
            "INSERT INTO play_stats(game_id, last_played_ms, seconds) VALUES (?1, ?2, ?3)
             ON CONFLICT(game_id) DO UPDATE SET last_played_ms = excluded.last_played_ms, seconds = excluded.seconds",
            params![id, s.last_played, s.seconds as i64],
        )?;
        c.execute("DELETE FROM sessions WHERE game_id = ?1", [id])?;
        let mut ins = c.prepare("INSERT INTO sessions(game_id, start_ms, end_ms) VALUES (?1, ?2, ?3)")?;
        for x in &s.sessions {
            ins.execute(params![id, x.start, x.end])?;
        }
        Ok(())
    })
}

/* ------------------------------------------------------------------ */
/* Juegos ocultos                                                       */
/* ------------------------------------------------------------------ */

pub fn hidden_load(c: &Connection) -> Result<Vec<HiddenEntry>> {
    let mut st = c.prepare("SELECT id, name, platform FROM hidden ORDER BY rowid")?;
    let v = st.query_map([], |r| Ok(HiddenEntry { id: r.get(0)?, name: r.get(1)?, platform: r.get(2)? }))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(v)
}

/// Oculta (o muestra) un titulo.
pub fn hidden_set(c: &Connection, e: &HiddenEntry, hidden: bool) -> Result<()> {
    atomic(c, || {
        c.execute("DELETE FROM hidden WHERE id = ?1", [&e.id])?;
        if hidden {
            c.execute("INSERT INTO hidden(id, name, platform) VALUES (?1, ?2, ?3)", params![e.id, e.name, e.platform])?;
        }
        Ok(())
    })
}

/* ------------------------------------------------------------------ */
/* Preferencias por titulo                                              */
/* ------------------------------------------------------------------ */

pub fn prefs_load(c: &Connection) -> Result<TitlePrefs> {
    let mut out = TitlePrefs::new();
    {
        let mut st = c.prepare("SELECT title_id, accent, sort FROM title_prefs")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, TitlePref { accent: r.get(1)?, sort: r.get(2)?, ..Default::default() })))?;
        for row in rows {
            let (id, p) = row?;
            out.insert(id, p);
        }
    }
    {
        let mut st = c.prepare("SELECT title_id, mode, native_w, native_h, monitor_w, monitor_h, class, exe, summary, managed FROM title_magpie")?;
        let rows = st.query_map([], |r| {
            let pair = |a: Option<i64>, b: Option<i64>| match (a, b) {
                (Some(a), Some(b)) => Some((a as u32, b as u32)),
                _ => None,
            };
            Ok((
                r.get::<_, String>(0)?,
                MagpiePref {
                    mode: r.get(1)?,
                    native: pair(r.get(2)?, r.get(3)?),
                    monitor: pair(r.get(4)?, r.get(5)?),
                    class: r.get(6)?,
                    exe: r.get(7)?,
                    summary: r.get(8)?,
                    managed: r.get::<_, i64>(9)? != 0,
                },
            ))
        })?;
        for row in rows {
            let (id, m) = row?;
            if let Some(p) = out.get_mut(&id) {
                p.magpie = Some(m);
            }
        }
    }
    {
        let mut st = c.prepare("SELECT title_id, status, finished_ms, rating, list_pos, snooze_ms, updated_ms FROM title_status")?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<i64>>(2)?, r.get::<_, Option<i64>>(3)?, r.get::<_, Option<i64>>(4)?, r.get::<_, Option<i64>>(5)?, r.get::<_, i64>(6)?))
        })?;
        for row in rows {
            let (id, status, fin, rating, pos, snooze, upd) = row?;
            let p = out.entry(id).or_default();
            p.status = Some(status);
            p.finished_ms = fin;
            p.rating = rating;
            p.list_pos = pos;
            p.snooze_ms = snooze;
            p.status_ms = if upd > 0 { Some(upd) } else { None };
        }
    }
    {
        let mut st = c.prepare("SELECT title_id, year, month, day, auto FROM title_release")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                ReleaseDate { year: r.get(1)?, month: r.get(2)?, day: r.get(3)?, auto: r.get::<_, i64>(4)? != 0 },
            ))
        })?;
        for row in rows {
            let (id, d) = row?;
            out.entry(id).or_default().release = Some(d);
        }
    }
    Ok(out)
}

/// Guarda (o quita, con `None`) la fecha de lanzamiento de un titulo.
pub fn release_save(c: &Connection, id: &str, d: Option<&ReleaseDate>) -> Result<()> {
    match d {
        Some(d) => {
            c.execute(
                "INSERT INTO title_release(title_id, year, month, day, auto) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(title_id) DO UPDATE SET year = excluded.year, month = excluded.month, day = excluded.day, auto = excluded.auto",
                params![id, d.year, d.month, d.day, d.auto as i64],
            )?;
        }
        None => {
            c.execute("DELETE FROM title_release WHERE title_id = ?1", [id])?;
        }
    }
    Ok(())
}

/// Guarda las preferencias de un titulo tal y como estan en memoria (`None` = no tiene: se borran).
pub fn pref_save(c: &Connection, id: &str, p: Option<&TitlePref>) -> Result<()> {
    atomic(c, || {
        let Some(p) = p else {
            c.execute("DELETE FROM title_prefs WHERE title_id = ?1", [id])?;
            return Ok(());
        };
        c.execute(
            "INSERT INTO title_prefs(title_id, accent, sort) VALUES (?1, ?2, ?3)
             ON CONFLICT(title_id) DO UPDATE SET accent = excluded.accent, sort = excluded.sort",
            params![id, p.accent, p.sort],
        )?;
        match &p.magpie {
            Some(m) => {
                c.execute(
                    "INSERT INTO title_magpie(title_id, mode, native_w, native_h, monitor_w, monitor_h, class, exe, summary, managed)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                     ON CONFLICT(title_id) DO UPDATE SET mode = excluded.mode, native_w = excluded.native_w, native_h = excluded.native_h,
                        monitor_w = excluded.monitor_w, monitor_h = excluded.monitor_h, class = excluded.class, exe = excluded.exe,
                        summary = excluded.summary, managed = excluded.managed",
                    params![
                        id,
                        m.mode,
                        m.native.map(|x| x.0),
                        m.native.map(|x| x.1),
                        m.monitor.map(|x| x.0),
                        m.monitor.map(|x| x.1),
                        m.class,
                        m.exe,
                        m.summary,
                        m.managed as i64
                    ],
                )?;
            }
            None => {
                c.execute("DELETE FROM title_magpie WHERE title_id = ?1", [id])?;
            }
        }
        Ok(())
    })
}

/// Guarda (o quita, con `None`) el estado de un titulo. Va aparte de `pref_save` para no tocar
/// `updated_ms` al cambiar el acento o el orden.
pub fn status_save(c: &Connection, id: &str, status: Option<(&str, Option<i64>)>, now_ms: i64) -> Result<()> {
    match status {
        Some((s, fin)) => {
            c.execute(
                "INSERT INTO title_status(title_id, status, finished_ms, updated_ms) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(title_id) DO UPDATE SET status = excluded.status, finished_ms = excluded.finished_ms,
                    updated_ms = excluded.updated_ms",
                params![id, s, fin, now_ms],
            )?;
        }
        None => {
            c.execute("DELETE FROM title_status WHERE title_id = ?1", [id])?;
        }
    }
    Ok(())
}

/// Nota, sitio en la lista de pendientes y «dejarlo» de un titulo que ya tiene estado (no hace nada si no lo tiene).
/// Va aparte de `status_save` para no tocar `updated_ms` al reordenar o al decir «dejarlo».
pub fn status_extra_save(c: &Connection, id: &str, rating: Option<i64>, list_pos: Option<i64>, snooze_ms: Option<i64>) -> Result<()> {
    c.execute("UPDATE title_status SET rating = ?2, list_pos = ?3, snooze_ms = ?4 WHERE title_id = ?1", params![id, rating, list_pos, snooze_ms])?;
    Ok(())
}

/// Guarda de golpe el orden de varios pendientes (`(id, posicion)`); los que no son pendientes no se tocan.
pub fn pending_order_save(c: &Connection, order: &[(String, i64)]) -> Result<()> {
    atomic(c, || {
        for (id, pos) in order {
            c.execute("UPDATE title_status SET list_pos = ?2 WHERE title_id = ?1 AND status = 'pending'", params![id, pos])?;
        }
        Ok(())
    })
}

/// Guarda de golpe el «dejarlo» de varios juegos.
pub fn snooze_save(c: &Connection, ids: &[String], now_ms: i64) -> Result<()> {
    atomic(c, || {
        for id in ids {
            c.execute("UPDATE title_status SET snooze_ms = ?2 WHERE title_id = ?1", params![id, now_ms])?;
        }
        Ok(())
    })
}

/* ------------------------------------------------------------------ */
/* Sagas                                                                */
/* ------------------------------------------------------------------ */

/// Convierte las sagas antiguas (sin categoria) con la biblioteca ya cargada y guarda el resultado solo si
/// algo ha cambiado. Devuelve la lista final.
pub fn sagas_convert_legacy(c: &Connection, platform_of: &HashMap<String, String>) -> Result<Vec<Saga>> {
    let list = sagas_load(c)?;
    if platform_of.is_empty() || list.iter().all(|s| s.platform.is_some()) {
        return Ok(list);
    }
    let (out, changed) = crate::sagas::convert_legacy(&list, platform_of);
    if changed {
        sagas_replace(c, &out)?;
    }
    Ok(out)
}

pub fn sagas_load(c: &Connection) -> Result<Vec<Saga>> {
    let mut sagas: Vec<Saga> = Vec::new();
    {
        let mut st = c.prepare("SELECT id, name, platform, color FROM sagas ORDER BY position")?;
        let rows = st.query_map([], |r| Ok(Saga { id: r.get(0)?, name: r.get(1)?, members: Vec::new(), platform: r.get(2)?, color: r.get::<_, Option<i64>>(3)?.map(|v| v as u8) }))?;
        for r in rows {
            sagas.push(r?);
        }
    }
    let idx: HashMap<String, usize> = sagas.iter().enumerate().map(|(i, s)| (s.id.clone(), i)).collect();
    let mut st = c.prepare("SELECT saga_id, title_id FROM saga_members ORDER BY saga_id, position")?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for r in rows {
        let (sid, tid) = r?;
        if let Some(&i) = idx.get(&sid) {
            sagas[i].members.push(tid);
        }
    }
    Ok(sagas)
}

/// Sustituye la lista completa de sagas (la interfaz siempre manda la lista entera). Ya saneada.
pub fn sagas_replace(c: &Connection, list: &[Saga]) -> Result<()> {
    atomic(c, || {
        c.execute("DELETE FROM sagas", [])?; // los miembros caen en cascada
        let mut s = c.prepare("INSERT INTO sagas(id, name, position, platform, color) VALUES (?1, ?2, ?3, ?4, ?5)")?;
        let mut m = c.prepare("INSERT OR IGNORE INTO saga_members(saga_id, title_id, position) VALUES (?1, ?2, ?3)")?;
        for (i, saga) in list.iter().enumerate() {
            s.execute(params![saga.id, saga.name, i as i64, saga.platform, saga.color.map(|v| v as i64)])?;
            for (j, t) in saga.members.iter().enumerate() {
                m.execute(params![saga.id, t, j as i64])?;
            }
        }
        Ok(())
    })
}

/* ------------------------------------------------------------------ */
/* SteamGridDB                                                          */
/* ------------------------------------------------------------------ */

pub fn sgdb_load(c: &Connection) -> Result<SgdbStore> {
    let mut st = c.prepare("SELECT title_id, game_id, game_name, auto_done, tried FROM sgdb")?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            SgdbEntry { game_id: r.get::<_, Option<i64>>(1)?.map(|x| x as u64), game_name: r.get(2)?, auto_done: r.get::<_, i64>(3)? != 0, tried: r.get(4)? },
        ))
    })?;
    let mut out = SgdbStore::new();
    for r in rows {
        let (id, e) = r?;
        out.insert(id, e);
    }
    Ok(out)
}

pub fn sgdb_put(c: &Connection, title_id: &str, e: &SgdbEntry) -> Result<()> {
    c.execute(
        "INSERT INTO sgdb(title_id, game_id, game_name, auto_done, tried) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(title_id) DO UPDATE SET game_id = excluded.game_id, game_name = excluded.game_name, auto_done = excluded.auto_done, tried = excluded.tried",
        params![title_id, e.game_id.map(|x| x as i64), e.game_name, e.auto_done as i64, e.tried],
    )?;
    Ok(())
}

/* ------------------------------------------------------------------ */
/* Novelas japonesas: preferencias y palabras conocidas                 */
/* ------------------------------------------------------------------ */

/// Preferencias de un juego; sin fila (o con JSON ilegible) = `None`.
pub fn novel_prefs_get(c: &Connection, game_id: &str) -> Result<Option<NovelPrefs>> {
    let s: Option<String> = c.query_row("SELECT prefs_json FROM novel_prefs WHERE game_id = ?1", [game_id], |r| r.get(0)).optional()?;
    Ok(s.and_then(|s| serde_json::from_str(&s).ok()))
}

pub fn novel_prefs_put(c: &Connection, game_id: &str, p: &NovelPrefs) -> Result<()> {
    c.execute(
        "INSERT INTO novel_prefs(game_id, prefs_json) VALUES (?1, ?2) ON CONFLICT(game_id) DO UPDATE SET prefs_json = excluded.prefs_json",
        params![game_id, serde_json::to_string(p)?],
    )?;
    Ok(())
}

pub fn novel_prefs_all(c: &Connection) -> Result<BTreeMap<String, NovelPrefs>> {
    let mut st = c.prepare("SELECT game_id, prefs_json FROM novel_prefs")?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = BTreeMap::new();
    for r in rows {
        let (id, s) = r?;
        if let Ok(p) = serde_json::from_str(&s) {
            out.insert(id, p);
        }
    }
    Ok(out)
}

pub fn known_load(c: &Connection) -> Result<Vec<String>> {
    let mut st = c.prepare("SELECT word FROM known_words ORDER BY word")?;
    let v = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(v)
}

pub fn known_set(c: &Connection, word: &str, on: bool) -> Result<()> {
    let w = word.trim();
    if w.is_empty() {
        return Ok(());
    }
    if on {
        c.execute("INSERT OR IGNORE INTO known_words(word) VALUES (?1)", [w])?;
    } else {
        c.execute("DELETE FROM known_words WHERE word = ?1", [w])?;
    }
    Ok(())
}

/* ------------------------------------------------------------------ */
/* Glosario de frases                                                   */
/* ------------------------------------------------------------------ */

const PHRASE_COLS: &str = "game_id, id, game_name, who, text, toks_json, words_json, grammar_json, context_json, created_at, last_seen_at, seen, \
     translation, explanation_json, starred, note, shot, audio, synth, anki_note_id, due_ms, interval_days, ease, reps, lapses";

fn json_col<T: serde::de::DeserializeOwned + Default>(s: Option<String>) -> T {
    s.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn phrase_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Phrase> {
    Ok(Phrase {
        game_id: r.get(0)?,
        id: r.get(1)?,
        game_name: r.get(2)?,
        who: r.get(3)?,
        text: r.get(4)?,
        toks: json_col(r.get(5)?),
        words: json_col(r.get(6)?),
        grammar: json_col(r.get(7)?),
        context: json_col(r.get(8)?),
        created_at: r.get(9)?,
        last_seen_at: r.get(10)?,
        seen: r.get::<_, i64>(11)?.max(0) as u32,
        translation: r.get(12)?,
        explanation: r.get::<_, Option<String>>(13)?.and_then(|s| serde_json::from_str(&s).ok()),
        starred: r.get::<_, i64>(14)? != 0,
        note: r.get(15)?,
        tags: Vec::new(),
        shot: r.get(16)?,
        audio: r.get(17)?,
        synth: r.get(18)?,
        anki_note_id: r.get(19)?,
        review: Review { due: r.get(20)?, interval_days: r.get::<_, f64>(21)? as f32, ease: r.get::<_, f64>(22)? as f32, reps: r.get::<_, i64>(23)?.max(0) as u32, lapses: r.get::<_, i64>(24)?.max(0) as u32 },
    })
}

/// Lee las frases (de un juego o de todos) y les pone sus etiquetas.
fn phrases_query(c: &Connection, game_id: Option<&str>) -> Result<Vec<Phrase>> {
    let mut out: Vec<Phrase> = Vec::new();
    match game_id {
        Some(g) => {
            let mut st = c.prepare(&format!("SELECT {PHRASE_COLS} FROM phrases WHERE game_id = ?1 ORDER BY created_at, rowid"))?;
            for r in st.query_map([g], phrase_from_row)? {
                out.push(r?);
            }
        }
        None => {
            let mut st = c.prepare(&format!("SELECT {PHRASE_COLS} FROM phrases ORDER BY game_id, created_at, rowid"))?;
            for r in st.query_map([], phrase_from_row)? {
                out.push(r?);
            }
        }
    }
    if out.is_empty() {
        return Ok(out);
    }
    let idx: HashMap<(String, String), usize> = out.iter().enumerate().map(|(i, p)| ((p.game_id.clone(), p.id.clone()), i)).collect();
    let sql = match game_id {
        Some(_) => "SELECT game_id, phrase_id, tag FROM phrase_tags WHERE game_id = ?1 ORDER BY game_id, phrase_id, position",
        None => "SELECT game_id, phrase_id, tag FROM phrase_tags ORDER BY game_id, phrase_id, position",
    };
    let mut st = c.prepare(sql)?;
    let map = |r: &rusqlite::Row<'_>| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?));
    let rows: Vec<_> = match game_id {
        Some(g) => st.query_map([g], map)?.collect::<rusqlite::Result<_>>()?,
        None => st.query_map([], map)?.collect::<rusqlite::Result<_>>()?,
    };
    for (g, id, tag) in rows {
        if let Some(&i) = idx.get(&(g, id)) {
            out[i].tags.push(tag);
        }
    }
    Ok(out)
}

/// Frases de un juego (en orden de creacion).
pub fn phrases_load_game(c: &Connection, game_id: &str) -> Result<Vec<Phrase>> {
    phrases_query(c, Some(game_id))
}

/// Frases de todos los juegos.
pub fn phrases_load_all(c: &Connection) -> Result<Vec<Phrase>> {
    phrases_query(c, None)
}

/// Una frase por juego e id.
pub fn phrase_get(c: &Connection, game_id: &str, id: &str) -> Result<Option<Phrase>> {
    let mut st = c.prepare(&format!("SELECT {PHRASE_COLS} FROM phrases WHERE game_id = ?1 AND id = ?2"))?;
    let Some(mut p) = st.query_row([game_id, id], phrase_from_row).optional()? else { return Ok(None) };
    let mut st = c.prepare("SELECT tag FROM phrase_tags WHERE game_id = ?1 AND phrase_id = ?2 ORDER BY position")?;
    p.tags = st.query_map([game_id, id], |r| r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(Some(p))
}

/// Inserta o reemplaza una frase entera (con sus etiquetas). Si no tiene id se lo calcula.
pub fn phrase_save(c: &Connection, p: &Phrase) -> Result<()> {
    let id = if p.id.is_empty() { crate::jp::phrases::phrase_id(&p.game_id, &p.text) } else { p.id.clone() };
    let explanation = match &p.explanation {
        Some(e) => Some(serde_json::to_string(e)?),
        None => None,
    };
    atomic(c, || {
        c.execute(
            &format!(
                "INSERT INTO phrases({PHRASE_COLS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)
                 ON CONFLICT(game_id, id) DO UPDATE SET game_name = excluded.game_name, who = excluded.who, text = excluded.text,
                    toks_json = excluded.toks_json, words_json = excluded.words_json, grammar_json = excluded.grammar_json,
                    context_json = excluded.context_json, created_at = excluded.created_at, last_seen_at = excluded.last_seen_at,
                    seen = excluded.seen, translation = excluded.translation, explanation_json = excluded.explanation_json,
                    starred = excluded.starred, note = excluded.note, shot = excluded.shot, audio = excluded.audio,
                    synth = excluded.synth, anki_note_id = excluded.anki_note_id, due_ms = excluded.due_ms,
                    interval_days = excluded.interval_days, ease = excluded.ease, reps = excluded.reps, lapses = excluded.lapses"
            ),
            params![
                p.game_id,
                id,
                p.game_name,
                p.who,
                p.text,
                serde_json::to_string(&p.toks)?,
                serde_json::to_string(&p.words)?,
                serde_json::to_string(&p.grammar)?,
                serde_json::to_string(&p.context)?,
                p.created_at,
                p.last_seen_at,
                p.seen as i64,
                p.translation,
                explanation,
                p.starred as i64,
                p.note,
                p.shot,
                p.audio,
                p.synth,
                p.anki_note_id,
                p.review.due,
                p.review.interval_days as f64,
                p.review.ease as f64,
                p.review.reps as i64,
                p.review.lapses as i64,
            ],
        )?;
        c.execute("DELETE FROM phrase_tags WHERE game_id = ?1 AND phrase_id = ?2", [&p.game_id, &id])?;
        let mut ins = c.prepare("INSERT OR IGNORE INTO phrase_tags(game_id, phrase_id, position, tag) VALUES (?1, ?2, ?3, ?4)")?;
        for (i, t) in p.tags.iter().enumerate() {
            ins.execute(params![p.game_id, id, i as i64, t])?;
        }
        Ok(())
    })
}

/// Borra una frase (sus etiquetas caen en cascada). `false` si no existia.
pub fn phrase_delete(c: &Connection, game_id: &str, id: &str) -> Result<bool> {
    Ok(c.execute("DELETE FROM phrases WHERE game_id = ?1 AND id = ?2", [game_id, id])? > 0)
}

/// Cuantas frases tocan repasar (`due_ms <= now`), de un juego o de todos.
pub fn phrases_due_count(c: &Connection, game_id: Option<&str>, now: i64) -> Result<usize> {
    let n: i64 = match game_id {
        Some(g) => c.query_row("SELECT count(*) FROM phrases WHERE game_id = ?1 AND due_ms <= ?2", params![g, now], |r| r.get(0))?,
        None => c.query_row("SELECT count(*) FROM phrases WHERE due_ms <= ?1", [now], |r| r.get(0))?,
    };
    Ok(n as usize)
}
