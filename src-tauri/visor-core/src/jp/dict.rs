//! Diccionarios en formato Yomitan (`.zip` con `index.json` + `term_bank_*.json`), p. ej. JMdict o jmdict-es.
//! Se usan para poner las acepciones de las palabras principales de cada linea en la pagina de lectura.
//!
//! - `term_bank` v3: `[termino, lectura, tagsDef, reglas, puntuacion, glosario[], secuencia, tagsTermino]`;
//!   v1: `[termino, lectura, tagsDef, reglas, puntuacion, ...glosario]`.
//! - El glosario puede ser texto, `{type:"text"}` o `{type:"structured-content"}`; el contenido estructurado se
//!   aplana a texto (cada bloque `li`/`div`/... es una acepcion) y se descartan ejemplos y atribuciones.
use super::{GrammarHit, KanjiInfo, Sense, Tok, WordDef, WordInfo};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

const MAX_ENTRIES: usize = 3;
const MAX_SENSES: usize = 8;
const MAX_GLOSSES: usize = 6;
const MAX_WORDS: usize = 16;
const MAX_GRAMMAR: usize = 8;
const MAX_KANJI: usize = 6;
/// Longitud maxima (en tokens) de una secuencia gramatical.
const MAX_GRAM_TOKS: usize = 4;

struct Entry {
    term: String,
    reading: String,
    tags: Vec<String>,
    glosses: Vec<String>,
    score: i64,
    seq: i64,
    dict: u16,
}

#[derive(Default)]
pub struct Dict {
    entries: Vec<Entry>,
    by_term: HashMap<String, Vec<u32>>,
    by_reading: HashMap<String, Vec<u32>>,
    /// Nombre corto -> descripcion (`tag_bank`), para el `title` de las etiquetas.
    tag_notes: HashMap<String, String>,
    names: Vec<String>,
    /// Entradas de los diccionarios de gramatica (aparte, para no ensuciar las acepciones de las palabras).
    g_entries: Vec<Entry>,
    /// Termino sin «〜» inicial -> entradas de gramatica.
    g_by_term: HashMap<String, Vec<u32>>,
    /// Raiz (termino sin su ultima kana conjugable) -> terminos, para casar formas conjugadas (てくれる ~ てくれた).
    g_stems: HashMap<String, Vec<String>>,
    /// Frecuencias (`term_meta_bank` modo `freq`): termino -> (lectura, valor).
    freq: HashMap<String, Vec<(String, u32)>>,
    /// Kanji (`kanji_bank`, p. ej. KANJIDIC).
    kanji: HashMap<char, KanjiInfo>,
}

impl Dict {
    /// Carga todos los `.zip` de la carpeta (los que no sean diccionarios de terminos se ignoran).
    pub fn load_dir(dir: &Path) -> Dict {
        let mut d = Dict::default();
        let Ok(rd) = std::fs::read_dir(dir) else { return d };
        let mut zips: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("zip")).unwrap_or(false))
            .collect();
        zips.sort();
        // `grammar.txt` (opcional): un fragmento por linea del titulo o del nombre de archivo de los diccionarios de gramatica.
        let hints: Vec<String> = std::fs::read_to_string(dir.join("grammar.txt"))
            .map(|t| t.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty() && !l.starts_with('#')).collect())
            .unwrap_or_default();
        for z in zips {
            if let Err(e) = d.load_zip(&z, &hints) {
                eprintln!("Diccionario {}: {e}", z.display());
            }
        }
        d
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn has_grammar(&self) -> bool {
        !self.g_entries.is_empty()
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    fn load_zip(&mut self, path: &Path, hints: &[String]) -> Result<(), String> {
        let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
        let read = |zip: &mut zip::ZipArchive<std::fs::File>, name: &str| -> Option<Value> {
            let mut s = String::new();
            zip.by_name(name).ok()?.read_to_string(&mut s).ok()?;
            serde_json::from_str(&s).ok()
        };
        let index = read(&mut zip, "index.json").ok_or("sin index.json")?;
        let title = index.get("title").and_then(Value::as_str).unwrap_or("?").to_string();
        let bank = |zip: &zip::ZipArchive<std::fs::File>, prefix: &str| -> Vec<String> {
            let mut v: Vec<String> = zip.file_names().filter(|n| n.starts_with(prefix) && n.ends_with(".json")).map(String::from).collect();
            v.sort();
            v
        };
        let terms = bank(&zip, "term_bank_");
        let metas = bank(&zip, "term_meta_bank_");
        let kanjis = bank(&zip, "kanji_bank_");
        if terms.is_empty() && metas.is_empty() && kanjis.is_empty() {
            return Ok(());
        }
        let file = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        let grammar = is_grammar_title(&title) || hints.iter().any(|h| title.to_lowercase().contains(h) || file.contains(h));
        let dict = self.names.len() as u16;
        self.names.push(title);
        for n in zip.file_names().filter(|n| n.starts_with("tag_bank_")).map(String::from).collect::<Vec<_>>() {
            if let Some(Value::Array(tags)) = read(&mut zip, &n) {
                for t in tags {
                    if let (Some(name), Some(notes)) = (t.get(0).and_then(Value::as_str), t.get(3).and_then(Value::as_str)) {
                        self.tag_notes.entry(name.to_string()).or_insert_with(|| notes.to_string());
                    }
                }
            }
        }
        for n in terms {
            let Some(Value::Array(rows)) = read(&mut zip, &n) else { continue };
            for row in rows {
                if let Some(e) = parse_row(&row, dict) {
                    if grammar {
                        self.push_grammar(e);
                    } else {
                        self.push(e);
                    }
                }
            }
        }
        for n in metas {
            let Some(Value::Array(rows)) = read(&mut zip, &n) else { continue };
            for row in rows {
                if let Some((term, reading, v)) = parse_freq(&row) {
                    self.freq.entry(term).or_default().push((reading, v));
                }
            }
        }
        for n in kanjis {
            let Some(Value::Array(rows)) = read(&mut zip, &n) else { continue };
            for row in rows {
                if let Some(k) = parse_kanji(&row) {
                    self.kanji.entry(k.ch.chars().next().unwrap_or(' ')).or_insert(k);
                }
            }
        }
        Ok(())
    }

    fn push_grammar(&mut self, e: Entry) {
        let i = self.g_entries.len() as u32;
        let key = gram_key(&e.term);
        if key.is_empty() {
            return;
        }
        let n = key.chars().count();
        if n >= 3 && key.chars().last().map(is_conj_end).unwrap_or(false) {
            let stem: String = key.chars().take(n - 1).collect();
            let v = self.g_stems.entry(stem).or_default();
            if !v.contains(&key) {
                v.push(key.clone());
            }
        }
        self.g_by_term.entry(key).or_default().push(i);
        self.g_entries.push(e);
    }

    fn push(&mut self, e: Entry) {
        let i = self.entries.len() as u32;
        self.by_term.entry(e.term.clone()).or_default().push(i);
        if !e.reading.is_empty() && e.reading != e.term {
            self.by_reading.entry(e.reading.clone()).or_default().push(i);
        }
        self.entries.push(e);
    }

    /// Entradas de `term` (si se da `reading`, primero las que coinciden con ella). Agrupa por termino+lectura.
    pub fn lookup(&self, term: &str, reading: Option<&str>) -> Vec<WordDef> {
        let mut ids: Vec<u32> = self.by_term.get(term).cloned().unwrap_or_default();
        if ids.is_empty() && term.chars().all(is_kana) {
            ids = self.by_reading.get(&kata_to_hira(term)).cloned().unwrap_or_default();
        }
        self.defs_of(&self.entries, &ids, reading)
    }

    fn defs_of(&self, store: &[Entry], ids: &[u32], reading: Option<&str>) -> Vec<WordDef> {
        if ids.is_empty() {
            return Vec::new();
        }
        let rd = reading.map(kata_to_hira);
        let mut ents: Vec<&Entry> = ids.iter().map(|&i| &store[i as usize]).collect();
        // orden: lectura coincidente, diccionario (orden de carga), puntuacion, secuencia
        ents.sort_by_key(|e| (rd.as_deref().map(|r| kata_to_hira(&e.reading) != r).unwrap_or(false), e.dict, -e.score, e.seq));
        let mut out: Vec<WordDef> = Vec::new();
        for e in ents {
            let sense = Sense { tags: e.tags.clone(), glosses: e.glosses.iter().take(MAX_GLOSSES).cloned().collect() };
            if sense.glosses.is_empty() {
                continue;
            }
            let n = out.len();
            match out.iter_mut().find(|d| d.term == e.term && d.reading == e.reading && d.dict == self.names[e.dict as usize]) {
                Some(d) => {
                    if d.senses.len() < MAX_SENSES && !d.senses.contains(&sense) {
                        d.senses.push(sense);
                    }
                }
                None if n < MAX_ENTRIES => out.push(WordDef {
                    term: e.term.clone(),
                    reading: e.reading.clone(),
                    dict: self.names[e.dict as usize].clone(),
                    senses: vec![sense],
                }),
                None => {}
            }
        }
        out
    }

    /// Frecuencia (puesto) de un termino; si hay varias lecturas, la que coincide con `reading` o la primera.
    pub fn freq_of(&self, term: &str, reading: Option<&str>) -> Option<u32> {
        let v = self.freq.get(term)?;
        let rd = reading.map(kata_to_hira);
        rd.and_then(|r| v.iter().find(|(x, _)| kata_to_hira(x) == r)).or_else(|| v.first()).map(|(_, f)| *f)
    }

    /// Kanji de una palabra con su significado (sin repetir).
    pub fn kanji_of(&self, word: &str) -> Vec<KanjiInfo> {
        let mut out: Vec<KanjiInfo> = Vec::new();
        for c in word.chars() {
            if let Some(k) = self.kanji.get(&c) {
                if !out.iter().any(|x| x.ch == k.ch) {
                    out.push(k.clone());
                }
            }
        }
        out.truncate(MAX_KANJI);
        out
    }

    /// Puntos gramaticales de una linea: secuencias de hasta 4 tokens consecutivos que coinciden con un termino de
    /// un diccionario de gramatica. Se prueba primero la secuencia mas larga; sin solapes. Como el tokenizador junta
    /// el verbo con sus auxiliares (行ってくれた), tambien se busca dentro de la superficie de los verbos/adjetivos.
    /// El rango devuelto es de tokens (el subrayado de la pagina cubre el token entero).
    pub fn grammar(&self, toks: &[Tok]) -> Vec<GrammarHit> {
        if self.g_by_term.is_empty() {
            return Vec::new();
        }
        let surf: Vec<String> = toks.iter().map(|t| t.parts.iter().map(|p| p.0.as_str()).collect()).collect();
        let mut out: Vec<GrammarHit> = Vec::new();
        let mut i = 0;
        while i < toks.len() && out.len() < MAX_GRAMMAR {
            let mut hit: Option<(usize, String)> = None;
            'len: for len in (1..=MAX_GRAM_TOKS.min(toks.len() - i)).rev() {
                for text in gram_texts(toks, &surf, i, len) {
                    if let Some(key) = self.match_gram(&text) {
                        hit = Some((len, key));
                        break 'len;
                    }
                }
            }
            match hit {
                Some((len, key)) => {
                    let ids = &self.g_by_term[&key];
                    let defs = self.defs_of(&self.g_entries, ids, None);
                    let term = defs.first().map(|d| d.term.clone()).unwrap_or(key);
                    out.push(GrammarHit { toks: [i as u32, (i + len - 1) as u32], term, defs });
                    i += len;
                }
                None => i += 1,
            }
        }
        out
    }

    /// Termino de gramatica que casa con `text` entero (exacto, o por raiz con una terminacion conjugada corta).
    fn match_gram(&self, text: &str) -> Option<String> {
        if self.g_by_term.contains_key(text) {
            return Some(text.to_string());
        }
        let chars: Vec<char> = text.chars().collect();
        // raiz mas larga primero; el resto debe ser una terminacion corta en kana (た, ない, ます, ...)
        for l in (2..chars.len().min(12) + 1).rev() {
            let (stem, rest) = (chars[..l].iter().collect::<String>(), &chars[l..]);
            if rest.len() > 3 || !rest.iter().all(|c| is_kana(*c)) {
                continue;
            }
            if let Some(keys) = self.g_stems.get(&stem) {
                if let Some(k) = keys.iter().find(|k| k.chars().count() == l + 1) {
                    return Some(k.clone());
                }
            }
        }
        None
    }

    pub fn tag_note(&self, tag: &str) -> Option<&str> {
        self.tag_notes.get(tag).map(String::as_str)
    }

    /// Palabras principales de una linea (sustantivos, verbos, adjetivos, adverbios, interjecciones) con acepciones.
    /// Sin diccionario, salen igualmente con su lectura y categoria.
    pub fn words(&self, toks: &[Tok]) -> Vec<WordInfo> {
        let mut out: Vec<WordInfo> = Vec::new();
        for (i, t) in toks.iter().enumerate() {
            if !matches!(t.pos.as_str(), "n" | "v" | "a" | "adv" | "i") {
                continue;
            }
            let surface: String = t.parts.iter().map(|p| p.0.as_str()).collect();
            if !surface.chars().any(|c| c.is_alphanumeric()) {
                continue;
            }
            let base = t.base.clone().unwrap_or_else(|| surface.clone());
            if out.iter().any(|w| w.base == base) {
                continue;
            }
            // lectura de la superficie (solo sirve para filtrar cuando la base es la propia superficie)
            let reading: String = t.parts.iter().map(|(b, r)| r.clone().unwrap_or_else(|| b.clone())).collect();
            let rd = (base == surface && reading != surface).then_some(reading.as_str());
            let mut defs = self.lookup(&base, rd);
            if defs.is_empty() && base != surface {
                defs = self.lookup(&surface, None);
            }
            // fusiones del tokenizador que no suelen estar como entrada: 勉強する -> 勉強, お茶 -> 茶
            if defs.is_empty() {
                let stem = base.strip_suffix("する").filter(|s| !s.is_empty()).or_else(|| ["お", "ご", "御"].iter().find_map(|p| base.strip_prefix(p)).filter(|s| !s.is_empty()));
                if let Some(s) = stem {
                    defs = self.lookup(s, None);
                }
            }
            // un sustantivo en kana sin entrada suele ser un trozo suelto: fuera, salvo que no haya diccionario
            if defs.is_empty() && !self.is_empty() && !surface.chars().any(is_kanji_or_kata) {
                continue;
            }
            let freq = self.freq_of(&base, defs.first().map(|d| d.reading.as_str()).or(rd));
            let kanji = self.kanji_of(&base);
            out.push(WordInfo { tok: i as u32, surface, base, pos: t.pos.clone(), defs, freq, kanji });
            if out.len() >= MAX_WORDS {
                break;
            }
        }
        out
    }
}

fn parse_row(row: &Value, dict: u16) -> Option<Entry> {
    let a = row.as_array()?;
    let term = a.first()?.as_str()?.to_string();
    let reading = a.get(1).and_then(Value::as_str).unwrap_or("").to_string();
    let mut tags: Vec<String> = a.get(2).and_then(Value::as_str).unwrap_or("").split_whitespace().map(String::from).collect();
    let score = a.get(4).and_then(Value::as_i64).unwrap_or(0);
    let (gloss, seq): (Vec<&Value>, i64) = match a.get(5) {
        Some(Value::Array(g)) => (g.iter().collect(), a.get(6).and_then(Value::as_i64).unwrap_or(0)),
        _ => (a.iter().skip(5).collect(), 0),
    };
    let mut glosses = Vec::new();
    for g in gloss {
        match g {
            Value::String(s) => push_gloss(&mut glosses, s),
            Value::Object(o) => match o.get("type").and_then(Value::as_str) {
                Some("text") => push_gloss(&mut glosses, o.get("text").and_then(Value::as_str).unwrap_or("")),
                Some("structured-content") => {
                    let mut buf = String::new();
                    flatten(o.get("content").unwrap_or(&Value::Null), &mut buf, &mut tags);
                    for l in buf.split('\n') {
                        push_gloss(&mut glosses, l);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    tags.dedup();
    Some(Entry { term, reading, tags, glosses, score, seq, dict })
}

/// Titulo de un diccionario de gramatica por convencion («Grammar», «文法», «gramática»).
fn is_grammar_title(t: &str) -> bool {
    let l = t.to_lowercase();
    l.contains("grammar") || l.contains("gramática") || l.contains("gramatica") || t.contains("文法")
}

/// Clave de busqueda de un termino de gramatica: sin marcas iniciales como «〜».
fn gram_key(term: &str) -> String {
    term.trim().trim_start_matches(|c: char| matches!(c, '〜' | '～' | '~' | '-' | '－' | '…' | '・' | '＋' | '+' | ' ')).trim().to_string()
}

fn is_conj_end(c: char) -> bool {
    matches!(c, 'る' | 'う' | 'く' | 'ぐ' | 'す' | 'つ' | 'ぬ' | 'ぶ' | 'む' | 'い')
}

/// Textos que se prueban contra el diccionario de gramatica para la secuencia `toks[i..i+len]`: la superficie junta,
/// con el ultimo token en su forma de diccionario, y partiendo de cada posicion interna del primer token si es un
/// verbo/adjetivo/auxiliar (行ってくれた -> てくれた). Una particula suelta de una letra, o un auxiliar corto, no cuentan.
fn gram_texts(toks: &[Tok], surf: &[String], i: usize, len: usize) -> Vec<String> {
    let first = &toks[i];
    let first_chars: Vec<char> = surf[i].chars().collect();
    let conj = matches!(first.pos.as_str(), "v" | "a" | "aux");
    if len == 1 {
        let n = first_chars.len();
        let ok0 = match first.pos.as_str() {
            "p" => n >= 2,
            "aux" => n >= 3,
            "v" | "a" => false, // la palabra entera no es gramatica: solo sus terminaciones
            _ => false,
        };
        let mut out = Vec::new();
        if ok0 {
            out.push(surf[i].clone());
        }
        if conj {
            for k in 1..n.saturating_sub(1) {
                out.push(first_chars[k..].iter().collect());
            }
        }
        return out;
    }
    // secuencia: solo si el primer token es particula, auxiliar, verbo/adjetivo o termina en て/で
    if !(conj || first.pos == "p" || surf[i].ends_with('て') || surf[i].ends_with('で')) {
        return Vec::new();
    }
    let mut starts: Vec<String> = vec![surf[i].clone()];
    if conj {
        for k in 1..first_chars.len() {
            starts.push(first_chars[k..].iter().collect());
        }
    } else if let Some(c) = first_chars.last().filter(|c| matches!(c, 'て' | 'で')) {
        starts.push(c.to_string());
    }
    let mid: String = surf[i + 1..i + len - 1].concat();
    let last = &toks[i + len - 1];
    let last_base = last.base.clone().unwrap_or_else(|| surf[i + len - 1].clone());
    let mut out = Vec::new();
    for f in &starts {
        out.push(format!("{f}{mid}{}", surf[i + len - 1]));
        out.push(format!("{f}{mid}{last_base}"));
    }
    out
}

/// `term_meta_bank`: `[termino, "freq", dato]` con dato = numero | `{value}` | `{reading, frequency: numero | {value}}`.
fn parse_freq(row: &Value) -> Option<(String, String, u32)> {
    let a = row.as_array()?;
    if a.get(1)?.as_str()? != "freq" {
        return None;
    }
    let term = a.first()?.as_str()?.to_string();
    let data = a.get(2)?;
    fn num(v: &Value) -> Option<u32> {
        match v {
            Value::Number(n) => n.as_f64().map(|f| f.max(0.0) as u32),
            Value::Object(o) => o.get("value").and_then(num),
            _ => None,
        }
    }
    match data {
        Value::Object(o) if o.contains_key("frequency") => {
            let reading = o.get("reading").and_then(Value::as_str).unwrap_or("").to_string();
            Some((term, reading, num(o.get("frequency")?)?))
        }
        v => Some((term, String::new(), num(v)?)),
    }
}

/// `kanji_bank`: `[caracter, onyomi, kunyomi, etiquetas, significados[], estadisticas]`.
fn parse_kanji(row: &Value) -> Option<KanjiInfo> {
    let a = row.as_array()?;
    let ch = a.first()?.as_str()?.to_string();
    if ch.chars().count() != 1 {
        return None;
    }
    let split = |v: Option<&Value>| -> Vec<String> { v.and_then(Value::as_str).unwrap_or("").split_whitespace().map(String::from).collect() };
    let meanings: Vec<String> = a.get(4)?.as_array()?.iter().filter_map(Value::as_str).map(String::from).collect();
    Some(KanjiInfo { ch, meanings, onyomi: split(a.get(1)), kunyomi: split(a.get(2)) })
}

fn push_gloss(out: &mut Vec<String>, s: &str) {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if !s.is_empty() && !out.contains(&s) {
        out.push(s);
    }
}

/// Aplana contenido estructurado: texto plano con `\n` entre bloques. Las etiquetas de categoria van a `tags`.
fn flatten(v: &Value, out: &mut String, tags: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push_str(s),
        Value::Array(a) => a.iter().for_each(|x| flatten(x, out, tags)),
        Value::Object(o) => {
            let tag = o.get("tag").and_then(Value::as_str).unwrap_or("");
            let data = o.get("data");
            let dc = data.and_then(|d| d.get("content")).and_then(Value::as_str).unwrap_or("");
            if matches!(dc, "example-sentence" | "attribution" | "forms" | "xref" | "references" | "antonym" | "info-gloss" | "source-language")
                || matches!(tag, "img" | "rt" | "rp")
            {
                return;
            }
            let content = o.get("content").unwrap_or(&Value::Null);
            // etiquetas de JMdict: <span data-code="n" title="noun">noun</span>
            if tag == "span" && (data.and_then(|d| d.get("code")).is_some() || dc == "part-of-speech-info" || dc == "misc-info" || dc == "field-info") {
                let mut t = String::new();
                flatten(content, &mut t, tags);
                let t = data.and_then(|d| d.get("code")).and_then(Value::as_str).map(String::from).unwrap_or(t);
                if !t.trim().is_empty() && !tags.contains(&t) {
                    tags.push(t.trim().to_string());
                }
                return;
            }
            let block = matches!(tag, "li" | "div" | "ul" | "ol" | "p" | "tr" | "table" | "details" | "summary" | "br");
            if block {
                out.push('\n');
            }
            flatten(content, out, tags);
            if block {
                out.push('\n');
            }
        }
        _ => {}
    }
}

fn is_kana(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{3096}' | '\u{30A1}'..='\u{30FA}' | 'ー')
}

fn is_kanji_or_kata(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}' | '々' | '\u{30A1}'..='\u{30FA}')
}

fn kata_to_hira(s: &str) -> String {
    s.chars()
        .map(|c| if matches!(c, '\u{30A1}'..='\u{30F6}') { char::from_u32(c as u32 - 0x60).unwrap_or(c) } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dict() -> Dict {
        let mut d = Dict::default();
        d.names.push("JMdict".into());
        for row in [
            json!(["雪", "ゆき", "n", "", 10, ["snow"], 1, ""]),
            json!(["降る", "ふる", "v5r vi", "v5", 5, [{"type": "structured-content", "content": [
                {"tag": "ul", "content": [{"tag": "li", "content": "to fall (of rain, snow, etc.)"}, {"tag": "li", "content": "to come down"}]},
                {"tag": "div", "data": {"content": "example-sentence"}, "content": "雨が降る"}
            ]}], 2, ""]),
            json!(["一緒", "いっしょ", "n vs", "", 1, ["together"], 3, ""]),
        ] {
            d.push(parse_row(&row, 0).unwrap());
        }
        d
    }

    #[test]
    fn contenido_estructurado() {
        let d = dict();
        let r = d.lookup("降る", None);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].senses[0].glosses, vec!["to fall (of rain, snow, etc.)", "to come down"]);
        assert_eq!(r[0].senses[0].tags, vec!["v5r", "vi"]);
    }

    #[test]
    fn palabras_de_linea() {
        let d = dict();
        let tk = super::super::Tokenizer::new().unwrap();
        let toks = tk.tokens("雪が降っていた。一緒に");
        let w = d.words(&toks);
        let bases: Vec<&str> = w.iter().map(|w| w.base.as_str()).collect();
        assert!(bases.contains(&"雪"), "{bases:?}");
        assert!(bases.contains(&"降る"), "{bases:?}");
        let f = w.iter().find(|w| w.base == "降る").unwrap();
        assert_eq!(f.defs[0].senses[0].glosses[0], "to fall (of rain, snow, etc.)");
        assert!(w.iter().all(|w| w.pos != "p"));
    }

    #[test]
    fn frecuencia_y_kanji() {
        assert_eq!(parse_freq(&json!(["雪", "freq", 1800])), Some(("雪".into(), "".into(), 1800)));
        assert_eq!(parse_freq(&json!(["雪", "freq", {"value": 7, "displayValue": "7㋕"}])), Some(("雪".into(), "".into(), 7)));
        assert_eq!(parse_freq(&json!(["降る", "freq", {"reading": "ふる", "frequency": {"value": 900}}])), Some(("降る".into(), "ふる".into(), 900)));
        assert_eq!(parse_freq(&json!(["雪", "pitch", 3])), None);
        let k = parse_kanji(&json!(["雪", "セツ", "ゆき", "", ["nieve", "nevada"], {}])).unwrap();
        assert_eq!((k.onyomi.clone(), k.kunyomi.clone(), k.meanings.len()), (vec!["セツ".to_string()], vec!["ゆき".to_string()], 2));
        let mut d = dict();
        d.freq.entry("雪".into()).or_default().push(("ゆき".into(), 1800));
        d.kanji.insert('雪', k);
        let tk = super::super::Tokenizer::new().unwrap();
        let w = d.words(&tk.tokens("雪が降った"));
        let yuki = w.iter().find(|w| w.base == "雪").unwrap();
        assert_eq!(yuki.freq, Some(1800));
        assert_eq!(yuki.kanji[0].ch, "雪");
        assert!(w.iter().find(|w| w.base == "降る").unwrap().freq.is_none());
    }

    fn grammar_dict() -> Dict {
        let mut d = Dict::default();
        d.names.push("Gramática".into());
        for (t, g) in [("〜てくれる", "hacer algo por mi"), ("かどうか", "si... o no"), ("まで", "hasta"), ("〜てる", "forma coloquial de ている"), ("は", "tema")] {
            d.push_grammar(parse_row(&json!([t, "", "", "", 0, [g], 1, ""]), 0).unwrap());
        }
        d
    }

    #[test]
    fn gramatica_por_secuencias() {
        let d = grammar_dict();
        assert!(d.has_grammar());
        let tk = super::super::Tokenizer::new().unwrap();
        let toks = tk.tokens("駅まで一緒に行ってくれた");
        let g = d.grammar(&toks);
        let terms: Vec<&str> = g.iter().map(|g| g.term.as_str()).collect();
        assert!(terms.contains(&"まで"), "{terms:?} {:?}", toks.iter().map(|t| (t.parts.iter().map(|p| p.0.clone()).collect::<String>(), t.pos.clone(), t.base.clone())).collect::<Vec<_>>());
        assert!(terms.contains(&"〜てくれる"), "{terms:?} {:?}", toks.iter().map(|t| (t.parts.iter().map(|p| p.0.clone()).collect::<String>(), t.pos.clone(), t.base.clone())).collect::<Vec<_>>());
        // «は» suelta (una letra) no cuenta
        let g2 = d.grammar(&tk.tokens("僕は行く"));
        assert!(g2.is_empty(), "{:?}", g2.iter().map(|g| &g.term).collect::<Vec<_>>());
        let g3 = d.grammar(&tk.tokens("来るかどうか迷う"));
        assert!(g3.iter().any(|g| g.term == "かどうか"));
        // rangos validos y sin solapes
        let toks = tk.tokens("駅まで一緒に行ってくれた");
        let mut last_end = -1i64;
        for h in d.grammar(&toks) {
            assert!(h.toks[0] as i64 > last_end && h.toks[0] <= h.toks[1] && (h.toks[1] as usize) < toks.len());
            last_end = h.toks[1] as i64;
        }
        // un diccionario sin gramatica no detecta nada
        assert!(Dict::default().grammar(&toks).is_empty());
    }

    #[test]
    fn titulos_de_gramatica() {
        assert!(is_grammar_title("Dictionary of Basic Japanese Grammar"));
        assert!(is_grammar_title("日本語文法辞典"));
        assert!(is_grammar_title("Gramática japonesa"));
        assert!(!is_grammar_title("JMdict (Spanish)"));
        assert_eq!(gram_key("〜てくれる"), "てくれる");
        assert_eq!(gram_key("～ ている"), "ている");
    }

    #[test]
    fn busqueda_por_lectura() {
        let d = dict();
        assert_eq!(d.lookup("ゆき", None)[0].term, "雪");
        assert!(d.lookup("ない", None).is_empty());
    }
}
