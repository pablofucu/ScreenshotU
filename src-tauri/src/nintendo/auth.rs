#![allow(dead_code)] // connect/disconnect/is_connected los usa la ventana de login (SW-15)
//! Autenticacion con la cuenta de Nintendo: canjea el `session_token_code` del login por un
//! `session_token` (que se guarda en el Administrador de credenciales) y pide `access_token`s de
//! corta vida, que solo viven en memoria y se renuevan solos.

use std::time::{Duration, Instant};

use serde::Deserialize;
use visor_core::nintendo::pkce::Pkce;
use visor_core::nintendo::{
    HISTORY_CLIENT_ID, LOGIN_PLATFORM, LOGIN_PRODUCT_VERSION, LOGIN_USER_AGENT, SESSION_TOKEN_URL, TOKEN_URL,
};

use super::secrets::{self, Secret};

/// Por que fallo una operacion de cuenta. La interfaz distingue «hay que volver a conectar» de
/// «prueba mas tarde».
#[derive(Debug)]
pub enum AuthError {
    /// No hay cuenta conectada.
    NotConnected,
    /// Nintendo rechazo el token (caducado o revocado): hay que volver a iniciar sesion.
    Revoked(String),
    /// Sin red, servidor caido, etc. Se reintenta mas tarde.
    Transient(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::NotConnected => write!(f, "No hay ninguna cuenta de Nintendo conectada"),
            AuthError::Revoked(m) => write!(f, "Nintendo ha cerrado la sesion, hay que volver a conectar la cuenta ({m})"),
            AuthError::Transient(m) => write!(f, "No se pudo hablar con Nintendo: {m}"),
        }
    }
}

impl std::error::Error for AuthError {}

#[derive(Debug, Deserialize)]
struct SessionTokenResponse {
    session_token: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// Token de acceso en memoria y cuando caduca.
#[derive(Debug, Clone)]
pub struct AccessToken {
    pub access_token: String,
    pub id_token: Option<String>,
    expires_at: Instant,
}

impl AccessToken {
    /// Vigente con un margen de 60 s.
    pub fn is_fresh(&self) -> bool {
        Instant::now() + Duration::from_secs(60) < self.expires_at
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build()
}

fn classify(e: ureq::Error) -> AuthError {
    match e {
        // 400/401/403 en estos endpoints significan token invalido, caducado o revocado.
        ureq::Error::Status(code, resp) if matches!(code, 400 | 401 | 403) => {
            let body = resp.into_string().unwrap_or_default();
            AuthError::Revoked(format!("HTTP {code} {}", body.chars().take(200).collect::<String>()))
        }
        ureq::Error::Status(code, _) => AuthError::Transient(format!("HTTP {code}")),
        ureq::Error::Transport(t) => AuthError::Transient(t.to_string()),
    }
}

fn parse_session_token(body: &str) -> Result<String, AuthError> {
    serde_json::from_str::<SessionTokenResponse>(body)
        .map(|r| r.session_token)
        .map_err(|e| AuthError::Transient(format!("respuesta de session_token inesperada: {e}")))
}

fn parse_token(body: &str) -> Result<AccessToken, AuthError> {
    let r: TokenResponse =
        serde_json::from_str(body).map_err(|e| AuthError::Transient(format!("respuesta de token inesperada: {e}")))?;
    Ok(AccessToken {
        access_token: r.access_token,
        id_token: r.id_token,
        expires_at: Instant::now() + Duration::from_secs(r.expires_in.unwrap_or(900)),
    })
}

/// Canjea el `session_token_code` (de un solo uso, caduca a los 10 min) por el `session_token`.
fn exchange_code(code: &str, verifier: &str) -> Result<String, AuthError> {
    let body = agent()
        .post(SESSION_TOKEN_URL)
        .set("User-Agent", LOGIN_USER_AGENT)
        .set("X-Platform", LOGIN_PLATFORM)
        .set("X-ProductVersion", LOGIN_PRODUCT_VERSION)
        .send_form(&[
            ("client_id", HISTORY_CLIENT_ID),
            ("session_token_code", code),
            ("session_token_code_verifier", verifier),
        ])
        .map_err(classify)?
        .into_string()
        .map_err(|e| AuthError::Transient(e.to_string()))?;
    parse_session_token(&body)
}

/// Pide un `access_token` con el `session_token`.
fn request_access(session_token: &str) -> Result<AccessToken, AuthError> {
    let body = agent()
        .post(TOKEN_URL)
        .set("User-Agent", LOGIN_USER_AGENT)
        .set("X-Platform", LOGIN_PLATFORM)
        .set("X-ProductVersion", LOGIN_PRODUCT_VERSION)
        .send_json(serde_json::json!({
            "client_id": HISTORY_CLIENT_ID,
            "session_token": session_token,
            "grant_type": "urn:ietf:params:oauth:grant-type:jwt-bearer-session-token",
        }))
        .map_err(classify)?
        .into_string()
        .map_err(|e| AuthError::Transient(e.to_string()))?;
    parse_token(&body)
}

/// Termina el login: valida la URL de vuelta, canjea el codigo y guarda el `session_token`.
/// Devuelve ya un `access_token` para empezar a sincronizar sin otra peticion.
pub fn connect(pkce: &Pkce, redirect_url: &str) -> Result<AccessToken, AuthError> {
    let code = pkce.parse_redirect(redirect_url).map_err(|e| AuthError::Revoked(e.to_string()))?;
    let session = exchange_code(&code, &pkce.verifier)?;
    let access = request_access(&session)?;
    secrets::set(Secret::History, &session).map_err(|e| AuthError::Transient(e.to_string()))?;
    Ok(access)
}

/// Hay un `session_token` guardado (no comprueba que siga siendo valido).
pub fn is_connected() -> bool {
    matches!(secrets::get(Secret::History), Ok(Some(_)))
}

/// Olvida la cuenta: borra el `session_token`.
pub fn disconnect() -> anyhow::Result<()> {
    secrets::delete(Secret::History)
}

/// Mantiene el `access_token` en memoria y lo renueva cuando caduca.
#[derive(Debug, Default)]
pub struct Session {
    cached: Option<AccessToken>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// `access_token` vigente, renovandolo si hace falta. Si Nintendo revoca el `session_token`
    /// se borra del llavero para que la interfaz pida volver a conectar.
    pub fn access(&mut self) -> Result<AccessToken, AuthError> {
        if let Some(t) = self.cached.as_ref().filter(|t| t.is_fresh()) {
            return Ok(t.clone());
        }
        let session = secrets::get(Secret::History)
            .map_err(|e| AuthError::Transient(e.to_string()))?
            .ok_or(AuthError::NotConnected)?;
        match request_access(&session) {
            Ok(t) => {
                self.cached = Some(t.clone());
                Ok(t)
            }
            Err(AuthError::Revoked(m)) => {
                self.cached = None;
                let _ = secrets::delete(Secret::History);
                Err(AuthError::Revoked(m))
            }
            Err(e) => Err(e),
        }
    }

    /// Descarta el `access_token` en memoria (p. ej. tras un 401 del historial).
    pub fn invalidate(&mut self) {
        self.cached = None;
    }

    /// Instala un `access_token` ya obtenido (el que devuelve [`connect`]).
    pub fn set(&mut self, t: AccessToken) {
        self.cached = Some(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_token_response_is_parsed() {
        assert_eq!(parse_session_token(r#"{"code":"x","session_token":"abc.def"}"#).unwrap(), "abc.def");
        assert!(matches!(parse_session_token(r#"{"error":"invalid_grant"}"#), Err(AuthError::Transient(_))));
    }

    #[test]
    fn token_response_is_parsed_with_and_without_expiry() {
        let t = parse_token(r#"{"access_token":"A","id_token":"I","expires_in":900,"token_type":"Bearer"}"#).unwrap();
        assert_eq!((t.access_token.as_str(), t.id_token.as_deref()), ("A", Some("I")));
        assert!(t.is_fresh());
        // Caduca ya (o casi): dentro del margen de 60 s no se considera vigente.
        let short = parse_token(r#"{"access_token":"A","expires_in":30}"#).unwrap();
        assert!(!short.is_fresh());
        // Sin `expires_in` se asumen 15 minutos.
        assert!(parse_token(r#"{"access_token":"A"}"#).unwrap().is_fresh());
        assert!(parse_token("{}").is_err());
    }

    #[test]
    fn connect_rejects_a_foreign_redirect_without_network() {
        let p = Pkce::new();
        let err = connect(&p, "https://example.com/?session_token_code=x").unwrap_err();
        assert!(matches!(err, AuthError::Revoked(_)));
        let err = connect(&p, "npf5c38e31cd085304b://auth#session_token_code=x&state=otro").unwrap_err();
        assert!(matches!(err, AuthError::Revoked(_)), "state distinto");
    }
}
