//! Traduccion y explicacion de frases con DeepSeek (API compatible con OpenAI), SOLO bajo demanda desde la pagina de
//! lectura. La clave vive solo en la configuracion de la app: la pagina nunca la recibe, las llamadas las hace este
//! modulo en un hilo propio. Todo se cachea en el glosario de frases (`glossary`): volver a una frase no vuelve a llamar.
use crate::glossary;
use crate::state::AppState;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use visor_core::jp::phrases::{CtxLine, Explanation};
use visor_core::jp::prompt;

/// Peticiones en curso: (linea, es explicacion). Evita llamadas duplicadas si la pagina insiste.
static INFLIGHT: Mutex<Option<HashSet<(u64, bool)>>> = Mutex::new(None);

struct Guard((u64, bool));

impl Guard {
    fn new(key: (u64, bool)) -> Option<Guard> {
        let mut g = INFLIGHT.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(HashSet::new).insert(key).then_some(Guard(key))
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(s) = INFLIGHT.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            s.remove(&self.0);
        }
    }
}

fn kind_name(explain: bool) -> &'static str {
    if explain {
        "explain"
    } else {
        "translate"
    }
}

fn send_error(line_id: u64, explain: bool, msg: &str) {
    crate::reader::server::send(json!({ "type": "ai", "lineId": line_id, "kind": kind_name(explain), "done": true, "error": msg }));
}

/// (clave, modelo) de DeepSeek de los ajustes.
fn credentials(app: &AppHandle) -> (String, String) {
    let st = app.state::<AppState>();
    let c = st.config.lock().unwrap();
    let model = if c.deepseek_model.trim().is_empty() { prompt::DEFAULT_MODEL.to_string() } else { c.deepseek_model.trim().to_string() };
    (c.deepseek_key.trim().to_string(), model)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(10)).timeout_read(Duration::from_secs(60)).build()
}

fn error_text(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            // La API suele dar `{"error":{"message":...}}`; se anade al mensaje por si ayuda.
            let detail = resp.into_string().ok().and_then(|b| serde_json::from_str::<Value>(&b).ok()).and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(String::from));
            match detail {
                Some(d) if !matches!(code, 401 | 402 | 429) => format!("{} ({d})", prompt::http_error_text(code)),
                _ => prompt::http_error_text(code),
            }
        }
        ureq::Error::Transport(t) => format!("Sin conexión con DeepSeek: {t}"),
    }
}

/// Pide la traduccion de una frase (en un hilo). Si ya esta en el glosario, se devuelve sin llamar a la IA.
pub fn translate(app: &AppHandle, line_id: u64) {
    start(app, line_id, false);
}

/// Pide la explicacion completa («Estoy perdido»). Si ya esta en el glosario, se devuelve sin llamar a la IA.
pub fn explain(app: &AppHandle, line_id: u64) {
    start(app, line_id, true);
}

fn start(app: &AppHandle, line_id: u64, explain: bool) {
    let app = app.clone();
    let Some(guard) = Guard::new((line_id, explain)) else { return };
    let _ = std::thread::Builder::new().name("ai".into()).spawn(move || {
        let _guard = guard;
        run(&app, line_id, explain);
    });
}

fn run(app: &AppHandle, line_id: u64, explain: bool) {
    let Some(line) = crate::reader::server::line(line_id) else {
        return send_error(line_id, explain, "Esa frase ya no está en el historial de la página.");
    };
    // 1. Cache: el glosario.
    if let Some(p) = glossary::find_by_line(app, line_id) {
        if explain {
            if let Some(e) = &p.explanation {
                crate::reader::server::send(json!({ "type": "ai", "lineId": line_id, "kind": "explain", "explanation": e, "done": true }));
                return glossary::push_state(&p, line_id);
            }
        } else if let Some(t) = &p.translation {
            crate::reader::server::send(json!({ "type": "ai", "lineId": line_id, "kind": "translate", "text": t, "done": true }));
            return glossary::push_state(&p, line_id);
        }
    }
    // 2. Llamada a la API.
    let (key, model) = credentials(app);
    if key.is_empty() {
        return send_error(line_id, explain, "Falta la clave de DeepSeek: ponla en Ajustes › Lectura.");
    }
    let ctx: Vec<CtxLine> = crate::reader::server::lines_before(line_id, 3).into_iter().map(|l| CtxLine { who: l.who, text: l.text }).collect();
    let result = if explain {
        let known = glossary::find_by_line(app, line_id).and_then(|p| p.translation);
        let body = prompt::explain_body(&model, &ctx, line.who.as_deref(), &line.text, known.as_deref());
        run_explain(app, line_id, &key, &body)
    } else {
        let body = prompt::translate_body(&model, &ctx, line.who.as_deref(), &line.text);
        run_translate(app, line_id, &key, &body)
    };
    if let Err(e) = result {
        send_error(line_id, explain, &e);
    }
}

fn post(key: &str, body: &Value) -> Result<ureq::Response, String> {
    agent().post(prompt::API_URL).set("Authorization", &format!("Bearer {key}")).set("Content-Type", "application/json").send_json(body.clone()).map_err(error_text)
}

/// Traduccion con streaming (SSE): la pagina va viendo el texto crecer.
fn run_translate(app: &AppHandle, line_id: u64, key: &str, body: &Value) -> Result<(), String> {
    let resp = post(key, body)?;
    let mut acc = String::new();
    let mut last_sent = Instant::now() - Duration::from_secs(1);
    for l in BufReader::new(resp.into_reader()).lines() {
        let l = l.map_err(|e| format!("Se cortó la conexión con DeepSeek: {e}"))?;
        match prompt::parse_sse_line(&l) {
            prompt::Sse::Delta(t) => {
                acc.push_str(&t);
                if last_sent.elapsed() >= Duration::from_millis(60) {
                    last_sent = Instant::now();
                    crate::reader::server::send(json!({ "type": "ai", "lineId": line_id, "kind": "translate", "text": acc, "done": false }));
                }
            }
            prompt::Sse::Done => break,
            prompt::Sse::Skip => {}
        }
    }
    let text = acc.trim().trim_matches(|c| c == '"' || c == '「' || c == '」').trim().to_string();
    if text.is_empty() {
        return Err("DeepSeek no devolvió ninguna traducción.".into());
    }
    let saved = glossary::update_line(app, line_id, |p| p.translation = Some(text.clone()));
    crate::reader::server::send(json!({ "type": "ai", "lineId": line_id, "kind": "translate", "text": text, "done": true }));
    if let Some((p, _)) = saved {
        glossary::push_state(&p, line_id);
    }
    Ok(())
}

/// Hay clave de DeepSeek (se puede pedir una explicacion).
pub fn explain_blocking_available(app: &AppHandle) -> bool {
    !credentials(app).0.is_empty()
}

/// Explicacion pedida en el mismo hilo, sin pasar por la pagina (para las tarjetas de Anki). `None` = sin clave de
/// DeepSeek. Quien llama decide donde guardarla.
pub fn explain_blocking(app: &AppHandle, ctx: &[CtxLine], who: Option<&str>, text: &str, known: Option<&str>) -> Option<Result<Explanation, String>> {
    let (key, model) = credentials(app);
    if key.is_empty() {
        return None;
    }
    let body = prompt::explain_body(&model, ctx, who, text, known);
    Some((|| {
        let v: Value = post(&key, &body)?.into_json().map_err(|e| format!("Respuesta ilegible de DeepSeek: {e}"))?;
        let content = prompt::message_content(&v).ok_or("DeepSeek no devolvió contenido.")?;
        prompt::parse_explanation(&content)
    })())
}

/// Explicacion en JSON (sin streaming).
fn run_explain(app: &AppHandle, line_id: u64, key: &str, body: &Value) -> Result<(), String> {
    let resp = post(key, body)?;
    let v: Value = resp.into_json().map_err(|e| format!("Respuesta ilegible de DeepSeek: {e}"))?;
    let content = prompt::message_content(&v).ok_or("DeepSeek no devolvió contenido.")?;
    let e = prompt::parse_explanation(&content)?;
    let saved = glossary::update_line(app, line_id, |p| {
        if p.translation.is_none() && !e.translation.is_empty() {
            p.translation = Some(e.translation.clone());
        }
        p.explanation = Some(e.clone());
    });
    crate::reader::server::send(json!({ "type": "ai", "lineId": line_id, "kind": "explain", "explanation": e, "done": true }));
    if let Some((p, _)) = saved {
        glossary::push_state(&p, line_id);
    }
    Ok(())
}
