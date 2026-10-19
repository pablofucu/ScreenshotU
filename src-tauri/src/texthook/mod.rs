//! LunaHook: carga de LunaHost, inyeccion en el juego, hilos de texto, codigos H y busqueda de hooks.
//! (Agente A.) Ver `docs/novelas-japonesas.md`.
//!
//! Estructura: `luna.rs` es la capa FFI (callbacks `extern "C"` que solo encolan `luna::Event`); aqui un unico hilo de
//! trabajo consume la cola, mantiene el registro de hilos y emite los eventos Tauri del contrato (`th-proc`, `th-thread`,
//! `th-thread-removed`, `th-search`, `th-search-done`, `th-log`). El texto de cada hilo se entrega a
//! `crate::novel::on_text`, que filtra y reparte (`th-text`, `th-line`, pagina de lectura). El estado es global
//! (no hace falta registrarlo en Tauri).
pub mod commands;
pub mod luna;

use crate::novel::{ProcInfo, SearchHit, ThStatus, ThreadInfo};
use luna::{Event, Host, SearchParam, ThreadParam};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Un hilo esta «vivo» si recibio texto en este tiempo.
const LIVE_FOR: Duration = Duration::from_secs(3);
/// Minimo entre dos `th-thread` de un mismo hilo que no cambia de estado (evita inundar la ventana).
const EMIT_EVERY: Duration = Duration::from_millis(150);
/// Longitud maxima del ultimo texto que se guarda en `ThreadInfo.last`.
const LAST_MAX: usize = 200;
/// Textos mas largos se descartan (igual que `maxOutputSize` en LunaTranslator).
const TEXT_MAX: usize = 10_000;
/// Parametros de `Luna_Settings` (valores por defecto de LunaTranslator): retardo, codepage, buffer, historial.
const FLUSH_DELAY_MS: i32 = 500;
const CODEPAGE: i32 = 932;
const MAX_BUFFER: i32 = 3000;
const MAX_HISTORY: i32 = 1_000_000;

struct Th {
    tp: ThreadParam,
    info: ThreadInfo,
    last_at: Option<Instant>,
    dirty: bool,
    last_emit: Instant,
}

struct Search {
    deadline: Instant,
    last_hit: Option<Instant>,
    codes: HashMap<String, u32>,
}

#[derive(Clone)]
struct Params {
    game_id: String,
    game_name: String,
    hook_codes: Vec<String>,
}

struct State {
    app: Option<AppHandle>,
    host: Option<Arc<Host>>,
    pid: Option<u32>,
    proc: Option<ProcInfo>,
    params: Option<Params>,
    threads: Vec<Th>,
    selected: Option<String>,
    name_thread: Option<String>,
    search: Option<Search>,
    /// Se incrementa en cada attach/detach para que las tareas en segundo plano sepan si siguen vigentes.
    gen: u64,
}

static STATE: Mutex<State> = Mutex::new(State {
    app: None,
    host: None,
    pid: None,
    proc: None,
    params: None,
    threads: Vec::new(),
    selected: None,
    name_thread: None,
    search: None,
    gen: 0,
});

fn st() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn emit<S: Serialize + Clone>(app: &AppHandle, ev: &str, payload: S) {
    let _ = app.emit(ev, payload);
}

fn log(app: &AppHandle, msg: impl Into<String>) {
    emit(app, "th-log", msg.into());
}

/// Clave estable de un hilo entre sesiones: codigo de hook + contextos enmascarados a 16 bits (la direccion base de los
/// modulos cambia con ASLR pero sus 16 bits bajos no), como hace `match_compatibility` de LunaTranslator. Los
/// hooks de usuario («UserHook...») se comparan solo por contextos.
pub fn thread_key(code: &str, ctx: u64, ctx2: u64) -> String {
    let base = if code.starts_with("UserHook") { "UserHook" } else { code };
    format!("{}|{:04X}|{:04X}", base, ctx & 0xFFFF, ctx2 & 0xFFFF)
}

fn clip(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() <= LAST_MAX {
        t.to_string()
    } else {
        t.chars().take(LAST_MAX).collect()
    }
}

fn is_live(t: &Th, now: Instant) -> bool {
    t.last_at.map(|a| now.duration_since(a) < LIVE_FOR).unwrap_or(false)
}

/// Carga el host una sola vez (si falla no se guarda nada, para poder reintentar tras copiar los archivos).
fn ensure_host(app: &AppHandle) -> Result<Arc<Host>, String> {
    let mut s = st();
    if let Some(h) = &s.host {
        return Ok(h.clone());
    }
    let cfg = app.path().app_config_dir().ok();
    let dir = luna::find_dir(cfg.as_deref())?;
    let (tx, rx) = mpsc::channel();
    let host = Arc::new(Host::load(&dir, tx)?);
    host.settings(FLUSH_DELAY_MS, CODEPAGE, MAX_BUFFER, MAX_HISTORY);
    s.host = Some(host.clone());
    s.app = Some(app.clone());
    drop(s);
    std::thread::Builder::new()
        .name("texthook".into())
        .spawn(move || worker(rx))
        .map_err(|e| format!("No se pudo crear el hilo de LunaHook: {e}"))?;
    Ok(host)
}

/// Inyecta LunaHook en `pid`. Devuelve en cuanto el host esta cargado y el proceso es valido; la inyeccion (con
/// reintentos mientras el proceso no esta listo) y la conexion siguen en segundo plano, y su progreso se notifica
/// con `th-log`/`th-proc`. Al conectar se insertan `hook_codes`. La seleccion de hilos no se borra aqui.
pub fn attach(app: &AppHandle, pid: u32, game_id: &str, game_name: &str, hook_codes: &[String]) -> Result<(), String> {
    let host = ensure_host(app)?;
    let (exe, is64) = Host::process_info(pid)?;
    let info = ProcInfo { pid, exe, arch: if is64 { 64 } else { 32 }, connected: false };
    let (gen, removed, old) = {
        let mut s = st();
        s.app = Some(app.clone());
        let old = s.pid.filter(|&p| p != pid);
        let removed: Vec<String> = s.threads.drain(..).map(|t| t.info.key).collect();
        s.gen += 1;
        s.pid = Some(pid);
        s.proc = Some(info.clone());
        s.search = None;
        s.params = Some(Params {
            game_id: game_id.to_string(),
            game_name: game_name.to_string(),
            hook_codes: hook_codes.to_vec(),
        });
        (s.gen, removed, old)
    };
    if let Some(o) = old {
        host.detach(o);
    }
    for key in removed {
        emit(app, "th-thread-removed", serde_json::json!({ "key": key }));
    }
    emit(app, "th-proc", info);
    let app2 = app.clone();
    std::thread::Builder::new()
        .name("texthook-inject".into())
        .spawn(move || inject_task(app2, host, pid, is64, gen))
        .map_err(|e| format!("No se pudo crear el hilo de inyeccion: {e}"))?;
    Ok(())
}

fn current(gen: u64) -> bool {
    st().gen == gen
}

fn inject_task(app: AppHandle, host: Arc<Host>, pid: u32, is64: bool, gen: u64) {
    host.connect_process(pid);
    log(&app, format!("Inyectando LunaHook en el proceso {pid} ({} bits)...", if is64 { 64 } else { 32 }));
    let mut last_err = String::new();
    let mut done = false;
    for _ in 0..60 {
        if !current(gen) {
            return;
        }
        // Si el mutex del hook ya existe, el juego ya tiene LunaHook cargado.
        if !host.need_inject(pid) {
            done = true;
            break;
        }
        match host.inject(pid, is64) {
            Ok(()) => {
                done = true;
                break;
            }
            Err(e) => {
                if e != last_err {
                    log(&app, format!("No se pudo inyectar todavia ({e}); reintentando..."));
                    last_err = e;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
    if !done {
        log(&app, format!("Error: no se pudo inyectar LunaHook en el juego: {last_err}"));
        return;
    }
    for _ in 0..80 {
        if !current(gen) {
            return;
        }
        if st().proc.as_ref().map(|p| p.connected).unwrap_or(false) {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    log(&app, "LunaHook no se ha conectado con el juego (puede que lo bloquee un antivirus o que falten las DLL de Visual C++).");
}

/// Hilo de trabajo: consume los eventos del host y, cada 200 ms sin eventos, refresca `live` y la busqueda.
fn worker(rx: Receiver<Event>) {
    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(ev) => handle(ev),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        tick();
    }
}

fn handle(ev: Event) {
    let Some(app) = st().app.clone() else { return };
    match ev {
        Event::Connect(pid) => {
            let (info, host, codes) = {
                let mut s = st();
                if s.pid != Some(pid) {
                    return;
                }
                let Some(p) = s.proc.as_mut() else { return };
                p.connected = true;
                let info = p.clone();
                (info, s.host.clone(), s.params.as_ref().map(|p| p.hook_codes.clone()).unwrap_or_default())
            };
            emit(&app, "th-proc", info);
            log(&app, "LunaHook conectado con el juego.");
            if let Some(h) = host {
                for c in codes.iter().map(|c| c.trim()).filter(|c| !c.is_empty()) {
                    h.insert_code(pid, c);
                    log(&app, format!("Codigo H insertado: {c}"));
                }
            }
        }
        Event::Disconnect(pid) => {
            let (info, removed) = {
                let mut s = st();
                if s.pid != Some(pid) {
                    return;
                }
                let Some(p) = s.proc.as_mut() else { return };
                p.connected = false;
                let info = p.clone();
                let removed: Vec<String> = s.threads.drain(..).map(|t| t.info.key).collect();
                (info, removed)
            };
            for key in removed {
                emit(&app, "th-thread-removed", serde_json::json!({ "key": key }));
            }
            emit(&app, "th-proc", info);
            log(&app, "LunaHook desconectado del juego.");
        }
        Event::Create { code, name, tp } => {
            let info = {
                let mut s = st();
                let t = upsert(&mut s.threads, tp, &code, &name);
                t.last_emit = Instant::now();
                t.dirty = false;
                t.info.clone()
            };
            emit(&app, "th-thread", info);
        }
        Event::Destroy { tp } => {
            let key = {
                let mut s = st();
                s.threads.iter().position(|t| t.tp == tp).map(|i| s.threads.remove(i).info.key)
            };
            if let Some(key) = key {
                emit(&app, "th-thread-removed", serde_json::json!({ "key": key }));
            }
        }
        Event::Output { code, name, tp, text } => {
            if text.is_empty() || text.chars().count() > TEXT_MAX {
                return;
            }
            let (key, emit_now) = {
                let mut s = st();
                let t = upsert(&mut s.threads, tp, &code, &name);
                let now = Instant::now();
                t.info.lines += 1;
                t.info.last = clip(&text);
                t.last_at = Some(now);
                t.info.live = true;
                let first = t.info.lines == 1;
                t.dirty = true;
                let go = first || now.duration_since(t.last_emit) >= EMIT_EVERY;
                if go {
                    t.dirty = false;
                    t.last_emit = now;
                }
                (t.info.key.clone(), go.then(|| t.info.clone()))
            };
            if let Some(info) = emit_now {
                emit(&app, "th-thread", info);
            }
            raw_log(&app, &key, &text);
            // Fuera del bloqueo: `on_text` puede consultar el estado (p. ej. el hilo elegido).
            crate::novel::on_text(&app, &key, &text);
        }
        Event::Info(kind, text) => {
            let msg = match kind {
                0 | 3 => text,
                1 | 2 => format!("[Aviso] {text}"),
                4 => format!("Motor del juego: {text}"),
                _ => text,
            };
            log(&app, msg);
        }
        Event::Found { code, text } => {
            let hit = {
                let mut s = st();
                let Some(se) = s.search.as_mut() else { return };
                se.last_hit = Some(Instant::now());
                let n = se.codes.entry(code.clone()).or_insert(0);
                *n += 1;
                // Unos pocos textos de ejemplo por hook bastan para reconocerlo.
                (*n <= 3).then(|| SearchHit { code, text: clip(&text) })
            };
            if let Some(h) = hit {
                emit(&app, "th-search", h);
            }
        }
    }
}

/// Registro en bruto de lo que sacan TODOS los hilos (`<config>/texthook-raw.log`), para diagnosticar texto que no
/// llega al hilo elegido. Una linea por salida: `hora UTC \t clave \t texto` (saltos como `\n`). Se vacia al pasar de
/// `RAW_LOG_MAX`.
fn raw_log(app: &AppHandle, key: &str, text: &str) {
    use std::io::Write;
    const RAW_LOG_MAX: u64 = 4 * 1024 * 1024;
    let Ok(dir) = app.path().app_config_dir() else { return };
    let path = dir.join("texthook-raw.log");
    let big = std::fs::metadata(&path).map(|m| m.len() > RAW_LOG_MAX).unwrap_or(false);
    let file = std::fs::OpenOptions::new().create(true).write(true).append(!big).truncate(big).open(&path);
    let Ok(mut f) = file else { return };
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let day = ms % 86_400_000;
    let at = format!("{:02}:{:02}:{:02}.{:03}", day / 3_600_000, day / 60_000 % 60, day / 1000 % 60, day % 1000);
    let _ = writeln!(f, "{at}\t{key}\t{}", text.replace('\r', "\\r").replace('\n', "\\n"));
}

/// Busca el hilo por `tp` y, si no, por clave (misma instancia tras reconectar); si no existe lo crea.
fn upsert<'a>(threads: &'a mut Vec<Th>, tp: ThreadParam, code: &str, name: &str) -> &'a mut Th {
    let key = thread_key(code, tp.ctx, tp.ctx2);
    let idx = threads.iter().position(|t| t.tp == tp).or_else(|| threads.iter().position(|t| t.info.key == key));
    match idx {
        Some(i) => {
            let t = &mut threads[i];
            t.tp = tp;
            if !code.is_empty() {
                t.info.code = code.to_string();
            }
            if !name.is_empty() {
                t.info.name = name.to_string();
            }
            t
        }
        None => {
            threads.push(Th {
                tp,
                info: ThreadInfo { key, name: name.to_string(), code: code.to_string(), lines: 0, last: String::new(), live: false },
                last_at: None,
                dirty: false,
                last_emit: Instant::now(),
            });
            threads.last_mut().expect("recien insertado")
        }
    }
}

/// Reenvia los hilos con cambios pendientes o que han dejado de estar vivos y cierra busquedas terminadas.
fn tick() {
    let now = Instant::now();
    let (app, emits, done) = {
        let mut s = st();
        let Some(app) = s.app.clone() else { return };
        let mut emits = Vec::new();
        for t in s.threads.iter_mut() {
            let live = is_live(t, now);
            let pendiente = t.dirty && now.duration_since(t.last_emit) >= EMIT_EVERY;
            if pendiente || t.info.live != live {
                t.info.live = live;
                t.dirty = false;
                t.last_emit = now;
                emits.push(t.info.clone());
            }
        }
        let fin = s
            .search
            .as_ref()
            .map(|se| now > se.deadline || se.last_hit.map(|h| now.duration_since(h) > Duration::from_secs(2)).unwrap_or(false))
            .unwrap_or(false);
        let done = if fin { s.search.take().map(|se| se.codes.len()) } else { None };
        (app, emits, done)
    };
    for info in emits {
        emit(&app, "th-thread", info);
    }
    if let Some(count) = done {
        emit(&app, "th-search-done", serde_json::json!({ "count": count }));
    }
}

/// Desconecta de todos los procesos, vacia los hilos y la seleccion. Emite `th-thread-removed` y `th-proc`
/// (con `connected: false`).
pub fn detach_all(app: &AppHandle) {
    teardown(app, false);
}

fn teardown(app: &AppHandle, keep_selection: bool) {
    let (host, pid, proc, removed, search) = {
        let mut s = st();
        s.gen += 1;
        let pid = s.pid.take();
        let proc = s.proc.take();
        let removed: Vec<String> = s.threads.drain(..).map(|t| t.info.key).collect();
        let search = s.search.take().is_some();
        s.params = None;
        if !keep_selection {
            s.selected = None;
            s.name_thread = None;
        }
        (s.host.clone(), pid, proc, removed, search)
    };
    if let (Some(h), Some(p)) = (host, pid) {
        h.detach(p);
    }
    for key in removed {
        emit(app, "th-thread-removed", serde_json::json!({ "key": key }));
    }
    if search {
        emit(app, "th-search-done", serde_json::json!({ "count": 0 }));
    }
    if let Some(mut p) = proc {
        p.connected = false;
        emit(app, "th-proc", p);
    }
}

/// Estado actual para la ventana de hook.
pub fn status() -> ThStatus {
    let s = st();
    let now = Instant::now();
    ThStatus {
        proc: s.proc.clone(),
        threads: s
            .threads
            .iter()
            .map(|t| {
                let mut i = t.info.clone();
                i.live = is_live(t, now);
                i
            })
            .collect(),
        selected: s.selected.clone(),
        name_thread: s.name_thread.clone(),
        game_id: s.params.as_ref().map(|p| p.game_id.clone()),
        game_name: s.params.as_ref().map(|p| p.game_name.clone()),
    }
}

/// Clave del hilo principal elegido (para que la integracion la consulte).
pub fn selected_key() -> Option<String> {
    st().selected.clone()
}

/// Clave del hilo de nombre elegido.
pub fn name_thread_key() -> Option<String> {
    st().name_thread.clone()
}

pub fn select_thread(key: String) {
    st().selected = Some(key);
}

pub fn select_name_thread(key: Option<String>) {
    st().name_thread = key;
}

/// Inserta un codigo H (p. ej. `/HW-1C@42E640:game.exe`) en el juego conectado.
pub fn insert_code(code: &str) -> Result<(), String> {
    let code = code.trim();
    if code.is_empty() {
        return Err("El codigo H esta vacio".into());
    }
    let (host, pid, app) = {
        let s = st();
        let ok = s.proc.as_ref().map(|p| p.connected).unwrap_or(false);
        if !ok {
            return Err("LunaHook no esta conectado con ningun juego".into());
        }
        (s.host.clone(), s.pid, s.app.clone())
    };
    let (Some(host), Some(pid)) = (host, pid) else { return Err("LunaHook no esta conectado con ningun juego".into()) };
    host.insert_code(pid, code);
    if let Some(app) = app {
        log(&app, format!("Codigo H insertado: {code}"));
    }
    Ok(())
}

/// Quita el hook del hilo `key` (por la direccion de su hook) y lo borra de la lista.
pub fn remove_hook(key: &str) -> Result<(), String> {
    let (host, pid, app, tp) = {
        let mut s = st();
        let Some(i) = s.threads.iter().position(|t| t.info.key == key) else {
            return Err("Ese hilo ya no existe".into());
        };
        let t = s.threads.remove(i);
        (s.host.clone(), s.pid, s.app.clone(), t.tp)
    };
    if let (Some(h), Some(_)) = (host, pid) {
        h.remove_hook(tp.process_id, tp.addr);
    }
    if let Some(app) = app {
        emit(&app, "th-thread-removed", serde_json::json!({ "key": key }));
    }
    Ok(())
}

/// Busca hooks cuyo texto contiene `text` (minimo 3 caracteres). `deep` amplia la busqueda a todos los modulos del
/// proceso (mas lento y mas arriesgado para el juego); si no, solo al modulo del exe.
pub fn search_text(app: &AppHandle, text: &str, codepage: u32, deep: bool) -> Result<(), String> {
    let text = text.trim();
    if text.chars().count() < 3 {
        return Err("El texto a buscar es demasiado corto (minimo 3 caracteres)".into());
    }
    let (host, pid, proc) = {
        let s = st();
        (s.host.clone(), s.pid, s.proc.clone())
    };
    let (Some(host), Some(pid), Some(proc)) = (host, pid, proc) else {
        return Err("LunaHook no esta conectado con ningun juego".into());
    };
    if !proc.connected {
        return Err("LunaHook aun no esta conectado con el juego".into());
    }
    let sp = SearchParam::for_text(text, codepage, proc.arch == 64, if deep { "" } else { &proc.exe });
    {
        let mut s = st();
        if s.search.is_some() {
            return Err("Ya hay una busqueda en curso".into());
        }
        // La busqueda dura `search_time` (30 s) y los resultados llegan al final.
        s.search = Some(Search { deadline: Instant::now() + Duration::from_millis(sp.search_time as u64 + 40_000), last_hit: None, codes: HashMap::new() });
    }
    log(app, format!("Buscando el texto en los hooks del juego ({} s)... Haz avanzar el texto del juego.", sp.search_time / 1000));
    std::thread::spawn(move || host.find_hooks(pid, sp));
    Ok(())
}

/// LunaHook no permite cancelar: se dejan de mostrar resultados y se da la busqueda por terminada.
pub fn search_cancel(app: &AppHandle) {
    let count = st().search.take().map(|se| se.codes.len());
    if let Some(count) = count {
        emit(app, "th-search-done", serde_json::json!({ "count": count }));
    }
}

/// Desconecta y vuelve a inyectar en el mismo proceso conservando la seleccion de hilos.
pub fn reinject(app: &AppHandle) -> Result<(), String> {
    let (pid, params) = {
        let s = st();
        (s.pid, s.params.clone())
    };
    let (Some(pid), Some(p)) = (pid, params) else {
        return Err("No hay ningun juego conectado".into());
    };
    teardown(app, true);
    std::thread::sleep(Duration::from_millis(800));
    attach(app, pid, &p.game_id, &p.game_name, &p.hook_codes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_estable_con_aslr() {
        let a = thread_key("HW-1C@42E640:game.exe", 0x7FF6_1234_ABCD, 0x1_0002);
        let b = thread_key("HW-1C@42E640:game.exe", 0x7FF6_9999_ABCD, 0x2_0002);
        assert_eq!(a, b);
        assert_eq!(a, "HW-1C@42E640:game.exe|ABCD|0002");
        assert_ne!(a, thread_key("HW-1C@42E640:game.exe", 0xABCE, 2));
    }

    #[test]
    fn key_userhook() {
        assert_eq!(thread_key("UserHook1", 1, 0), thread_key("UserHook2", 1, 0));
    }

    #[test]
    fn recorte() {
        let t = "あ".repeat(500);
        assert_eq!(clip(&format!("  {t}  ")).chars().count(), LAST_MAX);
        assert_eq!(clip(" hola "), "hola");
    }
}
