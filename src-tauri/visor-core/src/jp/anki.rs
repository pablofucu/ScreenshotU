//! Notas de Anki (AnkiConnect): construye el `addNote` a partir de la palabra o la frase, y adivina el mapeo de campos
//! de un tipo de nota. Solo logica pura; las llamadas HTTP estan en `src/anki.rs`.
use super::{ReaderLine, WordInfo};
use crate::config::AnkiMap;
use serde_json::{json, Value};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// La frase con la palabra (token `tok`) en negrita; sin `tok`, la frase tal cual.
pub fn sentence_html(line: &ReaderLine, tok: Option<u32>) -> String {
    match tok.filter(|_| !line.toks.is_empty()) {
        Some(t) => line
            .toks
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let s: String = k.parts.iter().map(|p| p.0.as_str()).collect();
                if i as u32 == t {
                    format!("<b>{}</b>", esc(&s))
                } else {
                    esc(&s)
                }
            })
            .collect(),
        None => esc(&line.text),
    }
}

/// Acepciones de la palabra como lista HTML (primera entrada del diccionario, hasta 6 acepciones).
pub fn meaning_html(w: &WordInfo) -> String {
    let Some(d) = w.defs.first() else { return String::new() };
    let items: String = d.senses.iter().take(6).map(|s| format!("<li>{}</li>", esc(&s.glosses.join("; ")))).collect();
    format!("<ol>{items}</ol>")
}

/// Resumen de las palabras de la frase («palabra【lectura】 acepcion») para las notas de frase.
pub fn words_html(line: &ReaderLine) -> String {
    line.words
        .iter()
        .take(8)
        .filter_map(|w| {
            let d = w.defs.first()?;
            let g = d.senses.first()?.glosses.first()?;
            let rd = if d.reading.is_empty() || d.reading == w.base { String::new() } else { format!("【{}】", esc(&d.reading)) };
            Some(format!("{}{rd} {}", esc(&w.base), esc(g)))
        })
        .collect::<Vec<_>>()
        .join("<br>")
}

/// Todo lo que entra en una nota.
pub struct NoteInput<'a> {
    pub line: &'a ReaderLine,
    /// Palabra elegida (`None` = nota de la frase).
    pub word: Option<&'a WordInfo>,
    pub translation: Option<&'a str>,
    pub game_name: &'a str,
    /// Explicacion de DeepSeek de la frase, si se pidio.
    pub explanation: Option<&'a super::phrases::Explanation>,
    /// Rutas absolutas de la captura y del audio (ya guardados en disco) y el nombre con el que viajan a Anki.
    pub shot: Option<(String, String)>,
    pub audio: Option<(String, String)>,
}

/// Parametros de `addNote`. Los campos sin asignar en `map` (cadena vacia) no se envian.
pub fn build_add_note(deck: &str, model: &str, map: &AnkiMap, n: &NoteInput) -> Value {
    let mut fields = serde_json::Map::new();
    let mut put = |name: &str, v: String| {
        if !name.trim().is_empty() {
            fields.insert(name.trim().to_string(), Value::String(v));
        }
    };
    match n.word {
        Some(w) => {
            put(&map.word, w.base.clone());
            put(&map.reading, w.defs.first().map(|d| d.reading.clone()).filter(|r| *r != w.base).unwrap_or_default());
            put(&map.meaning, meaning_html(w));
            put(&map.sentence, sentence_html(n.line, Some(w.tok)));
        }
        None => {
            put(&map.word, esc(&n.line.text));
            put(&map.reading, String::new());
            put(&map.meaning, words_html(n.line));
            put(&map.sentence, sentence_html(n.line, None));
        }
    }
    put(&map.translation, n.translation.map(esc).unwrap_or_default());
    let mut note = json!({
        "deckName": deck,
        "modelName": model,
        "fields": fields,
        "options": { "allowDuplicate": false },
        "tags": ["visor-lectura", n.game_name.split_whitespace().collect::<Vec<_>>().join("_")],
    });
    if let (Some((path, filename)), false) = (&n.shot, map.shot.trim().is_empty()) {
        note["picture"] = json!([{ "path": path, "filename": filename, "fields": [map.shot.trim()] }]);
    }
    if let (Some((path, filename)), false) = (&n.audio, map.audio.trim().is_empty()) {
        note["audio"] = json!([{ "path": path, "filename": filename, "fields": [map.audio.trim()] }]);
    }
    json!({ "note": note })
}

/// Adivina que campo del tipo de nota es cada dato (por el nombre); los que no encuentra quedan vacios.
pub fn guess_map(fields: &[String]) -> AnkiMap {
    let find = |keys: &[&str], not: &[&str]| -> String {
        for k in keys {
            if let Some(f) = fields.iter().find(|f| {
                let l = f.to_lowercase();
                l.contains(k) && !not.iter().any(|n| l.contains(n))
            }) {
                return f.clone();
            }
        }
        String::new()
    };
    AnkiMap {
        word: find(&["expression", "word", "term", "vocab", "palabra", "front"], &["audio", "reading", "furigana", "picture", "sentence"]),
        reading: find(&["reading", "furigana", "lectura"], &["audio", "sentence"]),
        meaning: find(&["maindefinition", "primarydefinition", "definition", "meaning", "glossary", "significado", "back"], &["picture", "audio"]),
        sentence: find(&["sentence", "frase", "example"], &["audio", "furigana", "translation", "picture", "reading"]),
        translation: find(&["translation", "traduc"], &["audio"]),
        shot: find(&["picture", "image", "screenshot", "imagen", "captura"], &["definition", "audio"]),
        audio: find(&["sentenceaudio", "audio"], &["expression", "word"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jp::{Sense, Tok, WordDef};

    fn line() -> ReaderLine {
        ReaderLine {
            id: 1,
            at: 0,
            who: None,
            color: None,
            text: "雪が<降る>".into(),
            toks: vec![
                Tok { parts: vec![("雪".into(), Some("ゆき".into()))], pos: "n".into(), base: None },
                Tok { parts: vec![("が".into(), None)], pos: "p".into(), base: None },
                Tok { parts: vec![("<降る>".into(), None)], pos: "v".into(), base: None },
            ],
            words: vec![WordInfo {
                tok: 0,
                surface: "雪".into(),
                base: "雪".into(),
                pos: "n".into(),
                defs: vec![WordDef { term: "雪".into(), reading: "ゆき".into(), dict: "D".into(), senses: vec![Sense { tags: vec![], glosses: vec!["nieve".into(), "nevada".into()] }] }],
                freq: None,
                kanji: vec![],
            }],
            grammar: vec![],
        }
    }

    fn map() -> AnkiMap {
        AnkiMap { word: "Word".into(), reading: "Reading".into(), meaning: "Meaning".into(), sentence: "Sentence".into(), translation: "Tr".into(), shot: "Pic".into(), audio: "Aud".into() }
    }

    #[test]
    fn html_escapa_y_pone_negrita() {
        let l = line();
        assert_eq!(sentence_html(&l, Some(0)), "<b>雪</b>が&lt;降る&gt;");
        assert_eq!(sentence_html(&l, None), "雪が&lt;降る&gt;");
        assert_eq!(meaning_html(&l.words[0]), "<ol><li>nieve; nevada</li></ol>");
        assert_eq!(words_html(&l), "雪【ゆき】 nieve");
    }

    #[test]
    fn nota_de_palabra_con_adjuntos() {
        let l = line();
        let n = NoteInput { line: &l, word: Some(&l.words[0]), translation: Some("Nieva & mucho"), game_name: "Mi juego", explanation: None, shot: Some(("C:/a.jpg".into(), "a.jpg".into())), audio: Some(("C:/a.wav".into(), "a.wav".into())) };
        let v = build_add_note("Mazo", "Modelo", &map(), &n);
        let note = &v["note"];
        assert_eq!(note["deckName"], "Mazo");
        assert_eq!(note["fields"]["Word"], "雪");
        assert_eq!(note["fields"]["Reading"], "ゆき");
        assert_eq!(note["fields"]["Sentence"], "<b>雪</b>が&lt;降る&gt;");
        assert_eq!(note["fields"]["Tr"], "Nieva &amp; mucho");
        assert_eq!(note["picture"][0]["fields"][0], "Pic");
        assert_eq!(note["audio"][0]["path"], "C:/a.wav");
        assert_eq!(note["options"]["allowDuplicate"], false);
        assert_eq!(note["tags"][1], "Mi_juego");
    }

    #[test]
    fn nota_de_frase_y_campos_vacios() {
        let l = line();
        let mut m = map();
        m.shot.clear();
        m.audio.clear();
        m.reading.clear();
        let n = NoteInput { line: &l, word: None, translation: None, game_name: "J", explanation: None, shot: Some(("x".into(), "x.jpg".into())), audio: Some(("y".into(), "y.wav".into())) };
        let v = build_add_note("D", "M", &m, &n);
        let note = &v["note"];
        assert_eq!(note["fields"]["Word"], "雪が&lt;降る&gt;");
        assert_eq!(note["fields"]["Meaning"], "雪【ゆき】 nieve");
        assert!(note["fields"].get("Reading").is_none());
        assert!(note.get("picture").is_none() && note.get("audio").is_none()); // sin campo asignado no se adjunta
        assert_eq!(note["fields"]["Tr"], "");
    }

    #[test]
    fn adivina_campos() {
        let lapis: Vec<String> = ["Expression", "ExpressionFurigana", "ExpressionReading", "ExpressionAudio", "SelectionText", "MainDefinition", "DefinitionPicture", "Sentence", "SentenceFurigana", "SentenceAudio", "Picture", "Glossary"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let m = guess_map(&lapis);
        assert_eq!((m.word.as_str(), m.reading.as_str(), m.meaning.as_str()), ("Expression", "ExpressionReading", "MainDefinition"));
        assert_eq!((m.sentence.as_str(), m.shot.as_str(), m.audio.as_str(), m.translation.as_str()), ("Sentence", "Picture", "SentenceAudio", ""));
        let basic: Vec<String> = vec!["Front".into(), "Back".into()];
        let b = guess_map(&basic);
        assert_eq!((b.word.as_str(), b.meaning.as_str(), b.audio.as_str()), ("Front", "Back", ""));
    }
}
