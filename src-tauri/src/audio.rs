//! Grabacion MANUAL del audio del juego: una pulsacion (atajo global, boton de la pagina o de la ventana de Anki)
//! empieza, otra para. Sin bufer continuo. Se captura solo el proceso del juego con WASAPI process loopback
//! (Windows 10 2004+; si no se puede, el loopback del dispositivo de salida por defecto) en estereo, se quitan las
//! partes de solo musica (la voz va en el centro y la musica es estereo; ver `visor_core::audio::trim_voice_stereo`) y
//! se guarda un WAV mono de 16 bits en la carpeta de medios del glosario.
//! Tambien genera el audio de respaldo con la voz sintetica de Windows (`voice`).
use crate::glossary;
use crate::state::AppState;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use visor_core::audio as pcm;

const RATE: u32 = 44_100;
/// Tope de una grabacion (por si se olvida pararla).
const MAX_SECS: u64 = 60;

struct Rec {
    stop: Arc<AtomicBool>,
    join: JoinHandle<Result<Vec<i16>, String>>,
    line_id: u64,
}

static REC: Mutex<Option<Rec>> = Mutex::new(None);
/// Linea elegida en la pagina (0 = la ultima): a ella va la grabacion del atajo global.
static SELECTED: AtomicU64 = AtomicU64::new(0);

/// Frases (ids del glosario) grabadas en esta partida. El audio guardado de partidas anteriores sigue en el glosario,
/// pero la pagina de lectura y la ventana de Anki solo ofrecen el de esta partida: asi una frase que ya salio otro dia
/// no aparece con «Volver a grabar» y su grabacion vieja puesta.
static RECORDED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

pub fn recorded_this_session(phrase_id: &str) -> bool {
    RECORDED.lock().unwrap().contains(phrase_id)
}

pub fn set_selected(id: u64) {
    SELECTED.store(id, Ordering::Relaxed);
}

fn rec_msg(state: &str, secs: u64, line_id: u64) {
    crate::reader::server::send(json!({ "type": "rec", "state": state, "secs": secs, "lineId": line_id }));
}

fn toast(msg: &str) {
    crate::reader::server::send(json!({ "type": "toast", "msg": msg, "bad": true }));
}

fn game_pid(app: &AppHandle) -> Option<u32> {
    let st = app.state::<AppState>();
    let from_active = st.active.lock().unwrap().as_ref().and_then(|a| a.pids.lock().unwrap().first().copied());
    from_active.or_else(|| crate::texthook::status().proc.map(|p| p.pid))
}

/// Empieza o termina la grabacion. `line_id` = frase a la que va el audio (None = la elegida en la pagina o la ultima).
pub fn toggle(app: &AppHandle, line_id: Option<u64>) {
    let running = REC.lock().unwrap().take();
    match running {
        Some(r) => stop(app, r),
        None => start(app, line_id),
    }
}

/// Fin de la partida: se descarta la grabacion en curso (sin guardarla) y se olvida la frase elegida, que era de esta
/// partida (los ids de linea vuelven a empezar en la siguiente).
pub fn cancel(app: &AppHandle) {
    SELECTED.store(0, Ordering::Relaxed);
    RECORDED.lock().unwrap().clear();
    if let Some(r) = REC.lock().unwrap().take() {
        r.stop.store(true, Ordering::SeqCst);
        rec_msg("idle", 0, r.line_id);
        crate::overlay::hide(app);
    }
}

/// ¿Hay una grabacion en curso?
pub fn is_recording() -> bool {
    REC.lock().unwrap().is_some()
}

fn start(app: &AppHandle, line_id: Option<u64>) {
    let Some(pid) = game_pid(app) else {
        return toast("No hay ningún juego en marcha para grabar su audio.");
    };
    let line = line_id
        .filter(|&i| i != 0 && crate::reader::server::line(i).is_some())
        .or_else(|| Some(SELECTED.load(Ordering::Relaxed)).filter(|&i| i != 0 && crate::reader::server::line(i).is_some()))
        .or_else(crate::reader::server::last_line_id);
    let Some(line) = line else {
        return toast("Aún no ha salido ninguna frase: no hay a qué asociar el audio.");
    };
    let stop_flag = Arc::new(AtomicBool::new(false));
    let flag = stop_flag.clone();
    let join = match std::thread::Builder::new().name("rec".into()).spawn(move || capture(pid, flag)) {
        Ok(j) => j,
        Err(e) => return toast(&format!("No se pudo empezar a grabar: {e}")),
    };
    let started = Instant::now();
    *REC.lock().unwrap() = Some(Rec { stop: stop_flag.clone(), join, line_id: line });
    rec_msg("recording", 0, line);
    crate::overlay::show("recording", 0);
    // Reloj: avisa cada segundo a la pagina y al indicador; para solo si se acaba el tiempo.
    let app = app.clone();
    let _ = std::thread::Builder::new().name("rec-tick".into()).spawn(move || {
        let mut last = 0;
        while !stop_flag.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(200));
            let secs = started.elapsed().as_secs();
            if secs != last {
                last = secs;
                if REC.lock().unwrap().is_none() {
                    break;
                }
                rec_msg("recording", secs, line);
                crate::overlay::show("recording", secs);
                if secs >= MAX_SECS {
                    toggle(&app, None);
                    break;
                }
            }
        }
    });
}

fn stop(app: &AppHandle, r: Rec) {
    r.stop.store(true, Ordering::SeqCst);
    let app = app.clone();
    let _ = std::thread::Builder::new().name("rec-save".into()).spawn(move || {
        let line = r.line_id;
        let result = r.join.join().map_err(|_| "La captura de audio falló.".to_string()).and_then(|x| x).and_then(|samples| {
            let (trimmed, report) = pcm::trim_voice_stereo_report(&samples, RATE);
            // La ultima grabacion sin recortar y como se recorto, para poder ajustar el filtro de musica.
            let raw_dir = glossary::dir(&app);
            let _ = std::fs::write(raw_dir.join("ultima-grabacion.wav"), pcm::wav16(&samples, RATE, 2));
            let _ = std::fs::write(raw_dir.join("ultima-grabacion.txt"), &report);
            let trimmed = trimmed.ok_or("No se oyó nada del juego: comprueba que suena y que grabas mientras habla.")?;
            let secs = pcm::secs(trimmed.len(), RATE);
            let wav = pcm::wav_mono16(&trimmed, RATE);
            let dir = glossary::media_dir(&app);
            std::fs::create_dir_all(&dir).map_err(|e| format!("No se pudo crear la carpeta de audio: {e}"))?;
            let mut write_err = None;
            let saved = glossary::update_line(&app, line, |p| {
                let name = format!("audio-{}.wav", p.id);
                match std::fs::write(dir.join(&name), &wav) {
                    Ok(()) => p.audio = Some(name),
                    Err(e) => write_err = Some(e.to_string()),
                }
            });
            if let Some(e) = write_err {
                return Err(format!("No se pudo guardar el audio: {e}"));
            }
            let (p, _) = saved.ok_or("No se pudo guardar la frase en el glosario.")?;
            Ok((p, secs))
        });
        match result {
            Ok((p, secs)) => {
                RECORDED.lock().unwrap().insert(p.id.clone());
                glossary::push_state(&p, line);
                let shown = secs.round().max(1.0) as u64;
                rec_msg("done", shown, line);
                crate::overlay::show("done", shown);
                crate::shots::keep_for_phrase(&app, line);
            }
            Err(e) => {
                rec_msg("idle", 0, line);
                crate::overlay::show("error", 0);
                toast(&e);
            }
        }
        std::thread::sleep(Duration::from_millis(2500));
        if !is_recording() {
            rec_msg("idle", 0, line);
            crate::overlay::hide(&app);
        }
    });
}

/// Genera la voz sintetica de una frase y la guarda en el glosario (para la ventana de Anki).
pub fn synth(app: &AppHandle, line_id: u64) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("synth".into()).spawn(move || {
        let Some(line) = crate::reader::server::line(line_id) else { return toast("Esa frase ya no está en el historial.") };
        match crate::voice::synthesize(&line.text) {
            Ok(wav) => {
                let dir = glossary::media_dir(&app);
                let _ = std::fs::create_dir_all(&dir);
                let mut err = None;
                let saved = glossary::update_line(&app, line_id, |p| {
                    let name = format!("synth-{}.wav", p.id);
                    match std::fs::write(dir.join(&name), &wav) {
                        Ok(()) => p.synth = Some(name),
                        Err(e) => err = Some(e.to_string()),
                    }
                });
                match (err, saved) {
                    (Some(e), _) => toast(&format!("No se pudo guardar la voz sintética: {e}")),
                    (None, Some((p, _))) => glossary::push_state(&p, line_id),
                    _ => toast("No se pudo guardar la frase en el glosario."),
                }
            }
            Err(e) => toast(&e),
        }
    });
}

/* ------------------------------------------------------------------ */
/* Captura WASAPI                                                       */
/* ------------------------------------------------------------------ */

#[cfg(windows)]
fn capture(pid: u32, stop: Arc<AtomicBool>) -> Result<Vec<i16>, String> {
    use std::collections::VecDeque;
    use wasapi::{initialize_mta, AudioClient, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};
    let _ = initialize_mta();
    let fmt = WaveFormat::new(16, 16, &SampleType::Int, RATE as usize, 2, None);
    let mode = StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: 0 };
    // Primero solo el proceso del juego (y sus hijos); si Windows no lo permite, lo que suena en la salida por defecto.
    let process = AudioClient::new_application_loopback_client(pid, true).and_then(|mut c| c.initialize_client(&fmt, &Direction::Capture, &mode).map(|_| c));
    let client = match process {
        Ok(c) => c,
        Err(e) => {
            eprintln!("audio del proceso no disponible ({e}); se usa la salida por defecto");
            let dev = DeviceEnumerator::new().and_then(|d| d.get_default_device(&Direction::Render)).map_err(|e| format!("No hay dispositivo de audio: {e}"))?;
            let mut c = dev.get_iaudioclient().map_err(|e| format!("No se pudo abrir el audio: {e}"))?;
            c.initialize_client(&fmt, &Direction::Capture, &mode).map_err(|e| format!("No se pudo capturar el audio: {e}"))?;
            c
        }
    };
    let h_event = client.set_get_eventhandle().map_err(|e| e.to_string())?;
    let cap = client.get_audiocaptureclient().map_err(|e| e.to_string())?;
    client.start_stream().map_err(|e| e.to_string())?;
    let mut queue: VecDeque<u8> = VecDeque::new();
    let mut all: Vec<i16> = Vec::new();
    let t0 = Instant::now();
    while !stop.load(Ordering::SeqCst) && t0.elapsed() < Duration::from_secs(MAX_SECS + 2) {
        let n = cap.get_next_packet_size().map_err(|e| e.to_string())?.unwrap_or(0);
        if n > 0 {
            let _ = cap.read_from_device_to_deque(&mut queue);
            let bytes: Vec<u8> = queue.drain(..).collect();
            all.extend(pcm::bytes_to_i16(&bytes));
        }
        // Sin sonido el proceso no entrega paquetes: se rellena con silencio para mantener los tiempos.
        if h_event.wait_for_event(100).is_err() {
            let want = (t0.elapsed().as_secs_f64() * RATE as f64) as usize * 2;
            if all.len() < want {
                all.resize(want, 0);
            }
        }
    }
    let _ = client.stop_stream();
    Ok(all) // estereo entrelazado: el recorte usa los dos canales para separar la voz de la musica
}

#[cfg(not(windows))]
fn capture(_pid: u32, _stop: Arc<AtomicBool>) -> Result<Vec<i16>, String> {
    Err("La grabación de audio solo está disponible en Windows".into())
}
