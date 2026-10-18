//! Glosario de frases: las frases que el usuario ha traducido, explicado, marcado, enviado a Anki o grabado.
//! Las frases viven en la base (`db::data::phrase_*`; antes un archivo por juego `<config>/phrases/<juego>.json`);
//! la media (capturas, audio) sigue en disco, en `<config>/phrases/media/`.
//! Incluye un SRS tipo SM-2 simplificado para repasarlas.
use super::{GrammarHit, Tok, WordInfo};
use serde::{Deserialize, Serialize};

const DAY_MS: i64 = 86_400_000;
const AGAIN_MS: i64 = 10 * 60_000;

/// Una frase anterior (contexto para la IA y para el glosario).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CtxLine {
    pub who: Option<String>,
    pub text: String,
}

/// Pieza de la cadena de una explicacion: `kind` = "v" (vocabulario), "g" (gramatica) o "n" (neutra).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ChainPiece {
    pub kind: String,
    pub piece: String,
    pub meaning: String,
}

impl Default for ChainPiece {
    fn default() -> Self {
        ChainPiece { kind: "n".into(), piece: String::new(), meaning: String::new() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Explanation {
    pub translation: String,
    pub literal: String,
    pub chain: Vec<ChainPiece>,
    pub notes: Vec<String>,
}

/// Estado de repaso (SM-2 simplificado). `due` en milisegundos desde epoch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Review {
    pub due: i64,
    pub interval_days: f32,
    pub ease: f32,
    pub reps: u32,
    pub lapses: u32,
}

impl Default for Review {
    fn default() -> Self {
        Review { due: 0, interval_days: 0.0, ease: 2.5, reps: 0, lapses: 0 }
    }
}

/// Nota de un repaso.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grade {
    Again,
    Hard,
    Good,
    Easy,
}

impl Grade {
    /// 0 = Otra vez, 1 = Dificil, 2 = Bien, 3 = Facil.
    pub fn from_u8(n: u8) -> Option<Grade> {
        match n {
            0 => Some(Grade::Again),
            1 => Some(Grade::Hard),
            2 => Some(Grade::Good),
            3 => Some(Grade::Easy),
            _ => None,
        }
    }
}

impl Review {
    /// Aplica una nota y fija la siguiente fecha.
    pub fn grade(&mut self, g: Grade, now: i64) {
        match g {
            Grade::Again => {
                self.lapses += 1;
                self.reps = 0;
                self.interval_days = 0.0;
                self.ease = (self.ease - 0.2).max(1.3);
                self.due = now + AGAIN_MS;
                return;
            }
            Grade::Hard => {
                self.interval_days = (self.interval_days * 1.2).max(1.0);
                self.ease = (self.ease - 0.15).max(1.3);
            }
            Grade::Good => {
                self.interval_days = match self.reps {
                    0 => 1.0,
                    1 => 3.0,
                    _ => self.interval_days * self.ease,
                };
            }
            Grade::Easy => {
                self.interval_days = match self.reps {
                    0 => 4.0,
                    _ => (self.interval_days * self.ease * 1.3).max(4.0),
                };
                self.ease += 0.15;
            }
        }
        self.reps += 1;
        self.due = now + (self.interval_days as f64 * DAY_MS as f64) as i64;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Phrase {
    pub id: String,
    pub game_id: String,
    pub game_name: String,
    pub who: Option<String>,
    pub text: String,
    pub toks: Vec<Tok>,
    pub words: Vec<WordInfo>,
    pub grammar: Vec<GrammarHit>,
    /// Las 2-3 frases anteriores.
    pub context: Vec<CtxLine>,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub seen: u32,
    pub translation: Option<String>,
    pub explanation: Option<Explanation>,
    pub starred: bool,
    pub note: String,
    pub tags: Vec<String>,
    /// Rutas relativas a `<config>/phrases/media/`.
    pub shot: Option<String>,
    pub audio: Option<String>,
    /// Audio de la voz sintetica de Windows (aparte del grabado del juego).
    pub synth: Option<String>,
    pub anki_note_id: Option<i64>,
    pub review: Review,
}

impl Default for Phrase {
    fn default() -> Self {
        Phrase {
            id: String::new(),
            game_id: String::new(),
            game_name: String::new(),
            who: None,
            text: String::new(),
            toks: Vec::new(),
            words: Vec::new(),
            grammar: Vec::new(),
            context: Vec::new(),
            created_at: 0,
            last_seen_at: 0,
            seen: 1,
            translation: None,
            explanation: None,
            starred: false,
            note: String::new(),
            tags: Vec::new(),
            shot: None,
            audio: None,
            synth: None,
            anki_note_id: None,
            review: Review::default(),
        }
    }
}

/// Id estable de una frase: hash FNV-1a de 64 bits de `juego\0texto`, en hexadecimal.
pub fn phrase_id(game_id: &str, text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in game_id.bytes().chain(std::iter::once(0)).chain(text.trim().bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

pub fn get<'a>(list: &'a [Phrase], id: &str) -> Option<&'a Phrase> {
    list.iter().find(|p| p.id == id)
}

/// Inserta la frase o, si ya existe (mismo id), actualiza solo lo que cambia con una nueva visita
/// (ultima vez vista, palabras, contexto) y conserva traduccion, nota, audio, etc. Devuelve la frase guardada.
pub fn upsert(list: &mut Vec<Phrase>, mut p: Phrase, now: i64) -> &mut Phrase {
    if p.id.is_empty() {
        p.id = phrase_id(&p.game_id, &p.text);
    }
    match list.iter().position(|x| x.id == p.id) {
        Some(i) => {
            let old = &mut list[i];
            old.last_seen_at = now;
            if !p.toks.is_empty() {
                old.toks = p.toks;
                old.words = p.words;
                old.grammar = p.grammar;
            }
            if !p.context.is_empty() && old.context.is_empty() {
                old.context = p.context;
            }
            if old.who.is_none() {
                old.who = p.who;
            }
            if !p.game_name.is_empty() {
                old.game_name = p.game_name;
            }
            &mut list[i]
        }
        None => {
            p.created_at = now;
            p.last_seen_at = now;
            p.review.due = now;
            list.push(p);
            list.last_mut().unwrap()
        }
    }
}

/// La frase vuelve a salir en la lectura: suma una vista.
pub fn mark_seen(list: &mut [Phrase], id: &str, now: i64) -> bool {
    match list.iter_mut().find(|p| p.id == id) {
        Some(p) => {
            p.seen += 1;
            p.last_seen_at = now;
            true
        }
        None => false,
    }
}

/// Filtro de la lista del glosario.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Filter {
    /// Busca en el texto, la traduccion, la nota y las etiquetas.
    pub query: String,
    pub starred: bool,
    pub audio: bool,
    pub anki: bool,
    pub due: bool,
}

impl Filter {
    pub fn matches(&self, p: &Phrase, now: i64) -> bool {
        if (self.starred && !p.starred) || (self.audio && p.audio.is_none()) || (self.anki && p.anki_note_id.is_none()) || (self.due && p.review.due > now) {
            return false;
        }
        let q = self.query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        let hay = |s: &str| s.to_lowercase().contains(&q);
        hay(&p.text)
            || p.translation.as_deref().map(hay).unwrap_or(false)
            || hay(&p.note)
            || p.tags.iter().any(|t| hay(t))
            || p.who.as_deref().map(hay).unwrap_or(false)
    }
}

/// Frases que cumplen el filtro, las vistas mas recientemente primero.
pub fn list<'a>(all: &'a [Phrase], f: &Filter, now: i64) -> Vec<&'a Phrase> {
    let mut v: Vec<&Phrase> = all.iter().filter(|p| f.matches(p, now)).collect();
    v.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at).then(b.created_at.cmp(&a.created_at)));
    v
}

/// Pendientes de repaso (`due <= now`), las mas atrasadas primero.
pub fn due(all: &[Phrase], now: i64) -> Vec<&Phrase> {
    let mut v: Vec<&Phrase> = all.iter().filter(|p| p.review.due <= now).collect();
    v.sort_by_key(|p| p.review.due);
    v
}

/// Califica un repaso; devuelve el estado nuevo.
pub fn grade(list: &mut [Phrase], id: &str, g: Grade, now: i64) -> Option<Review> {
    let p = list.iter_mut().find(|p| p.id == id)?;
    p.review.grade(g, now);
    Some(p.review.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ph(game: &str, text: &str) -> Phrase {
        Phrase { game_id: game.into(), game_name: "Demo".into(), text: text.into(), ..Default::default() }
    }

    #[test]
    fn id_estable_y_distinto() {
        assert_eq!(phrase_id("a", "雪"), phrase_id("a", "雪 "));
        assert_ne!(phrase_id("a", "雪"), phrase_id("b", "雪"));
        assert_ne!(phrase_id("a", "雪"), phrase_id("a", "雨"));
        assert_eq!(phrase_id("a", "x").len(), 16);
    }

    #[test]
    fn upsert_no_duplica_y_conserva() {
        let mut l = Vec::new();
        let id = upsert(&mut l, ph("g", "雪が降る"), 100).id.clone();
        l[0].translation = Some("Nieva".into());
        l[0].starred = true;
        let p = upsert(&mut l, ph("g", "雪が降る"), 500);
        assert_eq!(p.id, id);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].created_at, 100);
        assert_eq!(l[0].last_seen_at, 500);
        assert_eq!(l[0].translation.as_deref(), Some("Nieva"));
        assert!(l[0].starred);
        assert!(mark_seen(&mut l, &id, 900));
        assert_eq!(l[0].seen, 2);
        assert!(!mark_seen(&mut l, "nope", 1));
    }

    #[test]
    fn repaso_progresa_y_baja() {
        let mut r = Review::default();
        let now = 1_000_000;
        r.grade(Grade::Good, now);
        assert_eq!((r.reps, r.interval_days), (1, 1.0));
        assert_eq!(r.due, now + DAY_MS);
        r.grade(Grade::Good, now);
        assert_eq!(r.interval_days, 3.0);
        r.grade(Grade::Good, now);
        assert!((r.interval_days - 7.5).abs() < 1e-3);
        r.grade(Grade::Again, now);
        assert_eq!((r.reps, r.lapses, r.interval_days), (0, 1, 0.0));
        assert_eq!(r.due, now + AGAIN_MS);
        assert!(r.ease < 2.5 && r.ease >= 1.3);
        let mut e = Review::default();
        e.grade(Grade::Easy, now);
        assert_eq!(e.interval_days, 4.0);
        assert!(e.ease > 2.5);
        for _ in 0..30 {
            e.grade(Grade::Hard, now);
        }
        assert!(e.ease >= 1.3);
    }

    #[test]
    fn filtros_y_pendientes() {
        let mut l = Vec::new();
        upsert(&mut l, ph("g", "雪が降る"), 100);
        upsert(&mut l, ph("g", "駅まで行く"), 200);
        l[0].starred = true;
        l[0].translation = Some("Nieva mucho".into());
        l[1].audio = Some("a.wav".into());
        l[1].review.due = 10_000;
        let f = |q: &str, s: bool, a: bool, d: bool| Filter { query: q.into(), starred: s, audio: a, due: d, ..Default::default() };
        assert_eq!(list(&l, &f("", false, false, false), 5000).len(), 2);
        assert_eq!(list(&l, &f("", false, false, false), 5000)[0].text, "駅まで行く"); // la mas reciente primero
        assert_eq!(list(&l, &f("nieva", false, false, false), 5000).len(), 1);
        assert_eq!(list(&l, &f("", true, false, false), 5000)[0].text, "雪が降る");
        assert_eq!(list(&l, &f("", false, true, false), 5000)[0].text, "駅まで行く");
        // pendientes: la primera (due = 100) si; la segunda (due = 10000) aun no
        assert_eq!(list(&l, &f("", false, false, true), 5000).len(), 1);
        assert_eq!(due(&l, 5000).len(), 1);
        assert_eq!(due(&l, 20_000).len(), 2);
        assert_eq!(due(&l, 20_000)[0].text, "雪が降る");
        let id = l[0].id.clone();
        assert!(grade(&mut l, &id, Grade::Good, 5000).is_some());
        assert_eq!(due(&l, 5000).len(), 0);
        assert!(grade(&mut l, "x", Grade::Good, 5000).is_none());
    }

    #[test]
    fn ida_y_vuelta_por_la_base() {
        use crate::db::data::*;
        let db = crate::db::Db::open_memory().unwrap();
        assert!(db.with(|c| phrases_load_game(c, "steam:1")).unwrap().is_empty());
        let mut l = Vec::new();
        let fr = upsert(&mut l, ph("steam:1", "雪"), 1);
        fr.explanation = Some(Explanation { translation: "Nieve".into(), chain: vec![ChainPiece { kind: "v".into(), piece: "雪".into(), meaning: "nieve".into() }], ..Default::default() });
        fr.tags = vec!["b".into(), "a".into()];
        fr.translation = Some("Nieve".into());
        fr.starred = true;
        fr.anki_note_id = Some(42);
        fr.shot = Some("s.jpg".into());
        fr.review.grade(Grade::Good, 1000);
        fr.context = vec![CtxLine { who: Some("A".into()), text: "antes".into() }];
        db.with(|c| phrase_save(c, &l[0])).unwrap();
        assert_eq!(db.with(|c| phrases_load_game(c, "steam:1")).unwrap(), l);
        assert_eq!(db.with(|c| phrase_get(c, "steam:1", &l[0].id)).unwrap().as_ref(), Some(&l[0]));
        assert_eq!(db.with(|c| phrase_get(c, "otro", &l[0].id)).unwrap(), None);
        // Guardar otra vez reemplaza la fila y sus etiquetas.
        l[0].tags = vec!["z".into()];
        l[0].note = "nota".into();
        db.with(|c| phrase_save(c, &l[0])).unwrap();
        assert_eq!(db.with(phrases_load_all).unwrap(), l);
        // Otra frase de otro juego; la cuenta de pendientes distingue juegos y fechas.
        let mut l2 = Vec::new();
        upsert(&mut l2, ph("g2", "駅まで行く"), 5);
        db.with(|c| phrase_save(c, &l2[0])).unwrap();
        assert_eq!(db.with(phrases_load_all).unwrap().len(), 2);
        assert_eq!(db.with(|c| phrases_due_count(c, None, 10)).unwrap(), 1); // la nueva (due = 5); la repasada vence en 1 dia
        assert_eq!(db.with(|c| phrases_due_count(c, Some("steam:1"), 10)).unwrap(), 0);
        assert_eq!(db.with(|c| phrases_due_count(c, Some("steam:1"), i64::MAX)).unwrap(), 1);
        // Borrar arrastra las etiquetas.
        assert!(db.with(|c| phrase_delete(c, "steam:1", &l[0].id)).unwrap());
        assert!(!db.with(|c| phrase_delete(c, "steam:1", &l[0].id)).unwrap());
        let n: i64 = db.with(|c| c.query_row("SELECT count(*) FROM phrase_tags", [], |r| r.get(0))).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn campos_ausentes_en_json_usan_valores_por_defecto() {
        let r: Vec<Phrase> = serde_json::from_str(r#"[{"id":"x","text":"a"}]"#).unwrap();
        assert_eq!((r[0].seen, r[0].review.ease), (1, 2.5));
    }
}
