//! Novelas japonesas: une LunaHook (`texthook`), el procesado de texto (`visor_core::jp`) y la
//! pagina de lectura (`reader`). Ver `docs/novelas-japonesas.md`.
pub mod readstats;

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::process::Child;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use visor_core::games::Game;
use visor_core::jp::known::Known;
use visor_core::db::data as dbd;
use visor_core::jp::{filters, names, tokenize, Dict, LineDeduper, NameJoiner, NovelPrefs, Tokenizer};

/// Proceso del juego al que esta conectado LunaHook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcInfo {
    pub pid: u32,
    pub exe: String,
    pub arch: u8,
    pub connected: bool,
}

/// Un hilo de texto de LunaHook. `key` es estable entre sesiones (codigo de hook + contexto).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadInfo {
    pub key: String,
    pub name: String,
    pub code: String,
    pub lines: u32,
    pub last: String,
    pub live: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThStatus {
    pub proc: Option<ProcInfo>,
    pub threads: Vec<ThreadInfo>,
    pub selected: Option<String>,
    pub name_thread: Option<String>,
    pub game_id: Option<String>,
    pub game_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub code: String,
    pub text: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextEvent<'a> {
    key: &'a str,
    text: &'a str,
}

/// Lo que llega al hilo que construye las lineas de lectura.
enum Msg {
    Main(String),
    Name(String),
}

/// Partida de novela en curso.
struct Session {
    game_id: String,
    game_name: String,
    prefs: NovelPrefs,
    tx: Sender<Msg>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);
static TOKENIZER: OnceLock<Option<Tokenizer>> = OnceLock::new();
/// Diccionarios Yomitan de `<biblioteca>/Diccionarios` (acepciones de la pagina de lectura). Se recargan bajo demanda.
static DICT: RwLock<Option<Arc<Dict>>> = RwLock::new(None);
static DICT_LOADING: AtomicBool = AtomicBool::new(false);
/// Palabras conocidas (globales): copia en memoria de la tabla `known_words`.
static KNOWN: Mutex<Option<Known>> = Mutex::new(None);
/// Navegador de la pagina de lectura: se reutiliza entre partidas mientras siga abierto.
static BROWSER: Mutex<Option<Child>> = Mutex::new(None);

/// El tokenizador carga el diccionario (caro): una vez, la primera vez que hace falta.
fn tokenizer() -> Option<&'static Tokenizer> {
    TOKENIZER
        .get_or_init(|| match Tokenizer::new() {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("No se pudo cargar el diccionario del tokenizador: {e}");
                None
            }
        })
        .as_ref()
}

fn dicts_dir(app: &AppHandle) -> std::path::PathBuf {
    app.state::<AppState>().paths.dicts.clone()
}

fn dict() -> Option<Arc<Dict>> {
    DICT.read().unwrap().clone()
}

fn publish_dicts(app: &AppHandle) {
    let names: Vec<String> = dict().map(|d| d.names().to_vec()).unwrap_or_default();
    crate::reader::server::set_dicts(serde_json::json!({
        "names": names,
        "loading": DICT_LOADING.load(Ordering::SeqCst),
        "dir": dicts_dir(app).display().to_string(),
    }));
}

/// Carga (o recarga) los diccionarios en segundo plano. `force` = aunque ya esten cargados.
fn load_dicts(app: &AppHandle, force: bool) {
    if (!force && DICT.read().unwrap().is_some()) || DICT_LOADING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        publish_dicts(&app);
        let dir = dicts_dir(&app);
        let _ = std::fs::create_dir_all(&dir);
        let d = Dict::load_dir(&dir);
        *DICT.write().unwrap() = Some(Arc::new(d));
        DICT_LOADING.store(false, Ordering::SeqCst);
        publish_dicts(&app);
    });
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn db(app: &AppHandle) -> Arc<visor_core::db::Db> {
    app.state::<AppState>().db.clone()
}

fn load_prefs(app: &AppHandle, game_id: &str) -> NovelPrefs {
    crate::state::log_db("preferencias de novela", db(app).with(|c| dbd::novel_prefs_get(c, game_id))).unwrap_or_default()
}

fn store_prefs(app: &AppHandle, game_id: &str, prefs: &NovelPrefs) -> Result<(), String> {
    db(app).with(|c| dbd::novel_prefs_put(c, game_id, prefs)).map_err(|e| format!("No se pudieron guardar las preferencias de la novela: {e}"))
}

/// Guarda las prefs de la partida en curso y las reparte (pagina de lectura y ventana de hook).
fn update_prefs(app: &AppHandle, game_id: &str, prefs: NovelPrefs) -> Result<(), String> {
    store_prefs(app, game_id, &prefs)?;
    if let Some(s) = SESSION.lock().unwrap().as_mut().filter(|s| s.game_id == game_id) {
        s.prefs = prefs.clone();
    }
    crate::reader::server::send_prefs(&prefs);
    let _ = app.emit("novel-prefs", &prefs);
    Ok(())
}

/* ------------------------------------------------------------------ */
/* Ciclo de vida                                                        */
/* ------------------------------------------------------------------ */

/// Se llama en un hilo propio al lanzar un juego de la categoria de novelas japonesas.
pub fn on_launch(app: AppHandle, game: Game, pids: Arc<Mutex<Vec<u32>>>) {
    let prefs = load_prefs(&app, &game.id);
    let (tx, rx) = mpsc::channel::<Msg>();
    *SESSION.lock().unwrap() = Some(Session { game_id: game.id.clone(), game_name: game.name.clone(), prefs: prefs.clone(), tx });
    {
        let app2 = app.clone();
        std::thread::spawn(move || line_worker(app2, rx));
    }
    // Diccionario en segundo plano, para que la primera linea no espere.
    std::thread::spawn(|| {
        tokenizer();
    });
    load_dicts(&app, false);

    if let Err(e) = crate::reader::server::start(&app) {
        let _ = app.emit("th-log", format!("Pagina de lectura: {e}"));
    }
    crate::shots::clear();
    crate::anki::ensure_running(&app);
    // Sin hilo guardado (primera vez con este juego) la pagina de lectura abre sola la configuracion del texthook.
    crate::reader::server::set_extra("needHook", serde_json::json!(prefs.thread_key.is_none()));
    crate::reader::server::set_session(Some(game.name.clone()), Some(prefs.clone()));
    readstats::start(&app, &game.id, &game.name);

    // Esperar al proceso del juego (el vigilante de `launcher` va rellenando los PIDs).
    let started = Instant::now();
    let pid = loop {
        if let Some(p) = pids.lock().unwrap().first().copied() {
            break Some(p);
        }
        if started.elapsed() > Duration::from_secs(90) || !session_is(&game.id) {
            break None;
        }
        std::thread::sleep(Duration::from_millis(300));
    };
    let Some(pid) = pid else { return };
    // El hilo guardado se elige antes de conectar para no perder las primeras lineas.
    if let Some(k) = prefs.thread_key.clone() {
        crate::texthook::select_thread(k);
    }
    crate::texthook::select_name_thread(prefs.name_thread_key.clone());
    if let Err(e) = crate::texthook::attach(&app, pid, &game.id, &game.name, &prefs.hook_codes) {
        let _ = app.emit("th-log", e);
    }

    // Dar tiempo a que el juego abra su ventana para saber en que monitor esta.
    std::thread::sleep(Duration::from_secs(3));
    let r = crate::reader::monitor::reading_monitor(Some(pid));
    // La configuracion del texthook va dentro de la pagina de lectura; la ventana aparte solo sale si no se usa la pagina.
    if prefs.out.reader {
        open_browser(&app, (r.x, r.y, r.w, r.h));
    } else {
        open_hook_window(&app, (r.x, r.y, r.w, r.h));
    }
}

/// Se llama cuando termina la partida de una novela japonesa.
pub fn on_exit(app: &AppHandle, game_id: &str) {
    {
        let mut s = SESSION.lock().unwrap();
        if s.as_ref().map(|s| s.game_id == game_id).unwrap_or(false) {
            *s = None; // al soltar el `Sender` termina el hilo de lineas
        }
    }
    readstats::finish(app);
    // Una grabacion a medias no pasa a la partida siguiente.
    crate::audio::cancel(app);
    crate::texthook::detach_all(app);
    crate::reader::server::bye();
    crate::shots::clear();
    if let Some(w) = app.get_webview_window("texthook") {
        let _ = w.close();
    }
}

/// (id, nombre) del juego de la partida en curso.
pub fn current_game() -> Option<(String, String)> {
    SESSION.lock().unwrap().as_ref().map(|s| (s.game_id.clone(), s.game_name.clone()))
}

fn session_is(game_id: &str) -> bool {
    SESSION.lock().unwrap().as_ref().map(|s| s.game_id == game_id).unwrap_or(false)
}

fn open_browser(app: &AppHandle, rect: (i32, i32, u32, u32)) {
    let mut b = BROWSER.lock().unwrap();
    if let Some(c) = b.as_mut() {
        if matches!(c.try_wait(), Ok(None)) {
            return; // sigue abierto: la pagina se reconecta sola
        }
    }
    let profile = app.state::<AppState>().config_dir.join("reader-profile");
    match crate::reader::browser::open(&crate::reader::server::url(), &profile, rect) {
        Ok(c) => *b = Some(c),
        Err(e) => {
            let _ = app.emit("th-log", e);
        }
    }
}

/// Ventana de configuracion del hook, centrada en el monitor de lectura.
fn open_hook_window(app: &AppHandle, (x, y, w, h): (i32, i32, u32, u32)) {
    if let Some(win) = app.get_webview_window("texthook") {
        let _ = win.unminimize();
        let _ = win.set_focus();
        return;
    }
    let app2 = app.clone();
    // Las ventanas se crean en el hilo principal.
    let _ = app.run_on_main_thread(move || {
        let scale = app2
            .monitor_from_point(x as f64 + 1.0, y as f64 + 1.0)
            .ok()
            .flatten()
            .map(|m| m.scale_factor())
            .unwrap_or(1.0);
        let (mw, mh) = (w as f64 / scale, h as f64 / scale);
        let (ww, wh) = ((mw * 0.9).min(1280.0), (mh * 0.9).min(760.0));
        let (px, py) = (x as f64 / scale + (mw - ww) / 2.0, y as f64 / scale + (mh - wh) / 2.0);
        let built = WebviewWindowBuilder::new(&app2, "texthook", WebviewUrl::App("hook.html".into()))
            .title("Texthook")
            .inner_size(ww, wh)
            .min_inner_size(900.0, 560.0)
            .position(px, py)
            .background_color(tauri::window::Color(236, 238, 242, 255))
            .build();
        if let Err(e) = built {
            eprintln!("No se pudo abrir la ventana de texthook: {e}");
        }
    });
}

/* ------------------------------------------------------------------ */
/* Texto                                                                */
/* ------------------------------------------------------------------ */

/// Texto bruto de un hilo recibido de LunaHook. Lo llama `texthook` desde su hilo de callbacks.
pub fn on_text(app: &AppHandle, key: &str, text: &str) {
    let (filters, tx) = {
        let s = SESSION.lock().unwrap();
        let Some(s) = s.as_ref() else { return };
        (s.prefs.filters.clone(), s.tx.clone())
    };
    let clean = filters::apply(&filters, text);
    if clean.trim().is_empty() {
        return;
    }
    let _ = app.emit("th-text", TextEvent { key, text: &clean });
    if crate::texthook::name_thread_key().as_deref() == Some(key) {
        let _ = tx.send(Msg::Name(clean));
    } else if crate::texthook::selected_key().as_deref() == Some(key) {
        let _ = tx.send(Msg::Main(clean));
    }
}

/// Construye las lineas del hilo elegido: deduplica, pone nombre, tokeniza y las reparte.
fn line_worker(app: AppHandle, rx: mpsc::Receiver<Msg>) {
    let mut dedupe = LineDeduper::new();
    let mut joiner = NameJoiner::new(5);
    let mut next_id: u64 = 1;
    // La linea pendiente se da por completa cuando lleva este tiempo sin cambiar.
    const SETTLE: Duration = Duration::from_millis(250);
    loop {
        let wait = dedupe.pending_age().map(|a| SETTLE.saturating_sub(a)).unwrap_or(SETTLE);
        let mut ready: Vec<String> = Vec::new();
        match rx.recv_timeout(wait) {
            Ok(Msg::Name(n)) => joiner.set_name(&n),
            Ok(Msg::Main(t)) => {
                let dedupe_on = SESSION.lock().unwrap().as_ref().map(|s| s.prefs.filters.dedupe_lines).unwrap_or(true);
                if dedupe_on {
                    ready.extend(dedupe.push(&t));
                } else {
                    ready.extend(dedupe.flush());
                    ready.push(t);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        // Aunque el juego siga repitiendo la misma linea, se emite en cuanto deja de cambiar.
        if dedupe.pending_age().map(|a| a >= SETTLE).unwrap_or(false) {
            ready.extend(dedupe.flush());
        }
        for text in ready {
            emit_line(&app, &mut joiner, next_id, &text);
            next_id += 1;
        }
    }
}

fn emit_line(app: &AppHandle, joiner: &mut NameJoiner, id: u64, text: &str) {
    let (game_id, mut prefs) = {
        let s = SESSION.lock().unwrap();
        let Some(s) = s.as_ref() else { return };
        (s.game_id.clone(), s.prefs.clone())
    };
    let (who, body) = match joiner.take() {
        Some(n) => (Some(n), text.to_string()),
        None if prefs.filters.split_name => names::split_name(text),
        None => (None, text.to_string()),
    };
    if let Some(n) = &who {
        if names::ensure_character(&mut prefs, n) {
            let _ = update_prefs(app, &game_id, prefs.clone());
        }
    }
    let color = who.as_deref().and_then(|n| names::color_of(&prefs, n));
    let line = match tokenizer() {
        Some(t) => {
            let mut l = tokenize::build_line(t, id, now_ms(), who, color, &body);
            if let Some(d) = dict() {
                l.words = d.words(&l.toks);
                l.grammar = d.grammar(&l.toks);
            }
            l
        }
        None => visor_core::jp::ReaderLine { id, at: now_ms(), who, color, text: body.clone(), toks: Vec::new(), words: Vec::new(), grammar: Vec::new() },
    };
    let _ = app.emit("th-line", &line);
    readstats::on_line(app, &line.text);
    if prefs.out.reader {
        crate::reader::server::broadcast(&line);
        // Captura del juego de esta frase (solo en memoria; se guarda al pedir Anki o ★).
        crate::shots::capture_line(app, line.id);
        // Si la frase ya esta en el glosario, la pagina recibe su estado (traduccion guardada, estrella...).
        crate::glossary::on_line(app, &line);
    }
    if prefs.out.clipboard {
        let copy = match &line.who {
            Some(n) => format!("{n}{}", line.text),
            None => line.text.clone(),
        };
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(copy);
        }
    }
}

fn known_list() -> Vec<String> {
    KNOWN.lock().unwrap().as_ref().map(|k| k.list()).unwrap_or_default()
}

/// La pagina alterna una palabra conocida: se guarda y se reparte la lista nueva a todas las paginas.
fn toggle_known(app: &AppHandle, base: &str) {
    let list = {
        let mut g = KNOWN.lock().unwrap();
        let k = g.get_or_insert_with(Known::default);
        let on = k.toggle(base);
        readstats::on_known(app, base, on);
        crate::state::log_db_err("palabras conocidas", db(app).with(|c| dbd::known_set(c, base, on)));
        k.list()
    };
    publish_known(&list);
}

fn publish_known(list: &[String]) {
    crate::reader::server::set_extra("known", serde_json::json!(list));
    crate::reader::server::send(serde_json::json!({ "type": "known", "list": list }));
}

/// Eventos de la ventana «Texthook» que tambien recibe su panel en la pagina de lectura.
const HOOK_EVENTS: [&str; 8] = ["th-proc", "th-thread", "th-thread-removed", "th-text", "th-line", "th-search", "th-search-done", "th-log"];

/// Comando del panel de texthook de la pagina de lectura (`{"type":"hook","id","cmd","args"}`): los mismos que los
/// comandos Tauri de la ventana, respondidos con `{"type":"hookRes","id","ok","value"|"error"}`. En un hilo propio
/// porque algunos bloquean (reinyectar).
fn hook_cmd(app: &AppHandle, v: serde_json::Value) {
    use serde_json::{json, Value};
    let app = app.clone();
    std::thread::spawn(move || {
        let id = v.get("id").cloned().unwrap_or(Value::Null);
        let cmd = v.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
        let args = v.get("args").cloned().unwrap_or_else(|| json!({}));
        let s = |k: &str| args.get(k).and_then(|x| x.as_str()).map(str::to_string);
        let none = |r: Result<(), String>| r.map(|_| Value::Null);
        let r: Result<Value, String> = match cmd {
            "th_status" => Ok(json!(crate::texthook::status())),
            "th_select_thread" => {
                if let Some(k) = s("key") {
                    crate::texthook::select_thread(k);
                }
                Ok(Value::Null)
            }
            "th_select_name_thread" => {
                crate::texthook::select_name_thread(s("key"));
                Ok(Value::Null)
            }
            "th_insert_code" => none(crate::texthook::insert_code(&s("code").unwrap_or_default())),
            "th_remove_hook" => none(crate::texthook::remove_hook(&s("key").unwrap_or_default())),
            "th_search_text" => {
                let cp = args.get("codepage").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
                let deep = args.get("deep").and_then(|x| x.as_bool()).unwrap_or(false);
                none(crate::texthook::search_text(&app, &s("text").unwrap_or_default(), cp, deep))
            }
            "th_search_cancel" => {
                crate::texthook::search_cancel(&app);
                Ok(Value::Null)
            }
            "th_reinject" => none(crate::texthook::reinject(&app)),
            "th_detach" => {
                crate::texthook::detach_all(&app);
                Ok(Value::Null)
            }
            "novel_get_prefs" => match s("gameId") {
                Some(g) => Ok(json!(novel_get_prefs(app.clone(), g))),
                None => Err("Falta el juego".into()),
            },
            "novel_save_prefs" => match (s("gameId"), args.get("prefs").cloned().map(serde_json::from_value::<NovelPrefs>)) {
                (Some(g), Some(Ok(p))) => none(update_prefs(&app, &g, p)),
                (_, Some(Err(e))) => Err(format!("Preferencias no válidas: {e}")),
                _ => Err("Faltan el juego o las preferencias".into()),
            },
            _ => Err(format!("Comando desconocido: {cmd}")),
        };
        let msg = match r {
            Ok(value) => json!({ "type": "hookRes", "id": id, "ok": true, "value": value }),
            Err(error) => json!({ "type": "hookRes", "id": id, "ok": false, "error": error }),
        };
        crate::reader::server::send(msg);
    });
}

/// Cambios de furigana/colores/tamano hechos desde la propia pagina de lectura.
pub fn init(app: &AppHandle) {
    use tauri::Listener;
    for ev in HOOK_EVENTS {
        app.listen_any(ev, move |e| {
            let payload: serde_json::Value = serde_json::from_str(e.payload()).unwrap_or(serde_json::Value::Null);
            crate::reader::server::send(serde_json::json!({ "type": "hookEvt", "event": ev, "payload": payload }));
        });
    }
    *KNOWN.lock().unwrap() = Some(Known::from_list(crate::state::log_db("palabras conocidas", db(app).with(dbd::known_load))));
    crate::reader::server::set_extra("known", serde_json::json!(known_list()));
    let app2 = app.clone();
    crate::reader::server::on_page_prefs(move |v| {
        let Some(reader) = v.get("reader").cloned() else { return };
        let Some((game_id, mut prefs)) = SESSION.lock().unwrap().as_ref().map(|s| (s.game_id.clone(), s.prefs.clone())) else { return };
        if let Ok(r) = serde_json::from_value(reader) {
            prefs.reader = r;
            let _ = update_prefs(&app2, &game_id, prefs);
        }
    });
    let app3 = app.clone();
    crate::reader::server::on_page_cmd(move |v| match v.get("type").and_then(|t| t.as_str()) {
        Some("openDicts") => {
            let dir = dicts_dir(&app3);
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("explorer").arg(&dir).spawn();
        }
        Some("reloadDicts") => load_dicts(&app3, true),
        Some("translate") => {
            if let Some(id) = v.get("lineId").and_then(|x| x.as_u64()) {
                crate::ai::translate(&app3, id);
            }
        }
        Some("explain") => {
            if let Some(id) = v.get("lineId").and_then(|x| x.as_u64()) {
                crate::ai::explain(&app3, id);
            }
        }
        Some("star") => {
            if let Some(id) = v.get("lineId").and_then(|x| x.as_u64()) {
                crate::glossary::toggle_star(&app3, id);
            }
        }
        Some("record") => {
            // `lineId` solo viene al empezar; al parar basta con alternar.
            crate::audio::toggle(&app3, v.get("lineId").and_then(|x| x.as_u64()));
        }
        Some("synth") => {
            if let Some(id) = v.get("lineId").and_then(|x| x.as_u64()) {
                crate::audio::synth(&app3, id);
            }
        }
        Some("current") => crate::audio::set_selected(v.get("lineId").and_then(|x| x.as_u64()).unwrap_or(0)),
        Some(t @ ("glossaryList" | "phraseUpdate" | "phraseDelete" | "phraseGrade" | "phraseAnki")) => {
            if t == "glossaryList" {
                let f = v.get("filter").cloned().and_then(|f| serde_json::from_value(f).ok()).unwrap_or_default();
                crate::glossary::push_list(&app3, &f);
            } else {
                crate::glossary::on_page_cmd(&app3, t, &v);
            }
        }
        Some("hook") => hook_cmd(&app3, v.clone()),
        Some("statsGet") => readstats::push_stats(&app3),
        Some("ankiTest") => crate::anki::ensure_running(&app3),
        Some("anki") => crate::anki::send_note(&app3, v.clone()),
        Some("known") => {
            if let Some(b) = v.get("base").and_then(|b| b.as_str()) {
                toggle_known(&app3, b);
            }
        }
        _ => {}
    });
}

/* ------------------------------------------------------------------ */
/* Comandos                                                             */
/* ------------------------------------------------------------------ */

#[tauri::command]
pub fn novel_get_prefs(app: AppHandle, game_id: String) -> NovelPrefs {
    if let Some(s) = SESSION.lock().unwrap().as_ref().filter(|s| s.game_id == game_id) {
        return s.prefs.clone();
    }
    load_prefs(&app, &game_id)
}

#[tauri::command]
pub fn novel_save_prefs(app: AppHandle, game_id: String, prefs: NovelPrefs) -> Result<(), String> {
    update_prefs(&app, &game_id, prefs)
}

#[tauri::command]
pub fn novel_open_reader(app: AppHandle) -> Result<(), String> {
    crate::reader::server::start(&app)?;
    load_dicts(&app, false);
    let pid = crate::texthook::status().proc.map(|p| p.pid);
    let r = crate::reader::monitor::reading_monitor(pid);
    open_browser(&app, (r.x, r.y, r.w, r.h));
    Ok(())
}

#[tauri::command]
pub fn novel_reader_url() -> String {
    crate::reader::server::url()
}

#[tauri::command]
pub fn novel_yomitan_css() -> String {
    include_str!("../../src/reader/yomitan-popup.css").to_string()
}
