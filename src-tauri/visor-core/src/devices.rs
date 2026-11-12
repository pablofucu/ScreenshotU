//! Dispositivos emparejados con este equipo (solo se usa en el host): PCs cliente y tablets.
//!
//! Cada dispositivo tiene un id aleatorio, un nombre puesto por el usuario y un token propio, del que solo se
//! guarda el SHA-256. En las demas tablas, `device_id = ''` significa «el dueño de esta base».
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::remote::{new_id, new_token, sha256_hex};

/// Migracion 16: tabla de dispositivos y dispositivo/uid de cada partida y sesion de lectura.
pub const SCHEMA: &str = r#"
CREATE TABLE devices (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    kind         TEXT NOT NULL,
    token_sha256 TEXT NOT NULL UNIQUE,
    created_ms   INTEGER NOT NULL,
    last_seen_ms INTEGER NOT NULL DEFAULT 0,
    app_version  TEXT NOT NULL DEFAULT '',
    api          INTEGER NOT NULL DEFAULT 0,
    revoked      INTEGER NOT NULL DEFAULT 0
);
ALTER TABLE sessions ADD COLUMN device_id TEXT NOT NULL DEFAULT '';
ALTER TABLE sessions ADD COLUMN uid TEXT;
ALTER TABLE sessions ADD COLUMN synced_ms INTEGER;
CREATE UNIQUE INDEX sessions_uid ON sessions(uid) WHERE uid IS NOT NULL;
CREATE INDEX sessions_device ON sessions(device_id, start_ms);
ALTER TABLE read_sessions ADD COLUMN device_id TEXT NOT NULL DEFAULT '';
ALTER TABLE read_sessions ADD COLUMN uid TEXT;
"#;

/// `touch` solo escribe `last_seen_ms` si ha pasado al menos este tiempo desde la ultima vez.
const TOUCH_EVERY_MS: i64 = 60_000;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub created_ms: i64,
    pub last_seen_ms: i64,
    pub app_version: String,
    pub api: i64,
    pub revoked: bool,
    /// Id del cliente en Apollo (streaming emparejado), si lo hay.
    pub apollo_id: Option<String>,
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn clean_name(name: &str) -> Result<String> {
    let n: String = name.trim().chars().take(60).collect();
    if n.is_empty() {
        bail!("El nombre del dispositivo no puede estar vacío");
    }
    Ok(n)
}

fn row(r: &rusqlite::Row) -> rusqlite::Result<Device> {
    Ok(Device {
        id: r.get(0)?,
        name: r.get(1)?,
        kind: r.get(2)?,
        created_ms: r.get(3)?,
        last_seen_ms: r.get(4)?,
        app_version: r.get(5)?,
        api: r.get(6)?,
        revoked: r.get::<_, i64>(7)? != 0,
        apollo_id: r.get(8)?,
    })
}

const COLS: &str = "id, name, kind, created_ms, last_seen_ms, app_version, api, revoked, apollo_id";

/// Guarda (o quita, con `None`) el id del cliente en Apollo de un dispositivo.
pub fn set_apollo_id(c: &Connection, device_id: &str, apollo_id: Option<&str>) -> Result<()> {
    if c.execute("UPDATE devices SET apollo_id = ?2 WHERE id = ?1", params![device_id, apollo_id])? == 0 {
        bail!("Ese dispositivo no existe");
    }
    Ok(())
}

/// Id del cliente en Apollo de un dispositivo (`None` si no esta emparejado el streaming o no existe).
pub fn apollo_id(c: &Connection, device_id: &str) -> Result<Option<String>> {
    Ok(c.query_row("SELECT apollo_id FROM devices WHERE id = ?1", [device_id], |r| r.get(0)).optional()?.flatten())
}

/// Da de alta un dispositivo y devuelve `(id, token)`. El token solo se conoce aqui: se guarda su SHA-256.
pub fn create(c: &Connection, name: &str, kind: &str, version: &str, api: u32) -> Result<(String, String)> {
    let name = clean_name(name)?;
    let kind = if kind == "tablet" { "tablet" } else { "pc" };
    let id = new_id();
    let token = new_token();
    let now = now_ms();
    c.execute(
        "INSERT INTO devices (id, name, kind, token_sha256, created_ms, last_seen_ms, app_version, api)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7)",
        params![id, name, kind, sha256_hex(&token), now, version, api],
    )?;
    Ok((id, token))
}

/// Dispositivo activo (no revocado) con ese token.
pub fn by_token(c: &Connection, token: &str) -> Result<Option<Device>> {
    if token.is_empty() {
        return Ok(None);
    }
    let sql = format!("SELECT {COLS} FROM devices WHERE token_sha256 = ?1 AND revoked = 0");
    Ok(c.query_row(&sql, [sha256_hex(token)], row).optional()?)
}

pub fn get(c: &Connection, id: &str) -> Result<Option<Device>> {
    let sql = format!("SELECT {COLS} FROM devices WHERE id = ?1");
    Ok(c.query_row(&sql, [id], row).optional()?)
}

/// Todos los dispositivos, los activos primero y por ultima conexion.
pub fn list(c: &Connection) -> Result<Vec<Device>> {
    let sql = format!("SELECT {COLS} FROM devices ORDER BY revoked, last_seen_ms DESC, name");
    let mut st = c.prepare(&sql)?;
    let v = st.query_map([], row)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(v)
}

pub fn rename(c: &Connection, id: &str, name: &str) -> Result<()> {
    let name = clean_name(name)?;
    if c.execute("UPDATE devices SET name = ?2 WHERE id = ?1", params![id, name])? == 0 {
        bail!("Ese dispositivo no existe");
    }
    Ok(())
}

/// Quita un dispositivo: su token deja de valer. La fila se queda para que sus partidas sigan teniendo nombre.
pub fn revoke(c: &Connection, id: &str) -> Result<()> {
    c.execute("UPDATE devices SET revoked = 1 WHERE id = ?1", [id])?;
    Ok(())
}

/// Apunta que el dispositivo se ha conectado (como mucho una escritura por minuto, salvo cambio de version).
pub fn touch(c: &Connection, id: &str, version: &str, api: u32) -> Result<()> {
    let now = now_ms();
    c.execute(
        "UPDATE devices SET last_seen_ms = ?2,
                app_version = CASE WHEN ?3 = '' THEN app_version ELSE ?3 END,
                api = CASE WHEN ?4 = 0 THEN api ELSE ?4 END
         WHERE id = ?1 AND (last_seen_ms < ?2 - ?5
                OR (?3 <> '' AND app_version <> ?3) OR (?4 <> 0 AND api <> ?4))",
        params![id, now, version, api, TOUCH_EVERY_MS],
    )?;
    Ok(())
}

/// Nombres de todos los dispositivos (tambien los quitados), para etiquetar partidas.
pub fn names(c: &Connection) -> Result<std::collections::HashMap<String, String>> {
    let mut st = c.prepare("SELECT id, name FROM devices")?;
    let v = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn apollo_id_is_saved_and_cleared() {
        let d = tempfile::tempdir().unwrap();
        let db = Db::open_path(&d.path().join("visor.db")).unwrap();
        db.with(|c| {
            let (id, _) = create(c, "Tablet", "tablet", "1.0", 3).unwrap();
            assert_eq!(get(c, &id).unwrap().unwrap().apollo_id, None);
            set_apollo_id(c, &id, Some("uuid-1")).unwrap();
            assert_eq!(apollo_id(c, &id).unwrap().as_deref(), Some("uuid-1"));
            assert_eq!(list(c).unwrap()[0].apollo_id.as_deref(), Some("uuid-1"));
            set_apollo_id(c, &id, None).unwrap();
            assert_eq!(apollo_id(c, &id).unwrap(), None);
            assert!(set_apollo_id(c, "nada", Some("x")).is_err());
            assert_eq!(apollo_id(c, "nada").unwrap(), None);
        });
    }

    #[test]
    fn create_find_rename_revoke() {
        let d = tempfile::tempdir().unwrap();
        let db = Db::open_path(&d.path().join("visor.db")).unwrap();
        db.with(|c| {
            let (id, token) = create(c, "  Portátil ", "pc", "1.2.0", 2).unwrap();
            assert_eq!(token.len(), 64);
            let dev = by_token(c, &token).unwrap().unwrap();
            assert_eq!((dev.id.as_str(), dev.name.as_str(), dev.kind.as_str()), (id.as_str(), "Portátil", "pc"));
            // El token no se guarda en claro.
            let n: i64 = c.query_row("SELECT count(*) FROM devices WHERE token_sha256 = ?1", [&token], |r| r.get(0)).unwrap();
            assert_eq!(n, 0);
            assert!(by_token(c, "otro").unwrap().is_none());
            assert!(by_token(c, "").unwrap().is_none());
            rename(c, &id, "Sobremesa").unwrap();
            assert!(rename(c, &id, "  ").is_err());
            assert_eq!(get(c, &id).unwrap().unwrap().name, "Sobremesa");
            touch(c, &id, "1.3.0", 2).unwrap();
            assert_eq!(get(c, &id).unwrap().unwrap().app_version, "1.3.0");
            revoke(c, &id).unwrap();
            assert!(by_token(c, &token).unwrap().is_none());
            assert!(list(c).unwrap()[0].revoked);
            assert_eq!(names(c).unwrap()[&id], "Sobremesa");
        });
    }
}
