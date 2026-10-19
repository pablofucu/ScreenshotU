//! Tipo de nota propio de la app para Anki («Visor · Japonés»): campos, diseños de tarjeta (plantillas y CSS) y la
//! nota lista para `addNote`. Una sola tarjeta por nota; `EsFrase` decide si la tarjeta es de palabra o de frase.
use super::anki::{build_add_note, NoteInput};
use super::phrases::Explanation;
use super::{ReaderLine, WordInfo};
use crate::config::AnkiMap;
use serde_json::{json, Value};

/// Nombre del tipo de nota que crea la app.
pub const OWN_MODEL: &str = "Visor · Japonés";

/// Campos del tipo de nota, en orden (el primero decide los duplicados). Los nuevos van al final: a un tipo de nota
/// ya creado se le anaden ahi.
pub const FIELDS: [&str; 12] =
    ["Expresion", "Lectura", "Significado", "Frase", "FraseFurigana", "Traduccion", "Captura", "Audio", "Juego", "EsFrase", "Vocabulario", "Explicacion"];

/// Diseños disponibles: (id, nombre visible).
pub const STYLES: [(&str, &str); 4] = [("limpio", "Limpio"), ("escena", "Escena de juego"), ("cuaderno", "Cuaderno"), ("contexto", "Contexto primero")];

pub fn style_name(id: &str) -> &'static str {
    STYLES.iter().find(|s| s.0 == id).map(|s| s.1).unwrap_or(STYLES[0].1)
}

/// Mapeo fijo del tipo de nota propio.
pub fn own_map() -> AnkiMap {
    AnkiMap {
        word: "Expresion".into(),
        reading: "Lectura".into(),
        meaning: "Significado".into(),
        sentence: "Frase".into(),
        translation: "Traduccion".into(),
        shot: "Captura".into(),
        audio: "Audio".into(),
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('[', "［").replace(']', "］")
}

/// La frase en el formato de furigana de Anki (` 漢字[かんじ]`), con el token `tok` en negrita.
pub fn sentence_furigana(line: &ReaderLine, tok: Option<u32>) -> String {
    if line.toks.is_empty() {
        return esc(&line.text);
    }
    let mut out = String::new();
    for (i, k) in line.toks.iter().enumerate() {
        let mut s = String::new();
        for (text, rd) in &k.parts {
            match rd.as_deref().filter(|r| !r.is_empty() && *r != text) {
                // El espacio separa el kanji del texto anterior para el filtro `furigana:` (no se ve).
                Some(r) => s.push_str(&format!(" {}[{}]", esc(text), esc(r))),
                None => s.push_str(&esc(text)),
            }
        }
        if tok == Some(i as u32) {
            out.push_str(&format!("<b>{s}</b>"));
        } else {
            out.push_str(&s);
        }
    }
    out.trim_start().to_string()
}

/// Nombre en castellano de la categoria gramatical simplificada.
fn pos_name(pos: &str) -> &'static str {
    match pos {
        "n" => "sustantivo",
        "v" => "verbo",
        "a" => "adjetivo",
        "adv" => "adverbio",
        "p" => "partícula",
        "aux" => "auxiliar",
        "i" => "interjección",
        _ => "",
    }
}

/// Ficha de la palabra: categoria, frecuencia, acepciones con sus etiquetas y los kanji con su significado.
pub fn meaning_rich(w: &WordInfo) -> String {
    let mut out = String::from(r#"<div class="vx-def">"#);
    let pos = pos_name(&w.pos);
    if !pos.is_empty() || w.freq.is_some() {
        out.push_str(r#"<div class="vx-meta">"#);
        if !pos.is_empty() {
            out.push_str(&format!(r#"<span class="vx-pos vx-p-{}">{pos}</span>"#, esc(&w.pos)));
        }
        if let Some(f) = w.freq {
            let (cls, label) = match f {
                0..=5000 => ("hi", "muy común"),
                5001..=20000 => ("mid", "común"),
                _ => ("lo", "poco común"),
            };
            out.push_str(&format!(r#"<span class="vx-freq vx-f-{cls}">{label} · #{f}</span>"#));
        }
        out.push_str("</div>");
    }
    if let Some(d) = w.defs.first() {
        out.push_str(r#"<ol class="vx-senses">"#);
        for s in d.senses.iter().take(6) {
            let tags: String = s.tags.iter().take(3).map(|t| format!("<i>{}</i>", esc(t))).collect();
            let tags = if tags.is_empty() { String::new() } else { format!(r#"<span class="vx-tags">{tags}</span>"#) };
            out.push_str(&format!(r#"<li>{tags}<span class="vx-gl">{}</span></li>"#, esc(&s.glosses.join("; "))));
        }
        out.push_str("</ol>");
    }
    if !w.kanji.is_empty() {
        out.push_str(r#"<div class="vx-kanji">"#);
        for k in w.kanji.iter().take(4) {
            let rd: Vec<&str> = k.onyomi.iter().chain(k.kunyomi.iter()).take(3).map(String::as_str).collect();
            out.push_str(&format!(
                r#"<div class="vx-k"><span class="vx-kc">{}</span><span class="vx-km">{}</span><span class="vx-kr">{}</span></div>"#,
                esc(&k.ch),
                esc(&k.meanings.iter().take(3).cloned().collect::<Vec<_>>().join(", ")),
                esc(&rd.join(" · "))
            ));
        }
        out.push_str("</div>");
    }
    out.push_str("</div>");
    out
}

/// Palabras de la frase (menos la del token `skip`) como fichas: palabra con su lectura, categoria y significado.
pub fn words_rich(line: &ReaderLine, skip: Option<u32>) -> String {
    let items: String = line
        .words
        .iter()
        .filter(|w| Some(w.tok) != skip)
        .filter_map(|w| {
            let d = w.defs.first()?;
            let gl = d.senses.first()?.glosses.iter().take(2).cloned().collect::<Vec<_>>().join("; ");
            let word = if d.reading.is_empty() || d.reading == w.base { esc(&w.base) } else { format!("<ruby>{}<rt>{}</rt></ruby>", esc(&w.base), esc(&d.reading)) };
            Some(format!(r#"<div class="vx-w vx-p-{}"><div class="vx-wt">{word}</div><div class="vx-wg">{}</div></div>"#, esc(&w.pos), esc(&gl)))
        })
        .take(12)
        .collect();
    if items.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="vx-words">{items}</div>"#)
    }
}

/// Explicacion de la frase: la de DeepSeek si se pidio («Estoy perdido») y los puntos de gramatica detectados.
pub fn explanation_rich(line: &ReaderLine, ex: Option<&Explanation>) -> String {
    let mut out = String::new();
    if let Some(e) = ex {
        if !e.literal.trim().is_empty() {
            out.push_str(&format!(r#"<div class="vx-lit"><span class="vx-h">Literal</span>{}</div>"#, esc(&e.literal)));
        }
        if !e.chain.is_empty() {
            out.push_str(r#"<div class="vx-chain">"#);
            for p in &e.chain {
                let k = if matches!(p.kind.as_str(), "v" | "g") { p.kind.as_str() } else { "n" };
                out.push_str(&format!(r#"<span class="vx-c vx-c-{k}"><b>{}</b><small>{}</small></span>"#, esc(&p.piece), esc(&p.meaning)));
            }
            out.push_str("</div>");
        }
        if !e.notes.is_empty() {
            let notes: String = e.notes.iter().map(|n| format!("<li>{}</li>", esc(n))).collect();
            out.push_str(&format!(r#"<ul class="vx-notes">{notes}</ul>"#));
        }
    }
    let gram: String = line
        .grammar
        .iter()
        .filter_map(|g| {
            let gl = g.defs.first()?.senses.first()?.glosses.first()?;
            Some(format!(r#"<div class="vx-g"><b>{}</b><span>{}</span></div>"#, esc(&g.term), esc(gl)))
        })
        .take(6)
        .collect();
    if !gram.is_empty() {
        out.push_str(&format!(r#"<div class="vx-gram"><span class="vx-h">Gramática</span>{gram}</div>"#));
    }
    out
}

/// Parametros de `addNote` para el tipo de nota propio.
pub fn build_own_note(deck: &str, n: &NoteInput) -> Value {
    let mut v = build_add_note(deck, OWN_MODEL, &own_map(), n);
    let f = &mut v["note"]["fields"];
    let tok = n.word.map(|w| w.tok);
    f["Significado"] = json!(n.word.map(meaning_rich).unwrap_or_default());
    f["Vocabulario"] = json!(words_rich(n.line, tok));
    f["Explicacion"] = json!(explanation_rich(n.line, n.explanation));
    f["FraseFurigana"] = json!(sentence_furigana(n.line, tok));
    f["Juego"] = json!(esc(n.game_name));
    f["EsFrase"] = json!(if n.word.is_none() { "1" } else { "" });
    v
}

/// Parametros de `createModel` con el diseño `style`.
pub fn create_model_params(style: &str) -> Value {
    let (front, back, css) = templates(style);
    json!({
        "modelName": OWN_MODEL,
        "inOrderFields": FIELDS,
        "css": css,
        "isCloze": false,
        "cardTemplates": [{ "Name": "Tarjeta", "Front": front, "Back": back }],
    })
}

/// Parametros de `updateModelTemplates` y `updateModelStyling` con el diseño `style`.
pub fn update_model_params(style: &str) -> (Value, Value) {
    let (front, back, css) = templates(style);
    (
        json!({ "model": { "name": OWN_MODEL, "templates": { "Tarjeta": { "Front": front, "Back": back } } } }),
        json!({ "model": { "name": OWN_MODEL, "css": css } }),
    )
}

/// (anverso, reverso, CSS) de un diseño.
pub fn templates(style: &str) -> (String, String, String) {
    let (front, back, css) = match style {
        // Escena lleva ademas su script (en anverso y reverso).
        "escena" => {
            let js = ESCENA_JS.trim();
            let css = format!("{}\n{}", BASE_CSS.trim(), ESCENA_CSS.trim());
            return (format!("{}\n{js}", ESCENA_FRONT.trim()), format!("{}\n{js}", ESCENA_BACK.trim()), css);
        }
        "cuaderno" => (CUADERNO_FRONT, CUADERNO_BACK, CUADERNO_CSS),
        "contexto" => (CONTEXTO_FRONT, CONTEXTO_BACK, CONTEXTO_CSS),
        _ => (LIMPIO_FRONT, LIMPIO_BACK, LIMPIO_CSS),
    };
    (front.trim().to_string(), back.trim().to_string(), format!("{}\n{}", BASE_CSS.trim(), css.trim()))
}

/* ------------------------------------------------------------------ */
/* CSS comun                                                            */
/* ------------------------------------------------------------------ */

const BASE_CSS: &str = r#"
.card { margin: 0; padding: 0; text-align: left; font-family: "Segoe UI", "Hiragino Sans", "Yu Gothic UI", "Noto Sans JP", sans-serif; }
.v { box-sizing: border-box; max-width: 720px; margin: 0 auto; padding: 22px 20px 28px; line-height: 1.55; }
.v img { max-width: 100%; height: auto; display: block; }
.v ol { margin: 4px 0 0; padding-left: 22px; }
.v li { margin: 2px 0; }
.v ruby rt { font-size: .5em; opacity: .75; }
.v .jp { font-family: "Hiragino Sans", "Yu Gothic UI", "Meiryo", "Noto Sans JP", sans-serif; }
.v .v-audio { margin-top: 14px; }
.v .v-audio:empty { display: none; }
.v .v-game:empty { display: none; }
.replay-button svg { width: 34px; height: 34px; }
/* Ficha de palabra, palabras de la frase y explicacion (HTML que genera la app); cada diseño puede darle color. */
.vx-meta { display: flex; gap: 6px; flex-wrap: wrap; margin: 4px 0 8px; }
.vx-pos, .vx-freq { font-size: 12px; padding: 2px 9px; border-radius: 999px; border: 1px solid currentColor; opacity: .75; }
.vx-senses { margin: 0; padding-left: 22px; text-align: left; }
.vx-tags i { font-style: normal; font-size: 11px; padding: 0 6px; margin-right: 4px; border-radius: 4px; background: rgba(128,128,128,.18); }
.vx-kanji { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 10px; }
.vx-k { display: flex; flex-direction: column; align-items: center; min-width: 72px; max-width: 120px; padding: 6px 8px; border-radius: 8px; background: rgba(128,128,128,.1); }
.vx-kc { font-size: 26px; line-height: 1.2; }
.vx-km { font-size: 12px; text-align: center; }
.vx-kr { font-size: 11px; opacity: .6; text-align: center; }
.vx-words { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 8px; text-align: left; }
.vx-w { padding: 8px 10px; border-radius: 8px; background: rgba(128,128,128,.1); }
.vx-wt { font-size: 18px; }
.vx-wt rt { font-size: 10px; }
.vx-wg { font-size: 13px; opacity: .75; line-height: 1.35; }
.vx-h { display: block; font-size: 11px; text-transform: uppercase; letter-spacing: .06em; opacity: .6; margin: 10px 0 4px; }
.vx-chain { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px; }
.vx-c { display: inline-flex; flex-direction: column; padding: 4px 9px; border-radius: 8px; background: rgba(128,128,128,.12); }
.vx-c small { font-size: 11px; opacity: .7; }
.vx-notes { margin: 8px 0 0; padding-left: 20px; font-size: 14px; }
.vx-g { font-size: 14px; margin: 2px 0; }
.vx-g b { margin-right: 8px; }
"#;

/* ------------------------------------------------------------------ */
/* 1. Limpio: palabra grande centrada, frase debajo; detras definicion */
/* ------------------------------------------------------------------ */

const LIMPIO_FRONT: &str = r#"
<div class="v limpio">
  <div class="v-game">{{Juego}}</div>
  {{^EsFrase}}
  <div class="w jp">{{Expresion}}</div>
  <div class="s jp">{{Frase}}</div>
  {{/EsFrase}}
  {{#EsFrase}}<div class="s big jp">{{Frase}}</div>{{/EsFrase}}
  <div class="v-audio">{{Audio}}</div>
</div>
"#;

const LIMPIO_BACK: &str = r#"
<div class="v limpio">
  <div class="v-game">{{Juego}}</div>
  {{^EsFrase}}
  <div class="w jp"><ruby>{{Expresion}}<rt>{{Lectura}}</rt></ruby></div>
  <div class="s jp">{{furigana:FraseFurigana}}</div>
  <hr>
  <div class="def">{{Significado}}</div>
  {{#Traduccion}}<div class="tr">{{Traduccion}}</div>{{/Traduccion}}
  {{/EsFrase}}
  {{#EsFrase}}
  <div class="s big jp">{{furigana:FraseFurigana}}</div>
  <hr>
  {{#Traduccion}}<div class="tr main">{{Traduccion}}</div>{{/Traduccion}}
  <div class="words jp">{{Vocabulario}}</div>
  {{/EsFrase}}
  {{#Captura}}<div class="shot">{{Captura}}</div>{{/Captura}}
  <div class="v-audio">{{Audio}}</div>
</div>
"#;

const LIMPIO_CSS: &str = r#"
.card { background: #fafaf8; color: #1d1d1f; }
.nightMode.card, .night_mode .card, .nightMode .card { background: #1c1c1e; color: #ececec; }
.limpio { text-align: center; }
.limpio .v-game { font-size: 12px; letter-spacing: .08em; text-transform: uppercase; opacity: .45; margin-bottom: 18px; }
.limpio .w { font-size: 56px; font-weight: 600; line-height: 1.25; margin: 6px 0 14px; }
.limpio .w rt { font-size: 18px; }
.limpio .s { font-size: 22px; }
.limpio .s.big { font-size: 30px; margin: 20px 0; }
.limpio .s b { color: #d4572a; }
.nightMode .limpio .s b, .night_mode .limpio .s b { color: #ff8a5c; }
.limpio hr { border: 0; border-top: 1px solid rgba(128,128,128,.25); margin: 20px auto; width: 60%; }
.limpio .def { display: inline-block; text-align: left; font-size: 18px; }
.limpio .tr { margin-top: 12px; font-size: 16px; opacity: .7; font-style: italic; }
.limpio .tr.main { font-size: 20px; opacity: .9; font-style: normal; margin: 0 0 12px; }
.limpio .words { font-size: 15px; opacity: .7; }
.limpio .shot { margin: 20px auto 0; max-width: 520px; }
.limpio .shot img { margin: 0 auto; border-radius: 10px; box-shadow: 0 4px 18px rgba(0,0,0,.18); }
"#;

/* ------------------------------------------------------------------ */
/* 2. Escena de juego: la captura de fondo con la frase en un cuadro   */
/*    de dialogo de novela visual                                       */
/* ------------------------------------------------------------------ */

const ESCENA_FRONT: &str = r#"
<div class="v escena">
  <div class="amb">{{Captura}}</div>
  <div class="stage{{^Captura}} noimg{{/Captura}}">
    <div class="bg">{{Captura}}</div>
    <div class="box">
      <div class="name">{{Juego}}</div>
      <div class="s jp tw">{{Frase}}</div>
      <div class="more">▼</div>
    </div>
  </div>
  {{^EsFrase}}<div class="ask">¿Qué significa <span class="jp">{{Expresion}}</span>?</div>{{/EsFrase}}
  {{#EsFrase}}<div class="ask">¿Qué dice?</div>{{/EsFrase}}
  <div class="aud">{{Audio}}</div>
  <div class="lb"></div>
</div>
"#;

const ESCENA_BACK: &str = r#"
<div class="v escena back">
  <div class="amb">{{Captura}}</div>
  <div class="stage{{^Captura}} noimg{{/Captura}}">
    <div class="bg">{{Captura}}</div>
    <div class="box">
      <div class="name">{{Juego}}</div>
      <div class="s jp furi">{{furigana:FraseFurigana}}</div>
      {{#Traduccion}}<div class="trw"><div class="tr">{{Traduccion}}</div></div>{{/Traduccion}}
      {{#Audio}}<div class="more">♪</div>{{/Audio}}
    </div>
  </div>
  <div class="tools">
    <button class="t-furi" tabindex="-1">ふりがな</button>
    {{#Audio}}<button class="t-audio" tabindex="-1">▶ Escuchar</button>{{/Audio}}
  </div>
  <div class="panel">
    {{^EsFrase}}
    <div class="w jp"><ruby>{{Expresion}}<rt>{{Lectura}}</rt></ruby></div>
    <div class="def">{{Significado}}</div>
    {{/EsFrase}}
    {{#EsFrase}}<div class="ph">Palabras de la frase</div><div class="jp">{{Vocabulario}}</div>{{/EsFrase}}
  </div>
  {{^EsFrase}}{{#Vocabulario}}
  <details class="fold"><summary>Resto de palabras de la frase</summary><div class="fold-in jp">{{Vocabulario}}</div></details>
  {{/Vocabulario}}{{/EsFrase}}
  {{#Explicacion}}
  <details class="fold ex"><summary>Explicación de la frase</summary><div class="fold-in jp">{{Explicacion}}</div></details>
  {{/Explicacion}}
  <div class="aud">{{Audio}}</div>
  <div class="lb"></div>
</div>
"#;

/// Interactividad del diseño Escena (anverso y reverso). Va en una funcion inmediata porque Anki reutiliza la
/// misma pagina entre tarjetas y no admite declarar dos veces las mismas variables.
const ESCENA_JS: &str = r#"
<script>
(function () {
  var all = document.querySelectorAll('.v.escena');
  var root = all[all.length - 1];
  if (!root || root.getAttribute('data-ok')) return;
  root.setAttribute('data-ok', '1');
  var store = {
    get: function (k) { try { return localStorage.getItem(k); } catch (e) { return null; } },
    set: function (k, v) { try { localStorage.setItem(k, v); } catch (e) {} }
  };
  var on = function (sel, ev, fn) { root.querySelectorAll(sel).forEach(function (el) { el.addEventListener(ev, fn); }); };

  // Un color por juego, sacado de su nombre.
  var name = ((root.querySelector('.name') || {}).textContent || '').trim();
  var h = 265;
  if (name) { h = 0; for (var i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) % 360; }
  root.style.setProperty('--h', h);

  // Anverso: la frase se escribe letra a letra, como en la novela; tocar el cuadro la termina.
  var tw = root.querySelector('.tw');
  var calm = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (tw && !calm) {
    var nodes = [], walk = document.createTreeWalker(tw, NodeFilter.SHOW_TEXT), n;
    while ((n = walk.nextNode())) { nodes.push([n, n.nodeValue]); n.nodeValue = ''; }
    var ni = 0, ci = 0, timer = null;
    var finish = function () {
      clearInterval(timer);
      nodes.forEach(function (p) { p[0].nodeValue = p[1]; });
      root.classList.add('typed');
    };
    timer = setInterval(function () {
      while (ni < nodes.length && ci >= nodes[ni][1].length) { ni++; ci = 0; }
      if (ni >= nodes.length) { finish(); return; }
      ci++;
      nodes[ni][0].nodeValue = nodes[ni][1].slice(0, ci);
    }, 40);
    on('.box', 'click', finish);
  } else {
    root.classList.add('typed');
  }

  // Reverso: audio al tocar el cuadro o «Escuchar».
  var play = function () {
    var b = root.querySelector('.aud .replay-button, .aud .replaybutton, .aud a[href*="playsound"], .aud .soundLink');
    if (b) b.click();
  };
  on('.t-audio', 'click', play);
  if (root.classList.contains('back')) {
    on('.box', 'click', function (e) { if (!e.target.closest('ruby, .trw')) play(); });
  }

  // Furigana: escondida; se ve tocando cada palabra o con el boton (se recuerda entre tarjetas).
  var KEY = 'visor-escena-furigana';
  root.classList.toggle('furi-on', store.get(KEY) === '1');
  on('.t-furi', 'click', function () {
    var v = !root.classList.contains('furi-on');
    root.classList.toggle('furi-on', v);
    store.set(KEY, v ? '1' : '0');
  });
  on('.furi ruby', 'click', function (e) { e.stopPropagation(); this.classList.toggle('on'); });

  // Traduccion tapada hasta tocarla.
  on('.trw', 'click', function (e) { e.stopPropagation(); this.classList.add('open'); });

  // Captura a pantalla completa.
  var lb = root.querySelector('.lb');
  on('.bg img', 'click', function () { lb.innerHTML = ''; lb.appendChild(this.cloneNode()); lb.classList.add('open'); });
  if (lb) lb.addEventListener('click', function () { lb.classList.remove('open'); });

  // Que los botones no se queden con el foco: si no, el espacio de Anki los volveria a pulsar.
  on('button', 'mousedown', function (e) { e.preventDefault(); });
})();
</script>
"#;

const ESCENA_CSS: &str = r#"
.card { background: #0b0d18; color: #eef0f8; }
.escena { --h: 265; --a: hsl(var(--h) 90% 66%); --b: hsl(calc(var(--h) + 55) 95% 68%); --c: hsl(calc(var(--h) + 150) 85% 66%);
  position: relative; padding-top: 18px; }
/* Fondo: la propia captura, muy desenfocada y saturada, llena la pantalla de color. */
.escena .amb { position: fixed; inset: -80px; z-index: -1; overflow: hidden; pointer-events: none; }
.escena .amb img { width: 100%; height: 100%; max-width: none; object-fit: cover; filter: blur(50px) saturate(1.9) brightness(.5); }
.escena .amb:empty { background: radial-gradient(circle at 15% 10%, hsl(var(--h) 75% 35% / .6), transparent 55%),
  radial-gradient(circle at 90% 85%, hsl(calc(var(--h) + 55) 75% 35% / .55), transparent 55%); }
.escena .stage { position: relative; }
.escena .bg { border-radius: 16px; overflow: hidden; background: #000; cursor: zoom-in;
  box-shadow: 0 14px 40px rgba(0,0,0,.55), 0 0 0 1px hsl(var(--h) 80% 70% / .3); }
.escena .bg img { width: 100%; }
.escena .stage.noimg .bg { display: none; }
/* El cuadro sube sobre la parte baja de la captura (proporcional a su ancho) sin taparla entera. */
.escena .box { position: relative; margin: -17% 4% 0; padding: 20px 22px 18px; border-radius: 14px; border: 2px solid transparent; cursor: pointer;
  background: linear-gradient(rgba(12,14,30,.88), rgba(12,14,30,.88)) padding-box, linear-gradient(120deg, var(--a), var(--b), var(--c)) border-box;
  box-shadow: 0 12px 32px rgba(0,0,0,.5), 0 0 26px hsl(var(--h) 90% 60% / .3); backdrop-filter: blur(6px); }
.escena .stage.noimg .box { margin: 16px 0 0; padding: 32px 22px 26px; }
.escena .name { position: absolute; top: -15px; left: 16px; padding: 3px 14px; border-radius: 8px; font-size: 12px; font-weight: 800;
  letter-spacing: .05em; color: #fff; background: linear-gradient(120deg, var(--a), var(--b)); box-shadow: 0 4px 12px hsl(var(--h) 90% 45% / .5); }
.escena .name:empty { display: none; }
.escena .s { font-size: 23px; line-height: 1.8; }
.escena .s b { color: var(--b); font-weight: 800; text-shadow: 0 0 14px hsl(calc(var(--h) + 55) 95% 60% / .6); }
.escena.typed .s b { animation: esc-glow 2.6s ease-in-out infinite; }
@keyframes esc-glow { 50% { text-shadow: 0 0 24px hsl(calc(var(--h) + 55) 95% 65% / 1); } }
.escena .more { position: absolute; right: 14px; bottom: 6px; font-size: 13px; color: var(--b); opacity: 0; }
.escena.typed .more, .escena.back .more { opacity: 1; animation: esc-bob 1.1s ease-in-out infinite; }
@keyframes esc-bob { 50% { transform: translateY(3px); } }
.escena .ask { margin-top: 22px; text-align: center; font-size: 18px; color: #c9cde6; }
.escena .ask .jp { font-size: 30px; font-weight: 800; background: linear-gradient(120deg, var(--a), var(--b), var(--c));
  -webkit-background-clip: text; background-clip: text; color: transparent; }
/* Furigana escondida hasta tocar la palabra o pulsar «ふりがな». */
.escena .furi ruby { cursor: pointer; }
.escena .furi rt { opacity: 0; transition: opacity .2s; color: var(--a); }
.escena.furi-on .furi rt, .escena .furi ruby.on rt { opacity: 1; }
.escena .trw { position: relative; margin-top: 10px; cursor: pointer; }
.escena .tr { font-size: 15px; color: #c8cdee; font-style: italic; filter: blur(7px); transition: filter .3s; user-select: none; }
.escena .trw::after { content: "Toca para ver la traducción"; position: absolute; inset: 0; display: flex; align-items: center; justify-content: center;
  font-size: 13px; font-weight: 700; color: var(--a); letter-spacing: .03em; }
.escena .trw.open .tr { filter: none; user-select: auto; }
.escena .trw.open::after { display: none; }
.escena .tools { display: flex; gap: 10px; justify-content: center; flex-wrap: wrap; margin-top: 18px; }
.escena .tools button { border: 1px solid transparent; border-radius: 999px; padding: 8px 18px; font: 700 14px/1 "Segoe UI", sans-serif; color: #fff;
  cursor: pointer; background: linear-gradient(120deg, var(--a), var(--b)); box-shadow: 0 4px 14px hsl(var(--h) 90% 45% / .4); }
.escena .tools .t-furi { background: rgba(255,255,255,.07); border-color: hsl(var(--h) 80% 70% / .55); box-shadow: none; }
.escena.furi-on .tools .t-furi { background: hsl(var(--h) 80% 60% / .35); }
.escena .panel { margin-top: 16px; padding: 18px 22px; border-radius: 14px; background: rgba(18,21,40,.85);
  border: 1px solid hsl(var(--h) 60% 60% / .3); box-shadow: inset 0 3px 0 var(--b), 0 10px 28px rgba(0,0,0,.35); }
.escena .w { font-size: 38px; font-weight: 800; color: var(--b); margin-bottom: 4px; }
.escena .w rt { color: var(--a); font-size: 15px; opacity: 1; }
.escena .def, .escena .words { font-size: 17px; }
.escena .def li::marker { color: var(--a); font-weight: 800; }
.escena .words { line-height: 1.9; }
.escena .ph { font-size: 12px; font-weight: 800; letter-spacing: .08em; text-transform: uppercase; color: var(--a); margin-bottom: 10px; }
/* Colores por categoria gramatical. */
.escena .vx-p-n { --p: hsl(200 85% 66%); } .escena .vx-p-v { --p: hsl(340 88% 70%); } .escena .vx-p-a { --p: hsl(40 92% 62%); }
.escena .vx-p-adv { --p: hsl(150 62% 58%); } .escena .vx-p-p, .escena .vx-p-aux { --p: hsl(270 70% 74%); } .escena .vx-p-i { --p: hsl(18 88% 66%); }
.escena .vx-meta { margin: 2px 0 12px; }
.escena .vx-pos { opacity: 1; border: 0; color: #0b0d18; font-weight: 800; background: var(--p, var(--a)); }
.escena .vx-freq { opacity: 1; border: 0; font-weight: 700; background: rgba(255,255,255,.08); }
.escena .vx-f-hi { color: hsl(150 70% 62%); } .escena .vx-f-mid { color: hsl(42 92% 64%); } .escena .vx-f-lo { color: #9aa0bf; }
/* Acepciones como tarjetas numeradas. */
.escena .vx-senses { list-style: none; padding: 0; counter-reset: sn; display: grid; gap: 8px; }
.escena .vx-senses li { counter-increment: sn; position: relative; padding: 10px 14px 10px 46px; border-radius: 12px; font-size: 17px; line-height: 1.45;
  background: linear-gradient(90deg, hsl(var(--h) 70% 55% / .16), rgba(255,255,255,.03)); border: 1px solid hsl(var(--h) 70% 70% / .18); }
.escena .vx-senses li::before { content: counter(sn); position: absolute; left: 12px; top: 10px; width: 24px; height: 24px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center; font-size: 13px; font-weight: 800; color: #0b0d18;
  background: linear-gradient(135deg, var(--a), var(--b)); }
.escena .vx-senses li:first-child { font-weight: 700; font-size: 18px; }
.escena .vx-tags i { background: hsl(var(--h) 70% 65% / .22); color: hsl(var(--h) 90% 82%); font-weight: 700; }
/* Kanji como fichas. */
.escena .vx-kanji { margin-top: 14px; gap: 10px; }
.escena .vx-k { background: rgba(255,255,255,.05); border: 1px solid hsl(calc(var(--h) + 150) 70% 66% / .3); border-radius: 12px; padding: 8px 10px; }
.escena .vx-kc { font-size: 32px; font-weight: 800; color: var(--c); }
.escena .vx-km { color: #e3e6f7; }
.escena .vx-kr { color: #9aa0bf; opacity: 1; }
/* Palabras de la frase: una ficha por palabra con el color de su categoria. */
.escena .vx-words { gap: 10px; }
.escena .vx-w { background: rgba(255,255,255,.05); border-radius: 12px; padding: 10px 12px; border-left: 4px solid var(--p, var(--a)); }
.escena .vx-wt { font-size: 21px; font-weight: 700; color: var(--p, #fff); }
.escena .vx-wt rt { color: #c9cde6; font-size: 11px; font-weight: 400; }
.escena .vx-wg { color: #d5d9ef; opacity: 1; font-size: 14px; }
/* Desplegables discretos al fondo. */
.escena .fold { margin-top: 14px; border-radius: 12px; background: rgba(14,16,32,.6); border: 1px solid rgba(255,255,255,.07); }
.escena .fold > summary { list-style: none; cursor: pointer; padding: 10px 16px; font-size: 13px; font-weight: 700; color: #9aa0bf; user-select: none; }
.escena .fold > summary::-webkit-details-marker { display: none; }
.escena .fold > summary::before { content: "▸"; display: inline-block; margin-right: 8px; color: var(--a); transition: transform .2s; }
.escena .fold[open] > summary::before { transform: rotate(90deg); }
.escena .fold[open] > summary { color: #e3e6f7; }
.escena .fold-in { padding: 2px 16px 16px; font-size: 15px; }
.escena .ex { opacity: .7; } .escena .ex[open] { opacity: 1; }
.escena .ex > summary::before { content: "?"; width: 18px; height: 18px; border-radius: 50%; display: inline-flex; align-items: center; justify-content: center;
  font-size: 11px; color: #0b0d18; background: var(--a); }
.escena .ex[open] > summary::before { transform: none; }
.escena .vx-h { color: var(--a); opacity: 1; font-weight: 800; }
.escena .vx-lit { font-style: italic; color: #d5d9ef; }
.escena .vx-c { background: rgba(255,255,255,.06); border-bottom: 2px solid #9aa0bf; }
.escena .vx-c-v { border-color: hsl(200 85% 66%); } .escena .vx-c-g { border-color: hsl(340 88% 70%); }
.escena .vx-c b { font-size: 17px; } .escena .vx-c small { color: #c9cde6; opacity: 1; }
.escena .vx-notes li { margin: 4px 0; color: #d5d9ef; } .escena .vx-notes li::marker { color: var(--b); }
.escena .vx-g { padding: 6px 10px; border-radius: 8px; background: rgba(255,255,255,.04); margin: 6px 0; }
.escena .vx-g b { color: var(--b); }
.escena .aud { position: absolute; width: 1px; height: 1px; overflow: hidden; opacity: 0; pointer-events: none; }
.escena .lb { position: fixed; inset: 0; z-index: 50; display: none; align-items: center; justify-content: center; padding: 20px;
  background: rgba(0,0,0,.88); cursor: zoom-out; }
.escena .lb.open { display: flex; }
.escena .lb img { max-width: 100%; max-height: 100%; border-radius: 10px; }
"#;

/* ------------------------------------------------------------------ */
/* 3. Cuaderno: papel crema, letra de imprenta (mincho), la palabra en  */
/*    vertical y el juego como sello rojo                               */
/* ------------------------------------------------------------------ */

const CUADERNO_FRONT: &str = r#"
<div class="v cuaderno">
  <div class="paper">
    <div class="stamp v-game">{{Juego}}</div>
    {{^EsFrase}}
    <div class="row">
      <div class="tate jp">{{Expresion}}</div>
      <div class="s jp">{{Frase}}</div>
    </div>
    {{/EsFrase}}
    {{#EsFrase}}<div class="s big jp">{{Frase}}</div>{{/EsFrase}}
    <div class="v-audio">{{Audio}}</div>
  </div>
</div>
"#;

const CUADERNO_BACK: &str = r#"
<div class="v cuaderno">
  <div class="paper">
    <div class="stamp v-game">{{Juego}}</div>
    {{^EsFrase}}
    <div class="row">
      <div class="tate jp"><ruby>{{Expresion}}<rt>{{Lectura}}</rt></ruby></div>
      <div class="s jp">{{furigana:FraseFurigana}}</div>
    </div>
    <div class="notes"><div class="h">意味</div>{{Significado}}</div>
    {{#Traduccion}}<div class="notes"><div class="h">訳</div>{{Traduccion}}</div>{{/Traduccion}}
    {{/EsFrase}}
    {{#EsFrase}}
    <div class="s big jp">{{furigana:FraseFurigana}}</div>
    {{#Traduccion}}<div class="notes"><div class="h">訳</div>{{Traduccion}}</div>{{/Traduccion}}
    <div class="notes"><div class="h">語彙</div>{{Vocabulario}}</div>
    {{/EsFrase}}
    {{#Captura}}<div class="polaroid">{{Captura}}</div>{{/Captura}}
    <div class="v-audio">{{Audio}}</div>
  </div>
</div>
"#;

const CUADERNO_CSS: &str = r#"
.card { background: #e9e2d0; color: #2b2620; }
.nightMode.card, .night_mode .card, .nightMode .card { background: #23201b; color: #e8dfcc; }
.cuaderno .paper { position: relative; padding: 28px 26px 26px; border-radius: 4px; background: #fbf7ec;
  box-shadow: 0 1px 0 #d8cfb8, 0 8px 24px rgba(60,40,10,.15);
  background-image: repeating-linear-gradient(transparent 0 31px, rgba(80,110,160,.13) 31px 32px); }
.nightMode .cuaderno .paper, .night_mode .cuaderno .paper { background-color: #2f2b24; box-shadow: none;
  background-image: repeating-linear-gradient(transparent 0 31px, rgba(200,190,160,.08) 31px 32px); }
.cuaderno .jp, .cuaderno .s { font-family: "Yu Mincho", "Hiragino Mincho ProN", "MS Mincho", "Noto Serif JP", serif; }
.cuaderno .stamp { display: table; margin: -10px -8px 8px auto; max-width: 60%; padding: 3px 9px; border: 2px solid #c0392b; border-radius: 4px;
  color: #c0392b; font-size: 12px; font-weight: 700; transform: rotate(4deg); opacity: .85; }
.cuaderno .row { display: flex; gap: 22px; align-items: flex-start; flex-direction: row-reverse; margin-top: 10px; }
.cuaderno .tate { writing-mode: vertical-rl; font-size: 48px; font-weight: 700; line-height: 1.1; padding-left: 18px; border-left: 2px solid #c0392b; }
.cuaderno .tate rt { font-size: 16px; }
.cuaderno .s { flex: 1; font-size: 22px; line-height: 2; }
.cuaderno .s.big { font-size: 28px; margin-top: 10px; }
.cuaderno .s b { background: linear-gradient(transparent 60%, rgba(230,180,60,.55) 60%); font-weight: 700; }
.cuaderno .notes { margin-top: 18px; font-size: 17px; }
.cuaderno .notes .h { display: block; font-family: "Yu Mincho", serif; font-weight: 700; color: #c0392b; margin-bottom: 2px; }
.cuaderno .polaroid { margin: 22px auto 0; width: 78%; padding: 10px 10px 28px; background: #fff; transform: rotate(-1.5deg);
  box-shadow: 0 4px 14px rgba(0,0,0,.2); }
"#;

/* ------------------------------------------------------------------ */
/* 4. Contexto primero: solo la frase con la palabra subrayada; la      */
/*    captura como pista; detras todo junto en fichas                   */
/* ------------------------------------------------------------------ */

const CONTEXTO_FRONT: &str = r#"
<div class="v contexto">
  <div class="s jp">{{Frase}}</div>
  {{^EsFrase}}<div class="target jp">{{Expresion}}</div>{{/EsFrase}}
  <div class="hints">{{#Captura}}{{hint:Captura}}{{/Captura}}</div>
  <div class="v-audio">{{Audio}}</div>
</div>
"#;

const CONTEXTO_BACK: &str = r#"
<div class="v contexto">
  <div class="s jp">{{furigana:FraseFurigana}}</div>
  {{#Traduccion}}<div class="tr">{{Traduccion}}</div>{{/Traduccion}}
  <div class="grid">
    <div class="chip">
      {{^EsFrase}}
      <div class="w jp">{{Expresion}}</div>
      <div class="r jp">{{Lectura}}</div>
      <div class="def">{{Significado}}</div>
      {{/EsFrase}}
      {{#EsFrase}}<div class="words jp">{{Vocabulario}}</div>{{/EsFrase}}
      <div class="v-audio">{{Audio}}</div>
    </div>
    {{#Captura}}<div class="thumb">{{Captura}}</div>{{/Captura}}
  </div>
  <div class="foot v-game">{{Juego}}</div>
</div>
"#;

const CONTEXTO_CSS: &str = r#"
.card { background: #f3f5f9; color: #1b2230; }
.nightMode.card, .night_mode .card, .nightMode .card { background: #12161f; color: #e3e8f2; }
.contexto { padding-top: 36px; }
.contexto .s { font-size: 30px; line-height: 1.8; text-align: center; }
.contexto .s b { font-weight: 700; color: inherit; border-bottom: 3px solid #2f80ed; padding-bottom: 1px; }
.contexto .target { margin-top: 16px; text-align: center; font-size: 15px; opacity: .55; }
.contexto .target::before { content: "→ "; }
.contexto .hints { margin-top: 26px; text-align: center; font-size: 14px; }
.contexto .hints a { color: #2f80ed; }
.contexto .hints img { margin: 10px auto 0; max-height: 260px; border-radius: 8px; }
.contexto .tr { margin-top: 10px; text-align: center; font-size: 18px; opacity: .75; }
.contexto .grid { display: flex; gap: 14px; margin-top: 24px; flex-wrap: wrap; }
.contexto .chip { flex: 1 1 260px; padding: 16px 18px; border-radius: 14px; background: #fff; box-shadow: 0 2px 10px rgba(20,40,80,.08); }
.nightMode .contexto .chip, .night_mode .contexto .chip { background: #1c2230; box-shadow: none; }
.contexto .w { font-size: 30px; font-weight: 700; }
.contexto .r { font-size: 16px; color: #2f80ed; margin-bottom: 8px; }
.contexto .r:empty { display: none; }
.contexto .def, .contexto .words { font-size: 16px; }
.contexto .thumb { flex: 1 1 220px; }
.contexto .thumb img { border-radius: 14px; }
.contexto .foot { margin-top: 18px; text-align: right; font-size: 12px; opacity: .45; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jp::{Sense, Tok, WordDef, WordInfo};

    fn line() -> ReaderLine {
        ReaderLine {
            id: 1,
            at: 0,
            who: None,
            color: None,
            text: "雪が降る".into(),
            toks: vec![
                Tok { parts: vec![("雪".into(), Some("ゆき".into()))], pos: "n".into(), base: None },
                Tok { parts: vec![("が".into(), None)], pos: "p".into(), base: None },
                Tok { parts: vec![("降".into(), Some("ふ".into())), ("る".into(), None)], pos: "v".into(), base: None },
            ],
            words: vec![WordInfo {
                tok: 2,
                surface: "降る".into(),
                base: "降る".into(),
                pos: "v".into(),
                defs: vec![WordDef { term: "降る".into(), reading: "ふる".into(), dict: "D".into(), senses: vec![Sense { tags: vec![], glosses: vec!["caer".into()] }] }],
                freq: None,
                kanji: vec![],
            }],
            grammar: vec![],
        }
    }

    #[test]
    fn furigana_de_anki() {
        let l = line();
        assert_eq!(sentence_furigana(&l, None), "雪[ゆき]が 降[ふ]る");
        assert_eq!(sentence_furigana(&l, Some(2)), "雪[ゆき]が<b> 降[ふ]る</b>");
    }

    #[test]
    fn nota_propia() {
        let l = line();
        let n = NoteInput { line: &l, word: Some(&l.words[0]), translation: None, game_name: "Juego", explanation: None, shot: None, audio: None };
        let v = build_own_note("M", &n);
        let f = &v["note"]["fields"];
        assert_eq!(v["note"]["modelName"], OWN_MODEL);
        assert_eq!((f["Expresion"].as_str(), f["Lectura"].as_str(), f["EsFrase"].as_str()), (Some("降る"), Some("ふる"), Some("")));
        let n = NoteInput { word: None, ..n };
        assert_eq!(build_own_note("M", &n)["note"]["fields"]["EsFrase"], "1");
        for f in v["note"]["fields"].as_object().unwrap().keys() {
            assert!(FIELDS.contains(&f.as_str()), "campo desconocido {f}");
        }
    }

    #[test]
    fn plantillas_usan_solo_campos_propios() {
        for (id, _) in STYLES {
            let (front, back, _) = templates(id);
            for t in [front, back] {
                for cap in t.split("{{").skip(1) {
                    let name = cap.split("}}").next().unwrap().trim_start_matches(['#', '^', '/']);
                    let name = name.rsplit(':').next().unwrap();
                    assert!(FIELDS.contains(&name), "{id}: campo {name}");
                }
            }
        }
    }
}
