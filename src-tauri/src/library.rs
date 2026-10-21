//! Biblioteca del visor (ver `visor_core::library`): al arrancar se decide donde estan los datos, y el asistente de
//! la interfaz elige la carpeta, copia lo que habia y reinicia. Nunca se borra ni se modifica el origen.
use crate::backup::Notice;
use crate::state::{AppState, LibMode};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use visor_core::db::DB_FILE;
use visor_core::library::{self as core, CopyProgress, DataPaths, DestKind, Plan, Pointer};

/// Que hay al arrancar: modo y carpeta de la biblioteca (vacia si aun no hay).
pub fn resolve(config_dir: &Path) -> (LibMode, PathBuf) {
    match core::read_pointer(config_dir) {
        None => (LibMode::Legacy, PathBuf::new()),
        Some(p) => {
            let dir = PathBuf::from(p.dir.trim());
            // Sin `visor.db` no se crea una base vacia: puede ser un disco desconectado o una carpeta movida.
            let mode = if dir.join(DB_FILE).is_file() { LibMode::Library } else { LibMode::Missing };
            (mode, dir)
        }
    }
}

/// Aviso de arranque tras copiar la biblioteca (una sola vez): lo anterior sigue donde estaba.
pub fn copied_notice(config_dir: &Path) -> Option<Notice> {
    let mut p = core::read_pointer(config_dir)?;
    if p.previous.is_empty() {
        return None;
    }
    let msg = format!(
        "Biblioteca lista en {}. Lo anterior sigue en su sitio ({}): bórralo tú cuando compruebes que está todo.",
        p.dir, p.previous
    );
    p.previous.clear();
    let _ = core::write_pointer(config_dir, &p);
    Some(Notice { kind: "info", message: msg })
}

pub fn missing_notice(dir: &Path) -> Notice {
    Notice {
        kind: "error",
        message: format!(
            "No se encuentra la biblioteca en {}. ¿Está conectado el disco? Esta sesión funciona en memoria y no se guardará nada.",
            dir.display()
        ),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub mode: LibMode,
    /// Carpeta de la biblioteca ("" si aun no hay).
    pub dir: String,
    /// Propuesta para una biblioteca nueva: `Documentos\Visor de capturas`.
    pub suggested: String,
    pub busy: bool,
}

#[tauri::command]
pub fn library_info(app: AppHandle, state: State<'_, AppState>) -> LibraryInfo {
    let docs = app.path().document_dir().ok().or_else(|| app.path().home_dir().ok()).unwrap_or_default();
    LibraryInfo {
        mode: state.lib_mode.clone(),
        dir: state.lib_dir.to_string_lossy().into_owned(),
        suggested: docs.join("Visor de capturas").to_string_lossy().into_owned(),
        busy: state.lib_busy.load(Ordering::SeqCst),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    #[serde(flatten)]
    pub plan: Plan,
    /// Espacio libre en el disco del destino (None = no se pudo saber).
    pub free: Option<u64>,
    /// Hay sitio para copiarlo todo (con margen).
    pub fits: bool,
}

/// Que se copiaria a `dest` y si cabe. En `Missing` no hay nada que copiar: solo se puede elegir una biblioteca.
#[tauri::command]
pub async fn library_plan(state: State<'_, AppState>, dest: String) -> Result<PlanView, String> {
    let from = state.paths.clone();
    let missing = state.lib_mode == LibMode::Missing;
    tauri::async_runtime::spawn_blocking(move || {
        let dest = PathBuf::from(dest.trim());
        let plan = core::plan(&dest, &from).map_err(|e| e.to_string())?;
        if missing && plan.kind != DestKind::Existing {
            return Err("Elige la carpeta de tu biblioteca (la que tiene visor.db)".into());
        }
        let free = free_space(&dest);
        let fits = plan.kind == DestKind::Existing || free.map_or(true, |f| f >= plan.bytes + core::FREE_MARGIN);
        Ok(PlanView { plan, free, fits })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Copia todo a `dest` (emite `library-progress`) y la deja como biblioteca para el proximo arranque. No reinicia:
/// lo hace la interfaz con `library_restart` al enseñar que termino.
#[tauri::command]
pub async fn library_copy(app: AppHandle, state: State<'_, AppState>, dest: String) -> Result<(), String> {
    if state.lib_mode == LibMode::Missing {
        return Err("No se encuentra la biblioteca actual: elige una que ya exista".into());
    }
    if state.lib_busy.swap(true, Ordering::SeqCst) {
        return Err("Ya se está copiando la biblioteca".into());
    }
    let app2 = app.clone();
    let r = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let st = app2.state::<AppState>();
        let dest = PathBuf::from(dest.trim());
        let plan = core::plan(&dest, &st.paths).map_err(|e| e.to_string())?;
        if let Some(f) = free_space(&dest) {
            if f < plan.bytes + core::FREE_MARGIN {
                return Err("No hay sitio suficiente en ese disco".into());
            }
        }
        let cfg = st.config.lock().unwrap().clone();
        let emit = |p: &CopyProgress| {
            let _ = app2.emit("library-progress", p);
        };
        core::copy_into(&st.paths, &dest, &st.db, &cfg, emit).map_err(|e| format!("{e:#}"))?;
        let previous = match st.lib_mode {
            LibMode::Legacy => "capturas, partidas y datos de antes".to_string(),
            _ => st.paths.data.to_string_lossy().into_owned(),
        };
        core::write_pointer(&st.config_dir, &Pointer { dir: dest.to_string_lossy().into_owned(), previous }).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    state.lib_busy.store(false, Ordering::SeqCst);
    r
}

/// Usa como biblioteca una carpeta que ya lo es (otro equipo, reinstalacion, disco que cambio de letra).
#[tauri::command]
pub fn library_use(state: State<'_, AppState>, dest: String) -> Result<(), String> {
    let dest = PathBuf::from(dest.trim());
    if core::check_dest(&dest, &state.paths).map_err(|e| e.to_string())? != DestKind::Existing {
        return Err("Esa carpeta no es una biblioteca del visor".into());
    }
    core::write_pointer(&state.config_dir, &Pointer { dir: dest.to_string_lossy().into_owned(), previous: String::new() }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_restart(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state.lib_busy.load(Ordering::SeqCst) {
        return Err("Espera a que termine la copia".into());
    }
    app.restart()
}

/// Espacio libre para el usuario en el disco de `p` (o de su primera carpeta existente).
fn free_space(p: &Path) -> Option<u64> {
    let mut d = p;
    while !d.exists() {
        d = d.parent()?;
    }
    disk_free(d)
}

#[cfg(windows)]
fn disk_free(dir: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0u64;
    unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free), None, None) }.ok()?;
    Some(free)
}

#[cfg(not(windows))]
fn disk_free(_: &Path) -> Option<u64> {
    None
}

/// Rutas de datos al arrancar, antes de leer la configuracion (la de antes depende de ella: ver `DataPaths::legacy`).
pub fn early_paths(mode: &LibMode, lib_dir: &Path, config_dir: &Path, cache_dir: &Path) -> DataPaths {
    match mode {
        LibMode::Legacy => DataPaths::legacy(config_dir, cache_dir, &visor_core::config::Config::default()),
        _ => DataPaths::library(lib_dir),
    }
}
