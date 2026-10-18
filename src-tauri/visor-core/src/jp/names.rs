//! Reconocimiento del personaje que habla: patrones en la linea, hilo de nombre y lista de personajes.
use super::{Character, NovelPrefs};
use std::time::{Duration, Instant};

/// Caracter permitido en un nombre: ni espacios ni signos de puntuacion (la `ー` si vale: `ルーシー`).
fn name_char_ok(c: char) -> bool {
    if c.is_whitespace() || c.is_ascii_punctuation() {
        return false;
    }
    !matches!(
        c,
        '、' | '。' | '，' | '．' | '・' | '！' | '？' | '…' | '「' | '」' | '『' | '』' | '（' | '）' | '【' | '】'
            | '〈' | '〉' | '《' | '》' | '〔' | '〕' | '：' | '；' | '“' | '”' | '‘' | '’' | '―' | '─' | '〜' | '～' | '＂' | '＇'
    )
}

/// Un nombre valido: de 1 a 10 caracteres sin signos ni espacios. Tambien se admite `？？？` (hablante
/// desconocido). Un nombre de 2+ caracteres que acabe en particula tipica (は が を に で へ) se rechaza:
/// es casi seguro narracion tipo `彼は「…」`.
fn valid_name(n: &str) -> bool {
    let chars: Vec<char> = n.chars().collect();
    if chars.is_empty() || chars.len() > 10 {
        return false;
    }
    if chars.iter().all(|c| *c == '？' || *c == '?') {
        return true;
    }
    if !chars.iter().all(|c| name_char_ok(*c)) {
        return false;
    }
    if chars.len() >= 2 && matches!(chars[chars.len() - 1], 'は' | 'が' | 'を' | 'に' | 'で' | 'へ') {
        return false;
    }
    true
}

/// Separa el nombre del personaje del dialogo. Patrones: `名前「…」`, `名前『…』`, `【名前】…`,
/// `名前：…` / `名前:…`. Devuelve `(Some(nombre), dialogo)`; el dialogo conserva sus comillas.
/// Si no hay patron, `(None, texto)`. Para `名前「…」` el texto debe terminar con la comilla de cierre.
pub fn split_name(text: &str) -> (Option<String>, String) {
    let t = text.trim();
    let chars: Vec<char> = t.chars().collect();
    if chars.is_empty() {
        return (None, text.to_string());
    }
    // 【名前】…
    if chars[0] == '【' {
        if let Some(end) = chars.iter().position(|c| *c == '】') {
            let name: String = chars[1..end].iter().collect();
            let rest: String = chars[end + 1..].iter().collect::<String>().trim().to_string();
            if valid_name(name.trim()) && !rest.is_empty() {
                return (Some(name.trim().to_string()), rest);
            }
        }
        return (None, text.to_string());
    }
    // 名前「…」 / 名前『…』 / 名前：…
    let limit = chars.len().min(11);
    for i in 1..limit {
        let c = chars[i];
        if !name_char_ok(chars[i - 1]) && !(chars[..i].iter().all(|c| *c == '？')) {
            break;
        }
        let kind = match c {
            '「' => Some('」'),
            '『' => Some('』'),
            '：' | ':' => Some(' '),
            _ => None,
        };
        if let Some(close) = kind {
            let name: String = chars[..i].iter().collect();
            let rest: String = chars[i..].iter().collect();
            if !valid_name(&name) {
                break;
            }
            if close == ' ' {
                let rest = chars[i + 1..].iter().collect::<String>().trim().to_string();
                if rest.is_empty() {
                    break;
                }
                return (Some(name), rest);
            }
            if rest.ends_with(close) && rest.chars().count() >= 2 {
                return (Some(name), rest);
            }
            break;
        }
    }
    (None, text.to_string())
}

/// Modo «hilo de nombre»: algunos motores sacan el nombre en otro hilo. Guarda el ultimo nombre recibido
/// de ese hilo y lo entrega (una sola vez) a la siguiente linea del hilo principal si no ha caducado.
#[derive(Debug, Clone)]
pub struct NameJoiner {
    name: Option<(String, Instant)>,
    ttl: Duration,
}

impl NameJoiner {
    /// `ttl`: segundos que dura un nombre sin usar (3-5 es razonable).
    pub fn new(ttl_secs: u64) -> Self {
        NameJoiner { name: None, ttl: Duration::from_secs(ttl_secs) }
    }

    /// Texto recibido del hilo de nombre. Se limpian espacios y corchetes `【】`; un texto vacio o de mas
    /// de 20 caracteres se ignora.
    pub fn set_name(&mut self, text: &str) {
        self.set_name_at(text, Instant::now());
    }

    pub fn set_name_at(&mut self, text: &str, now: Instant) {
        let n: String = text.chars().filter(|c| !c.is_whitespace() && !matches!(c, '【' | '】')).collect();
        if n.is_empty() || n.chars().count() > 20 {
            return;
        }
        self.name = Some((n, now));
    }

    /// Nombre para la siguiente linea del hilo principal (se consume). `None` si no hay o caduco.
    pub fn take(&mut self) -> Option<String> {
        self.take_at(Instant::now())
    }

    pub fn take_at(&mut self, now: Instant) -> Option<String> {
        let (n, at) = self.name.take()?;
        if now.saturating_duration_since(at) <= self.ttl {
            Some(n)
        } else {
            None
        }
    }

    /// Olvida el nombre guardado (p. ej. al cambiar el hilo de nombre).
    pub fn clear(&mut self) {
        self.name = None;
    }
}

/// Anade el personaje con `color: None` si no existe. Devuelve `true` si se anadio (hay que guardar prefs).
pub fn ensure_character(prefs: &mut NovelPrefs, name: &str) -> bool {
    if name.is_empty() || prefs.characters.iter().any(|c| c.name == name) {
        return false;
    }
    prefs.characters.push(Character { name: name.to_string(), color: None });
    true
}

/// Color `#rrggbb` del personaje, si tiene.
pub fn color_of(prefs: &NovelPrefs, name: &str) -> Option<String> {
    prefs.characters.iter().find(|c| c.name == name).and_then(|c| c.color.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(n: Option<&str>, d: &str) -> (Option<String>, String) {
        (n.map(|x| x.to_string()), d.to_string())
    }

    #[test]
    fn patrones() {
        assert_eq!(split_name("美雪「明日、駅まで一緒に行ってくれないかな」"), s(Some("美雪"), "「明日、駅まで一緒に行ってくれないかな」"));
        assert_eq!(split_name("美雪『ふふ』"), s(Some("美雪"), "『ふふ』"));
        assert_eq!(split_name("【美雪】明日、駅まで"), s(Some("美雪"), "明日、駅まで"));
        assert_eq!(split_name("美雪：明日、駅まで"), s(Some("美雪"), "明日、駅まで"));
        assert_eq!(split_name("Miyuki: hola"), s(Some("Miyuki"), "hola"));
        assert_eq!(split_name("ルーシー「うん」"), s(Some("ルーシー"), "「うん」"));
        assert_eq!(split_name("？？？「誰？」"), s(Some("？？？"), "「誰？」"));
    }

    #[test]
    fn narracion_no_se_toca() {
        let n = "窓の外では、雪が音もなく降り続いていた。";
        assert_eq!(split_name(n), s(None, n));
        assert_eq!(split_name("「こんにちは」"), s(None, "「こんにちは」"));
        // nombre demasiado largo
        let l = "とても長い名前すぎるひとの「うん」";
        assert_eq!(split_name(l), s(None, l));
        // partícula: narracion
        assert_eq!(split_name("彼は「うん」"), s(None, "彼は「うん」"));
        // no termina en cierre
        assert_eq!(split_name("美雪「うん」と言った"), s(None, "美雪「うん」と言った"));
        assert_eq!(split_name(""), s(None, ""));
    }

    #[test]
    fn joiner_caduca_y_se_consume() {
        let mut j = NameJoiner::new(3);
        let t0 = Instant::now();
        j.set_name_at(" 【美雪】 ", t0);
        assert_eq!(j.take_at(t0 + Duration::from_secs(1)), Some("美雪".into()));
        assert_eq!(j.take_at(t0 + Duration::from_secs(1)), None);
        j.set_name_at("健", t0);
        assert_eq!(j.take_at(t0 + Duration::from_secs(10)), None);
    }

    #[test]
    fn personajes() {
        let mut p = NovelPrefs::default();
        assert!(ensure_character(&mut p, "美雪"));
        assert!(!ensure_character(&mut p, "美雪"));
        assert_eq!(color_of(&p, "美雪"), None);
        p.characters[0].color = Some("#ff8800".into());
        assert_eq!(color_of(&p, "美雪"), Some("#ff8800".into()));
        assert_eq!(color_of(&p, "健"), None);
    }
}
