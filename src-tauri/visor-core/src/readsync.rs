//! Multi-PC (fase 4): lo que se lee en un PC cliente (sesiones de lectura y frases del glosario) se sube al host.
//!
//! El cliente guarda en su base local lo que registra el texthook de un juego instalado alli; un hilo lo sube al
//! host como las partidas (`synced_ms` NULL = falta subirlo; se vuelve a NULL cuando la fila cambia).
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

use crate::db::data as dbd;
use crate::jp::phrases::Phrase;
use crate::jp::readstats as rs;

/// Migracion: marca de subida en las sesiones de lectura y en las frases.
pub const SCHEMA: &str = r#"
ALTER TABLE read_sessions ADD COLUMN synced_ms INTEGER;
ALTER TABLE phrases ADD COLUMN synced_ms INTEGER;
"#;

/// Una sesion de lectura tal como viaja del cliente al host.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteRead {
    pub uid: String,
    pub game_id: String,
    pub game_name: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub lines: i64,
    pub chars: i64,
    pub active_ms: i64,
}

/// Sesiones de lectura propias con algo leido y aun sin subir, de las mas antiguas a las mas nuevas.
pub fn unsynced_reads(c: &Connection, limit: usize) -> Result<Vec<RemoteRead>> {
    let mut st = c.prepare(
        "SELECT uid, game_id, game_name, start_ms, end_ms, lines, chars, active_ms FROM read_sessions
         WHERE device_id = '' AND uid IS NOT NULL AND synced_ms IS NULL AND lines > 0 ORDER BY start_ms LIMIT ?1",
    )?;
    let rows = st.query_map([limit as i64], |r| {
        Ok(RemoteRead {
            uid: r.get(0)?,
            game_id: r.get(1)?,
            game_name: r.get(2)?,
            start_ms: r.get(3)?,
            end_ms: r.get(4)?,
            lines: r.get(5)?,
            chars: r.get(6)?,
            active_ms: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Marca como subidas las sesiones de lectura con esos `uid`.
pub fn mark_reads_synced(c: &Connection, uids: &[String], now_ms: i64) -> Result<()> {
    let tx = c.unchecked_transaction()?;
    {
        let mut st = tx.prepare("UPDATE read_sessions SET synced_ms = ?1 WHERE uid = ?2")?;
        for u in uids {
            st.execute(params![now_ms, u])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Guarda en el host una sesion de lectura subida por `device_id` y suma lo nuevo a su dia (`offset_min` = zona
/// horaria del host). Ya conocida -> solo se suma lo que haya crecido. Un `uid` de otro dispositivo se ignora.
/// Devuelve si el `uid` queda guardado.
pub fn put_remote_read(c: &Connection, device_id: &str, r: &RemoteRead, offset_min: i32) -> Result<bool> {
    if r.uid.trim().is_empty() || r.game_id.trim().is_empty() || device_id.is_empty() {
        return Ok(false);
    }
    let end = r.end_ms.max(r.start_ms);
    let (lines, chars, active) = (r.lines.max(0), r.chars.max(0), r.active_ms.clamp(0, end - r.start_ms));
    let tx = c.unchecked_transaction()?;
    let known: Option<(i64, String, i64, i64, i64)> = tx
        .query_row("SELECT id, device_id, lines, chars, active_ms FROM read_sessions WHERE uid = ?1", [&r.uid], |x| {
            Ok((x.get(0)?, x.get(1)?, x.get(2)?, x.get(3)?, x.get(4)?))
        })
        .optional()?;
    let (dl, dc, da) = match &known {
        Some((_, dev, ..)) if dev != device_id => return Ok(false),
        Some((_, _, l, ch, a)) => ((lines - l).max(0), (chars - ch).max(0), (active - a).max(0)),
        None => (lines, chars, active),
    };
    match known {
        Some((id, ..)) => {
            tx.execute(
                "UPDATE read_sessions SET end_ms = MAX(end_ms, ?1), lines = MAX(lines, ?2), chars = MAX(chars, ?3),
                     active_ms = MAX(active_ms, ?4) WHERE id = ?5",
                params![end, lines, chars, active, id],
            )?;
        }
        None => {
            tx.execute(
                "INSERT INTO read_sessions(game_id, game_name, start_ms, end_ms, lines, chars, active_ms, device_id, uid, synced_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?4)",
                params![r.game_id, r.game_name, r.start_ms, end, lines, chars, active, device_id, r.uid],
            )?;
        }
    }
    if dl > 0 || dc > 0 || da > 0 {
        // El cliente reparte por dias al leer; aqui todo va al dia en que empezo la sesion.
        rs::day_add(&tx, &r.game_id, rs::day_of(r.start_ms, offset_min), dl as u32, dc as u32, da, 0)?;
    }
    tx.commit()?;
    Ok(true)
}

/* ------------------------------ frases ------------------------------ */

/// Frases propias aun sin subir (de las mas antiguas a las mas nuevas).
pub fn unsynced_phrases(c: &Connection, limit: usize) -> Result<Vec<Phrase>> {
    let keys: Vec<(String, String)> = {
        let mut st = c.prepare("SELECT game_id, id FROM phrases WHERE synced_ms IS NULL ORDER BY created_at LIMIT ?1")?;
        let rows = st.query_map([limit as i64], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let mut out = Vec::new();
    for (g, id) in keys {
        if let Some(p) = dbd::phrase_get(c, &g, &id)? {
            out.push(p);
        }
    }
    Ok(out)
}

/// Marca como subidas las frases `(game_id, id)`.
pub fn mark_phrases_synced(c: &Connection, keys: &[(String, String)], now_ms: i64) -> Result<()> {
    let tx = c.unchecked_transaction()?;
    {
        let mut st = tx.prepare("UPDATE phrases SET synced_ms = ?1 WHERE game_id = ?2 AND id = ?3")?;
        for (g, id) in keys {
            st.execute(params![now_ms, g, id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Junta la frase del host (si ya la tenia) con la que llega del cliente. Lo que ya tenga el host se queda (se pudo
/// editar o repasar alli); del cliente se toma lo que falte, las vistas y las etiquetas nuevas.
pub fn merge_phrase(host: Option<Phrase>, client: Phrase) -> Phrase {
    let Some(mut h) = host else { return client };
    h.seen = h.seen.max(client.seen);
    h.last_seen_at = h.last_seen_at.max(client.last_seen_at);
    if h.created_at == 0 || (client.created_at > 0 && client.created_at < h.created_at) {
        h.created_at = client.created_at;
    }
    h.starred |= client.starred;
    if h.note.trim().is_empty() {
        h.note = client.note;
    }
    for t in client.tags {
        if !h.tags.contains(&t) {
            h.tags.push(t);
        }
    }
    h.translation = h.translation.or(client.translation);
    h.explanation = h.explanation.or(client.explanation);
    h.shot = h.shot.or(client.shot);
    h.audio = h.audio.or(client.audio);
    h.synth = h.synth.or(client.synth);
    h.anki_note_id = h.anki_note_id.or(client.anki_note_id);
    if h.context.is_empty() {
        h.context = client.context;
    }
    h
}

/// Guarda en el host una frase subida por un cliente. Devuelve si era nueva (entonces cuenta en su dia).
pub fn put_remote_phrase(c: &Connection, p: Phrase, offset_min: i32) -> Result<bool> {
    let mut p = p;
    if p.id.is_empty() {
        p.id = crate::jp::phrases::phrase_id(&p.game_id, &p.text);
    }
    let old = dbd::phrase_get(c, &p.game_id, &p.id)?;
    let new = old.is_none();
    let created = p.created_at;
    let merged = merge_phrase(old, p);
    dbd::phrase_save(c, &merged)?;
    if new {
        let at = if created > 0 { created } else { merged.last_seen_at };
        rs::day_add(c, &merged.game_id, rs::day_of(at, offset_min), 0, 0, 0, 1)?;
    }
    Ok(new)
}

/// Archivos de media de una frase (rutas relativas a `<config>/phrases/media/`).
pub fn media_of(p: &Phrase) -> Vec<String> {
    [&p.shot, &p.audio, &p.synth].into_iter().flatten().filter(|m| !m.is_empty()).cloned().collect()
}

/// Ruta de un archivo de media dentro de `dir`: solo un nombre de archivo (sin carpetas) de un tipo conocido.
pub fn media_path(dir: &Path, rel: &str) -> Option<PathBuf> {
    let p = Path::new(rel);
    let mut comps = p.components();
    let (Some(Component::Normal(name)), None) = (comps.next(), comps.next()) else { return None };
    let ext = p.extension()?.to_string_lossy().to_ascii_lowercase();
    if !["jpg", "jpeg", "png", "webp", "wav", "mp3", "ogg", "opus"].contains(&ext.as_str()) {
        return None;
    }
    Some(dir.join(name))
}

/// Media de una frase en base64 (para subirla por `POST /call`).
pub fn encode_media(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub fn decode_media(data: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.decode(data.trim())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn read(uid: &str, lines: i64, chars: i64) -> RemoteRead {
        RemoteRead {
            uid: uid.into(),
            game_id: "g1".into(),
            game_name: "Novela".into(),
            start_ms: 1_700_000_000_000,
            end_ms: 1_700_000_600_000,
            lines,
            chars,
            active_ms: 300_000,
        }
    }

    fn days(c: &Connection) -> (i64, i64, i64) {
        c.query_row("SELECT COALESCE(SUM(lines),0), COALESCE(SUM(chars),0), COALESCE(SUM(phrases),0) FROM read_days", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap()
    }

    #[test]
    fn client_reads_go_once_and_grow() {
        let db = Db::open_memory().unwrap();
        db.with(|c| {
            // Cliente: una sesion con lineas y otra vacia; solo sube la primera, y vuelve a subir si crece.
            let id = rs::session_start(c, "g1", "Novela", 1000, "")?;
            let mut t = rs::Tracker::new(1000);
            t.push("こんにちは、世界。", 2000);
            rs::session_update(c, id, 2000, &t)?;
            rs::session_start(c, "g1", "Novela", 5000, "")?;
            let pend = unsynced_reads(c, 10)?;
            assert_eq!(pend.len(), 1);
            mark_reads_synced(c, &[pend[0].uid.clone()], 3000)?;
            assert!(unsynced_reads(c, 10)?.is_empty());
            t.push("次の行です。", 3000);
            rs::session_update(c, id, 3000, &t)?;
            assert_eq!(unsynced_reads(c, 10)?.len(), 1);
            anyhow::Ok(())
        })
        .unwrap();

        let host = Db::open_memory().unwrap();
        host.with(|c| {
            assert!(put_remote_read(c, "pc1", &read("u1", 10, 100), 0)?);
            assert_eq!(days(c), (10, 100, 0));
            // otra vez la misma: no suma; ha crecido: suma solo lo nuevo
            assert!(put_remote_read(c, "pc1", &read("u1", 10, 100), 0)?);
            assert!(put_remote_read(c, "pc1", &read("u1", 15, 160), 0)?);
            assert_eq!(days(c), (15, 160, 0));
            // el mismo uid desde otro equipo no vale
            assert!(!put_remote_read(c, "pc2", &read("u1", 99, 999), 0)?);
            assert_eq!(days(c), (15, 160, 0));
            let dev: String = c.query_row("SELECT device_id FROM read_sessions WHERE uid = 'u1'", [], |r| r.get(0))?;
            assert_eq!(dev, "pc1");
            anyhow::Ok(())
        })
        .unwrap();
    }

    #[test]
    fn phrases_merge_and_count_once() {
        let host = Db::open_memory().unwrap();
        host.with(|c| {
            let p = Phrase { game_id: "g1".into(), text: "猫だ".into(), created_at: 1_700_000_000_000, starred: true, ..Default::default() };
            assert!(put_remote_phrase(c, p.clone(), 0)?);
            // en el host se traduce; luego llega otra vez del cliente con mas vistas y audio
            let id = crate::jp::phrases::phrase_id("g1", "猫だ");
            let mut h = dbd::phrase_get(c, "g1", &id)?.unwrap();
            h.translation = Some("Es un gato".into());
            dbd::phrase_save(c, &h)?;
            let again = Phrase { seen: 4, audio: Some("audio-x.wav".into()), translation: Some("otra".into()), starred: false, ..p };
            assert!(!put_remote_phrase(c, again, 0)?);
            let h = dbd::phrase_get(c, "g1", &id)?.unwrap();
            assert_eq!((h.seen, h.translation.as_deref(), h.audio.as_deref(), h.starred), (4, Some("Es un gato"), Some("audio-x.wav"), true));
            assert_eq!(days(c).2, 1);
            // en el cliente: guardar la vuelve a dejar pendiente
            assert_eq!(unsynced_phrases(c, 10)?.len(), 1);
            mark_phrases_synced(c, &[("g1".into(), id.clone())], 1)?;
            assert!(unsynced_phrases(c, 10)?.is_empty());
            dbd::phrase_save(c, &h)?;
            assert_eq!(unsynced_phrases(c, 10)?.len(), 1);
            anyhow::Ok(())
        })
        .unwrap();
    }

    #[test]
    fn media_paths_are_plain_names() {
        let d = Path::new("/m");
        assert_eq!(media_path(d, "shot-1.jpg"), Some(d.join("shot-1.jpg")));
        assert!(media_path(d, "../x.jpg").is_none());
        assert!(media_path(d, "a/b.jpg").is_none());
        assert!(media_path(d, "x.exe").is_none());
        assert!(media_path(d, "C:/x.jpg").is_none());
    }
}
