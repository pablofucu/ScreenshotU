//! Prompts y respuestas de la IA (DeepSeek, API compatible con OpenAI). Solo logica pura: construir los mensajes,
//! interpretar las lineas SSE del streaming y leer la explicacion en JSON. Las llamadas de red estan en `src/ai.rs`.
use super::phrases::{ChainPiece, CtxLine, Explanation};
use serde_json::{json, Value};

pub const API_URL: &str = "https://api.deepseek.com/chat/completions";
pub const DEFAULT_MODEL: &str = "deepseek-chat";

fn ctx_text(ctx: &[CtxLine]) -> String {
    ctx.iter()
        .map(|c| match &c.who {
            Some(w) => format!("[{w}] {}", c.text),
            None => format!("[narración] {}", c.text),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_text(who: Option<&str>, text: &str) -> String {
    match who {
        Some(w) => format!("[{w}] {text}"),
        None => format!("[narración] {text}"),
    }
}

/// Cuerpo de la peticion de traduccion (con streaming).
pub fn translate_body(model: &str, ctx: &[CtxLine], who: Option<&str>, text: &str) -> Value {
    let sys = "Eres traductor de novelas visuales japonesas al español de España. Traduce SOLO la frase indicada, de forma natural y \
fiel al tono y al registro del personaje; el contexto previo es únicamente para entender quién habla y de qué. \
Responde únicamente con la traducción: sin comillas, sin notas, sin el nombre del personaje.";
    let mut user = String::new();
    if !ctx.is_empty() {
        user.push_str("Contexto (frases anteriores):\n");
        user.push_str(&ctx_text(ctx));
        user.push_str("\n\n");
    }
    user.push_str("Frase a traducir:\n");
    user.push_str(&line_text(who, text));
    json!({
        "model": model,
        "stream": true,
        "temperature": 1.0,
        "max_tokens": 600,
        "messages": [
            { "role": "system", "content": sys },
            { "role": "user", "content": user },
        ],
    })
}

/// Cuerpo de la peticion de explicacion (respuesta JSON). Si ya hay traduccion se reutiliza.
pub fn explain_body(model: &str, ctx: &[CtxLine], who: Option<&str>, text: &str, known_translation: Option<&str>) -> Value {
    let sys = "Eres profesor de japonés para hispanohablantes que leen novelas visuales. Explica la frase indicada con detalle pero \
sin rodeos. Responde SOLO con un objeto JSON con estas claves:\n\
- \"translation\": traducción natural al español.\n\
- \"literal\": traducción literal, empezando por «Lit.: » y entre comillas angulares «…».\n\
- \"chain\": lista de piezas de la frase en orden, cada una {\"kind\": \"v\"|\"g\"|\"n\", \"piece\": texto japonés, \"meaning\": significado o función en español, breve}. \
\"v\" = vocabulario, \"g\" = gramática (formas verbales, partículas con función, auxiliares), \"n\" = otra cosa (expresión fija, nombre).\n\
- \"notes\": lista de notas breves en español sobre matices, registro, hablante, gramática o vocabulario difícil; puedes resaltar términos japoneses con **negrita**.\n\
No añadas nada fuera del JSON.";
    let mut user = String::new();
    if !ctx.is_empty() {
        user.push_str("Contexto (frases anteriores):\n");
        user.push_str(&ctx_text(ctx));
        user.push_str("\n\n");
    }
    user.push_str("Frase a explicar:\n");
    user.push_str(&line_text(who, text));
    if let Some(t) = known_translation.filter(|t| !t.trim().is_empty()) {
        user.push_str("\n\nTraducción ya aceptada (úsala tal cual en \"translation\"): ");
        user.push_str(t.trim());
    }
    json!({
        "model": model,
        "stream": false,
        "temperature": 0.7,
        "max_tokens": 1500,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": sys },
            { "role": "user", "content": user },
        ],
    })
}

/// Resultado de leer una linea del streaming SSE.
#[derive(Debug, PartialEq)]
pub enum Sse {
    /// Trozo de texto nuevo.
    Delta(String),
    /// `data: [DONE]`.
    Done,
    /// Linea vacia, comentario u otra cosa sin texto.
    Skip,
}

/// Interpreta una linea del cuerpo SSE (`data: {...}`).
pub fn parse_sse_line(line: &str) -> Sse {
    let line = line.trim();
    let Some(data) = line.strip_prefix("data:") else { return Sse::Skip };
    let data = data.trim();
    if data == "[DONE]" {
        return Sse::Done;
    }
    let Ok(v) = serde_json::from_str::<Value>(data) else { return Sse::Skip };
    match v.pointer("/choices/0/delta/content").and_then(Value::as_str) {
        Some(t) if !t.is_empty() => Sse::Delta(t.to_string()),
        _ => Sse::Skip,
    }
}

/// Texto de la respuesta no-streaming (`choices[0].message.content`).
pub fn message_content(resp: &Value) -> Option<String> {
    resp.pointer("/choices/0/message/content").and_then(Value::as_str).map(String::from)
}

/// Lee la explicacion del texto devuelto por el modelo: JSON puro o dentro de un bloque ``` ```; tolera claves que falten.
pub fn parse_explanation(content: &str) -> Result<Explanation, String> {
    let t = content.trim();
    let t = t.strip_prefix("```json").or_else(|| t.strip_prefix("```")).unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t).trim();
    // por si el modelo antepone texto: de la primera «{» a la ultima «}»
    let t = match (t.find('{'), t.rfind('}')) {
        (Some(a), Some(b)) if b > a => &t[a..=b],
        _ => t,
    };
    let v: Value = serde_json::from_str(t).map_err(|e| format!("La respuesta de la IA no es JSON válido: {e}"))?;
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    let chain: Vec<ChainPiece> = v
        .get("chain")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|c| {
                    let piece = c.get("piece").and_then(Value::as_str)?.trim().to_string();
                    if piece.is_empty() {
                        return None;
                    }
                    let kind = match c.get("kind").and_then(Value::as_str).unwrap_or("n").trim() {
                        "v" => "v",
                        "g" => "g",
                        _ => "n",
                    };
                    Some(ChainPiece { kind: kind.into(), piece, meaning: c.get("meaning").and_then(Value::as_str).unwrap_or("").trim().to_string() })
                })
                .collect()
        })
        .unwrap_or_default();
    let notes: Vec<String> = v
        .get("notes")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect())
        .unwrap_or_default();
    let e = Explanation { translation: s("translation"), literal: s("literal"), chain, notes };
    if e.translation.is_empty() && e.chain.is_empty() && e.notes.is_empty() {
        return Err("La IA devolvió una explicación vacía.".into());
    }
    Ok(e)
}

/// Mensaje claro para un error HTTP de la API.
pub fn http_error_text(code: u16) -> String {
    match code {
        400 | 422 => format!("DeepSeek rechazó la petición ({code}). Revisa el modelo en Ajustes."),
        401 => "La clave de DeepSeek no es válida (401). Revísala en Ajustes › Lectura.".into(),
        402 => "Sin saldo en DeepSeek (402). Recarga tu cuenta en platform.deepseek.com.".into(),
        429 => "DeepSeek pide ir más despacio (429). Espera un momento y vuelve a intentarlo.".into(),
        500..=599 => format!("DeepSeek no está disponible ahora mismo ({code}). Inténtalo más tarde."),
        _ => format!("DeepSeek devolvió el error {code}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Vec<CtxLine> {
        vec![CtxLine { who: Some("美雪".into()), text: "ねえ、起きてる？".into() }, CtxLine { who: None, text: "雪が降っていた。".into() }]
    }

    #[test]
    fn traduccion_lleva_contexto_y_stream() {
        let b = translate_body("deepseek-chat", &ctx(), Some("僕"), "うん、起きてるよ。");
        assert_eq!(b["stream"], true);
        assert_eq!(b["model"], "deepseek-chat");
        let user = b["messages"][1]["content"].as_str().unwrap();
        assert!(user.contains("[美雪] ねえ、起きてる？") && user.contains("[narración] 雪が降っていた。"));
        assert!(user.ends_with("[僕] うん、起きてるよ。"));
        assert!(b["messages"][0]["content"].as_str().unwrap().contains("Responde únicamente con la traducción"));
        // sin contexto no hay seccion de contexto
        let b2 = translate_body("m", &[], None, "雪");
        assert!(!b2["messages"][1]["content"].as_str().unwrap().contains("Contexto"));
    }

    #[test]
    fn explicacion_pide_json_y_reutiliza_traduccion() {
        let b = explain_body("deepseek-chat", &ctx(), None, "雪が降る", Some("Nieva."));
        assert_eq!(b["response_format"]["type"], "json_object");
        assert_eq!(b["stream"], false);
        assert!(b["messages"][1]["content"].as_str().unwrap().contains("Traducción ya aceptada"));
        assert!(b["messages"][1]["content"].as_str().unwrap().contains("Nieva."));
        let b2 = explain_body("m", &[], None, "雪が降る", None);
        assert!(!b2["messages"][1]["content"].as_str().unwrap().contains("ya aceptada"));
        assert!(b2["messages"][0]["content"].as_str().unwrap().contains("chain"));
    }

    #[test]
    fn lineas_sse() {
        assert_eq!(parse_sse_line(r#"data: {"choices":[{"delta":{"content":"Hola"}}]}"#), Sse::Delta("Hola".into()));
        assert_eq!(parse_sse_line(r#"data: {"choices":[{"delta":{"role":"assistant","content":""}}]}"#), Sse::Skip);
        assert_eq!(parse_sse_line("data: [DONE]"), Sse::Done);
        assert_eq!(parse_sse_line(""), Sse::Skip);
        assert_eq!(parse_sse_line(": keep-alive"), Sse::Skip);
        assert_eq!(parse_sse_line("data: {roto"), Sse::Skip);
    }

    #[test]
    fn explicacion_json() {
        let raw = "```json\n{\"translation\":\"Nieva.\",\"literal\":\"Lit.: «Nieva»\",\"chain\":[{\"kind\":\"v\",\"piece\":\"降る\",\"meaning\":\"caer\"},{\"kind\":\"x\",\"piece\":\"が\"},{\"piece\":\"\"}],\"notes\":[\"**雪** = nieve\",\"\"]}\n```";
        let e = parse_explanation(raw).unwrap();
        assert_eq!(e.translation, "Nieva.");
        assert_eq!(e.chain.len(), 2);
        assert_eq!(e.chain[0].kind, "v");
        assert_eq!(e.chain[1].kind, "n"); // tipo desconocido -> neutro
        assert_eq!(e.notes, vec!["**雪** = nieve".to_string()]);
        // texto antes del JSON, claves ausentes
        let e2 = parse_explanation("Aquí tienes: {\"translation\":\"Hola\"}").unwrap();
        assert_eq!((e2.translation.as_str(), e2.chain.len()), ("Hola", 0));
        assert!(parse_explanation("no es json").is_err());
        assert!(parse_explanation("{}").is_err());
        let resp = json!({"choices":[{"message":{"content":"{\"translation\":\"x\"}"}}]});
        assert_eq!(message_content(&resp).as_deref(), Some("{\"translation\":\"x\"}"));
    }

    #[test]
    fn errores_http_claros() {
        assert!(http_error_text(401).contains("clave"));
        assert!(http_error_text(402).contains("saldo"));
        assert!(http_error_text(429).contains("despacio"));
        assert!(http_error_text(503).contains("503"));
    }
}
