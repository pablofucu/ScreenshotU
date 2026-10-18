//! Glosario de frases en la app: guarda y lee las frases (`visor_core::jp::phrases`) de la partida en curso y de
//! cualquier juego, y avisa a la pagina de lectura. Una frase entra al glosario cuando el usuario hace algo con ella
//! (traducir, explicar, ★, Anki, grabar audio); nada se guarda solo. Las frases viven en la base (`visor.db`);
//! solo la media (capturas y audio) sigue en `<config>/phrases/media/`.
use crate::state::{log_db, log_db_err, AppState};
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use visor_core::db::{data as dbd, Db};
use visor_core::jp::phrases::{self, CtxLine, Filter, Grade, Phrase};
use visor_core::jp::ReaderLine;

/// Serializa las lecturas-modificaciones-escrituras de las frases (varios hilos: IA, audio, Anki).
static LOCK: Mutex<()> = Mutex::new(());

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub fn dir(app: &AppHandle) -> PathBuf {
    app.state::<AppState>().config_dir.join("phrases")
}

/// Capturas y audio de las frases.
pub fn media_dir(app: &AppHandle) -> PathBuf {
    dir(app).join("media")
}

fn db(app: &AppHandle) -> Arc<Db> {
    app.state::<AppState>().db.clone()
}

/// Frase (sin guardar) de una linea de la partida en curso, con las 3 frases anteriores como contexto.
fn phrase_of(line: &ReaderLine, game_id: &str, game_name: &str) -> Phrase {
    let context: Vec<CtxLine> = crate::reader::server::lines_before(line.id, 3)
        .into_iter()
        .map(|l| CtxLine { who: l.who, text: l.text })
        .collect();
    Phrase {
        game_id: game_id.to_string(),
        game_name: game_name.to_string(),
        who: line.who.clone(),
        text: line.text.clone(),
        toks: line.toks.clone(),
        words: line.words.clone(),
        grammar: line.grammar.clone(),
        context,
        ..Default::default()
    }
}

/// Frase del glosario correspondiente a una linea de la partida en curso, si ya esta guardada.
pub fn find_by_line(app: &AppHandle, line_id: u64) -> Option<Phrase> {
    let line = crate::reader::server::line(line_id)?;
    let (game_id, _) = crate::novel::current_game()?;
    let id = phrases::phrase_id(&game_id, &line.text);
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    log_db("glosario", db(app).with(|c| dbd::phrase_get(c, &game_id, &id)))
}

/// Crea (o actualiza) la frase de una linea en el glosario, aplica `f` y la guarda. Devuelve la frase guardada.
pub fn update_line<R>(app: &AppHandle, line_id: u64, f: impl FnOnce(&mut Phrase) -> R) -> Option<(Phrase, R)> {
    let line = crate::reader::server::line(line_id)?;
    let (game_id, game_name) = crate::novel::current_game()?;
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let db = db(app);
    let now = now_ms();
    let fresh = phrase_of(&line, &game_id, &game_name);
    let id = phrases::phrase_id(&game_id, &line.text);
    let mut list: Vec<Phrase> = match db.with(|c| dbd::phrase_get(c, &game_id, &id)) {
        Ok(found) => found.into_iter().collect(),
        Err(e) => {
            eprintln!("No se pudo leer el glosario: {e:#}");
            return None;
        }
    };
    let existed = !list.is_empty();
    let p = phrases::upsert(&mut list, Phrase { id, ..fresh }, now);
    let r = f(p);
    let out = p.clone();
    if let Err(e) = db.with(|c| dbd::phrase_save(c, &out)) {
        eprintln!("No se pudo guardar el glosario: {e:#}");
        return None;
    }
    if !existed {
        crate::novel::readstats::on_phrase_added(app, &game_id);
    }
    Some((out, r))
}

/// Mensaje `phrase` para la pagina: estado de la frase de una linea (y su traduccion/explicacion guardadas).
pub fn push_state(p: &Phrase, line_id: u64) {
    crate::reader::server::send(json!({
        "type": "phrase",
        "lineId": line_id,
        "starred": p.starred,
        "inAnki": p.anki_note_id.is_some(),
        "hasAudio": p.audio.is_some() && crate::audio::recorded_this_session(&p.id),
        "hasSynth": p.synth.is_some(),
        "translation": p.translation,
        "explanation": p.explanation,
    }));
}

/// Una linea nueva de la partida: si ya esta en el glosario se cuenta una vista y se manda su estado a la pagina
/// (asi vuelve la traduccion guardada, sin llamar a la IA).
pub fn on_line(app: &AppHandle, line: &ReaderLine) {
    let Some((game_id, _)) = crate::novel::current_game() else { return };
    let id = phrases::phrase_id(&game_id, &line.text);
    let found = {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let db = db(app);
        let mut list: Vec<Phrase> = log_db("glosario", db.with(|c| dbd::phrase_get(c, &game_id, &id))).into_iter().collect();
        if phrases::mark_seen(&mut list, &id, now_ms()) {
            log_db_err("glosario", db.with(|c| dbd::phrase_save(c, &list[0])));
            list.pop()
        } else {
            None
        }
    };
    if let Some(p) = found {
        push_state(&p, line.id);
    }
}

/// ★ de la pagina: alterna la frase destacada.
pub fn toggle_star(app: &AppHandle, line_id: u64) {
    if let Some((p, _)) = update_line(app, line_id, |p| p.starred = !p.starred) {
        push_state(&p, line_id);
        // Al destacar una frase se guarda tambien la captura del momento.
        if p.starred && p.shot.is_none() {
            crate::shots::keep_for_phrase(app, line_id);
        }
    }
}

/* ------------------------------------------------------------------ */
/* Consultas generales (pagina de lectura y app principal)              */
/* ------------------------------------------------------------------ */

/// Frases de un juego (o de todos con `None`) que cumplen el filtro, las vistas mas recientemente primero.
pub fn list(app: &AppHandle, game_id: Option<&str>, f: &Filter) -> Vec<Phrase> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let all = log_db(
        "glosario",
        db(app).with(|c| match game_id {
            Some(g) => dbd::phrases_load_game(c, g),
            None => dbd::phrases_load_all(c),
        }),
    );
    phrases::list(&all, f, now_ms()).into_iter().cloned().collect()
}

/// Cuantas frases tocan repasar ahora (de un juego o de todos).
pub fn due_count(app: &AppHandle, game_id: Option<&str>) -> usize {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    log_db("glosario", db(app).with(|c| dbd::phrases_due_count(c, game_id, now_ms())))
}

/// Cambia una frase por id (se guarda solo su fila).
pub fn modify(app: &AppHandle, game_id: &str, id: &str, f: impl FnOnce(&mut Phrase)) -> Option<Phrase> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let db = db(app);
    let mut p = log_db("glosario", db.with(|c| dbd::phrase_get(c, game_id, id)))?;
    f(&mut p);
    db.with(|c| dbd::phrase_save(c, &p)).map_err(|e| eprintln!("No se pudo guardar el glosario: {e:#}")).ok()?;
    Some(p)
}

pub fn get(app: &AppHandle, game_id: &str, id: &str) -> Option<Phrase> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    log_db("glosario", db(app).with(|c| dbd::phrase_get(c, game_id, id)))
}

pub fn delete(app: &AppHandle, game_id: &str, id: &str) -> bool {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let db = db(app);
    let Some(gone) = log_db("glosario", db.with(|c| dbd::phrase_get(c, game_id, id))) else { return false };
    if !matches!(db.with(|c| dbd::phrase_delete(c, game_id, id)), Ok(true)) {
        return false;
    }
    let gone = [gone];
    // La media de la frase se borra con ella.
    let m = media_dir(app);
    for p in gone {
        for f in [p.shot, p.audio, p.synth].into_iter().flatten() {
            let _ = std::fs::remove_file(m.join(f));
        }
    }
    true
}

pub fn grade(app: &AppHandle, game_id: &str, id: &str, g: Grade) -> Option<Phrase> {
    modify(app, game_id, id, |p| p.review.grade(g, now_ms()))
}

/* ------------------------------------------------------------------ */
/* Pagina de lectura: glosario                                          */
/* ------------------------------------------------------------------ */

fn filter_of(v: &serde_json::Value) -> Filter {
    v.get("filter").cloned().and_then(|f| serde_json::from_value(f).ok()).unwrap_or_default()
}

/// Respuesta `glossary` a la pagina: las frases del juego en curso que cumplen el filtro y cuantas tocan repasar.
pub fn push_list(app: &AppHandle, filter: &Filter) {
    let Some((game_id, _)) = crate::novel::current_game() else { return };
    let list = list(app, Some(&game_id), filter);
    let due = due_count(app, Some(&game_id));
    crate::reader::server::send(json!({ "type": "glossary", "phrases": list, "dueCount": due }));
}

fn apply_patch(p: &mut Phrase, patch: &serde_json::Value) {
    if let Some(s) = patch.get("starred").and_then(|x| x.as_bool()) {
        p.starred = s;
    }
    if let Some(n) = patch.get("note").and_then(|x| x.as_str()) {
        p.note = n.trim().to_string();
    }
    if let Some(t) = patch.get("tags").and_then(|x| x.as_array()) {
        p.tags = t.iter().filter_map(|x| x.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }
}

/// Mensajes de la pagina: `glossaryList`, `phraseUpdate`, `phraseDelete`, `phraseGrade`, `phraseAnki`.
pub fn on_page_cmd(app: &AppHandle, kind: &str, v: &serde_json::Value) {
    let Some((game_id, _)) = crate::novel::current_game() else { return };
    let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
    match kind {
        "phraseUpdate" => {
            modify(app, &game_id, id, |p| apply_patch(p, v));
        }
        "phraseDelete" => {
            delete(app, &game_id, id);
        }
        "phraseGrade" => {
            if let Some(g) = v.get("grade").and_then(|x| x.as_u64()).and_then(|g| Grade::from_u8(g as u8)) {
                grade(app, &game_id, id, g);
            }
        }
        "phraseAnki" => return crate::anki::send_stored(app, id.to_string()),
        _ => {}
    }
    push_list(app, &filter_of(v));
}

/* ------------------------------------------------------------------ */
/* Comandos de la app principal (pantalla de frases y repaso)           */
/* ------------------------------------------------------------------ */

/// Frases de un juego (o de todos) segun el filtro, las vistas mas recientemente primero.
#[tauri::command]
pub fn phrases_list(app: AppHandle, game_id: Option<String>, filter: Option<Filter>) -> Vec<Phrase> {
    list(&app, game_id.as_deref(), &filter.unwrap_or_default())
}

#[tauri::command]
pub fn phrases_get(app: AppHandle, game_id: String, id: String) -> Option<Phrase> {
    get(&app, &game_id, &id)
}

/// Cambia ★, nota o etiquetas de una frase (`patch` = `{starred?, note?, tags?}`).
#[tauri::command]
pub fn phrases_update(app: AppHandle, game_id: String, id: String, patch: serde_json::Value) -> Result<Phrase, String> {
    modify(&app, &game_id, &id, |p| apply_patch(p, &patch)).ok_or_else(|| "No se encontró la frase".to_string())
}

/// Califica un repaso: 0 = Otra vez, 1 = Dificil, 2 = Bien, 3 = Facil.
#[tauri::command]
pub fn phrases_grade(app: AppHandle, game_id: String, id: String, grade_n: u8) -> Result<Phrase, String> {
    let g = Grade::from_u8(grade_n).ok_or("Nota de repaso desconocida")?;
    grade(&app, &game_id, &id, g).ok_or_else(|| "No se encontró la frase".to_string())
}

#[tauri::command]
pub fn phrases_delete(app: AppHandle, game_id: String, id: String) -> bool {
    delete(&app, &game_id, &id)
}

#[tauri::command]
pub fn phrases_due_count(app: AppHandle, game_id: Option<String>) -> usize {
    due_count(&app, game_id.as_deref())
}

/// Carpeta de capturas y audio de las frases (la interfaz arma las URL con `convertFileSrc`).
#[tauri::command]
pub fn phrases_media_dir(app: AppHandle) -> String {
    media_dir(&app).to_string_lossy().into_owned()
}
