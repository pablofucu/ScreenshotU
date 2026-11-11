//! Tiempo de juego de Nintendo Switch: logica pura (sin red ni Tauri).
//!
//! Aqui viven las constantes de las APIs de Nintendo (un solo sitio), la normalizacion de ids y
//! nombres de titulo, y el login PKCE ([`pkce`]). Ver `docs/archivo/plan-switch*.md` y `docs/referencia/nintendo-api.md`.

pub mod import;
pub mod link;
pub mod merge;
pub mod models;
pub mod pkce;
pub mod store;

/// Espera antes de la primera sincronizacion tras arrancar la app.
pub const FIRST_SYNC_DELAY: std::time::Duration = std::time::Duration::from_secs(60);

/// Espera hasta la proxima sincronizacion: 30 min sin fallos; con fallos seguidos 1, 2, 4, 8, 16 y 30 min (tope).
pub fn next_delay(fail_streak: u32) -> std::time::Duration {
    let mins = if fail_streak == 0 { 30 } else { (1u64 << (fail_streak - 1).min(5)).min(30) };
    std::time::Duration::from_secs(mins * 60)
}

/// Id de la plataforma Switch de la biblioteca (las fichas importadas solo viven aqui).
pub const SWITCH_PLATFORM: &str = "switch";

/// `client_id` de la app Nintendo Switch Online usado para el historial de juego.
pub const HISTORY_CLIENT_ID: &str = "5c38e31cd085304b";
/// Redireccion registrada para ese `client_id` (esquema propio: la webview la intercepta).
pub const HISTORY_REDIRECT_URI: &str = "npf5c38e31cd085304b://auth";
pub const HISTORY_SCOPE: &str = "openid user user.mii user.email user.links[].id";
pub const AUTHORIZE_URL: &str = "https://accounts.nintendo.com/connect/1.0.0/authorize";
pub const SESSION_TOKEN_URL: &str = "https://accounts.nintendo.com/connect/1.0.0/api/session_token";
pub const TOKEN_URL: &str = "https://accounts.nintendo.com/connect/1.0.0/api/token";
pub const PLAY_HISTORIES_URL: &str = "https://app-api.znej.nintendo.com/api/v2.0/users/me/play_histories";
/// Cabeceras de las peticiones de login (`session_token` y `token`).
pub const LOGIN_USER_AGENT: &str = "com.nintendo.znej/1.13.0 (Android/7.1.2)";
pub const LOGIN_PLATFORM: &str = "Android";
pub const LOGIN_PRODUCT_VERSION: &str = "2.5.0";
/// Cabeceras del historial. `Gentry-Locale` es obligatoria (sin ella, 400) y fija el idioma de los nombres.
pub const HISTORY_USER_AGENT: &str = "com.nintendo.znej/3.0.3 (iOS/26.0.1)";
pub const HISTORY_LOCALE: &str = "es-ES";

/// Mascara que quita los bits de actualizacion (`...800`) y DLC (`...1xxx`) de un id de titulo.
const TITLE_BASE_MASK: u64 = !0x1FFF;

/// Normaliza un id de titulo al id base: 16 hexadecimales en mayusculas. No supone el prefijo `0100`.
/// Devuelve `None` si no son 16 hexadecimales.
pub fn normalize_title_id(id: &str) -> Option<String> {
    let id = id.trim();
    if id.len() != 16 {
        return None;
    }
    let n = u64::from_str_radix(id, 16).ok()?;
    Some(format!("{:016X}", n & TITLE_BASE_MASK))
}

/// Nombre normalizado para emparejar un titulo de Nintendo con una ficha: minusculas, sin ™ ® ©,
/// sin `:` ni `-` y con los espacios colapsados.
pub fn normalize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '\u{2122}' | '\u{00AE}' | '\u{00A9}'))
        .map(|c| if matches!(c, ':' | '-' | '\u{2013}' | '\u{2014}') { ' ' } else { c })
        .flat_map(|c| c.to_lowercase())
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Nombre sin ™ ® © y sin el sufijo «Nintendo Switch 2 Edition» / «Switch 2 Edition» (con `–`, `-` o `:` delante).
/// Sirve para unir las versiones de un mismo juego en una sola ficha.
pub fn base_name(name: &str) -> String {
    let s: String = name.chars().filter(|c| !matches!(c, '\u{2122}' | '\u{00AE}' | '\u{00A9}')).collect();
    let mut s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = s.to_lowercase();
    for suffix in ["nintendo switch 2 edition", "switch 2 edition"] {
        if lower.ends_with(suffix) {
            let keep = s.chars().count().saturating_sub(suffix.chars().count());
            s = s.chars().take(keep).collect();
            break;
        }
    }
    s.trim_end_matches(|c: char| c.is_whitespace() || matches!(c, '\u{2013}' | '\u{2014}' | '-' | ':')).to_string()
}

/// Clave normalizada del nombre base (ver [`base_name`]).
pub fn base_key(name: &str) -> String {
    crate::games::norm_key(&base_name(name))
}

/// Las demos llegan mezcladas con los juegos: se reconocen por la palabra «demo» en el nombre.
pub fn is_demo(name: &str) -> bool {
    let n: String = normalize_name(name).chars().map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' }).collect();
    n.split_whitespace().any(|w| w == "demo")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_ids_are_normalized_to_base() {
        assert_eq!(normalize_title_id("01007ef00011e000").as_deref(), Some("01007EF00011E000"));
        // Actualizacion (…800) y DLC (…1xxx) vuelven al id base.
        assert_eq!(normalize_title_id("01007EF00011E800").as_deref(), Some("01007EF00011E000"));
        assert_eq!(normalize_title_id("01007EF00011F001").as_deref(), Some("01007EF00011E000"));
        // Prefijo de Switch 2 sin suponer 0100.
        assert_eq!(normalize_title_id("0400abcd00012000").as_deref(), Some("0400ABCD00012000"));
        assert_eq!(normalize_title_id("123"), None);
        assert_eq!(normalize_title_id("zzzzzzzzzzzzzzzz"), None);
    }

    #[test]
    fn names_are_normalized() {
        assert_eq!(normalize_name("The Legend of Zelda™: Breath of the Wild"), "the legend of zelda breath of the wild");
        assert_eq!(normalize_name("  Mario  Kart® 8 – Deluxe "), "mario kart 8 deluxe");
        assert_eq!(normalize_name("Zelda - Tears"), normalize_name("Zelda: Tears"));
    }

    #[test]
    fn base_names_drop_marks_and_switch2_suffix() {
        assert_eq!(base_name("Mina the Hollower \u{2013} Nintendo Switch 2 Edition"), "Mina the Hollower");
        assert_eq!(base_name("Xenoblade Chronicles X: Definitive Edition \u{2013} Nintendo Switch 2 Edition"), "Xenoblade Chronicles X: Definitive Edition");
        assert_eq!(base_name("Super Mario\u{2122} Party: Switch 2 Edition"), "Super Mario Party");
        assert_eq!(base_name("Hades"), "Hades");
        assert!(is_demo("Yoshi and the Mysterious Book Demo"));
        assert_ne!(base_key("Yoshi and the Mysterious Book Demo"), base_key("Yoshi and the Mysterious Book"));
    }

    #[test]
    fn retry_delay_backs_off_and_caps() {
        let m = |n| next_delay(n).as_secs() / 60;
        assert_eq!((m(0), m(1), m(2), m(3), m(5), m(6), m(10)), (30, 1, 2, 4, 16, 30, 30));
    }

    #[test]
    fn demos_are_detected() {
        assert!(is_demo("Hades Demo"));
        assert!(is_demo("Super Mario Bros. Wonder (Demo)"));
        assert!(!is_demo("Demonschool"));
    }
}
