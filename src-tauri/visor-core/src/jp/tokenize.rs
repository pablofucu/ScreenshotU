//! Tokenizador japones (lindera + IPADIC embebido) con furigana alineado.
//!
//! Decisiones:
//! - `pos`: 名詞->`n` (los 形容動詞語幹 como 静か ->`a`), 動詞->`v`, 形容詞->`a`, 副詞->`adv`, 助詞->`p`,
//!   助動詞->`aux`, 感動詞->`i`, el resto (記号, 接続詞, 連体詞, 接頭詞, フィラー, espacios) ->`x`.
//! - Fusion: un verbo o adjetivo absorbe lo que lo conjuga: auxiliares (助動詞), verbos no independientes
//!   o sufijos (いる, くれる, れる...), las particulas て/で, y otro verbo cuando el anterior esta en
//!   forma 連用 (verbos compuestos: 降り続く). Asi `降り続いて` y `行ってくれない` quedan como una palabra con
//!   `pos` `v`/`a`. Yomitan sigue funcionando porque busca por el texto bajo el cursor, no por token.
//! - Otras fusiones: sustantivo サ変 + する (勉強して -> verbo de base 勉強する), prefijo お/ご/御 + sustantivo
//!   (お茶), cifras seguidas (２０２４) y, tras un verbo o adjetivo, las particulas ば y たり/だり (行けば, 食べたり).
//! - Furigana: la lectura (katakana) se pasa a hiragana y se alinea con la superficie usando los kana de
//!   la superficie como anclas (okurigana). Si no encaja, ruby sobre la palabra entera.
//!   Katakana, latin y palabras sin kanji no llevan ruby.
use super::{ReaderLine, Tok};
use anyhow::{anyhow, Result};
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use std::borrow::Cow;

pub struct Tokenizer {
    seg: Segmenter,
}

/// Token intermedio: superficie, lectura en hiragana (si se conoce), pos simplificado y datos de fusion.
struct Raw {
    surface: String,
    reading: String,
    pos: &'static str,
    major: String,
    sub1: String,
    conj_form: String,
    /// Forma de diccionario del primer trozo (en verbos fusionados, la del verbo principal).
    base: String,
}

impl Tokenizer {
    /// Crea el tokenizador (carga el diccionario embebido; es caro, hacerlo una sola vez).
    pub fn new() -> Result<Self> {
        let dict = load_dictionary("embedded://ipadic").map_err(|e| anyhow!("diccionario ipadic: {e}"))?;
        let seg = Segmenter::new(Mode::Normal, dict, None).keep_whitespace(true);
        Ok(Tokenizer { seg })
    }

    /// Divide el texto en palabras con furigana. Si lindera falla, devuelve el texto como un solo token.
    pub fn tokens(&self, text: &str) -> Vec<Tok> {
        if text.is_empty() {
            return Vec::new();
        }
        let mut raws = match self.seg.segment(Cow::Borrowed(text)) {
            Ok(mut toks) => toks
                .iter_mut()
                .map(|t| {
                    let surface = t.surface.to_string();
                    let d = t.details();
                    let g = |i: usize| d.get(i).copied().filter(|s| *s != "*").unwrap_or("").to_string();
                    let major = g(0);
                    let reading = if d.len() > 7 { kata_to_hira(&g(7)) } else { String::new() };
                    let base = g(6);
                    Raw {
                        base,
                        pos: simple_pos(&major, &g(1), &surface),
                        major,
                        sub1: g(1),
                        conj_form: g(5),
                        reading,
                        surface,
                    }
                })
                .collect::<Vec<_>>(),
            Err(_) => return vec![Tok { parts: vec![(text.to_string(), None)], pos: "x".into(), base: None }],
        };
        merge_conjugations(&mut raws);
        raws.into_iter()
            .map(|r| {
                let base = (!r.base.is_empty() && r.base != r.surface).then_some(r.base);
                Tok { parts: furigana(&r.surface, &r.reading), pos: r.pos.to_string(), base }
            })
            .collect()
    }
}

/// Construye la linea final para la pagina de lectura.
pub fn build_line(tok: &Tokenizer, id: u64, at: i64, who: Option<String>, color: Option<String>, text: &str) -> ReaderLine {
    ReaderLine { id, at, who, color, text: text.to_string(), toks: tok.tokens(text), words: Vec::new(), grammar: Vec::new() }
}

fn simple_pos(major: &str, sub1: &str, surface: &str) -> &'static str {
    match major {
        "名詞" => {
            if sub1 == "形容動詞語幹" {
                "a"
            } else {
                "n"
            }
        }
        "動詞" => "v",
        "形容詞" => "a",
        "副詞" => "adv",
        "助詞" => "p",
        "助動詞" => "aux",
        "感動詞" => "i",
        "" => {
            // palabra desconocida: katakana/kanji suelen ser sustantivos
            if surface.chars().any(|c| is_kanji(c) || is_katakana(c)) {
                "n"
            } else {
                "x"
            }
        }
        _ => "x",
    }
}

/// Fusiones que no son de conjugacion: sustantivo サ変 + する, prefijo + sustantivo y cifras seguidas.
/// Devuelve `true` si `r` se ha unido a `prev`.
fn merge_other(prev: &mut Raw, r: &Raw) -> bool {
    let rd = |r: &Raw| if r.reading.is_empty() { kata_to_hira(&r.surface) } else { r.reading.clone() };
    // 勉強 + する: pasa a ser un verbo; su forma de diccionario es 勉強する
    if prev.major == "名詞" && prev.sub1 == "サ変接続" && r.major == "動詞" && r.base == "する" && prev.surface.chars().any(is_kanji) {
        prev.base = format!("{}する", prev.surface);
        prev.pos = "v";
    } else if prev.major == "接頭詞" && prev.sub1 == "名詞接続" && matches!(prev.surface.as_str(), "お" | "ご" | "御")
        && r.major == "名詞" && !matches!(r.sub1.as_str(), "数" | "代名詞" | "非自立" | "接尾")
    {
        prev.base = format!("{}{}", prev.surface, if r.base.is_empty() { &r.surface } else { &r.base });
        prev.pos = "n";
    } else if prev.major == "名詞" && prev.sub1 == "数" && r.major == "名詞" && r.sub1 == "数" {
        prev.base = format!("{}{}", prev.surface, r.surface);
    } else {
        return false;
    }
    prev.surface.push_str(&r.surface);
    let rr = rd(r);
    if prev.reading.is_empty() {
        prev.reading = kata_to_hira(&prev.surface[..prev.surface.len() - r.surface.len()]);
    }
    prev.reading.push_str(&rr);
    prev.major = r.major.clone();
    prev.sub1 = r.sub1.clone();
    prev.conj_form = r.conj_form.clone();
    true
}

/// Fusiona verbos/adjetivos con lo que los conjuga (ver documentacion del modulo).
fn merge_conjugations(raws: &mut Vec<Raw>) {
    let mut out: Vec<Raw> = Vec::with_capacity(raws.len());
    for r in raws.drain(..) {
        if let Some(prev) = out.last_mut() {
            if merge_other(prev, &r) {
                continue;
            }
        }
        let absorb = match out.last() {
            Some(prev) if prev.pos == "v" || prev.pos == "a" => {
                (prev.major == "動詞" || prev.major == "形容詞" || prev.major == "助動詞" || prev.sub1 == "接続助詞" || prev.sub1 == "並立助詞")
                    && can_attach(prev, &r)
            }
            _ => false,
        };
        if absorb {
            let prev = out.last_mut().unwrap();
            // verbo compuesto (降り + 続く): la forma de diccionario es la del conjunto
            if r.major == "動詞" && r.sub1 == "自立" && !r.base.is_empty() {
                prev.base = format!("{}{}", prev.surface, r.base);
            }
            prev.surface.push_str(&r.surface);
            // lectura: si falta en alguno, se usa la superficie (kana) para no perder alineacion
            let rd = if r.reading.is_empty() { kata_to_hira(&r.surface) } else { r.reading.clone() };
            prev.reading.push_str(&rd);
            prev.major = r.major.clone();
            prev.sub1 = r.sub1.clone();
            prev.conj_form = r.conj_form.clone();
        } else {
            out.push(r);
        }
    }
    *raws = out;
}

fn can_attach(prev: &Raw, next: &Raw) -> bool {
    match next.major.as_str() {
        "助動詞" => true,
        "動詞" => next.sub1 == "非自立" || next.sub1 == "接尾" || (next.sub1 == "自立" && prev.major == "動詞" && prev.conj_form.starts_with("連用")),
        "形容詞" => next.sub1 == "非自立" || next.sub1 == "接尾",
        "助詞" => {
            prev.sub1 != "接続助詞"
                && prev.sub1 != "並立助詞"
                && ((next.sub1 == "接続助詞" && matches!(next.surface.as_str(), "て" | "で" | "ば"))
                    || (next.sub1 == "並立助詞" && matches!(next.surface.as_str(), "たり" | "だり")))
        }
        _ => false,
    }
}

fn is_kanji(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}' | '々' | '〆' | 'ヶ')
        && c != 'ヶ'
}

fn is_hiragana(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{3096}')
}

fn is_katakana(c: char) -> bool {
    matches!(c, '\u{30A1}'..='\u{30FA}' | '\u{30FC}')
}

fn is_kana(c: char) -> bool {
    is_hiragana(c) || is_katakana(c) || c == 'ー'
}

fn kata_to_hira(s: &str) -> String {
    s.chars()
        .map(|c| if matches!(c, '\u{30A1}'..='\u{30F6}') { char::from_u32(c as u32 - 0x60).unwrap_or(c) } else { c })
        .collect()
}

/// Reparte `surface` en trozos con furigana segun la lectura (hiragana).
fn furigana(surface: &str, reading: &str) -> Vec<(String, Option<String>)> {
    let plain = || vec![(surface.to_string(), None)];
    if reading.is_empty() || !surface.chars().any(is_kanji) {
        return plain();
    }
    // segmentos: (es_kana, texto)
    let mut segs: Vec<(bool, String)> = Vec::new();
    for c in surface.chars() {
        let k = is_kana(c);
        match segs.last_mut() {
            Some((lk, t)) if *lk == k => t.push(c),
            _ => segs.push((k, c.to_string())),
        }
    }
    let rd: Vec<char> = reading.chars().collect();
    let hira_segs: Vec<Vec<char>> = segs.iter().map(|(k, t)| if *k { kata_to_hira(t).chars().collect() } else { Vec::new() }).collect();
    let mut cuts: Vec<usize> = Vec::new(); // fin en la lectura de cada segmento
    if align(&segs, &hira_segs, &rd, 0, 0, &mut cuts) {
        let mut parts = Vec::new();
        let mut start = 0;
        for (i, (k, t)) in segs.iter().enumerate() {
            let end = cuts[i];
            if *k {
                parts.push((t.clone(), None));
            } else {
                parts.push((t.clone(), Some(rd[start..end].iter().collect::<String>())));
            }
            start = end;
        }
        return parts;
    }
    // sin alineacion: ruby sobre la palabra entera (si difiere de la superficie)
    if reading == kata_to_hira(surface) {
        plain()
    } else {
        vec![(surface.to_string(), Some(reading.to_string()))]
    }
}

/// Backtracking: los segmentos kana son anclas que deben coincidir; los de kanji toman al menos 1 caracter.
fn align(segs: &[(bool, String)], hira: &[Vec<char>], rd: &[char], i: usize, j: usize, cuts: &mut Vec<usize>) -> bool {
    if i == segs.len() {
        return j == rd.len();
    }
    if segs[i].0 {
        let k = &hira[i];
        if rd.len() >= j + k.len() && rd[j..j + k.len()] == k[..] {
            cuts.push(j + k.len());
            if align(segs, hira, rd, i + 1, j + k.len(), cuts) {
                return true;
            }
            cuts.pop();
        }
        false
    } else {
        for end in (j + 1)..=rd.len() {
            cuts.push(end);
            if align(segs, hira, rd, i + 1, end, cuts) {
                return true;
            }
            cuts.pop();
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    fn tk() -> &'static Tokenizer {
        static T: OnceLock<Tokenizer> = OnceLock::new();
        T.get_or_init(|| Tokenizer::new().unwrap())
    }

    fn p(a: &str, b: Option<&str>) -> (String, Option<String>) {
        (a.to_string(), b.map(|x| x.to_string()))
    }

    #[test]
    fn alineacion() {
        assert_eq!(furigana("降り続いて", "ふりつづいて"), vec![p("降", Some("ふ")), p("り", None), p("続", Some("つづ")), p("いて", None)]);
        assert_eq!(furigana("一緒", "いっしょ"), vec![p("一緒", Some("いっしょ"))]);
        assert_eq!(furigana("ねえ", "ねえ"), vec![p("ねえ", None)]);
        assert_eq!(furigana("カメラ", "かめら"), vec![p("カメラ", None)]);
        assert_eq!(furigana("ABC", "えーびーしー"), vec![p("ABC", None)]);
        // no encaja: ruby entero
        assert_eq!(furigana("行く", "ゆく"), vec![p("行", Some("ゆ")), p("く", None)]);
        assert_eq!(furigana("昨日", "きのう"), vec![p("昨日", Some("きのう"))]);
        assert_eq!(furigana("読む", "かく"), vec![p("読む", Some("かく"))]);
    }

    fn dump(text: &str) -> Vec<(String, String)> {
        tk().tokens(text).iter().map(|t| (t.parts.iter().map(|p| p.0.as_str()).collect::<String>(), t.pos.clone())).collect()
    }

    #[test]
    fn frase_narracion() {
        let text = "窓の外では、雪が音もなく降り続いていた。";
        let toks = tk().tokens(text);
        // el texto se conserva
        let joined: String = toks.iter().flat_map(|t| t.parts.iter().map(|p| p.0.clone())).collect();
        assert_eq!(joined, text);
        let d = dump(text);
        let verbo = toks.iter().find(|t| t.parts.iter().map(|p| p.0.as_str()).collect::<String>().starts_with("降り続")).expect(&format!("{d:?}"));
        assert_eq!(verbo.pos, "v");
        assert_eq!(verbo.parts[0], p("降", Some("ふ")));
        assert_eq!(verbo.parts[1], p("り", None));
        assert_eq!(verbo.parts[2], p("続", Some("つづ")));
        let ventana = &toks[0];
        assert_eq!(ventana.parts, vec![p("窓", Some("まど"))]);
        assert_eq!(ventana.pos, "n");
        assert!(toks.iter().any(|t| t.pos == "p"), "{d:?}");
    }

    #[test]
    fn frase_dialogo() {
        let line = build_line(tk(), 7, 1000, Some("美雪".into()), None, "「明日、駅まで一緒に行ってくれないかな」");
        assert_eq!(line.id, 7);
        let toks = &line.toks;
        let joined: String = toks.iter().flat_map(|t| t.parts.iter().map(|p| p.0.clone())).collect();
        assert_eq!(joined, line.text);
        let d = dump(&line.text);
        assert!(toks.iter().any(|t| t.parts == vec![p("一緒", Some("いっしょ"))]), "{d:?}");
        let ik = toks.iter().find(|t| t.parts[0].0 == "行").expect(&format!("{d:?}"));
        assert_eq!(ik.pos, "v");
        assert_eq!(ik.parts[0].1.as_deref(), Some("い"));
        assert_eq!(toks[0].pos, "x");
        assert!(tk().tokens("").is_empty());
    }

    fn find<'a>(toks: &'a [Tok], start: &str) -> Option<&'a Tok> {
        toks.iter().find(|t| t.parts.iter().map(|p| p.0.as_str()).collect::<String>().starts_with(start))
    }

    #[test]
    fn fusiones_extra() {
        let toks = tk().tokens("毎日勉強していた。");
        let d = dump("毎日勉強していた。");
        let v = find(&toks, "勉強").expect(&format!("{d:?}"));
        assert_eq!((v.pos.as_str(), v.base.as_deref()), ("v", Some("勉強する")), "{d:?}");
        assert_eq!(v.parts.iter().map(|p| p.0.as_str()).collect::<String>(), "勉強していた", "{d:?}");
        assert_eq!(v.parts[0], p("勉強", Some("べんきょう")));

        let toks = tk().tokens("お茶を飲めば、本を読んだりする");
        let d = dump("お茶を飲めば、本を読んだりする");
        assert!(find(&toks, "お茶").is_some_and(|t| t.pos == "n"), "{d:?}");
        assert!(find(&toks, "飲めば").is_some_and(|t| t.pos == "v" && t.base.as_deref() == Some("飲む")), "{d:?}");
        assert!(find(&toks, "読んだり").is_some_and(|t| t.pos == "v"), "{d:?}");
        // el texto se conserva
        let joined: String = toks.iter().flat_map(|t| t.parts.iter().map(|p| p.0.clone())).collect();
        assert_eq!(joined, "お茶を飲めば、本を読んだりする");
    }

    #[test]
    fn katakana_y_latin_sin_ruby() {
        for t in tk().tokens("カメラとABC") {
            assert!(t.parts.iter().all(|p| p.1.is_none()));
        }
    }
}
