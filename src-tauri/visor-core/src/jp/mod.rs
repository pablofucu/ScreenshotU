//! Texto japones de las novelas: filtros, nombres, tokenizador con furigana y preferencias por juego.
//! (Agente B.) Los tipos de este archivo forman parte del contrato de `docs/novelas-japonesas.md`.
use serde::{Deserialize, Serialize};

pub mod anki;
pub mod anki_tpl;
pub mod dict;
pub mod filters;
pub mod known;
pub mod names;
pub mod phrases;
pub mod prompt;
pub mod readstats;
pub mod score;
pub mod tokenize;

pub use dict::Dict;
pub use filters::LineDeduper;
pub use names::NameJoiner;
pub use tokenize::Tokenizer;

/// Categoria gramatical simplificada para colorear: n, v, a, adv, p, aux, i, x.
pub type Pos = String;

/// Una palabra: trozos con su furigana (`None` = sin ruby) y su categoria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tok {
    pub parts: Vec<(String, Option<String>)>,
    pub pos: Pos,
    /// Forma de diccionario (降り続いて -> 降り続く) cuando difiere de la superficie.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
}

/// Una acepcion: etiquetas (n, v5r...) y glosas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sense {
    pub tags: Vec<String>,
    pub glosses: Vec<String>,
}

/// Entrada de diccionario para una palabra.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WordDef {
    pub term: String,
    pub reading: String,
    pub dict: String,
    pub senses: Vec<Sense>,
}

/// Palabra principal de una linea (indice del token en `toks`) con sus entradas de diccionario.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WordInfo {
    pub tok: u32,
    pub surface: String,
    pub base: String,
    pub pos: Pos,
    pub defs: Vec<WordDef>,
    /// Puesto/valor de frecuencia (diccionario `freq` tipo JPDB); menor = mas comun.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freq: Option<u32>,
    /// Kanji de la palabra con su significado (KANJIDIC).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kanji: Vec<KanjiInfo>,
}

/// Un kanji de un diccionario de kanji de Yomitan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KanjiInfo {
    pub ch: String,
    pub meanings: Vec<String>,
    pub onyomi: Vec<String>,
    pub kunyomi: Vec<String>,
}

/// Punto gramatical detectado: tokens `toks[0]..=toks[1]` de la linea y su entrada del diccionario de gramatica.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GrammarHit {
    pub toks: [u32; 2],
    pub term: String,
    pub defs: Vec<WordDef>,
}

/// Linea final que se envia a la pagina de lectura.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReaderLine {
    pub id: u64,
    pub at: i64,
    pub who: Option<String>,
    pub color: Option<String>,
    pub text: String,
    pub toks: Vec<Tok>,
    #[serde(default)]
    pub words: Vec<WordInfo>,
    #[serde(default)]
    pub grammar: Vec<GrammarHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegexRule {
    pub pattern: String,
    pub replace: String,
    pub on: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Filters {
    /// AAAABBBBCCCC -> ABC (deteccion automatica del numero de repeticiones, como `_2` de LunaTranslator).
    pub dedupe_chars: bool,
    pub join_lines: bool,
    pub dedupe_lines: bool,
    pub split_name: bool,
    /// ABCDABCD -> ABCD (`_3` de LunaTranslator).
    pub dedupe_phrase: bool,
    /// S1S1S1S2S2S2 -> S1S2 (`_3_2`).
    pub dedupe_segments: bool,
    /// AABABCABCD -> ABCD: texto que se redibuja creciendo dentro de una misma salida (`_13EX`).
    pub dedupe_incremental: bool,
    /// Caracteres de control (salvo saltos de linea).
    pub remove_control: bool,
    /// Etiquetas `<...>` (`_4`).
    pub remove_tags: bool,
    /// Llaves `{...}` de motores tipo `{漢字/かんじ}` (`_1`).
    pub remove_braces: bool,
    pub regex: Vec<RegexRule>,
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            dedupe_chars: true,
            join_lines: true,
            dedupe_lines: true,
            split_name: true,
            dedupe_phrase: true,
            dedupe_segments: false,
            dedupe_incremental: false,
            remove_control: true,
            remove_tags: false,
            remove_braces: false,
            regex: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub name: String,
    /// `#rrggbb`; `None` = gris por defecto.
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ReaderPrefs {
    /// "all" | "off"
    pub furigana: String,
    pub pos_colors: bool,
    pub font_size: u32,
}

impl Default for ReaderPrefs {
    fn default() -> Self {
        ReaderPrefs { furigana: "all".into(), pos_colors: true, font_size: 30 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct OutPrefs {
    pub reader: bool,
    pub clipboard: bool,
}

impl Default for OutPrefs {
    fn default() -> Self {
        OutPrefs { reader: true, clipboard: false }
    }
}

/// Configuracion de texthook y lectura de un juego (se guarda en `novels.json`, por id de juego).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NovelPrefs {
    pub hook_codes: Vec<String>,
    pub thread_key: Option<String>,
    pub name_thread_key: Option<String>,
    pub filters: Filters,
    pub characters: Vec<Character>,
    pub reader: ReaderPrefs,
    pub out: OutPrefs,
}
