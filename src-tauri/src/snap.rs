//! Orquesta una captura propia: elige el juego, captura, guarda el PNG en la biblioteca y avisa.
use crate::platform;
use crate::state::AppState;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use visor_core::capture::{self, UNCLASSIFIED_FOLDER, UNCLASSIFIED_PLATFORM};
use visor_core::games::{self, resolve_folder};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub path: String,
    pub name: String,
    pub platform_id: String,
    pub folder: String,
}

static BUSY: AtomicBool = AtomicBool::new(false);

/// Libera el indicador de «captura en curso» pase lo que pase.
struct Busy;
impl Drop for Busy {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::SeqCst);
    }
}

/// Lanza una captura en un hilo aparte (el atajo, el mando y el raton no deben bloquearse). Ignora
/// pulsaciones mientras hay otra captura en curso.
pub fn trigger(app: &AppHandle) {
    trigger_from_pad(app, 0);
}

/// Como [`trigger`], pero desde el mando: `pads` son los puertos XInput que hicieron la combinacion (vibran al capturar).
pub fn trigger_from_pad(app: &AppHandle, pads: u8) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let r = {
            let _busy = Busy;
            run(&app, pads)
        };
        match r {
            Ok(s) => {
                let _ = app.emit("capture-saved", s);
            }
            Err(e) => {
                eprintln!("captura fallida: {e}");
                let _ = app.notification().builder().title("Visor de capturas").body(format!("No se pudo capturar: {e}")).show();
                let _ = app.emit("capture-error", e);
            }
        }
    });
}

fn exe_of(pid: u32) -> Option<String> {
    let mut sys = System::new();
    let p = Pid::from_u32(pid);
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[p]), true, ProcessRefreshKind::nothing().with_exe(UpdateKind::Always));
    sys.process(p).and_then(|p| p.exe()).map(|e| e.to_string_lossy().into_owned())
}

/// Carpeta de emergencia (Imagenes\Visor de capturas) para no perder la captura si la biblioteca no esta
/// configurada o no se puede escribir en ella (disco desconectado, sin permisos...).
fn fallback_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().picture_dir().ok().or_else(|| app.path().home_dir().ok()).map(|d| d.join("Visor de capturas"))
}

fn run(app: &AppHandle, pads: u8) -> Result<Saved, String> {
    let at = chrono::Local::now().naive_local();
    let st = app.state::<AppState>();
    let cfg = st.config.lock().unwrap().clone();
    let active = st.active.lock().unwrap().clone();
    let pids = active.as_ref().map(|a| a.pids.lock().unwrap().clone()).unwrap_or_default();

    let g = platform::grab(&pids, cfg.capture_magpie != "source")?;
    eprintln!("captura {}x{} por {}", g.width, g.height, g.method);
    // Los avisos salen en cuanto se tiene la imagen; guardar el PNG puede tardar un poco mas.
    if cfg.capture_sound {
        platform::play_shutter();
    }
    if cfg.capture_rumble {
        crate::pad::rumble(pads);
    }
    if cfg.capture_peek {
        match capture::rgba_to_jpeg(g.width, g.height, &g.rgba, crate::peek::THUMB_W, 80) {
            Ok(jpeg) => crate::peek::show(app, &jpeg, g.width, g.height, g.fg_pid),
            Err(e) => eprintln!("miniatura de captura: {e}"),
        }
    }

    // A que juego pertenece: el lanzado desde el visor; si no, el del proceso capturado si esta dentro de
    // la carpeta de un juego conocido; si no, «Sin clasificar».
    let (platform_id, wanted, display) = match active {
        Some(a) => (a.platform, a.folder, a.name),
        None => {
            let games_list = st.games.lock().unwrap();
            let hit = g.fg_pid.and_then(exe_of).and_then(|exe| games::game_for_exe(&games_list, &exe).cloned());
            match hit {
                Some(game) => (game.platform, game.folder, game.name),
                None => (UNCLASSIFIED_PLATFORM.to_string(), UNCLASSIFIED_FOLDER.to_string(), UNCLASSIFIED_FOLDER.to_string()),
            }
        }
    };

    // Destino en la biblioteca: Raiz/<Plataforma>/<Juego>.
    let mut why = String::new();
    let lib_dest = match (&cfg.root, cfg.platforms.iter().find(|p| p.id == platform_id).or_else(|| cfg.platforms.first())) {
        (Some(root), Some(plat)) if Path::new(root).is_dir() => {
            let root_p = Path::new(root);
            let plat_folder = resolve_folder(root_p, &plat.folder);
            let folder = resolve_folder(&root_p.join(&plat_folder), &wanted);
            Some((capture::destination_dir(root_p, &plat_folder, &folder), plat.id.clone(), folder))
        }
        (None, _) => {
            why = "no hay carpeta de biblioteca".into();
            None
        }
        (Some(root), _) => {
            why = format!("no se encuentra {root}");
            None
        }
    };

    let (width, height, rgba) = (g.width, g.height, g.rgba);
    if let Some((dir, plat_id, folder)) = lib_dest {
        let path = capture::unique_path(&dir, &capture::capture_filename(&folder, at));
        // Se guarda una copia por si falla y hay que intentarlo en la carpeta de emergencia.
        match capture::save_rgba_png(&path, width, height, rgba.clone()) {
            Ok(()) => {
                let _ = app.notification().builder().title("Visor de capturas").body(format!("Captura guardada: {display}")).show();
                return Ok(Saved { path: path.to_string_lossy().into_owned(), name: display, platform_id: plat_id, folder });
            }
            Err(e) => why = format!("no se pudo guardar en la biblioteca: {e}"),
        }
    }

    let dir = fallback_dir(app).ok_or_else(|| format!("No se pudo guardar ({why})"))?;
    let path = capture::unique_path(&dir, &capture::capture_filename(&wanted, at));
    capture::save_rgba_png(&path, width, height, rgba).map_err(|e| format!("No se pudo guardar ({why}; {e})"))?;
    let shown = path.to_string_lossy().into_owned();
    let _ = app
        .notification()
        .builder()
        .title("Visor de capturas")
        .body(format!("Captura guardada en {} ({why})", dir.display()))
        .show();
    Ok(Saved { path: shown, name: format!("{display} (en {})", dir.display()), platform_id: platform_id.clone(), folder: wanted })
}
