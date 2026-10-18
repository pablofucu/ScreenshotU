//! Anki por AnkiConnect (complemento de Anki, por defecto `http://127.0.0.1:8765`): estado para la cabecera de la
//! pagina de lectura, envio de notas con la captura y el audio, y comandos de Ajustes (probar conexion, campos).
use crate::glossary;
use crate::state::AppState;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use visor_core::config::AnkiMap;
use visor_core::jp::anki::{build_add_note, guess_map, NoteInput};
use visor_core::jp::anki_tpl::{self, OWN_MODEL};
use visor_core::jp::phrases::{Explanation, Phrase};

fn cfg(app: &AppHandle) -> visor_core::Config {
    app.state::<AppState>().config.lock().unwrap().clone()
}

/// Llamada a AnkiConnect (version 6). Devuelve `result` o el texto del error.
fn call(url: &str, action: &str, params: Value) -> Result<Value, String> {
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(3)).timeout(Duration::from_secs(30)).build();
    let resp = agent
        .post(url)
        .send_json(json!({ "action": action, "version": 6, "params": params }))
        .map_err(|e| match e {
            ureq::Error::Status(c, _) => format!("AnkiConnect respondió {c}"),
            ureq::Error::Transport(t) => format!("No se encuentra AnkiConnect en {url}: abre Anki (con el complemento AnkiConnect instalado). ({t})"),
        })?;
    let v: Value = resp.into_json().map_err(|e| format!("Respuesta ilegible de AnkiConnect: {e}"))?;
    match v.get("error") {
        Some(Value::String(e)) => Err(friendly(e)),
        _ => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
    }
}

/// Crea el mazo si no existe (AnkiConnect `createDeck`; si ya existe no hace nada).
fn ensure_deck(url: &str, deck: &str) -> Result<(), String> {
    if strings(&call(url, "deckNames", json!({}))?).iter().any(|d| d == deck) {
        return Ok(());
    }
    call(url, "createDeck", json!({ "deck": deck })).map(|_| ())
}

/// Crea el tipo de nota propio («Visor · Japonés») si no existe, y le anade los campos que le falten.
fn ensure_model(url: &str, style: &str) -> Result<(), String> {
    if !strings(&call(url, "modelNames", json!({}))?).iter().any(|m| m == OWN_MODEL) {
        return call(url, "createModel", anki_tpl::create_model_params(style)).map(|_| ());
    }
    let have = strings(&call(url, "modelFieldNames", json!({ "modelName": OWN_MODEL }))?);
    for (i, f) in anki_tpl::FIELDS.iter().filter(|f| !have.iter().any(|h| h == *f)).enumerate() {
        call(url, "modelFieldAdd", json!({ "modelName": OWN_MODEL, "fieldName": f, "index": have.len() + i }))?;
    }
    Ok(())
}

/// Aplica el diseño elegido al tipo de nota propio (lo crea si hace falta). Sobrescribe sus plantillas y su CSS.
fn apply_style(url: &str, style: &str) -> Result<(), String> {
    ensure_model(url, style)?;
    let (tpl, css) = anki_tpl::update_model_params(style);
    call(url, "updateModelTemplates", tpl)?;
    call(url, "updateModelStyling", css).map(|_| ())
}

/// `addNote` con el tipo de nota propio (si `model` esta vacio o es el propio) o con uno ajeno y el mapeo de Ajustes.
fn add_note(c: &visor_core::Config, deck: &str, model: &str, input: &NoteInput) -> Result<i64, String> {
    ensure_deck(&c.anki_url, deck)?;
    let params = if model.is_empty() || model == OWN_MODEL {
        ensure_model(&c.anki_url, &c.anki_style)?;
        anki_tpl::build_own_note(deck, input)
    } else {
        build_add_note(deck, model, &c.anki_map, input)
    };
    call(&c.anki_url, "addNote", params)?.as_i64().ok_or_else(|| "Anki no devolvió el id de la nota.".to_string())
}

fn friendly(e: &str) -> String {
    let l = e.to_lowercase();
    if l.contains("duplicate") {
        "Esa nota ya existe en Anki (duplicada).".into()
    } else if l.contains("model was not found") || l.contains("model not found") {
        "El tipo de nota no existe en Anki: elígelo en la ventana de envío o en Ajustes › Lectura.".into()
    } else if l.contains("deck was not found") || l.contains("deck not found") {
        "El mazo no existe en Anki: elígelo en la ventana de envío o en Ajustes › Lectura.".into()
    } else if l.contains("field") {
        format!("Algún campo del mapeo no existe en el tipo de nota ({e}). Revisa Ajustes › Lectura.")
    } else {
        format!("Anki: {e}")
    }
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default()
}

/// Estado de Anki para la pagina: `{ok, deck, model, decks, models, error?}`.
pub fn state(app: &AppHandle) -> Value {
    let c = cfg(app);
    let pick = |want: &str, list: &[String]| if list.iter().any(|x| x == want) { want.to_string() } else { list.first().cloned().unwrap_or_else(|| want.to_string()) };
    // El mazo configurado se respeta aunque aun no exista: se crea al enviar la primera nota (o con «Crear mazo»).
    let deck_of = |list: &[String]| if c.anki_deck.is_empty() { pick("", list) } else { c.anki_deck.clone() };
    // Sin tipo de nota en Ajustes (o si el elegido ya no existe), el propio de la app: se crea al enviar.
    let model_of = |list: &[String]| if list.iter().any(|m| *m == c.anki_model) { c.anki_model.clone() } else { OWN_MODEL.to_string() };
    let r = call(&c.anki_url, "version", json!({}))
        .and_then(|_| Ok((strings(&call(&c.anki_url, "deckNames", json!({}))?), strings(&call(&c.anki_url, "modelNames", json!({}))?))));
    match r {
        Ok((decks, models)) => json!({ "ok": true, "deck": deck_of(&decks), "model": model_of(&models), "decks": decks, "models": models }),
        Err(e) => json!({ "ok": false, "deck": c.anki_deck, "model": c.anki_model, "decks": [], "models": [], "error": e }),
    }
}

/// Envia una nota (palabra o frase) con captura y audio. `req` = mensaje `anki` de la pagina. Responde con `ankiResult`.
pub fn send_note(app: &AppHandle, req: Value) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("anki-send".into()).spawn(move || {
        let line_id = req.get("lineId").and_then(Value::as_u64).unwrap_or(0);
        let word = req.get("word").and_then(Value::as_str).map(String::from);
        let reply = match send_inner(&app, &req, line_id) {
            Ok(id) => json!({ "type": "ankiResult", "lineId": line_id, "word": word, "ok": true, "noteId": id }),
            Err(e) => json!({ "type": "ankiResult", "lineId": line_id, "word": word, "ok": false, "error": e }),
        };
        crate::reader::server::send(reply);
    });
}

/// Explicacion para la tarjeta del tipo de nota propio: la guardada o, si falta y hay clave de DeepSeek, una nueva
/// (con un aviso en la pagina, porque tarda unos segundos). Si DeepSeek falla, la tarjeta se envia igual sin ella.
fn explanation_for(app: &AppHandle, model: &str, p: &Phrase) -> Option<Explanation> {
    if !(model.is_empty() || model == OWN_MODEL) {
        return None;
    }
    if p.explanation.is_some() {
        return p.explanation.clone();
    }
    if crate::ai::explain_blocking_available(app) {
        crate::reader::server::send(json!({ "type": "toast", "msg": "Pidiendo la explicación de la frase a DeepSeek…" }));
    }
    match crate::ai::explain_blocking(app, &p.context, p.who.as_deref(), &p.text, p.translation.as_deref())? {
        Ok(e) => Some(e),
        Err(e) => {
            crate::reader::server::send(json!({ "type": "toast", "msg": format!("Tarjeta sin explicación: {e}"), "bad": true }));
            None
        }
    }
}

/// Guarda en la frase del glosario la explicacion recien pedida (y su traduccion si no tenia).
fn store_explanation(p: &mut Phrase, e: &Explanation) {
    if p.translation.is_none() && !e.translation.is_empty() {
        p.translation = Some(e.translation.clone());
    }
    p.explanation = Some(e.clone());
}

fn send_inner(app: &AppHandle, req: &Value, line_id: u64) -> Result<i64, String> {
    let line = crate::reader::server::line(line_id).ok_or("Esa frase ya no está en el historial de la página.")?;
    let c = cfg(app);
    let tok = req.get("tok").and_then(Value::as_u64).map(|t| t as u32);
    let word = tok.and_then(|t| line.words.iter().find(|w| w.tok == t)).or_else(|| {
        let b = req.get("word").and_then(Value::as_str)?;
        line.words.iter().find(|w| w.base == b)
    });
    let deck = req.get("deck").and_then(Value::as_str).filter(|d| !d.is_empty()).map(String::from).unwrap_or(c.anki_deck.clone());
    // Tipo de nota vacio = el propio de la app.
    let model = req.get("model").and_then(Value::as_str).filter(|d| !d.is_empty()).map(String::from).unwrap_or(c.anki_model.clone());
    if deck.is_empty() {
        return Err("Elige el mazo.".into());
    }
    // La frase entra al glosario; su captura se guarda en disco ahora.
    let shot_line = req.get("shotLineId").and_then(Value::as_u64);
    let (mut phrase, _) = glossary::update_line(app, line_id, |_| ()).ok_or("No se pudo guardar la frase en el glosario.")?;
    let new_ex = explanation_for(app, &model, &phrase).filter(|_| phrase.explanation.is_none());
    if let Some(e) = &new_ex {
        store_explanation(&mut phrase, e);
    }
    let media = glossary::media_dir(app);
    let mut shot = None;
    if let Some(sl) = shot_line {
        if let Some(f) = crate::shots::save(app, sl, &phrase.id) {
            shot = Some(f);
        }
    }
    let shot_file = shot.clone().or(phrase.shot.clone().filter(|_| shot_line.is_some()));
    let audio_file = match req.get("audio").and_then(Value::as_str).unwrap_or("none") {
        "rec" => Some(phrase.audio.clone().ok_or("Esta frase no tiene audio grabado.")?),
        "synth" => Some(phrase.synth.clone().ok_or("La voz sintética aún no está lista.")?),
        _ => None,
    };
    let abs = |f: &str| media.join(f).to_string_lossy().into_owned();
    let input = NoteInput {
        line: &line,
        word,
        translation: phrase.translation.as_deref(),
        game_name: &phrase.game_name,
        explanation: phrase.explanation.as_ref(),
        shot: shot_file.as_deref().filter(|f| media.join(f).is_file()).map(|f| (abs(f), format!("visor_{}.jpg", phrase.id))),
        audio: audio_file.as_deref().filter(|f| media.join(f).is_file()).map(|f| (abs(f), format!("visor_{}.wav", phrase.id))),
    };
    let id = add_note(&c, &deck, &model, &input)?;
    if let Some((p, _)) = glossary::update_line(app, line_id, |p| {
        p.anki_note_id = Some(id);
        if let Some(s) = &shot {
            p.shot = Some(s.clone());
        }
        if let Some(e) = &new_ex {
            store_explanation(p, e);
        }
    }) {
        glossary::push_state(&p, line_id);
    }
    Ok(id)
}


/// Envia a Anki una frase ya guardada en el glosario (desde el panel del glosario de la pagina), como nota de frase,
/// con la captura y el audio guardados (grabado si lo hay, si no sintetico). Avisa con un `toast` y refresca la lista.
pub fn send_stored(app: &AppHandle, phrase_id: String) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("anki-stored".into()).spawn(move || {
        let r = send_stored_inner(&app, &phrase_id);
        let (msg, bad) = match r {
            Ok(_) => ("Frase enviada a Anki".to_string(), false),
            Err(e) => (e, true),
        };
        crate::reader::server::send(json!({ "type": "toast", "msg": msg, "bad": bad }));
        glossary::push_list(&app, &Default::default());
    });
}

fn send_stored_inner(app: &AppHandle, phrase_id: &str) -> Result<i64, String> {
    let (game_id, _) = crate::novel::current_game().ok_or("No hay una partida en curso.")?;
    let mut p = glossary::get(app, &game_id, phrase_id).ok_or("No se encontró la frase.")?;
    let c = cfg(app);
    let st = state(app);
    if st["ok"] != true {
        return Err(st["error"].as_str().unwrap_or("Anki no responde.").to_string());
    }
    let pick = |want: &str, key: &str| if want.is_empty() { st[key].as_str().unwrap_or("").to_string() } else { want.to_string() };
    let (deck, model) = (pick(&c.anki_deck, "deck"), pick(&c.anki_model, "model"));
    if deck.is_empty() {
        return Err("Elige el mazo en Ajustes › Lectura.".into());
    }
    let new_ex = explanation_for(app, &model, &p).filter(|_| p.explanation.is_none());
    if let Some(e) = &new_ex {
        store_explanation(&mut p, e);
    }
    let line = visor_core::jp::ReaderLine { id: 0, at: p.created_at, who: p.who.clone(), color: None, text: p.text.clone(), toks: p.toks.clone(), words: p.words.clone(), grammar: p.grammar.clone() };
    let media = glossary::media_dir(app);
    let abs = |f: &str| media.join(f).to_string_lossy().into_owned();
    let existing = |f: &Option<String>| f.clone().filter(|f| media.join(f).is_file());
    let input = NoteInput {
        line: &line,
        word: None,
        translation: p.translation.as_deref(),
        game_name: &p.game_name,
        explanation: p.explanation.as_ref(),
        shot: existing(&p.shot).map(|f| (abs(&f), format!("visor_{}.jpg", p.id))),
        audio: existing(&p.audio).or(existing(&p.synth)).map(|f| (abs(&f), format!("visor_{}.wav", p.id))),
    };
    let id = add_note(&c, &deck, &model, &input)?;
    glossary::modify(app, &game_id, phrase_id, |p| {
        p.anki_note_id = Some(id);
        if let Some(e) = &new_ex {
            store_explanation(p, e);
        }
    });
    Ok(id)
}

/* ------------------------------------------------------------------ */
/* Comandos de Ajustes                                                  */
/* ------------------------------------------------------------------ */

/// «Probar conexion»: estado de Anki con los mazos y tipos de nota.
#[tauri::command]
pub async fn anki_test(app: AppHandle) -> Value {
    let a = app.clone();
    let s = tauri::async_runtime::spawn_blocking(move || state(&a)).await.unwrap_or_else(|_| json!({ "ok": false, "error": "Error interno" }));
    crate::reader::server::set_extra("anki", s.clone());
    crate::reader::server::send(json!({ "type": "anki", "state": s }));
    s
}

/// «Crear mazo»: crea el mazo configurado en Anki si no existe.
#[tauri::command]
pub async fn anki_create_deck(app: AppHandle) -> Result<String, String> {
    let c = cfg(&app);
    let deck = c.anki_deck.clone();
    if deck.is_empty() {
        return Err("Escribe antes el nombre del mazo.".into());
    }
    let d = deck.clone();
    tauri::async_runtime::spawn_blocking(move || {
        ensure_deck(&c.anki_url, &d)?;
        ensure_model(&c.anki_url, &c.anki_style)
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(deck)
}

/// «Diseño de tarjeta»: aplica el diseño configurado al tipo de nota propio (cambia tambien las notas ya creadas).
#[tauri::command]
pub async fn anki_apply_style(app: AppHandle) -> Result<String, String> {
    let c = cfg(&app);
    let name = anki_tpl::style_name(&c.anki_style).to_string();
    tauri::async_runtime::spawn_blocking(move || apply_style(&c.anki_url, &c.anki_style)).await.map_err(|e| e.to_string())??;
    Ok(name)
}

/* ------------------------------------------------------------------ */
/* Abrir Anki al lanzar una novela                                      */
/* ------------------------------------------------------------------ */

/// `anki.exe`: el de Ajustes o el de las rutas de instalacion habituales.
fn anki_exe(c: &visor_core::Config) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if !c.anki_exe.is_empty() {
        return Some(PathBuf::from(&c.anki_exe)).filter(|p| p.is_file());
    }
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    [
        env("LOCALAPPDATA").map(|p| p.join("Programs").join("Anki").join("anki.exe")),
        env("ProgramFiles").map(|p| p.join("Anki").join("anki.exe")),
        env("ProgramFiles(x86)").map(|p| p.join("Anki").join("anki.exe")),
    ]
    .into_iter()
    .flatten()
    .find(|p| p.is_file())
}

fn anki_process_running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes().values().any(|p| p.name().to_string_lossy().eq_ignore_ascii_case("anki.exe"))
}

/// Al lanzar una novela: si AnkiConnect no responde y esta activado en Ajustes, abre Anki minimizado (para no quitar
/// el foco al juego) y espera a que AnkiConnect conteste. Siempre termina repartiendo el estado de Anki a la pagina.
pub fn ensure_running(app: &AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("anki-start".into()).spawn(move || {
        let c = cfg(&app);
        let up = || call(&c.anki_url, "version", json!({})).is_ok();
        if c.anki_autostart && !up() {
            if !anki_process_running() {
                match anki_exe(&c) {
                    Some(exe) => {
                        use std::os::windows::process::CommandExt;
                        // `start /min`: la primera ventana de Anki sale minimizada.
                        let r = std::process::Command::new("cmd")
                            .args(["/C", "start", "", "/MIN"])
                            .arg(&exe)
                            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
                            .spawn();
                        if let Err(e) = r {
                            let _ = app.emit("th-log", format!("No se pudo abrir Anki: {e}"));
                        }
                    }
                    None => {
                        let _ = app.emit("th-log", "No se encontró anki.exe: indica su ruta en Ajustes › Lectura.".to_string());
                    }
                }
            }
            let t = std::time::Instant::now();
            while t.elapsed() < Duration::from_secs(60) && !up() {
                std::thread::sleep(Duration::from_millis(1500));
            }
        }
        let s = state(&app);
        crate::reader::server::set_extra("anki", s.clone());
        crate::reader::server::send(json!({ "type": "anki", "state": s }));
    });
}

/// Campos de un tipo de nota.
#[tauri::command]
pub async fn anki_fields(app: AppHandle, model: String) -> Result<Vec<String>, String> {
    let url = cfg(&app).anki_url;
    tauri::async_runtime::spawn_blocking(move || call(&url, "modelFieldNames", json!({ "modelName": model })).map(|v| strings(&v)))
        .await
        .map_err(|e| e.to_string())?
}

/// Mapeo de campos adivinado a partir de los campos de un tipo de nota.
#[tauri::command]
pub async fn anki_guess_map(app: AppHandle, model: String) -> Result<AnkiMap, String> {
    Ok(guess_map(&anki_fields(app, model).await?))
}
