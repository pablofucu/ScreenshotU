//! Salida de audio de Windows: lista los dispositivos de reproduccion activos y cambia el predeterminado.
//! Listar es la API oficial (MMDevice, via `wasapi`); cambiar el predeterminado no tiene API publica y se hace con
//! la interfaz COM no documentada `IPolicyConfig` (la que usan SoundSwitch y similares). Se cambian los tres roles
//! (consola, multimedia y comunicaciones) para que se comporte como elegirlo en Configuracion > Sonido.
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOutput {
    pub id: String,
    /// Nombre completo de Windows («Altavoces (Realtek High Definition Audio)»).
    pub name: String,
    /// Nombre corto para el aviso en pantalla («Altavoces»).
    pub short: String,
    /// Es el predeterminado ahora.
    pub default: bool,
}

/// «Altavoces (Realtek High Definition Audio)» -> «Altavoces».
pub fn short_name(full: &str) -> String {
    let s = full.split(" (").next().unwrap_or(full).trim();
    if s.is_empty() { full.trim().to_string() } else { s.to_string() }
}

#[cfg(windows)]
#[allow(non_snake_case, dead_code)]
mod win {
    use super::{short_name, AudioOutput};
    use std::ffi::c_void;
    use windows::core::{HRESULT, HSTRING, PCWSTR};
    use windows::Win32::Media::Audio::{eCommunications, eConsole, eMultimedia};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    /// `IPolicyConfig` (Vista+). Solo se usa `SetDefaultEndpoint`; el resto de la tabla esta para que la posicion
    /// del metodo sea la correcta (los parametros no importan, son punteros).
    #[windows::core::interface("f8679f50-850a-41cf-9c72-430f290290c8")]
    unsafe trait IPolicyConfig: windows::core::IUnknown {
        unsafe fn GetMixFormat(&self, a: PCWSTR, b: *mut c_void) -> HRESULT;
        unsafe fn GetDeviceFormat(&self, a: PCWSTR, b: i32, c: *mut c_void) -> HRESULT;
        unsafe fn ResetDeviceFormat(&self, a: PCWSTR) -> HRESULT;
        unsafe fn SetDeviceFormat(&self, a: PCWSTR, b: *mut c_void, c: *mut c_void) -> HRESULT;
        unsafe fn GetProcessingPeriod(&self, a: PCWSTR, b: i32, c: *mut c_void, d: *mut c_void) -> HRESULT;
        unsafe fn SetProcessingPeriod(&self, a: PCWSTR, b: *mut c_void) -> HRESULT;
        unsafe fn GetShareMode(&self, a: PCWSTR, b: *mut c_void) -> HRESULT;
        unsafe fn SetShareMode(&self, a: PCWSTR, b: *mut c_void) -> HRESULT;
        unsafe fn GetPropertyValue(&self, a: PCWSTR, b: i32, c: *const c_void, d: *mut c_void) -> HRESULT;
        unsafe fn SetPropertyValue(&self, a: PCWSTR, b: i32, c: *const c_void, d: *mut c_void) -> HRESULT;
        unsafe fn SetDefaultEndpoint(&self, a: PCWSTR, role: i32) -> HRESULT;
        unsafe fn SetEndpointVisibility(&self, a: PCWSTR, b: i32) -> HRESULT;
    }

    /// CLSID de `CPolicyConfigClient`.
    const POLICY_CONFIG_CLIENT: windows::core::GUID = windows::core::GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

    pub fn list() -> Result<Vec<AudioOutput>, String> {
        use wasapi::{initialize_mta, DeviceEnumerator, Direction};
        let _ = initialize_mta();
        let en = DeviceEnumerator::new().map_err(|e| format!("No se pudo leer el audio de Windows: {e}"))?;
        let default_id = en.get_default_device(&Direction::Render).and_then(|d| d.get_id()).ok();
        let col = en.get_device_collection(&Direction::Render).map_err(|e| format!("No se pudo listar las salidas de audio: {e}"))?;
        let mut out = Vec::new();
        for d in &col {
            let Ok(d) = d else { continue };
            let (Ok(id), Ok(name)) = (d.get_id(), d.get_friendlyname()) else { continue };
            out.push(AudioOutput { default: default_id.as_deref() == Some(id.as_str()), short: short_name(&name), id, name });
        }
        Ok(out)
    }

    pub fn set_default(id: &str) -> Result<(), String> {
        let _ = wasapi::initialize_mta();
        unsafe {
            let pc: IPolicyConfig = CoCreateInstance(&POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)
                .map_err(|e| format!("No se pudo cambiar la salida de audio: {e}"))?;
            let wid = HSTRING::from(id);
            for role in [eConsole, eMultimedia, eCommunications] {
                pc.SetDefaultEndpoint(PCWSTR(wid.as_ptr()), role.0).ok().map_err(|e| format!("No se pudo cambiar la salida de audio: {e}"))?;
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
pub fn list() -> Result<Vec<AudioOutput>, String> {
    win::list()
}
#[cfg(windows)]
pub fn set_default(id: &str) -> Result<(), String> {
    win::set_default(id)
}
#[cfg(not(windows))]
pub fn list() -> Result<Vec<AudioOutput>, String> {
    Ok(Vec::new())
}
#[cfg(not(windows))]
pub fn set_default(_id: &str) -> Result<(), String> {
    Err("Solo disponible en Windows".into())
}

/// Pone `id` como salida predeterminada y avisa a la interfaz. Devuelve la lista nueva.
pub fn select(app: &AppHandle, id: &str) -> Result<Vec<AudioOutput>, String> {
    set_default(id)?;
    let outs = list()?;
    let _ = app.emit("audio-outputs", &outs);
    Ok(outs)
}

/// Pasa a la salida siguiente (`dir` = 1) o anterior (-1), dando la vuelta. Devuelve la nueva y su posicion
/// (empezando en 0) o `None` si solo hay una.
pub fn cycle(app: &AppHandle, dir: i32) -> Result<Option<(AudioOutput, usize)>, String> {
    let outs = list()?;
    if outs.len() < 2 {
        return Ok(None);
    }
    let cur = outs.iter().position(|o| o.default).unwrap_or(0) as i32;
    let n = outs.len() as i32;
    let next = (((cur + dir) % n + n) % n) as usize;
    let outs = select(app, &outs[next].id)?;
    Ok(outs.into_iter().enumerate().find(|(_, o)| o.default).map(|(i, o)| (o, i)))
}

fn rumble_enabled(app: &AppHandle) -> bool {
    use tauri::Manager;
    app.state::<crate::state::AppState>().config.lock().unwrap().capture_rumble
}

/// Cambio hecho con el mando o el atajo: cambia de salida y avisa sobre el juego (cartel y, con el mando, una
/// vibracion de tantos pulsos como la posicion de la salida: 1 para la primera, 2 para la segunda...).
pub fn cycle_with_feedback(app: &AppHandle, pads: u8) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("audio-out".into()).spawn(move || match cycle(&app, 1) {
        Ok(Some((o, i))) => {
            crate::overlay::flash(&o.short);
            if rumble_enabled(&app) {
                crate::pad::rumble_n(pads, (i + 1).min(3) as u8);
            }
        }
        Ok(None) => crate::overlay::flash("Solo hay una salida de audio"),
        Err(e) => {
            eprintln!("salida de audio: {e}");
            crate::overlay::flash("No se pudo cambiar el audio");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::short_name;

    #[test]
    fn short_names() {
        assert_eq!(short_name("Altavoces (Realtek High Definition Audio)"), "Altavoces");
        assert_eq!(short_name("LG ULTRAGEAR (NVIDIA High Definition Audio)"), "LG ULTRAGEAR");
        assert_eq!(short_name("Auriculares"), "Auriculares");
        assert_eq!(short_name(" (raro)"), "(raro)");
    }
}
