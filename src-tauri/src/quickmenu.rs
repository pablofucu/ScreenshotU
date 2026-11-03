//! Menu sobre el juego: ventana a pantalla completa con una foto congelada del juego de fondo (`quickmenu.html`).
//! Toma el primer plano al abrirse para que los escaladores como Magpie cedan `HWND_TOPMOST` sin detener el escalado,
//! y devuelve el foco a la ventana anterior al cerrarse; cuando Magpie no escala se pausa el proceso antes de mostrarla.
//! Teclado, mando y raton comparten el gesto de `visor_core::trigger`: pulsar abre o cierra, mantener fuerza el cierre.
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use visor_core::trigger::{TriggerAct, TriggerCtx, TriggerFsm};

const LABEL: &str = "quickmenu";
/// Lo que dura la animacion de salida de la pagina.
const CLOSE_ANIM_MS: u64 = 230;
/// Capturas que se recuerdan de esta partida.
const MAX_SHOTS: usize = 12;
/// Una pulsacion cuya subida no llega nunca (se perdio el evento) no debe bloquear el atajo.
const STUCK_MS: u64 = 15_000;

/// De donde viene la pulsacion.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Src {
    Key,
    Pad,
    Mouse,
}

struct Trig {
    fsm: TriggerFsm,
    src: Option<Src>,
    /// Mandos que hicieron el gesto; 0 = teclado o raton (vibran todos).
    pads: u8,
    since: u64,
}

static TRIG: Mutex<Trig> = Mutex::new(Trig { fsm: TriggerFsm::new(), src: None, pads: 0, since: 0 });
static EPOCH: OnceLock<Instant> = OnceLock::new();
static IS_OPEN: AtomicBool = AtomicBool::new(false);
/// Cada apertura tiene su numero: los hilos de la anterior terminan y una salida pendiente no oculta la nueva.
static GEN: AtomicU64 = AtomicU64::new(0);
/// Abrir y cerrar no se solapan.
static LIFE: Mutex<()> = Mutex::new(());
/// Contexto del juego que manda la ventana principal al empezar la partida.
static CONTEXT: Mutex<Option<Value>> = Mutex::new(None);
static SHOTS: Mutex<Vec<QmShot>> = Mutex::new(Vec::new());
/// Foto completa del juego al abrir: sirve de fondo y de captura mientras el juego esta en pausa.
static FROZEN: Mutex<Option<crate::platform::Grabbed>> = Mutex::new(None);
static BACKGROUND: Mutex<Option<String>> = Mutex::new(None);
/// Procesos que hay que reanudar.
static SUSPENDED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
/// Ventana que tenia el primer plano antes de abrir el menu (se restaura al cerrar para que Magpie vuelva a escalar).
static PREV_FG: Mutex<Option<isize>> = Mutex::new(None);
/// HWND (como `isize`) de la ventana del menu, fijado por `show_window` tras mostrarla.
static OWN_HWND: Mutex<Option<isize>> = Mutex::new(None);
/// Posicion y tamano con los que se mostro la ventana la ultima vez (para el reintento de `confirm`).
static LAST_RECT: Mutex<Option<(i32, i32, u32, u32)>> = Mutex::new(None);

/// Cierre forzado pedido en esta partida: el vigilante deja de esperar y da la sesion por terminada.
static FORCED: AtomicBool = AtomicBool::new(false);
/// Ruta de `quickmenu.log` (se fija al empezar la partida).
static LOG_PATH: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);
/// Ultimo `gen` que la pagina confirmo haber pintado (comando `quickmenu_painted`).
static PAINTED: AtomicU64 = AtomicU64::new(0);
/// La partida esta terminando: ningun atajo abre ni cierra nada mas.
static ENDING: AtomicBool = AtomicBool::new(false);
/// Sube en cada `prewarm`: la ventana solo se destruye al final si sigue siendo la de esa misma partida.
static SESSION: AtomicU64 = AtomicU64::new(0);
/// Hay un `to_top` encolado en el hilo principal sin ejecutar todavia (no se amontonan).
static TOP_PENDING: AtomicBool = AtomicBool::new(false);

/// `lock()` que tolera un mutex envenenado: un fallo en otro hilo no debe impedir guardar la sesion.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Se ha pedido cerrar el juego a la fuerza.
pub fn force_requested() -> bool {
    FORCED.load(Ordering::SeqCst)
}

/// El vigilante termina: la proxima partida empieza sin cierre pendiente.
pub fn clear_forced() {
    FORCED.store(false, Ordering::SeqCst);
}

/// Vacia `quickmenu.log` (empieza la partida).
fn log_reset(app: &AppHandle) {
    let path = app.state::<AppState>().config_dir.join("quickmenu.log");
    let _ = std::fs::write(&path, b"");
    *lock(&LOG_PATH) = Some(path);
}

/// Una linea con hora en `quickmenu.log`. Un fallo al escribir no afecta a nada.
pub fn log(msg: &str) {
    use std::io::Write;
    let Some(path) = lock(&LOG_PATH).clone() else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{} {msg}", chrono::Local::now().format("%H:%M:%S%.3f"));
    }
}

/// Mata un proceso. `true` si Windows lo acepto.
pub fn terminate_pid(pid: u32) -> bool {
    proc::terminate(pid)
}

fn now_ms() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

pub fn is_open() -> bool {
    IS_OPEN.load(Ordering::SeqCst)
}

/// El hilo que pregunta sigue siendo el de la apertura actual.
pub fn is_current(gen: u64) -> bool {
    is_open() && GEN.load(Ordering::SeqCst) == gen
}

fn active_pids(app: &AppHandle) -> Option<Vec<u32>> {
    let st = app.state::<AppState>();
    let a = lock(&st.active);
    a.as_ref().map(|a| lock(&a.pids).clone())
}

/* ---------------- atajo ---------------- */

/// Pulsacion de un disparador. Sin partida en marcha no hace nada.
pub fn trigger_down(app: &AppHandle, src: Src, pads: u8) {
    if active_pids(app).is_none() {
        return;
    }
    if ENDING.load(Ordering::SeqCst) || force_requested() {
        log("atajo ignorado: la partida termina");
        return;
    }
    let (hold_force, hold_ms) = {
        let st = app.state::<AppState>();
        let c = st.config.lock().unwrap();
        (c.overlay_hold_force, c.overlay_hold_ms)
    };
    let open = is_open();
    let thread = std::thread::current().name().unwrap_or("?").to_string();
    log(&format!("atajo pulsado (src: {src:?}, hilo: {thread}, menu abierto: {open})"));
    let ctx = TriggerCtx { can_show: open || !exclusive_fullscreen(), is_open: open, hold_force, hold_ms };
    let now = now_ms();
    let act = {
        let mut t = lock(&TRIG);
        if t.fsm.is_active() {
            if now.saturating_sub(t.since) < STUCK_MS {
                return;
            }
            t.fsm = TriggerFsm::new();
        }
        t.src = Some(src);
        t.pads = pads;
        t.since = now;
        t.fsm.down(now, ctx)
    };
    run_act(app, act, pads);
    let app = app.clone();
    let _ = std::thread::Builder::new().name("qm-trigger".into()).spawn(move || loop {
        std::thread::sleep(Duration::from_millis(30));
        let now = now_ms();
        let (act, pads) = {
            let mut t = lock(&TRIG);
            if !t.fsm.is_active() || t.since + STUCK_MS < now {
                return;
            }
            (t.fsm.tick(now), t.pads)
        };
        run_act(&app, act, pads);
    });
}

/// Se solto un disparador. La subida de uno que no es el de la pulsacion activa no cuenta.
pub fn trigger_up(app: &AppHandle, src: Src) {
    let (act, pads) = {
        let mut t = lock(&TRIG);
        if t.src != Some(src) {
            return;
        }
        t.src = None;
        (t.fsm.up(now_ms()), t.pads)
    };
    run_act(app, act, pads);
}

fn run_act(app: &AppHandle, act: TriggerAct, pads: u8) {
    match act {
        TriggerAct::None => {}
        TriggerAct::Open => {
            log("accion: abrir");
            in_thread(app, open)
        }
        TriggerAct::Close => {
            log("accion: cerrar");
            in_thread(app, close)
        }
        TriggerAct::Warn => {
            log("accion: aviso (no se puede abrir)");
            if feedback_pads(pads) == 0 {
                beep();
            } else {
                crate::pad::rumble_n(feedback_pads(pads), 2);
            }
        }
        TriggerAct::HoldStart => emit_hold(app, 0.0),
        TriggerAct::HoldProgress(p) => emit_hold(app, p),
        TriggerAct::HoldCancel => emit_hold(app, -1.0),
        TriggerAct::HoldTick => crate::pad::rumble_tick(feedback_pads(pads)),
        TriggerAct::Force => {
            log("accion: forzar cierre");
            let fb = feedback_pads(pads);
            if fb == 0 {
                beep();
            } else {
                crate::pad::rumble_long(fb, 400);
            }
            let app = app.clone();
            std::thread::spawn(move || force_close_game(&app));
        }
    }
}

/// Mandos que deben notar el gesto: el que lo hizo o, desde teclado o raton, todos los conectados (con DS4Windows la
/// «tecla» viene de un mando).
fn feedback_pads(pads: u8) -> u8 {
    if pads != 0 {
        pads
    } else {
        crate::pad::connected_mask()
    }
}

fn in_thread(app: &AppHandle, f: fn(&AppHandle)) {
    let app = app.clone();
    std::thread::spawn(move || f(&app));
}

fn emit_hold(app: &AppHandle, p: f32) {
    if is_open() {
        let _ = app.emit_to(LABEL, "qm-hold", serde_json::json!({ "p": p }));
    }
}

#[cfg(windows)]
fn beep() {
    use windows::Win32::System::Diagnostics::Debug::MessageBeep;
    use windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING;
    unsafe {
        let _ = MessageBeep(MB_ICONWARNING);
    }
}

#[cfg(not(windows))]
fn beep() {}

/// Ninguna ventana se ve encima de un juego en pantalla completa exclusiva de verdad. La ventana de escalado de Magpie
/// (y algunos juegos sin bordes) hace que Windows diga «D3D a pantalla completa» sin serlo: con Magpie escalando no
/// cuenta como exclusiva.
fn exclusive_fullscreen() -> bool {
    let d3d = d3d_fullscreen();
    let magpie = crate::platform::magpie_scaling();
    let exclusive = d3d && !magpie;
    log(&format!("pantalla completa D3D: {d3d}; Magpie escalando: {magpie}; exclusiva: {exclusive}"));
    exclusive
}

#[cfg(windows)]
fn d3d_fullscreen() -> bool {
    use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
    unsafe { SHQueryUserNotificationState().map(|s| s == QUNS_RUNNING_D3D_FULL_SCREEN).unwrap_or(false) }
}

#[cfg(not(windows))]
fn d3d_fullscreen() -> bool {
    false
}

/* ---------------- ventana ---------------- */

/// Crea la ventana oculta al empezar la partida, para que abrir sea inmediato.
pub fn prewarm(app: &AppHandle) {
    clear_forced();
    ENDING.store(false, Ordering::SeqCst);
    SESSION.fetch_add(1, Ordering::SeqCst);
    log_reset(app);
    log("partida empezada");
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        if a.get_webview_window(LABEL).is_none() {
            build(&a);
        }
    });
}

/// Al acabar la partida: cierra el menu y libera la ventana (WebView2 ocupa memoria).
/// En un hilo propio: nada del menu puede impedir que el vigilante guarde la sesion.
pub fn destroy(app: &AppHandle) {
    // Sincrono y antes de lanzar nada: ningun atajo que llegue de aqui en adelante debe abrir el menu sobre una
    // partida que ya se esta cerrando (era la fuente de la carrera con un `open()` que llegaba tarde).
    ENDING.store(true, Ordering::SeqCst);
    let spawned = std::thread::Builder::new().name("qm-destroy".into()).spawn({
        let app = app.clone();
        move || destroy_now(&app)
    });
    if let Err(e) = spawned {
        log(&format!("no se pudo lanzar el cierre del menu: {e}"));
    }
}

fn destroy_now(app: &AppHandle) {
    let session = SESSION.load(Ordering::SeqCst);
    // `close_now` deshace la pausa (`resume_all`).
    close_now(app);
    *lock(&CONTEXT) = None;
    lock(&SHOTS).clear();
    // Un `open()` que llegase tarde (p. ej. el que lanza un cierre forzado) puede seguir mostrando/ocultando la
    // ventana en el hilo principal un rato despues de este punto: esperar antes de destruirla evita destruirla a
    // medio mostrar, que es lo que podia colgar el hilo principal o tirar la app entera.
    std::thread::sleep(Duration::from_millis(CLOSE_ANIM_MS + 1500));
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        if is_open() || !ENDING.load(Ordering::SeqCst) || SESSION.load(Ordering::SeqCst) != session {
            // Una partida nueva ya reutilizo la ventana: no es la nuestra que destruir.
            return;
        }
        if let Some(w) = a.get_webview_window(LABEL) {
            let _ = w.destroy();
            log("ventana destruida");
        }
    });
}

fn build(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("quickmenu.html".into()))
        .title("Menú del juego")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|e| eprintln!("No se pudo crear el menú del juego: {e}"))
        .ok()?;
    crate::overlay::no_activate_clickable(&w);
    Some(w)
}

/// Muestra la ventana del menu, sin tomar el foco (el gancho de teclado y `pad::menu_nav` no lo necesitan).
/// Devuelve `true` solo si el hilo principal de verdad llego a mostrarla dentro de los 800 ms de margen: si el
/// hilo principal no responde (p. ej. acoplado a un hilo ajeno colgado) antes se devolvia `()` igualmente y el
/// resto del codigo seguia como si se hubiera mostrado.
fn show_window(app: &AppHandle, pid: Option<u32>, gen: u64) -> bool {
    let a = app.clone();
    let (tx, rx) = std::sync::mpsc::sync_channel::<bool>(1);
    let t0 = Instant::now();
    log("pido mostrar");
    let _ = app.run_on_main_thread(move || {
        if !is_current(gen) {
            let _ = tx.send(false);
            return;
        }
        log("hp: empieza");
        let shown = if let Some(win) = a.get_webview_window(LABEL).or_else(|| build(&a)) {
            let m = crate::reader::monitor::game_monitor(pid);
            let _ = win.set_position(tauri::PhysicalPosition::new(m.x, m.y));
            let _ = win.set_size(tauri::PhysicalSize::new(m.w, m.h));
            let _ = win.set_position(tauri::PhysicalPosition::new(m.x, m.y));
            let _ = win.show();
            log("hp: show hecho");
            if let Ok(h) = win.hwnd() {
                *lock(&OWN_HWND) = Some(h.0 as isize);
            }
            *lock(&LAST_RECT) = Some((m.x, m.y, m.w, m.h));
            let (fg_ok, fg_pid) = crate::overlay::raise_menu(&win, m.x, m.y, m.w, m.h);
            let fg_exe = fg_pid.and_then(crate::snap::exe_of).unwrap_or_default();
            log(&format!("hp: encima hecho (SetForegroundWindow={fg_ok}, primer plano={fg_pid:?} {fg_exe})"));
            log(&format!("ventana mostrada en ({}, {}) {}x{}", m.x, m.y, m.w, m.h));
            true
        } else {
            log("no se pudo obtener ni crear la ventana del menu");
            false
        };
        let _ = tx.send(shown);
    });
    match rx.recv_timeout(Duration::from_millis(800)) {
        Ok(shown) => shown,
        Err(_) => {
            log(&format!("el hilo principal no respondio en {} ms", t0.elapsed().as_millis()));
            false
        }
    }
}

/// Al hacer clic Windows podria subir el juego por encima (sobre todo si el mismo va «siempre encima»). No
/// encola otro `to_top` si el anterior aun no ha corrido en el hilo principal (no se amontonan).
fn keep_on_top(app: &AppHandle, gen: u64) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("qm-top".into()).spawn(move || {
        while is_current(gen) {
            std::thread::sleep(Duration::from_millis(250));
            if !is_current(gen) || TOP_PENDING.swap(true, Ordering::SeqCst) {
                continue;
            }
            let a = app.clone();
            let _ = app.run_on_main_thread(move || {
                TOP_PENDING.store(false, Ordering::SeqCst);
                if is_current(gen) {
                    if let Some(w) = a.get_webview_window(LABEL) {
                        crate::overlay::to_top(&w);
                    }
                }
            });
        }
    });
}

fn open(app: &AppHandle) {
    if ENDING.load(Ordering::SeqCst) || force_requested() || is_open() {
        return;
    }
    let Some(pids) = active_pids(app) else { return };
    let (pause, scaled) = {
        let st = app.state::<AppState>();
        let c = st.config.lock().unwrap();
        (c.overlay_pause, c.capture_magpie != "source")
    };
    let t0 = Instant::now();
    // La foto puede tardar varios segundos si falla WGC: fuera de `LIFE`, para no bloquear un cierre mientras tanto.
    let grabbed = crate::platform::grab(&pids, scaled);

    let _life = lock(&LIFE);
    if ENDING.load(Ordering::SeqCst) || force_requested() || is_open() || active_pids(app).is_none() {
        log("apertura cancelada: la partida termino mientras se preparaba la captura");
        return;
    }
    *lock(&PREV_FG) = crate::overlay::current_foreground();
    match grabbed {
        Ok(g) => {
            log(&format!("captura de fondo {}x{} ({})", g.width, g.height, g.method));
            let jpeg = visor_core::capture::rgba_to_jpeg(g.width, g.height, &g.rgba, 960, 72).ok();
            *lock(&BACKGROUND) = jpeg.map(|j| format!("data:image/jpeg;base64,{}", visor_core::capture::base64(&j)));
            *lock(&FROZEN) = Some(g);
        }
        Err(e) => {
            log(&format!("sin foto de fondo ({e})"));
            eprintln!("menú del juego: sin foto de fondo ({e})");
            *lock(&BACKGROUND) = None;
            *lock(&FROZEN) = None;
        }
    }

    let magpie = crate::platform::magpie_scaling();
    let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
    IS_OPEN.store(true, Ordering::SeqCst);

    // Si Magpie NO escala, pausamos antes de mostrar: un emulador a pantalla completa debe quedar ya congelado al
    // perder el foco. Si Magpie SI escala, la pausa se decide mas abajo, solo si se confirma que el menu se ve.
    if pause && !magpie {
        suspend(app, &pids);
    }

    let _ = app.emit_to(LABEL, "qm-open", state(app));
    let shown = show_window(app, pids.first().copied(), gen);
    let (ok, painted, on_top, why) = confirm(app, gen, &pids, shown);
    if !ok {
        log(&format!(
            "no confirmado tras {} ms (mostrada: {shown}, pintada: {painted}, encima: {on_top} — {why})",
            t0.elapsed().as_millis()
        ));
        teardown(app);
        let _ = app.emit_to(LABEL, "qm-close", ());
        hide_window(app);
        let fb = feedback_pads(0);
        if fb == 0 {
            beep();
        } else {
            crate::pad::rumble_n(fb, 2);
        }
        return;
    }
    log(&format!("menu visible tras {} ms", t0.elapsed().as_millis()));

    if pause && magpie {
        // Con el primer plano propio, dar un momento a que Magpie termine su `IsWindowHung(WM_NULL)` con el juego
        // aun vivo antes de suspenderlo: solo informativo ahora, ya no decide si se pausa o no.
        if crate::platform::own_window_focused() {
            let t1 = Instant::now();
            while crate::platform::magpie_topmost() && t1.elapsed() < Duration::from_millis(300) {
                std::thread::sleep(Duration::from_millis(5));
            }
            log(&format!(
                "Magpie topmost={} tras confirmar ({} ms)",
                crate::platform::magpie_topmost(),
                t1.elapsed().as_millis()
            ));
        }
        if ENDING.load(Ordering::SeqCst) || force_requested() || !is_current(gen) {
            teardown(app);
            let _ = app.emit_to(LABEL, "qm-close", ());
            hide_window(app);
            return;
        }
        suspend(app, &pids);
        let _ = app.emit_to(LABEL, "qm-paused", serde_json::json!({ "paused": true }));
    }

    keep_on_top(app, gen);
    crate::pad::menu_nav(app.clone(), gen);
    keyboard::start(app, gen);
}

/// Confirma que el menu de verdad se ve antes de fiarse de el: la pagina avisa de que pinto esta apertura
/// (`PAINTED`) y la ventana esta por encima de Magpie y del juego en el orden Z. Sin esto, mejor avisar con una
/// vibracion que dejar el juego pausado detras de una ventana que nadie ve (el fallo real reportado). Un solo
/// reintento (volver a subir la ventana) antes de rendirse.
fn confirm(app: &AppHandle, gen: u64, pids: &[u32], shown: bool) -> (bool, bool, bool, String) {
    if !shown {
        return (false, false, false, "ventana no mostrada".into());
    }
    let poll = |deadline: Duration| -> (bool, bool, String) {
        let t0 = Instant::now();
        loop {
            if ENDING.load(Ordering::SeqCst) || force_requested() || !is_current(gen) {
                return (false, false, "la partida termino mientras se confirmaba".into());
            }
            let painted = PAINTED.load(Ordering::SeqCst) >= gen;
            let own = *lock(&OWN_HWND);
            let (on_top, why) = match own {
                Some(h) => crate::platform::menu_on_top(h, pids),
                None => (false, "sin ventana propia".into()),
            };
            if painted && on_top {
                return (true, true, why);
            }
            if t0.elapsed() >= deadline {
                return (painted, on_top, why);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    let (painted, on_top, why) = poll(Duration::from_millis(1000));
    if painted && on_top {
        return (true, painted, on_top, why);
    }
    if ENDING.load(Ordering::SeqCst) || force_requested() || !is_current(gen) {
        return (false, painted, on_top, why);
    }
    log(&format!("confirmar: reintento (pintada: {painted}, encima: {on_top} — {why})"));
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        if is_current(gen) {
            if let (Some(w), Some(r)) = (a.get_webview_window(LABEL), *lock(&LAST_RECT)) {
                let _ = crate::overlay::raise_menu(&w, r.0, r.1, r.2, r.3);
            }
        }
    });
    let (painted2, on_top2, why2) = poll(Duration::from_millis(300));
    (painted2 && on_top2, painted2, on_top2, why2)
}

/// Cierra con la animacion de salida de la pagina.
fn close(app: &AppHandle) {
    log("cerrar");
    let _life = lock(&LIFE);
    if !teardown(app) {
        return;
    }
    let _ = app.emit_to(LABEL, "qm-close", ());
    let gen = GEN.load(Ordering::SeqCst);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(CLOSE_ANIM_MS));
        if !is_open() && GEN.load(Ordering::SeqCst) == gen {
            hide_window(&app);
        }
    });
}

/// Cierra sin animacion (el juego ya no esta).
fn close_now(app: &AppHandle) {
    log("cerrar ya");
    let _life = lock(&LIFE);
    if teardown(app) {
        let _ = app.emit_to(LABEL, "qm-close", ());
        hide_window(app);
    }
}

/// Deshace lo de `open`, en orden inverso. Devuelve `false` si el menu no estaba abierto.
fn teardown(app: &AppHandle) -> bool {
    if !IS_OPEN.swap(false, Ordering::SeqCst) {
        return false;
    }
    keyboard::stop();
    resume_all(app);
    *lock(&FROZEN) = None;
    *lock(&BACKGROUND) = None;
    true
}

fn hide_window(app: &AppHandle) {
    let prev = lock(&PREV_FG).take();
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = a.get_webview_window(LABEL) {
            let _ = w.hide();
            crate::overlay::hide_quiet(&w);
            log("ventana oculta");
        }
        if let Some(fg) = prev {
            crate::overlay::restore_foreground(fg);
        }
    });
}

/* ---------------- pausa y cierre del juego ---------------- */

#[derive(Serialize, Deserialize)]
struct SuspendedEntry {
    pid: u32,
    exe: String,
}

fn suspended_file(app: &AppHandle) -> std::path::PathBuf {
    app.state::<AppState>().config_dir.join("quickmenu-suspended.json")
}

fn suspend(app: &AppHandle, pids: &[u32]) {
    if force_requested() {
        return;
    }
    // Primero el fichero: si el visor muere a medias, el juego no se queda congelado.
    let entries: Vec<SuspendedEntry> = pids.iter().filter_map(|&pid| crate::snap::exe_of(pid).map(|exe| SuspendedEntry { pid, exe })).collect();
    if let Ok(b) = serde_json::to_vec(&entries) {
        let _ = std::fs::write(suspended_file(app), b);
    }
    let mut done = lock(&SUSPENDED);
    for &pid in pids {
        if proc::suspend(pid) {
            done.push(pid);
        }
    }
}

fn resume_all(app: &AppHandle) {
    let pids = std::mem::take(&mut *lock(&SUSPENDED));
    for pid in pids {
        proc::resume(pid);
    }
    let _ = std::fs::remove_file(suspended_file(app));
}

/// Al arrancar: si el visor se cerro con el juego en pausa, lo reanuda (si sigue siendo el mismo programa).
pub fn recover(app: &AppHandle) {
    let file = suspended_file(app);
    let Ok(b) = std::fs::read(&file) else { return };
    let _ = std::fs::remove_file(&file);
    for e in serde_json::from_slice::<Vec<SuspendedEntry>>(&b).unwrap_or_default() {
        if crate::snap::exe_of(e.pid).as_deref() == Some(e.exe.as_str()) {
            proc::resume(e.pid);
        }
    }
}

/// Cierre forzado e inmediato. El vigilante de la partida ve que no quedan procesos y guarda la sesion.
pub fn force_close_game(app: &AppHandle) {
    FORCED.store(true, Ordering::SeqCst);
    let mut pids = active_pids(app).unwrap_or_default();
    {
        // `TerminateProcess` funciona igual con procesos suspendidos: no hace falta reanudarlos antes.
        let mut s = lock(&SUSPENDED);
        for p in s.drain(..) {
            if !pids.contains(&p) {
                pids.push(p);
            }
        }
    }
    log(&format!("cierre forzado pedido; pids: {pids:?}"));
    for pid in pids {
        let exe = crate::snap::exe_of(pid).unwrap_or_default();
        let ok = proc::terminate(pid);
        log(&format!("terminate {pid} ({exe}): {}", if ok { "ok" } else { "fallo" }));
    }
    let _ = std::fs::remove_file(suspended_file(app));
    close(app);
}

#[cfg(windows)]
mod proc {
    use windows::core::{s, w};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_SUSPEND_RESUME, PROCESS_TERMINATE};

    type NtProc = unsafe extern "system" fn(HANDLE) -> i32;

    /// `NtSuspendProcess`/`NtResumeProcess` no estan en ninguna cabecera publica: se buscan en ntdll.
    fn nt(name: windows::core::PCSTR) -> Option<NtProc> {
        unsafe {
            let lib = GetModuleHandleW(w!("ntdll.dll")).ok()?;
            let f = GetProcAddress(lib, name)?;
            Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, NtProc>(f))
        }
    }

    fn call(pid: u32, f: Option<NtProc>) -> bool {
        let Some(f) = f else { return false };
        unsafe {
            let Ok(h) = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) else { return false };
            let ok = f(h) >= 0;
            let _ = CloseHandle(h);
            ok
        }
    }

    pub fn suspend(pid: u32) -> bool {
        call(pid, nt(s!("NtSuspendProcess")))
    }

    pub fn resume(pid: u32) {
        call(pid, nt(s!("NtResumeProcess")));
    }

    pub fn terminate(pid: u32) -> bool {
        unsafe {
            let Ok(h) = OpenProcess(PROCESS_TERMINATE, false, pid) else { return false };
            let ok = TerminateProcess(h, 1).is_ok();
            let _ = CloseHandle(h);
            ok
        }
    }
}

#[cfg(not(windows))]
mod proc {
    pub fn suspend(_pid: u32) -> bool {
        false
    }
    pub fn resume(_pid: u32) {}
    pub fn terminate(_pid: u32) -> bool {
        false
    }
}

/* ---------------- teclado ---------------- */

/// Sin foco no llegan teclas a la pagina: mientras el menu esta abierto un gancho de bajo nivel se queda las del menu
/// (y no llegan al juego); el resto pasan.
#[cfg(windows)]
mod keyboard {
    use super::LABEL;
    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::{Mutex, OnceLock};
    use tauri::{AppHandle, Emitter};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ADD, VK_BACK, VK_CONTROL, VK_DOWN, VK_E, VK_ESCAPE, VK_LEFT, VK_LWIN, VK_Q, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SUBTRACT, VK_TAB, VK_UP, VK_X, VK_Y};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, PeekMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, MSG, PM_NOREMOVE, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
    };

    static TX: OnceLock<Mutex<Sender<&'static str>>> = OnceLock::new();
    /// Hilo del gancho (0 = no hay).
    static THREAD: AtomicU32 = AtomicU32::new(0);
    /// Teclas de un solo golpe que siguen pulsadas (Windows repite la pulsacion mientras se mantiene).
    static HELD: AtomicU32 = AtomicU32::new(0);
    /// `gen` para el que se pidio el gancho: si cambia antes de instalarlo, se quita enseguida sin bucle de mensajes.
    static WANT: AtomicU64 = AtomicU64::new(0);

    /// (boton del menu, se repite al mantener, bit en `HELD`)
    fn key(vk: u16) -> Option<(&'static str, bool, u32)> {
        Some(match vk {
            v if v == VK_UP.0 => ("up", true, 0),
            v if v == VK_DOWN.0 => ("down", true, 0),
            v if v == VK_LEFT.0 => ("left", true, 0),
            v if v == VK_RIGHT.0 => ("right", true, 0),
            v if v == VK_Q.0 || v == VK_SUBTRACT.0 => ("lt", true, 0),
            v if v == VK_E.0 || v == VK_ADD.0 => ("rt", true, 0),
            v if v == VK_RETURN.0 => ("a", false, 1),
            v if v == VK_ESCAPE.0 => ("b", false, 2),
            v if v == VK_BACK.0 => ("b", false, 4),
            v if v == VK_X.0 => ("x", false, 8),
            v if v == VK_Y.0 => ("y", false, 16),
            v if v == VK_TAB.0 => ("rb", false, 32),
            _ => return None,
        })
    }

    unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && super::is_open() {
            let k = &*(lp.0 as *const KBDLLHOOKSTRUCT);
            // Alt+Tab, Ctrl+algo y Win+algo son del sistema.
            let chord = k.flags.0 & LLKHF_ALTDOWN.0 != 0
                || [VK_CONTROL, VK_LWIN, VK_RWIN].iter().any(|v| GetAsyncKeyState(v.0 as i32) < 0);
            if !chord {
                if let Some((btn, repeats, bit)) = key(k.vkCode as u16) {
                    let down = matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
                    let first = down && HELD.fetch_or(bit, Ordering::Relaxed) & bit == 0;
                    if !down {
                        HELD.fetch_and(!bit, Ordering::Relaxed);
                    }
                    if down && (repeats || first) {
                        if let Some(tx) = TX.get() {
                            if let Ok(tx) = tx.try_lock() {
                                let _ = tx.send(btn);
                            }
                        }
                    }
                    return LRESULT(1);
                }
            }
        }
        CallNextHookEx(None, code, wp, lp)
    }

    pub fn start(app: &AppHandle, gen: u64) {
        // Por si quedaba uno de una apertura anterior (no deberia, pero sin fugas).
        stop();
        WANT.store(gen, Ordering::SeqCst);
        // El gancho debe volver enseguida (si tarda, Windows lo quita): solo avisa a otro hilo.
        if TX.get().is_none() {
            let (tx, rx) = channel::<&'static str>();
            let _ = TX.set(Mutex::new(tx));
            let app = app.clone();
            std::thread::spawn(move || {
                for btn in rx {
                    let _ = app.emit_to(LABEL, "qm-pad", serde_json::json!({ "btn": btn }));
                }
            });
        }
        HELD.store(0, Ordering::Relaxed);
        let spawned = std::thread::Builder::new().name("qm-keyboard".into()).spawn(move || unsafe {
            // Crea la cola de mensajes del hilo antes de instalar el gancho: si no, un `stop()` que llegue
            // enseguida puede fallar al enviarle `WM_QUIT` (la fuga que se tragaba flechas/Enter/Esc en todo
            // el sistema).
            let mut peek = MSG::default();
            let _ = PeekMessageW(&mut peek, None, 0, 0, PM_NOREMOVE);
            let module = GetModuleHandleW(None).ok();
            let h = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), module.map(|m| m.into()), 0) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("no se pudo instalar el gancho del teclado: {e}");
                    return;
                }
            };
            let tid = GetCurrentThreadId();
            THREAD.store(tid, Ordering::SeqCst);
            super::log(&format!("gancho teclado puesto ({tid})"));
            if WANT.load(Ordering::SeqCst) != gen {
                let _ = UnhookWindowsHookEx(h);
                let _ = THREAD.compare_exchange(tid, 0, Ordering::SeqCst, Ordering::SeqCst);
                super::log(&format!("gancho teclado quitado enseguida ({tid}): ya no hacia falta"));
                return;
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = UnhookWindowsHookEx(h);
            super::log(&format!("gancho teclado quitado ({tid})"));
        });
        if let Err(e) = spawned {
            eprintln!("no se pudo arrancar el hilo del teclado: {e}");
        }
    }

    pub fn stop() {
        WANT.store(0, Ordering::SeqCst);
        let t = THREAD.swap(0, Ordering::SeqCst);
        if t != 0 {
            unsafe {
                let _ = PostThreadMessageW(t, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        } else {
            super::log("stop sin gancho que quitar");
        }
    }
}

#[cfg(not(windows))]
mod keyboard {
    use tauri::AppHandle;
    pub fn start(_app: &AppHandle, _gen: u64) {}
    pub fn stop() {}
}

/* ---------------- estado para la pagina ---------------- */

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QmShot {
    pub path: String,
    /// Miniatura (`data:image/jpeg`).
    pub thumb: String,
    /// Momento de la captura (ms desde epoch).
    pub at: i64,
}

/// Apunta una captura de esta partida y, con el menu abierto, se la manda a la pagina.
pub fn push_shot(app: &AppHandle, path: String, jpeg: &[u8], at: i64) {
    let shot = QmShot { path, thumb: format!("data:image/jpeg;base64,{}", visor_core::capture::base64(jpeg)), at };
    {
        let mut s = lock(&SHOTS);
        s.insert(0, shot.clone());
        s.truncate(MAX_SHOTS);
    }
    if is_open() {
        let _ = app.emit_to(LABEL, "qm-shot", shot);
    }
}

fn state(app: &AppHandle) -> Value {
    let st = app.state::<AppState>();
    let active = st.active.lock().unwrap().clone();
    let c = st.config.lock().unwrap().clone();
    let game = active.map(|a| {
        let seconds = st.stats.lock().unwrap().get(&a.id).map(|s| s.seconds).unwrap_or(0);
        let platform_name = c.platforms.iter().find(|p| p.id == a.platform).map(|p| p.name.clone()).unwrap_or_default();
        serde_json::json!({ "id": a.id, "name": a.name, "platform": a.platform, "platformName": platform_name, "since": a.since, "playSeconds": seconds })
    });
    let paused = !lock(&SUSPENDED).is_empty();
    serde_json::json!({
        "open": is_open(),
        "gen": GEN.load(Ordering::SeqCst),
        "game": game,
        "paused": paused,
        "background": lock(&BACKGROUND).clone(),
        "context": lock(&CONTEXT).clone(),
        "goals": { "vn": c.goal_vn, "jvn": c.goal_jvn },
        "holdMs": c.overlay_hold_ms,
        "shots": lock(&SHOTS).clone(),
        "audio": crate::mixer::state(active_pids(app).unwrap_or_default().as_slice()).ok(),
        "perf": crate::perf::snapshot(),
        "pads": crate::pad::connected_mask(),
    })
}

/* ---------------- comandos ---------------- */

/// La pagina acaba de cargar o quiere refrescar: todo lo que enseña de una vez.
#[tauri::command]
pub async fn quickmenu_state(app: AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || state(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn quickmenu_close(app: AppHandle) {
    in_thread(&app, close);
}

/// La pagina avisa de que ha pintado esta apertura (dos `requestAnimationFrame` tras recibir `qm-open`): sin esto
/// `open()` no se fia de que el menu se vea y no llega a pausar el juego.
#[tauri::command]
pub fn quickmenu_painted(gen: u64) {
    PAINTED.store(gen, Ordering::SeqCst);
}

/// «Cerrar juego»: cierre forzado inmediato.
#[tauri::command]
pub fn quickmenu_close_game(app: AppHandle) {
    in_thread(&app, force_close_game);
}

/// Datos para la baldosa de tiempo que solo tiene la ventana principal (ver `pushQuickmenuContext`).
#[tauri::command]
pub fn quickmenu_set_context(ctx: Value) {
    *lock(&CONTEXT) = Some(ctx);
}

/// Captura desde el menu. Con el juego en pausa se guarda la foto del momento de abrir: es lo que hay detras y no depende
/// de capturar un proceso suspendido.
#[tauri::command]
pub fn quickmenu_capture(app: AppHandle) {
    let frozen = if lock(&SUSPENDED).is_empty() {
        None
    } else {
        lock(&FROZEN).as_ref().map(|g| crate::platform::Grabbed { width: g.width, height: g.height, rgba: g.rgba.clone(), method: g.method.clone(), fg_pid: g.fg_pid })
    };
    match frozen {
        Some(g) => crate::snap::save_frozen(&app, g),
        None => crate::snap::trigger(&app),
    }
}

/// Manda capturas a la Papelera (el mismo camino que el visor) y avisa a la ventana principal para que las quite.
#[tauri::command]
pub async fn quickmenu_trash(app: AppHandle, state: tauri::State<'_, AppState>, paths: Vec<String>) -> Result<crate::TrashReport, String> {
    let rep = crate::trash_shots(state, paths).await?;
    lock(&SHOTS).retain(|s| !rep.trashed.contains(&s.path));
    let _ = app.emit("shots-trashed", &rep.trashed);
    Ok(rep)
}

/// «Vibrar para reconocerlo».
#[tauri::command]
pub fn quickmenu_rumble(port: u8) {
    if port < 4 {
        crate::pad::rumble_n(1 << port, 2);
    }
}

async fn audio_op(app: AppHandle, f: impl FnOnce() -> Result<(), String> + Send + 'static) -> Result<crate::mixer::AudioState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        f()?;
        crate::mixer::state(&active_pids(&app).unwrap_or_default())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn quickmenu_audio(app: AppHandle) -> Result<crate::mixer::AudioState, String> {
    audio_op(app, || Ok(())).await
}

#[tauri::command]
pub async fn quickmenu_set_volume(app: AppHandle, volume: f32) -> Result<crate::mixer::AudioState, String> {
    audio_op(app, move || crate::mixer::set_volume(volume)).await
}

#[tauri::command]
pub async fn quickmenu_set_mute(app: AppHandle, mute: bool) -> Result<crate::mixer::AudioState, String> {
    audio_op(app, move || crate::mixer::set_mute(mute)).await
}

#[tauri::command]
pub async fn quickmenu_set_mic_mute(app: AppHandle, mute: bool) -> Result<crate::mixer::AudioState, String> {
    audio_op(app, move || crate::mixer::set_mic_mute(mute)).await
}

#[tauri::command]
pub async fn quickmenu_select_output(app: AppHandle, id: String) -> Result<crate::mixer::AudioState, String> {
    let a = app.clone();
    audio_op(app, move || crate::outputs::select(&a, &id).map(|_| ())).await
}

#[tauri::command]
pub async fn quickmenu_select_input(app: AppHandle, id: String) -> Result<crate::mixer::AudioState, String> {
    audio_op(app, move || crate::outputs::set_default(&id)).await
}

#[tauri::command]
pub async fn quickmenu_set_app_volume(app: AppHandle, pid: u32, volume: f32) -> Result<crate::mixer::AudioState, String> {
    audio_op(app, move || crate::mixer::set_app_volume(pid, volume)).await
}

#[tauri::command]
pub async fn quickmenu_set_app_mute(app: AppHandle, pid: u32, mute: bool) -> Result<crate::mixer::AudioState, String> {
    audio_op(app, move || crate::mixer::set_app_mute(pid, mute)).await
}
