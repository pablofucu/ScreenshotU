//! Login con PKCE (S256) contra la cuenta de Nintendo: `state`, `code_verifier`, `code_challenge`,
//! URL de login y lectura de la URL de vuelta (`session_token_code`).

use anyhow::{anyhow, bail, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

use super::{AUTHORIZE_URL, HISTORY_CLIENT_ID, HISTORY_REDIRECT_URI, HISTORY_SCOPE};

/// Datos de un intento de login. `state` y `verifier` se guardan hasta que vuelve la redireccion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pkce {
    pub state: String,
    pub verifier: String,
}

fn random_b64(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::getrandom(&mut buf).expect("sin fuente de aleatoriedad del sistema");
    URL_SAFE_NO_PAD.encode(buf)
}

impl Pkce {
    pub fn new() -> Self {
        // 36 bytes -> 48 caracteres para el state; 32 bytes -> 43 caracteres para el verifier (RFC 7636).
        Pkce { state: random_b64(36), verifier: random_b64(32) }
    }

    pub fn challenge(&self) -> String {
        challenge_s256(&self.verifier)
    }

    /// URL que se abre en la ventana de login.
    pub fn login_url(&self) -> String {
        let challenge = self.challenge();
        let params = [
            ("state", self.state.as_str()),
            ("redirect_uri", HISTORY_REDIRECT_URI),
            ("client_id", HISTORY_CLIENT_ID),
            ("scope", HISTORY_SCOPE),
            ("response_type", "session_token_code"),
            ("session_token_code_challenge", challenge.as_str()),
            ("session_token_code_challenge_method", "S256"),
            ("theme", "login_form"),
        ];
        let q: Vec<String> = params.iter().map(|(k, v)| format!("{k}={}", percent_encode(v))).collect();
        format!("{AUTHORIZE_URL}?{}", q.join("&"))
    }

    /// Lee la URL de vuelta (`npf…://auth#session_token_code=…&state=…`), comprueba el `state` y
    /// devuelve el `session_token_code`.
    pub fn parse_redirect(&self, url: &str) -> Result<String> {
        let url = url.trim();
        let Some(rest) = url.strip_prefix(HISTORY_REDIRECT_URI) else {
            bail!("la URL no es la redireccion de Nintendo");
        };
        // Los datos van en el fragmento (#) o, segun el navegador, en la consulta (?).
        let params = rest.trim_start_matches(['#', '?', '/']);
        let (mut code, mut state, mut error) = (None, None, None);
        for pair in params.split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let v = percent_decode(v);
            match k {
                "session_token_code" => code = Some(v),
                "state" => state = Some(v),
                "error" => error = Some(v),
                _ => {}
            }
        }
        if let Some(e) = error {
            bail!("Nintendo devolvio un error de login: {e}");
        }
        if state.as_deref() != Some(self.state.as_str()) {
            bail!("el `state` no coincide: login rechazado");
        }
        code.filter(|c| !c.is_empty()).ok_or_else(|| anyhow!("la URL no trae `session_token_code`"))
    }
}

impl Default for Pkce {
    fn default() -> Self {
        Self::new()
    }
}

/// `BASE64URL(SHA256(verifier))` sin relleno.
pub fn challenge_s256(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() + 0 {
            if let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_matches_rfc7636_example() {
        // Ejemplo del apendice B del RFC 7636.
        assert_eq!(
            challenge_s256("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn generated_values_are_valid_and_different() {
        let a = Pkce::new();
        let b = Pkce::new();
        assert_eq!(a.verifier.len(), 43);
        assert_eq!(a.state.len(), 48);
        assert_ne!(a.verifier, b.verifier);
        assert_ne!(a.state, b.state);
    }

    #[test]
    fn login_url_has_everything() {
        let p = Pkce { state: "st".into(), verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into() };
        let u = p.login_url();
        assert!(u.starts_with("https://accounts.nintendo.com/connect/1.0.0/authorize?"));
        assert!(u.contains("client_id=5c38e31cd085304b"));
        assert!(u.contains("redirect_uri=npf5c38e31cd085304b%3A%2F%2Fauth"));
        assert!(u.contains("scope=openid%20user%20user.mii%20user.email%20user.links%5B%5D.id"));
        assert!(u.contains("session_token_code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"));
        assert!(u.contains("session_token_code_challenge_method=S256"));
        assert!(u.contains("response_type=session_token_code"));
        assert!(u.contains("state=st&"));
    }

    #[test]
    fn redirect_is_parsed_and_state_checked() {
        let p = Pkce { state: "abc".into(), verifier: "v".into() };
        let ok = "npf5c38e31cd085304b://auth#session_state=zz&session_token_code=eyJ.a%2Bb&state=abc";
        assert_eq!(p.parse_redirect(ok).unwrap(), "eyJ.a+b");
        assert!(p.parse_redirect("npf5c38e31cd085304b://auth#session_token_code=x&state=otro").is_err());
        assert!(p.parse_redirect("npf5c38e31cd085304b://auth#state=abc").is_err());
        assert!(p.parse_redirect("npf5c38e31cd085304b://auth#error=access_denied&state=abc").is_err());
        assert!(p.parse_redirect("https://example.com/#session_token_code=x&state=abc").is_err());
    }
}
