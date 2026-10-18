//! Lectura del mando con XInput desde un hilo propio (la webview no recibe mando sin foco). Dispara la
//! combinacion de captura y, mientras Ajustes la esta aprendiendo, informa de lo que se pulsa.
//! XInput no es exclusivo: el juego sigue recibiendo el mando igual; aqui solo se lee (y se vibra un instante).
use tauri::AppHandle;

/// Estado del aprendizaje de combinacion que se manda a la interfaz.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnPad {
    /// Combinacion pulsada hasta ahora (`back+rb`, `off` si nada).
    pub combo: String,
    /// Hay algun mando XInput conectado.
    pub connected: bool,
    /// La combinacion esta completa (se soltaron todos los botones).
    pub done: bool,
}

#[cfg(windows)]
mod xinput {
    use windows::core::w;
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::UI::Input::XboxController::{XInputGetState, XInputSetState, XINPUT_STATE, XINPUT_VIBRATION};

    /// Pone los motores del mando `i` (0-65535). No bloquea ni toma el mando: el juego puede seguir usandolo.
    pub fn vibrate(i: u32, left: u16, right: u16) {
        let v = XINPUT_VIBRATION { wLeftMotorSpeed: left, wRightMotorSpeed: right };
        unsafe {
            let _ = XInputSetState(i, &v);
        }
    }

    /// `XInputGetStateEx` (ordinal 100 de xinput1_4/1_3) rellena tambien el boton Xbox. Escribe un poco mas
    /// que `XINPUT_STATE`, de ahi el relleno.
    #[repr(C)]
    #[derive(Default)]
    struct StateEx {
        st: XINPUT_STATE,
        _pad: [u32; 4],
    }
    type GetStateEx = unsafe extern "system" fn(u32, *mut StateEx) -> u32;

    pub struct Reader {
        ex: Option<GetStateEx>,
    }

    impl Reader {
        pub fn new() -> Self {
            let ex = unsafe {
                [w!("xinput1_4.dll"), w!("xinput1_3.dll")].into_iter().find_map(|dll| {
                    let lib = LoadLibraryW(dll).ok()?;
                    // Ordinal 100: se pasa como puntero con el numero (MAKEINTRESOURCEA).
                    let f = GetProcAddress(lib, windows::core::PCSTR(100usize as *const u8))?;
                    Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, GetStateEx>(f))
                })
            };
            Reader { ex }
        }

        /// (wButtons, gatillo izq., gatillo der.) o `None` si no hay mando en ese puerto.
        pub fn read(&self, i: u32) -> Option<(u16, u8, u8)> {
            unsafe {
                if let Some(f) = self.ex {
                    let mut st = StateEx::default();
                    if f(i, &mut st) == 0 {
                        let g = st.st.Gamepad;
                        return Some((g.wButtons.0, g.bLeftTrigger, g.bRightTrigger));
                    }
                    return None;
                }
                let mut st = XINPUT_STATE::default();
                if XInputGetState(i, &mut st) == 0 {
                    let g = st.Gamepad;
                    return Some((g.wButtons.0, g.bLeftTrigger, g.bRightTrigger));
                }
                None
            }
        }
    }
}

/// Vibracion corta de confirmacion en los mandos de `pads` (bit `i` = puerto `i`). Solo el motor fino y ~0,1 s;
/// despues los deja parados (si el juego estaba vibrando, lo vuelve a pedir en su siguiente efecto).
#[cfg(windows)]
pub fn rumble(pads: u8) {
    if pads == 0 {
        return;
    }
    let _ = std::thread::Builder::new().name("pad-rumble".into()).spawn(move || {
        let each = |l: u16, r: u16| (0..4u32).filter(|i| pads & (1 << i) != 0).for_each(|i| xinput::vibrate(i, l, r));
        each(0, 40000);
        std::thread::sleep(std::time::Duration::from_millis(110));
        each(0, 0);
    });
}

#[cfg(not(windows))]
pub fn rumble(_pads: u8) {}

#[cfg(windows)]
pub fn spawn(app: AppHandle) {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    use tauri::{Emitter, Manager};
    use visor_core::capture::{combo_id, pad_mask, ComboDetector, ComboLearner};

    std::thread::spawn(move || {
        let reader = xinput::Reader::new();
        let t0 = Instant::now();
        let mut det = ComboDetector::new(0, 1200);
        let mut learner = ComboLearner::default();
        let mut was_learning = false;
        let mut last_sent: Option<(u32, bool)> = None;
        let mut connected = [true; 4];
        let mut last_rescan = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(12));
            let st = app.state::<crate::state::AppState>();
            let learning = st.learning_pad.load(Ordering::Relaxed);
            let mask = st.combo_mask.load(Ordering::Relaxed);
            if learning != was_learning {
                was_learning = learning;
                learner.reset();
                last_sent = None;
            }
            if mask == 0 && !learning {
                // Sin combinacion no hace falta consultar el mando.
                det.mask = 0;
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
            // XInputGetState sobre mandos desconectados es lento: solo se reintenta cada 2 s (cada 0,5 s
            // mientras se aprende, para notar enseguida un mando recien encendido).
            let every = if learning { 500 } else { 2000 };
            let rescan = last_rescan.elapsed() > Duration::from_millis(every);
            if rescan {
                last_rescan = Instant::now();
            }
            let mut buttons = 0u32;
            // Mandos que tienen pulsado algo de la combinacion (para vibrar solo el que captura).
            let mut pressing = 0u8;
            for (i, c) in connected.iter_mut().enumerate() {
                if !*c && !rescan {
                    continue;
                }
                match reader.read(i as u32) {
                    Some((b, lt, rt)) => {
                        *c = true;
                        let m = pad_mask(b, lt, rt);
                        buttons |= m;
                        if m & mask != 0 {
                            pressing |= 1 << i;
                        }
                    }
                    None => *c = false,
                }
            }
            let any = connected.iter().any(|c| *c);

            if learning {
                det.mask = 0;
                let got = learner.update(buttons);
                let cur = got.unwrap_or(learner.current());
                if got.is_some() {
                    st.learning_pad.store(false, Ordering::Relaxed);
                    let _ = app.emit("learn-pad", LearnPad { combo: combo_id(cur), connected: any, done: true });
                } else if last_sent != Some((cur, any)) {
                    last_sent = Some((cur, any));
                    let _ = app.emit("learn-pad", LearnPad { combo: combo_id(cur), connected: any, done: false });
                }
                continue;
            }

            det.mask = mask;
            // Con el visor delante el mando navega por los menus: no captura.
            if det.update(buttons, t0.elapsed().as_millis() as u64) && !crate::platform::own_window_focused() {
                crate::snap::trigger_from_pad(&app, pressing);
            }
        }
    });
}

#[cfg(not(windows))]
pub fn spawn(_app: AppHandle) {}
