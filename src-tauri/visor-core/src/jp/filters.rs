//! Filtros de texto de las novelas: caracteres y frases repetidas, etiquetas, saltos de linea, regex y lineas que se
//! dibujan letra a letra.
//!
//! Los filtros de repeticion son una adaptacion de los «HOOK_去除重复…» de LunaTranslator
//! (`src/LunaTranslator/myutils/post.py`: `_2`, `_3`, `_3_2`, `_13EX`), con salvaguardas para no estropear
//! palabras japonesas legitimas (`ああ`, `どうどう`, `人々`, `……`).
use super::Filters;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Textos mas largos no pasan por los filtros de repeticion mas caros (cuadraticos o peores).
const HEAVY_MAX: usize = 2000;

/// Aplica los filtros de `f` a un texto bruto, en este orden: control, `join_lines`, caracteres repetidos,
/// frase repetida, segmentos repetidos, texto incremental, llaves, etiquetas y regex activas.
/// (`dedupe_lines` y `split_name` no son filtros de una sola linea: ver `LineDeduper` y `names::split_name`.)
pub fn apply(f: &Filters, text: &str) -> String {
    let mut out = text.to_string();
    if f.remove_control {
        out = remove_control(&out);
    }
    if f.join_lines {
        out = join_lines(&out);
    }
    if f.dedupe_chars {
        out = dedupe_chars(&out);
    }
    if f.dedupe_phrase {
        out = dedupe_phrase(&out);
    }
    if f.dedupe_segments {
        out = dedupe_segments(&out);
    }
    if f.dedupe_incremental {
        out = dedupe_incremental(&out);
    }
    if f.remove_braces {
        out = remove_braces(&out);
    }
    if f.remove_tags {
        out = remove_tags(&out);
    }
    for rule in f.regex.iter().filter(|r| r.on) {
        // regex invalida = se ignora
        if let Ok(re) = Regex::new(&rule.pattern) {
            out = re.replace_all(&out, rule.replace.as_str()).into_owned();
        }
    }
    out
}

/// Signos que en japones se escriben dobles a proposito: no cuentan para adivinar las repeticiones.
fn doubled_on_purpose(c: char) -> bool {
    matches!(c, '…' | '‥' | '―' | '—' | '─' | 'ー' | '～' | '〜' | '・' | '.' | '-' | '~')
}

/// Arregla texto con cada caracter repetido N veces, tipico de hooks de GetGlyphOutline (`明明日日` -> `明日`).
/// N se adivina como la longitud de racha mas frecuente (como `_2` de LunaTranslator en modo automatico) y los
/// caracteres que no siguen el patron se conservan (`「明明日日」` -> `「明日」`). Solo actua si hay al menos dos
/// rachas de longitud N y son mas que los caracteres sueltos, asi `ああ`, `ああああ` o `そう……ねえ` no se tocan.
pub fn dedupe_chars(text: &str) -> String {
    match undouble(text) {
        Some(s) => s,
        None => text.to_string(),
    }
}

/// `Some(texto desduplicado)` si el texto sigue el patron de caracteres repetidos N veces.
pub(crate) fn undouble(text: &str) -> Option<String> {
    let runs = runs(text);
    let mut stats: HashMap<usize, usize> = HashMap::new();
    for &(c, n) in &runs {
        if !c.is_whitespace() && !doubled_on_purpose(c) {
            *stats.entry(n).or_insert(0) += 1;
        }
    }
    let singles = stats.get(&1).copied().unwrap_or(0);
    // La longitud mas frecuente entre las de 2 o mas; en empate, la menor.
    let (n, count) = stats.iter().map(|(&n, &c)| (n, c)).filter(|&(n, _)| n >= 2).max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))?;
    if count < 2 || count <= singles {
        return None;
    }
    let out: String = runs.iter().flat_map(|&(c, len)| std::iter::repeat(c).take(if c.is_whitespace() { len } else { len.div_ceil(n) })).collect();
    Some(out)
}

/// Rachas de caracteres iguales consecutivos: `(caracter, longitud)`.
fn runs(text: &str) -> Vec<(char, usize)> {
    let mut v: Vec<(char, usize)> = Vec::new();
    for c in text.chars() {
        match v.last_mut() {
            Some((p, n)) if *p == c => *n += 1,
            _ => v.push((c, 1)),
        }
    }
    v
}

/// Frase entera repetida (`ABCDABCD` -> `ABCD`), tambien en lineas distintas (`ABCD\nABCD`) y cuando la salida
/// corta la ultima repeticion a medias (`ABCDABCDAB` -> `ABCD`). La unidad que se repite debe tener al menos 3
/// caracteres para no tocar palabras como `どうどう` o `ねえねえ`.
pub fn dedupe_phrase(text: &str) -> String {
    const MIN_UNIT: usize = 3;
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.len() >= 2 {
        // Lineas que son todas la misma frase (o el principio de ella): se queda la mas larga.
        let longest = *lines.iter().max_by_key(|l| l.chars().count()).unwrap();
        if lines.iter().all(|l| longest.starts_with(l)) {
            return dedupe_phrase(longest);
        }
    }
    let t = text.trim();
    let chars: Vec<char> = t.chars().collect();
    let len = chars.len();
    if len > HEAVY_MAX * 5 {
        return text.to_string();
    }
    // Periodo mas corto con al menos dos repeticiones completas; lo que sobra al final es una repeticion cortada.
    for unit in MIN_UNIT..=len / 2 {
        if (unit..len).all(|i| chars[i] == chars[i - unit]) {
            return chars[..unit].iter().collect();
        }
    }
    text.to_string()
}

/// Cada trozo repetido seguido varias veces se deja una vez: `S1S1S1S2S2S2` -> `S1S2` (`_3_2` de LunaTranslator).
/// Solo se consideran trozos de 2 o mas caracteres.
pub fn dedupe_segments(text: &str) -> String {
    let mut line: &[char] = &text.chars().collect::<Vec<_>>();
    if line.len() > HEAVY_MAX {
        return text.to_string();
    }
    let mut out: Vec<char> = Vec::with_capacity(line.len());
    while !line.is_empty() {
        // El trozo mas largo que se repite justo a continuacion.
        let found = (2..=line.len() / 2).rev().find(|&d| line[..d] == line[d..2 * d]);
        match found {
            Some(d) => {
                let unit = &line[..d];
                out.extend_from_slice(unit);
                while line.len() >= d && &line[..d] == unit {
                    line = &line[d..];
                }
            }
            None => {
                out.push(line[0]);
                line = &line[1..];
            }
        }
    }
    out.into_iter().collect()
}

/// Texto que se redibuja creciendo dentro de una misma salida: `AABABCABCD` -> `ABCD` (`_13EX` de LunaTranslator).
/// Un texto normal queda igual.
pub fn dedupe_incremental(text: &str) -> String {
    let mut line: Vec<char> = text.chars().collect();
    if line.len() > HEAVY_MAX {
        return text.to_string();
    }
    let mut saves: Vec<Vec<char>> = Vec::new();
    while !line.is_empty() {
        // El sufijo mas largo S tal que la linea acaba en ...S[..1] S[..2] ... S (todos sus prefijos seguidos).
        let mut best = &line[line.len() - 1..];
        for i in 0..line.len() {
            let s = &line[i..];
            if s.len() * (s.len() + 1) / 2 > line.len() {
                continue;
            }
            let mut rest: &[char] = &line;
            let ok = (1..=s.len()).rev().all(|k| {
                let ok = rest.ends_with(&s[..k]);
                if ok {
                    rest = &rest[..rest.len() - k];
                }
                ok
            });
            if ok {
                best = s;
                break;
            }
        }
        let best = best.to_vec();
        let cut = best.len() * (best.len() + 1) / 2;
        line.truncate(line.len().saturating_sub(cut));
        saves.push(best);
    }
    saves.into_iter().rev().flatten().collect()
}

/// Quita caracteres de control (deja `\n` y `\r`, de eso se encarga `join_lines`).
pub fn remove_control(text: &str) -> String {
    text.chars().filter(|&c| !matches!(c as u32, 0x0..=0x9 | 0xB..=0xC | 0xE..=0x1F | 0x7F..=0x9F)).collect()
}

/// Quita etiquetas `<...>`.
pub fn remove_tags(text: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<[^<>]*>").unwrap()).replace_all(text, "").into_owned()
}

/// Llaves de los motores: `{rb}漢字{/rb}` -> `漢字`, `{漢字/かんじ}` o `{漢字:かんじ}` -> `漢字`, el resto `{...}` fuera.
pub fn remove_braces(text: &str) -> String {
    static RE: OnceLock<[Regex; 2]> = OnceLock::new();
    let [pair, ruby] = RE.get_or_init(|| [Regex::new(r"\{(\w+)[^}]*\}(.*?)\{/(\w+)\}").unwrap(), Regex::new(r"\{([^}]*?)[:/][^}]*\}").unwrap()]);
    // `regex` no admite referencias hacia atras: se comprueba a mano que la etiqueta de cierre es la de apertura.
    let out = pair.replace_all(text, |c: &regex::Captures| if c[1] == c[3] { c[2].to_string() } else { c[0].to_string() });
    let out = ruby.replace_all(&out, "$1");
    static REST: OnceLock<Regex> = OnceLock::new();
    REST.get_or_init(|| Regex::new(r"\{[^}]*\}").unwrap()).replace_all(&out, "").into_owned()
}

/// Quita saltos de linea y espacios de ancho completo de relleno; deja una sola linea.
pub fn join_lines(text: &str) -> String {
    text.chars().filter(|c| !matches!(c, '\r' | '\n' | '\u{3000}')).collect::<String>().trim().to_string()
}

/// `text` (de 3 o mas caracteres) repetido y cortado da `full`: `is_period_of("ABCA", "ABC")`.
fn is_period_of(full: &str, text: &str) -> bool {
    let f: Vec<char> = full.chars().collect();
    let u = text.chars().count();
    u >= 3 && f.len() > u && (u..f.len()).all(|i| f[i] == f[i - u])
}

/// Descarta lineas repetidas y el texto que se dibuja letra a letra.
///
/// Uso: por cada texto recibido del hilo, `push(texto)`; si devuelve `Some(linea)`, esa linea ya esta
/// completa (la anterior que estaba pendiente). La linea nueva queda pendiente porque puede seguir
/// creciendo (`明日`, `明日、駅`, `明日、駅まで`...). La integracion debe llamar a `flush()` cuando la pendiente
/// lleve un rato sin cambiar (`pending_age()`, p. ej. 150-300 ms), aunque sigan llegando repeticiones de ella.
/// Una linea igual a la ultima emitida o a la pendiente se descarta; una que sea prefijo de la pendiente tambien.
#[derive(Debug, Default, Clone)]
pub struct LineDeduper {
    pending: Option<String>,
    changed_at: Option<Instant>,
    last: Option<String>,
}

impl LineDeduper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra un texto. Devuelve la linea pendiente anterior si el texto nuevo ya no la continua.
    pub fn push(&mut self, text: &str) -> Option<String> {
        if text.is_empty() {
            return None;
        }
        if let Some(p) = &self.pending {
            // La pendiente era la frase con un trozo de su repeticion (`ABCA`) y llega la frase limpia (`ABC`).
            if p.starts_with(text) && p != text && is_period_of(p, text) {
                self.pending = Some(text.to_string());
                self.changed_at = Some(Instant::now());
                return None;
            }
            if p.contains(text) {
                return None;
            }
            if text.starts_with(p.as_str()) {
                self.pending = Some(text.to_string());
                self.changed_at = Some(Instant::now());
                return None;
            }
        } else if self.last.as_deref() == Some(text) {
            return None;
        }
        let done = self.pending.take();
        if let Some(d) = &done {
            self.last = Some(d.clone());
        }
        self.pending = Some(text.to_string());
        self.changed_at = Some(Instant::now());
        done
    }

    /// Emite la linea pendiente (si la hay). Llamar tras un tiempo sin que cambie.
    pub fn flush(&mut self) -> Option<String> {
        let p = self.pending.take();
        self.changed_at = None;
        if let Some(l) = &p {
            self.last = Some(l.clone());
        }
        p
    }

    /// Hay una linea esperando a ser emitida (para saber si programar el flush).
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Tiempo desde que la linea pendiente cambio por ultima vez (`None` si no hay pendiente).
    pub fn pending_age(&self) -> Option<Duration> {
        self.pending.as_ref().and(self.changed_at).map(|t| t.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jp::RegexRule;

    #[test]
    fn dedupe_basico() {
        assert_eq!(dedupe_chars("明明日日、、駅駅"), "明日、駅");
        assert_eq!(dedupe_chars("明明明日日日、、、駅駅駅"), "明日、駅");
        assert_eq!(dedupe_chars("明明明明日日日日"), "明日");
    }

    #[test]
    fn dedupe_conserva_sueltos() {
        // signos o saltos que el hook no duplica
        assert_eq!(dedupe_chars("「「明明日日、、駅駅ままでで」"), "「明日、駅まで」");
        assert_eq!(dedupe_chars("明明日日\n駅駅ままでで"), "明日\n駅まで");
        // «……» legitimo duplicado por el hook: …… x2
        assert_eq!(dedupe_chars("そそうう…………ねねええ"), "そう……ねえ");
    }

    #[test]
    fn dedupe_respeta_palabras_legitimas() {
        assert_eq!(dedupe_chars("ああ"), "ああ");
        assert_eq!(dedupe_chars("ああああ"), "ああああ");
        assert_eq!(dedupe_chars("どうどう"), "どうどう");
        assert_eq!(dedupe_chars("人々"), "人々");
        assert_eq!(dedupe_chars("明日、駅まで"), "明日、駅まで");
        assert_eq!(dedupe_chars("そう……ねえ――"), "そう……ねえ――");
        assert_eq!(dedupe_chars("ああ、いいよ。すっごく"), "ああ、いいよ。すっごく");
        // solo una parte duplicada: no se toca
        assert_eq!(dedupe_chars("明明日、駅"), "明明日、駅");
    }

    #[test]
    fn frase_repetida() {
        assert_eq!(dedupe_phrase("今日は晴れ。今日は晴れ。"), "今日は晴れ。");
        assert_eq!(dedupe_phrase("今日は晴れ。今日は晴れ。今日は晴れ。"), "今日は晴れ。");
        assert_eq!(dedupe_phrase("今日は晴れ。\n今日は晴れ。\n"), "今日は晴れ。");
        assert_eq!(dedupe_phrase("今日は晴れ。今日は晴れ。今日は"), "今日は晴れ。");
        assert_eq!(dedupe_phrase("今日は晴れ。\n今日は\n今日は晴れ。"), "今日は晴れ。");
        assert_eq!(dedupe_phrase("どうどう"), "どうどう");
        assert_eq!(dedupe_phrase("どうどうど"), "どうどうど");
        assert_eq!(dedupe_phrase("今日は晴れ。明日は雨。"), "今日は晴れ。明日は雨。");
    }

    #[test]
    fn segmentos_repetidos() {
        assert_eq!(dedupe_segments("今日は今日は今日は晴れ晴れ"), "今日は晴れ");
        assert_eq!(dedupe_segments("今日は晴れ。"), "今日は晴れ。");
    }

    #[test]
    fn incremental() {
        assert_eq!(dedupe_incremental("明明日明日、明日、駅"), "明日、駅");
        assert_eq!(dedupe_incremental("今日は晴れ。"), "今日は晴れ。");
    }

    #[test]
    fn etiquetas_y_llaves() {
        assert_eq!(remove_tags("<color=red>雪</color>が"), "雪が");
        assert_eq!(remove_braces("{rb}漢字{/rb}と{漢字/かんじ}と{w}"), "漢字と漢字と");
        assert_eq!(remove_control("a\u{1}b\nc"), "ab\nc");
    }

    #[test]
    fn join() {
        assert_eq!(join_lines("　窓の外\r\nでは、\n雪が　"), "窓の外では、雪が");
    }

    #[test]
    fn apply_orden_y_regex() {
        let mut f = Filters::default();
        f.regex.push(RegexRule { pattern: "<[^>]+>".into(), replace: "".into(), on: true });
        f.regex.push(RegexRule { pattern: "雪".into(), replace: "X".into(), on: false });
        f.regex.push(RegexRule { pattern: "(".into(), replace: "".into(), on: true }); // invalida
        assert_eq!(apply(&f, "窓の外\n<r>では"), "窓の外では");
        let mut g = Filters::default();
        g.regex.push(RegexRule { pattern: "雪".into(), replace: "月".into(), on: true });
        assert_eq!(apply(&g, "雪が降る\n。"), "月が降る。");
        // por defecto: caracteres y frase repetidos, con saltos de linea del hook
        assert_eq!(apply(&Filters::default(), "明明日日はは晴晴れれ\n明明日日はは晴晴れれ\n"), "明日は晴れ");
    }

    #[test]
    fn deduper_letra_a_letra() {
        let mut d = LineDeduper::new();
        assert_eq!(d.push("明日"), None);
        assert_eq!(d.push("明日、駅"), None);
        assert_eq!(d.push("明日、駅まで"), None);
        assert_eq!(d.push("明日、駅まで"), None);
        assert!(d.pending_age().is_some());
        assert_eq!(d.push("次の行"), Some("明日、駅まで".into()));
        assert_eq!(d.flush(), Some("次の行".into()));
        assert_eq!(d.flush(), None);
        assert!(d.pending_age().is_none());
        // repetida tras emitirse
        assert_eq!(d.push("次の行"), None);
        assert!(!d.has_pending());
    }

    #[test]
    fn deduper_ruido_de_repeticion() {
        let mut d = LineDeduper::new();
        assert_eq!(d.push("今日は晴れ。今日"), None);
        assert_eq!(d.push("今日は晴れ。"), None);
        assert_eq!(d.push("晴れ。"), None);
        assert_eq!(d.flush(), Some("今日は晴れ。".into()));
    }
}
