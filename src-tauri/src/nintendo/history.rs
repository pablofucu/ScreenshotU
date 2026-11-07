//! Cliente del historial de juego de Nintendo (`play_histories`).

use std::time::Duration;

use visor_core::nintendo::models::{parse_play_histories, PlayHistories};
use visor_core::nintendo::{HISTORY_LOCALE, HISTORY_USER_AGENT, PLAY_HISTORIES_URL};

use super::auth::{AuthError, Session};

/// Por que fallo una sincronizacion.
#[derive(Debug)]
pub enum SyncError {
    NotConnected,
    /// La sesion de Nintendo ya no vale: hay que volver a conectar.
    Expired(String),
    /// Sin red o servidor caido: se reintenta mas tarde.
    Transient(String),
    /// La respuesta no es la esperada: Nintendo ha cambiado la API.
    ApiChanged(String),
}

impl From<AuthError> for SyncError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::NotConnected => SyncError::NotConnected,
            AuthError::Revoked(m) => SyncError::Expired(m),
            AuthError::Transient(m) => SyncError::Transient(m),
        }
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::NotConnected => write!(f, "No hay ninguna cuenta de Nintendo conectada"),
            SyncError::Expired(m) => write!(f, "La sesion de Nintendo ha caducado, vuelve a conectar la cuenta ({m})"),
            SyncError::Transient(m) => write!(f, "No se pudo hablar con Nintendo: {m}"),
            SyncError::ApiChanged(m) => write!(f, "La API de Nintendo ha cambiado: {m}"),
        }
    }
}

impl std::error::Error for SyncError {}

enum Attempt {
    Ok(String),
    /// 401/403: probar con otro token.
    Unauthorized,
}

fn get(token: &str) -> Result<Attempt, SyncError> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build();
    let res = agent
        .get(PLAY_HISTORIES_URL)
        .set("User-Agent", HISTORY_USER_AGENT)
        .set("Accept", "application/json")
        .set("Gentry-Locale", HISTORY_LOCALE)
        .set("Authorization", &format!("Bearer {token}"))
        .call();
    match res {
        Ok(r) => r.into_string().map(Attempt::Ok).map_err(|e| SyncError::Transient(e.to_string())),
        Err(ureq::Error::Status(401 | 403, _)) => Ok(Attempt::Unauthorized),
        Err(ureq::Error::Status(400, _)) => Err(SyncError::ApiChanged("historial HTTP 400".into())),
        Err(ureq::Error::Status(code, _)) => Err(SyncError::Transient(format!("historial HTTP {code}"))),
        Err(ureq::Error::Transport(t)) => Err(SyncError::Transient(t.to_string())),
    }
}

/// Descarga el historial. Con 401/403 prueba el `id_token`, luego renueva el `access_token` y
/// reintenta una vez; si sigue sin valer es un fallo pasajero (no se borra el `session_token`:
/// eso lo decide `auth.rs`).
pub fn fetch(session: &mut Session) -> Result<PlayHistories, SyncError> {
    let tok = session.access()?;
    let mut body = get(&tok.access_token)?;
    if matches!(body, Attempt::Unauthorized) {
        if let Some(id) = tok.id_token.as_deref() {
            body = get(id)?;
        }
    }
    if matches!(body, Attempt::Unauthorized) {
        session.invalidate();
        let fresh = session.access()?;
        body = get(&fresh.access_token)?;
    }
    match body {
        Attempt::Ok(json) => parse_play_histories(&json).map_err(|e| SyncError::ApiChanged(e.to_string())),
        Attempt::Unauthorized => Err(SyncError::Transient("historial HTTP 401".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use visor_core::db::open_connection;
    use visor_core::nintendo::store;

    #[test]
    fn auth_errors_map_to_sync_errors() {
        assert!(matches!(SyncError::from(AuthError::NotConnected), SyncError::NotConnected));
        assert!(matches!(SyncError::from(AuthError::Revoked("x".into())), SyncError::Expired(_)));
        assert!(matches!(SyncError::from(AuthError::Transient("x".into())), SyncError::Transient(_)));
        assert!(SyncError::ApiChanged("x".into()).to_string().starts_with("La API de Nintendo ha cambiado"));
    }

    /// Con red y una cuenta ya conectada (usa el session token del llavero, sin escribirlo):
    /// `cargo test -p visor-capturas live_history -- --ignored --nocapture`. Solo imprime recuentos.
    #[test]
    #[ignore]
    fn live_history() {
        let mut s = Session::new();
        let ph = fetch(&mut s).expect("descarga del historial");
        let path = std::env::temp_dir().join(format!("visor-live-history-{}.db", std::process::id()));
        let c = open_connection(&path).unwrap();
        let saved = store::save_history(&c, &ph.title_upserts(), &ph.daily_rows(), 1).unwrap();
        println!("titulos={} dias={} nuevos={}", saved.titles, saved.days, saved.new.len());
        drop(c);
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", path.display()));
        }
        assert!(saved.titles > 0);
    }
}
