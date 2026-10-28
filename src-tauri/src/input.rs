//! Telemetria de raton y teclado de las estadisticas de juego: clics, muescas de rueda, distancia del cursor y
//! pulsaciones de teclado, con el pico por minuto de clics y de teclas. Usa Raw Input en una ventana solo de
//! mensajes (RIDEV_INPUTSINK): no se inyecta en ningun proceso, no retrasa la entrada y funciona aunque el juego
//! bloquee el cursor. Solo se cuenta con el juego delante. De las teclas solo se cuenta CUANTAS se pulsan: nunca se
//! guarda ni se mira cual es.
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use visor_core::db::data::InputCounts;

/// Ventana de los picos por minuto (ms).
const PEAK_WINDOW_MS: u64 = 60_000;

/// Ventana deslizante de eventos para sacar el maximo de eventos en un minuto.
#[derive(Default)]
pub struct PeakWindow {
    events: VecDeque<u64>,
    peak: u64,
}

impl PeakWindow {
    /// Anota `n` eventos en el instante `now_ms` y actualiza el pico.
    pub fn add(&mut self, now_ms: u64, n: u32) {
        for _ in 0..n {
            self.events.push_back(now_ms);
        }
        while self.events.front().map(|&t| now_ms.saturating_sub(t) > PEAK_WINDOW_MS).unwrap_or(false) {
            self.events.pop_front();
        }
        self.peak = self.peak.max(self.events.len() as u64);
    }

    pub fn peak(&self) -> u64 {
        self.peak
    }
}

/// Muescas de una rueda: 120 unidades por muesca (las ruedas finas dan menos: cuentan como una).
pub fn wheel_notches(delta: i16) -> u32 {
    ((delta as i32).unsigned_abs() / 120).max(1)
}

/// `true` si esta tecla acaba de bajar (no es autorrepeticion). Mantiene el conjunto de teclas pulsadas; las
/// subidas siempre se anotan, aunque el juego no este delante, para no dejar teclas «atascadas».
pub fn key_pressed_now(down: &mut HashSet<u16>, vkey: u16, is_break: bool) -> bool {
    if is_break {
        down.remove(&vkey);
        false
    } else {
        down.insert(vkey)
    }
}

/// Contadores compartidos entre el hilo de Raw Input y quien los lee.
#[derive(Default)]
struct Shared {
    clicks: AtomicU64,
    wheel: AtomicU64,
    dist: AtomicU64,
    keys: AtomicU64,
    peak_cpm: AtomicU64,
    peak_kpm: AtomicU64,
    /// Ventana de la ventana solo de mensajes (0 hasta que se crea), para pedirle que se cierre.
    hwnd: std::sync::atomic::AtomicIsize,
}

/// Captura en marcha. `stop` la cierra; `counts` lee lo contado hasta ahora.
pub struct Handle {
    shared: Arc<Shared>,
    thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Handle {
    pub fn counts(&self) -> InputCounts {
        let s = &self.shared;
        InputCounts {
            clicks: s.clicks.load(Ordering::Relaxed) as i64,
            wheel: s.wheel.load(Ordering::Relaxed) as i64,
            mouse_dist: s.dist.load(Ordering::Relaxed) as i64,
            keys: s.keys.load(Ordering::Relaxed) as i64,
            peak_cpm: s.peak_cpm.load(Ordering::Relaxed) as i64,
            peak_kpm: s.peak_kpm.load(Ordering::Relaxed) as i64,
            ..Default::default()
        }
    }

    /// Cierra la ventana, quita el registro de Raw Input y espera al hilo (como mucho ~1 s).
    pub fn stop(&self) {
        #[cfg(windows)]
        imp::request_close(&self.shared);
        if let Some(t) = self.thread.lock().unwrap().take() {
            let start = std::time::Instant::now();
            while !t.is_finished() && start.elapsed() < std::time::Duration::from_secs(1) {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if t.is_finished() {
                let _ = t.join();
            }
        }
    }
}

/// Empieza a contar. `None` si no se pide nada (`mouse` y `keys` a false) o si falla el registro.
/// Solo cuenta mientras `game_fg` es true.
pub fn start(mouse: bool, keys: bool, game_fg: Arc<AtomicBool>) -> Option<Handle> {
    if !mouse && !keys {
        return None;
    }
    #[cfg(windows)]
    {
        let shared = Arc::new(Shared::default());
        let (tx, rx) = std::sync::mpsc::channel::<bool>();
        let s2 = shared.clone();
        let thread = std::thread::Builder::new()
            .name("input-sink".into())
            .spawn(move || imp::run(s2, mouse, keys, game_fg, tx))
            .ok()?;
        if rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap_or(false) {
            Some(Handle { shared, thread: std::sync::Mutex::new(Some(thread)) })
        } else {
            let _ = thread.join();
            None
        }
    }
    #[cfg(not(windows))]
    {
        let _ = game_fg;
        None
    }
}

#[cfg(windows)]
mod imp {
    use super::{key_pressed_now, wheel_notches, PeakWindow, Shared};
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Instant;
    use windows::core::w;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::{
        GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RIDEV_INPUTSINK,
        RIDEV_REMOVE, RID_INPUT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, PostMessageW, PostQuitMessage,
        RegisterClassW, UnregisterClassW, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_DESTROY, WM_INPUT,
        WNDCLASSW,
    };

    // Banderas de RAWMOUSE.usButtonFlags y RAWKEYBOARD.Flags.
    const BTN_DOWN: [u16; 5] = [0x0001, 0x0004, 0x0010, 0x0040, 0x0100];
    const MOUSE_WHEEL: u16 = 0x0400;
    const MOUSE_MOVE_ABSOLUTE: u16 = 0x0001;
    const KEY_BREAK: u16 = 0x0001;
    const RIM_TYPEMOUSE: u32 = 0;
    const RIM_TYPEKEYBOARD: u32 = 1;

    /// Estado del hilo (la ventana y su procedimiento viven en el mismo hilo).
    struct Ctx {
        shared: Arc<Shared>,
        game_fg: Arc<AtomicBool>,
        t0: Instant,
        clicks: PeakWindow,
        keys: PeakWindow,
        down: HashSet<u16>,
        dist: f64,
    }

    thread_local! {
        static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
    }

    pub fn request_close(shared: &Shared) {
        let h = shared.hwnd.load(Ordering::SeqCst);
        if h != 0 {
            unsafe {
                let _ = PostMessageW(Some(HWND(h as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }

    pub fn run(shared: Arc<Shared>, mouse: bool, keys: bool, game_fg: Arc<AtomicBool>, ready: std::sync::mpsc::Sender<bool>) {
        unsafe {
            let Ok(hinst) = GetModuleHandleW(None) else {
                let _ = ready.send(false);
                return;
            };
            let class = w!("VisorInputSink");
            let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst.into(), lpszClassName: class, ..Default::default() };
            RegisterClassW(&wc);
            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE(0), class, w!(""), WINDOW_STYLE(0), 0, 0, 0, 0, Some(HWND_MESSAGE), None, Some(hinst.into()), None,
            ) {
                Ok(h) => h,
                Err(_) => {
                    let _ = UnregisterClassW(class, Some(hinst.into()));
                    let _ = ready.send(false);
                    return;
                }
            };
            let mut devs: Vec<RAWINPUTDEVICE> = Vec::new();
            if mouse {
                devs.push(RAWINPUTDEVICE { usUsagePage: 0x01, usUsage: 0x02, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd });
            }
            if keys {
                devs.push(RAWINPUTDEVICE { usUsagePage: 0x01, usUsage: 0x06, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd });
            }
            if RegisterRawInputDevices(&devs, std::mem::size_of::<RAWINPUTDEVICE>() as u32).is_err() {
                let _ = DestroyWindow(hwnd);
                let _ = UnregisterClassW(class, Some(hinst.into()));
                let _ = ready.send(false);
                return;
            }
            shared.hwnd.store(hwnd.0 as isize, Ordering::SeqCst);
            CTX.with(|c| {
                *c.borrow_mut() = Some(Ctx {
                    shared: shared.clone(),
                    game_fg,
                    t0: Instant::now(),
                    clicks: PeakWindow::default(),
                    keys: PeakWindow::default(),
                    down: HashSet::new(),
                    dist: 0.0,
                })
            });
            let _ = ready.send(true);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                DispatchMessageW(&msg);
            }

            // Se quita el registro (hwndTarget a null, como exige RIDEV_REMOVE) y se libera la clase.
            let rm: Vec<RAWINPUTDEVICE> = devs
                .iter()
                .map(|d| RAWINPUTDEVICE { usUsagePage: d.usUsagePage, usUsage: d.usUsage, dwFlags: RIDEV_REMOVE, hwndTarget: HWND::default() })
                .collect();
            let _ = RegisterRawInputDevices(&rm, std::mem::size_of::<RAWINPUTDEVICE>() as u32);
            shared.hwnd.store(0, Ordering::SeqCst);
            CTX.with(|c| *c.borrow_mut() = None);
            let _ = UnregisterClassW(class, Some(hinst.into()));
        }
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_INPUT => {
                on_input(HRAWINPUT(lparam.0 as *mut _));
                // WM_INPUT en segundo plano (INPUTSINK) se deja a DefWindowProc para que el sistema limpie el evento.
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
            WM_CLOSE => {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    fn on_input(h: HRAWINPUT) {
        let hdr = std::mem::size_of::<RAWINPUTHEADER>() as u32;
        let mut size = 0u32;
        unsafe {
            if GetRawInputData(h, RID_INPUT, None, &mut size, hdr) != 0 || size == 0 {
                return;
            }
        }
        // u64 para que el bufer quede alineado como RAWINPUT.
        let mut buf = vec![0u64; (size as usize).div_ceil(8)];
        let got = unsafe { GetRawInputData(h, RID_INPUT, Some(buf.as_mut_ptr() as *mut _), &mut size, hdr) };
        if got == u32::MAX || (got as usize) < std::mem::size_of::<RAWINPUTHEADER>() {
            return;
        }
        let ri = unsafe { &*(buf.as_ptr() as *const RAWINPUT) };
        CTX.with(|c| {
            let mut c = c.borrow_mut();
            let Some(ctx) = c.as_mut() else { return };
            let fg = ctx.game_fg.load(Ordering::Relaxed);
            let now = ctx.t0.elapsed().as_millis() as u64;
            match ri.header.dwType {
                RIM_TYPEMOUSE => {
                    if !fg {
                        return;
                    }
                    let m = unsafe { ri.data.mouse };
                    let (flags, data) = unsafe { (m.Anonymous.Anonymous.usButtonFlags, m.Anonymous.Anonymous.usButtonData) };
                    let downs = BTN_DOWN.iter().filter(|&&b| flags & b != 0).count() as u32;
                    if downs > 0 {
                        ctx.shared.clicks.fetch_add(downs as u64, Ordering::Relaxed);
                        ctx.clicks.add(now, downs);
                        ctx.shared.peak_cpm.store(ctx.clicks.peak(), Ordering::Relaxed);
                    }
                    if flags & MOUSE_WHEEL != 0 {
                        ctx.shared.wheel.fetch_add(wheel_notches(data as i16) as u64, Ordering::Relaxed);
                    }
                    // Solo movimientos relativos: los absolutos (tabletas, escritorio remoto) no miden recorrido.
                    if m.usFlags.0 & MOUSE_MOVE_ABSOLUTE == 0 && (m.lLastX != 0 || m.lLastY != 0) {
                        ctx.dist += ((m.lLastX as f64).powi(2) + (m.lLastY as f64).powi(2)).sqrt();
                        ctx.shared.dist.store(ctx.dist as u64, Ordering::Relaxed);
                    }
                }
                RIM_TYPEKEYBOARD => {
                    let k = unsafe { ri.data.keyboard };
                    // 0xFF es una tecla «falsa» que Windows manda en secuencias especiales.
                    if k.VKey == 0xFF {
                        return;
                    }
                    // Solo se mira si la tecla baja o sube para filtrar la autorrepeticion; el codigo no se guarda.
                    let new_down = key_pressed_now(&mut ctx.down, k.VKey, k.Flags & KEY_BREAK != 0);
                    if new_down && fg {
                        ctx.shared.keys.fetch_add(1, Ordering::Relaxed);
                        ctx.keys.add(now, 1);
                        ctx.shared.peak_kpm.store(ctx.keys.peak(), Ordering::Relaxed);
                    }
                }
                _ => {}
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_is_the_most_events_within_a_minute() {
        let mut w = PeakWindow::default();
        w.add(0, 3);
        w.add(30_000, 2);
        assert_eq!(w.peak(), 5);
        // A los 61 s los primeros 3 ya no cuentan: la ventana tiene 2 + 1, pero el pico no baja.
        w.add(61_000, 1);
        assert_eq!(w.peak(), 5);
        // Justo a 60 s del primero todavia entra (el limite es estricto).
        let mut w = PeakWindow::default();
        w.add(0, 1);
        w.add(60_000, 1);
        assert_eq!(w.peak(), 2);
        w.add(60_001, 1);
        assert_eq!(w.peak(), 2);
    }

    #[test]
    fn wheel_counts_notches_and_never_zero() {
        assert_eq!(wheel_notches(120), 1);
        assert_eq!(wheel_notches(-240), 2);
        assert_eq!(wheel_notches(30), 1);
        assert_eq!(wheel_notches(i16::MIN), 273);
    }

    #[test]
    fn key_repeat_is_not_counted_and_release_rearms() {
        let mut down = HashSet::new();
        assert!(key_pressed_now(&mut down, 65, false));
        assert!(!key_pressed_now(&mut down, 65, false)); // autorrepeticion
        assert!(key_pressed_now(&mut down, 66, false));
        assert!(!key_pressed_now(&mut down, 65, true)); // suelta
        assert!(key_pressed_now(&mut down, 65, false)); // otra vez
    }
}
