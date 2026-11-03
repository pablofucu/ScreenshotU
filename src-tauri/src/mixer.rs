//! Volumen de Windows para el menu sobre el juego: el general y el silencio de la salida predeterminada, el silencio del
//! microfono y el mezclador por aplicacion (sesiones de audio de la salida predeterminada).
use crate::outputs::AudioOutput;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppVolume {
    pub pid: u32,
    pub name: String,
    /// 0..100
    pub volume: f32,
    pub mute: bool,
    /// Es un proceso del juego en marcha.
    pub is_game: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    /// 0..100
    pub volume: f32,
    pub mute: bool,
    /// `None` si no hay microfono.
    pub mic_mute: Option<bool>,
    pub outputs: Vec<AudioOutput>,
    pub inputs: Vec<AudioOutput>,
    pub apps: Vec<AppVolume>,
}

#[cfg(windows)]
mod win {
    use super::{AppVolume, AudioState};
    use std::collections::BTreeMap;
    use windows::core::Interface;
    use windows::Win32::Foundation::S_OK;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eCapture, eConsole, eRender, AudioSessionStateExpired, IAudioSessionControl2, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    fn err(e: windows::core::Error) -> String {
        format!("No se pudo leer el audio de Windows: {e}")
    }

    fn default_device(capture: bool) -> Result<IMMDevice, String> {
        let _ = wasapi::initialize_mta();
        unsafe {
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(err)?;
            en.GetDefaultAudioEndpoint(if capture { eCapture } else { eRender }, eConsole).map_err(err)
        }
    }

    fn endpoint(capture: bool) -> Result<IAudioEndpointVolume, String> {
        unsafe { default_device(capture)?.Activate(CLSCTX_ALL, None).map_err(err) }
    }

    /// Sesiones de audio con su proceso. Las del sistema y las caducadas no se enseñan.
    fn sessions() -> Result<Vec<(u32, ISimpleAudioVolume)>, String> {
        unsafe {
            let mgr: IAudioSessionManager2 = default_device(false)?.Activate(CLSCTX_ALL, None).map_err(err)?;
            let list = mgr.GetSessionEnumerator().map_err(err)?;
            let mut out = Vec::new();
            for i in 0..list.GetCount().map_err(err)? {
                let Ok(s) = list.GetSession(i) else { continue };
                if s.GetState().map(|st| st == AudioSessionStateExpired).unwrap_or(true) {
                    continue;
                }
                let Ok(c2) = s.cast::<IAudioSessionControl2>() else { continue };
                if c2.IsSystemSoundsSession() == S_OK {
                    continue;
                }
                let pid = c2.GetProcessId().unwrap_or(0);
                if pid == 0 || pid == std::process::id() {
                    continue;
                }
                if let Ok(v) = s.cast::<ISimpleAudioVolume>() {
                    out.push((pid, v));
                }
            }
            Ok(out)
        }
    }

    fn apps(game_pids: &[u32]) -> Vec<AppVolume> {
        let mut by_pid: BTreeMap<u32, AppVolume> = BTreeMap::new();
        for (pid, v) in sessions().unwrap_or_default() {
            unsafe {
                let volume = v.GetMasterVolume().unwrap_or(1.0) * 100.0;
                let mute = v.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                by_pid.entry(pid).or_insert_with(|| AppVolume { pid, name: super::app_name(pid), volume, mute, is_game: game_pids.contains(&pid) });
            }
        }
        let mut v: Vec<AppVolume> = by_pid.into_values().collect();
        // Primero el juego; el resto por nombre.
        v.sort_by(|a, b| b.is_game.cmp(&a.is_game).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        v
    }

    pub fn state(game_pids: &[u32]) -> Result<AudioState, String> {
        let out = endpoint(false)?;
        let (volume, mute) = unsafe { (out.GetMasterVolumeLevelScalar().map_err(err)? * 100.0, out.GetMute().map_err(err)?.as_bool()) };
        let mic_mute = endpoint(true).ok().and_then(|m| unsafe { m.GetMute().ok().map(|b| b.as_bool()) });
        Ok(AudioState { volume, mute, mic_mute, outputs: crate::outputs::list().unwrap_or_default(), inputs: crate::outputs::list_inputs().unwrap_or_default(), apps: apps(game_pids) })
    }

    pub fn set_volume(v: f32) -> Result<(), String> {
        let out = endpoint(false)?;
        unsafe {
            // Subir el volumen quita el silencio, como con las teclas de volumen.
            out.SetMute(false, std::ptr::null()).map_err(err)?;
            out.SetMasterVolumeLevelScalar((v / 100.0).clamp(0.0, 1.0), std::ptr::null()).map_err(err)
        }
    }

    pub fn set_mute(m: bool) -> Result<(), String> {
        unsafe { endpoint(false)?.SetMute(m, std::ptr::null()).map_err(err) }
    }

    pub fn set_mic_mute(m: bool) -> Result<(), String> {
        unsafe { endpoint(true)?.SetMute(m, std::ptr::null()).map_err(err) }
    }

    pub fn set_app_volume(pid: u32, v: f32) -> Result<(), String> {
        for (p, s) in sessions()? {
            if p == pid {
                unsafe {
                    s.SetMute(false, std::ptr::null()).map_err(err)?;
                    s.SetMasterVolume((v / 100.0).clamp(0.0, 1.0), std::ptr::null()).map_err(err)?;
                }
            }
        }
        Ok(())
    }

    pub fn set_app_mute(pid: u32, m: bool) -> Result<(), String> {
        for (p, s) in sessions()? {
            if p == pid {
                unsafe { s.SetMute(m, std::ptr::null()).map_err(err)? };
            }
        }
        Ok(())
    }
}

/// Nombre de la aplicacion para el mezclador: el del ejecutable sin extension.
#[cfg(windows)]
fn app_name(pid: u32) -> String {
    crate::snap::exe_of(pid)
        .and_then(|e| std::path::Path::new(&e).file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| format!("Proceso {pid}"))
}

#[cfg(windows)]
pub use win::{set_app_mute, set_app_volume, set_mic_mute, set_mute, set_volume, state};

#[cfg(not(windows))]
mod other {
    use super::AudioState;
    const NO: &str = "Solo disponible en Windows";
    pub fn state(_game_pids: &[u32]) -> Result<AudioState, String> {
        Err(NO.into())
    }
    pub fn set_volume(_v: f32) -> Result<(), String> {
        Err(NO.into())
    }
    pub fn set_mute(_m: bool) -> Result<(), String> {
        Err(NO.into())
    }
    pub fn set_mic_mute(_m: bool) -> Result<(), String> {
        Err(NO.into())
    }
    pub fn set_app_volume(_pid: u32, _v: f32) -> Result<(), String> {
        Err(NO.into())
    }
    pub fn set_app_mute(_pid: u32, _m: bool) -> Result<(), String> {
        Err(NO.into())
    }
}

#[cfg(not(windows))]
pub use other::{set_app_mute, set_app_volume, set_mic_mute, set_mute, set_volume, state};
