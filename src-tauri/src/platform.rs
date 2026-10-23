//! Parte dependiente del sistema operativo de la captura: Windows.Graphics.Capture (con respaldo GDI), Magpie,
//! sonido y datos de la ventana en primer plano. En otros sistemas son funciones vacias que devuelven error.

pub struct Grabbed {
    pub width: u32,
    pub height: u32,
    /// RGBA.
    pub rgba: Vec<u8>,
    /// Metodo que funciono (para el registro).
    pub method: String,
    /// Proceso de la ventana capturada (para saber a que juego pertenece).
    pub fg_pid: Option<u32>,
}

#[cfg(windows)]
mod imp {
    use super::Grabbed;
    use std::sync::mpsc::{channel, Sender};
    use std::time::{Duration, Instant};
    use visor_core::capture::{bgra_to_rgba, crop_for, crop_rgba, is_blank, is_magpie_scaling_window, shutter_wav, Rect};
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
        GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HBITMAP, HDC, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        MONITOR_DEFAULTTOPRIMARY, SRCCOPY,
    };
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
    use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetClientRect, GetForegroundWindow, GetWindow, GetWindowDisplayAffinity, GetWindowRect,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, GW_HWNDNEXT,
    };
    use windows::core::BOOL;
    use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
    use windows_capture::frame::Frame;
    use windows_capture::graphics_capture_api::InternalCaptureControl;
    use windows_capture::monitor::Monitor;
    use windows_capture::settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, GraphicsCaptureItemType,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    };
    use windows_capture::window::Window;

    /// Imagen capturada (RGBA).
    struct Img {
        w: u32,
        h: u32,
        px: Vec<u8>,
    }

    impl Img {
        fn blank(&self) -> bool {
            is_blank(&self.px)
        }
        /// Recorta a `inner` si la imagen corresponde a `frame`; si no cuadra, la deja como esta.
        fn cropped(self, frame: Rect, inner: Rect) -> Img {
            match crop_for(self.w, self.h, frame, inner).and_then(|(x, y, w, h)| crop_rgba(&self.px, self.w, self.h, x, y, w, h)) {
                Some((w, h, px)) => Img { w, h, px },
                None => self,
            }
        }
    }

    /* ---------------- Windows.Graphics.Capture ---------------- */

    type Msg = Result<(u32, u32, Vec<u8>), String>;

    /// Recibe fotogramas hasta que llega uno que no es negro (o se cansa) y para la captura.
    struct Shot {
        tx: Sender<Msg>,
        n: u32,
    }

    impl GraphicsCaptureApiHandler for Shot {
        type Flags = Sender<Msg>;
        type Error = Box<dyn std::error::Error + Send + Sync>;

        fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
            Ok(Shot { tx: ctx.flags, n: 0 })
        }

        fn on_frame_arrived(&mut self, frame: &mut Frame, control: InternalCaptureControl) -> Result<(), Self::Error> {
            let (w, h) = (frame.width(), frame.height());
            let buf = frame.buffer()?;
            let mut scratch = Vec::new();
            let data = buf.as_nopadding_buffer(&mut scratch).to_vec();
            self.n += 1;
            // El primer fotograma de una captura recien creada a veces sale negro: se esperan unos cuantos.
            let done = !is_blank(&data) || self.n >= 8;
            let _ = self.tx.send(Ok((w, h, data)));
            if done {
                control.stop();
            }
            Ok(())
        }

        fn on_closed(&mut self) -> Result<(), Self::Error> {
            let _ = self.tx.send(Err("el elemento capturado se cerro".into()));
            Ok(())
        }
    }

    fn wgc<T>(item: T) -> Result<Img, String>
    where
        T: TryInto<GraphicsCaptureItemType> + Send + Copy + 'static,
    {
        let (tx, rx) = channel::<Msg>();
        let mk = |cursor, border, tx| {
            Settings::new(
                item,
                cursor,
                border,
                SecondaryWindowSettings::Default,
                MinimumUpdateIntervalSettings::Default,
                DirtyRegionSettings::Default,
                ColorFormat::Bgra8,
                tx,
            )
        };
        // Quitar el cursor y el borde amarillo no existe en Windows 10 antiguos: se reintenta sin pedirlo.
        let control = match Shot::start_free_threaded(mk(CursorCaptureSettings::WithoutCursor, DrawBorderSettings::WithoutBorder, tx.clone())) {
            Ok(c) => c,
            Err(_) => Shot::start_free_threaded(mk(CursorCaptureSettings::Default, DrawBorderSettings::Default, tx))
                .map_err(|e| format!("no se pudo iniciar: {e}"))?,
        };
        let mut deadline = Instant::now() + Duration::from_millis(2500);
        let mut last: Option<(u32, u32, Vec<u8>)> = None;
        let mut err: Option<String> = None;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            match rx.recv_timeout(left) {
                Ok(Ok(f)) => {
                    let ok = !is_blank(&f.2);
                    last = Some(f);
                    if ok {
                        break;
                    }
                    // Una ventana quieta no manda mas fotogramas: no se espera mucho por otro.
                    deadline = deadline.min(Instant::now() + Duration::from_millis(700));
                }
                Ok(Err(e)) => {
                    err = Some(e);
                    break;
                }
                Err(_) => break,
            }
        }
        let _ = control.stop();
        let (w, h, mut px) = last.ok_or_else(|| err.unwrap_or_else(|| "no llego ningun fotograma".into()))?;
        if w == 0 || h == 0 || px.len() < (w as usize) * (h as usize) * 4 {
            return Err("fotograma incompleto".into());
        }
        px.truncate((w as usize) * (h as usize) * 4);
        bgra_to_rgba(&mut px);
        Ok(Img { w, h, px })
    }

    /* ---------------- GDI ---------------- */

    /// Lee un bitmap de 32 bits a RGBA opaco. El bitmap no debe estar seleccionado en ningun DC.
    unsafe fn read_bitmap(dc: HDC, bmp: HBITMAP, w: i32, h: i32) -> Option<Vec<u8>> {
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
        let lines = GetDIBits(dc, bmp, 0, h as u32, Some(buf.as_mut_ptr().cast()), &mut bi, DIB_RGB_COLORS);
        if lines != h {
            return None;
        }
        bgra_to_rgba(&mut buf);
        for p in buf.chunks_exact_mut(4) {
            p[3] = 255;
        }
        Some(buf)
    }

    /// Copia una zona de la pantalla (o una ventana con PrintWindow) a RGBA.
    fn gdi(rect: RECT, window: Option<HWND>) -> Result<Img, String> {
        let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
        if w <= 0 || h <= 0 || w > 16384 || h > 16384 {
            return Err("tamano no valido".into());
        }
        unsafe {
            let screen = GetDC(None);
            if screen.is_invalid() {
                return Err("sin acceso a la pantalla".into());
            }
            let mem = CreateCompatibleDC(Some(screen));
            let bmp = CreateCompatibleBitmap(screen, w, h);
            let old = SelectObject(mem, bmp.into());
            let ok = match window {
                // PW_RENDERFULLCONTENT (2): tambien ventanas aceleradas por DirectComposition.
                Some(hwnd) => PrintWindow(hwnd, mem, PRINT_WINDOW_FLAGS(2)).as_bool(),
                None => BitBlt(mem, 0, 0, w, h, Some(screen), rect.left, rect.top, SRCCOPY | CAPTUREBLT).is_ok(),
            };
            SelectObject(mem, old);
            let px = if ok { read_bitmap(mem, bmp, w, h) } else { None };
            let _ = DeleteObject(bmp.into());
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);
            px.map(|px| Img { w: w as u32, h: h as u32, px })
                .ok_or_else(|| if window.is_some() { "PrintWindow fallo".into() } else { "BitBlt fallo".into() })
        }
    }

    /* ---------------- ventanas ---------------- */

    fn rect_t(r: RECT) -> Rect {
        (r.left, r.top, r.right, r.bottom)
    }

    fn pid_of(hwnd: HWND) -> Option<u32> {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        (pid != 0).then_some(pid)
    }

    fn exe_name(pid: u32) -> String {
        unsafe {
            let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
            let _ = CloseHandle(h);
            if r.is_err() {
                return String::new();
            }
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            full.rsplit(['\\', '/']).next().unwrap_or("").to_string()
        }
    }

    fn class_name(hwnd: HWND) -> String {
        let mut buf = [0u16; 256];
        let n = unsafe { GetClassNameW(hwnd, &mut buf) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    fn cloaked(hwnd: HWND) -> bool {
        let mut c = 0u32;
        unsafe { DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, (&mut c as *mut u32).cast(), 4).is_ok() && c != 0 }
    }

    /// Visible, no minimizada y no oculta por DWM (escritorios virtuales, apps suspendidas).
    fn showing(hwnd: HWND) -> bool {
        unsafe { IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() && !cloaked(hwnd) }
    }

    /// Rectangulo visible de la ventana (sin los bordes invisibles de Windows 10/11).
    fn frame_rect(hwnd: HWND) -> Option<RECT> {
        let mut r = RECT::default();
        let dwm = unsafe {
            DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, (&mut r as *mut RECT).cast(), std::mem::size_of::<RECT>() as u32)
        };
        if dwm.is_ok() && r.right > r.left {
            return Some(r);
        }
        window_rect(hwnd)
    }

    fn window_rect(hwnd: HWND) -> Option<RECT> {
        let mut r = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut r) }.ok()?;
        (r.right > r.left && r.bottom > r.top).then_some(r)
    }

    /// Zona cliente (sin barra de titulo ni bordes) en coordenadas de pantalla.
    fn client_rect(hwnd: HWND) -> Option<RECT> {
        let mut r = RECT::default();
        unsafe { GetClientRect(hwnd, &mut r) }.ok()?;
        let mut p = POINT::default();
        if !unsafe { ClientToScreen(hwnd, &mut p) }.as_bool() {
            return None;
        }
        let r = RECT { left: p.x, top: p.y, right: p.x + r.right, bottom: p.y + r.bottom };
        (r.right > r.left && r.bottom > r.top).then_some(r)
    }

    fn area(hwnd: HWND) -> i64 {
        frame_rect(hwnd).map(|r| (r.right - r.left) as i64 * (r.bottom - r.top) as i64).unwrap_or(0)
    }

    fn monitor_rect(m: HMONITOR) -> Option<RECT> {
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        unsafe { GetMonitorInfoW(m, &mut mi) }.as_bool().then_some(mi.rcMonitor)
    }

    fn top_windows() -> Vec<HWND> {
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            (*(lp.0 as *mut Vec<HWND>)).push(hwnd);
            BOOL(1)
        }
        let mut v: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(cb), LPARAM(&mut v as *mut Vec<HWND> as isize));
        }
        v
    }

    /// Ventana de escalado de Magpie visible, si la hay.
    fn find_magpie() -> Option<HWND> {
        top_windows().into_iter().find(|&h| {
            showing(h) && area(h) > 10_000 && {
                let class = class_name(h);
                class.to_ascii_lowercase().contains("magpie")
                    && is_magpie_scaling_window(&class, &pid_of(h).map(exe_name).unwrap_or_default())
            }
        })
    }

    /// Primera ventana «normal» por debajo de `above` en el orden Z (la que Magpie esta escalando cuando el
    /// primer plano es la propia ventana de Magpie).
    fn window_below(above: HWND, skip_pids: &[u32]) -> Option<HWND> {
        let r = frame_rect(above)?;
        let mut h = unsafe { GetWindow(above, GW_HWNDNEXT) }.ok()?;
        for _ in 0..512 {
            if h.is_invalid() {
                return None;
            }
            if showing(h) && area(h) > 10_000 && !pid_of(h).map(|p| skip_pids.contains(&p)).unwrap_or(true) {
                if let Some(o) = frame_rect(h) {
                    if o.left < r.right && o.right > r.left && o.top < r.bottom && o.bottom > r.top {
                        return Some(h);
                    }
                }
            }
            h = unsafe { GetWindow(h, GW_HWNDNEXT) }.ok()?;
        }
        None
    }

    /// Ventana mas grande y visible de los procesos del juego.
    fn game_window(pids: &[u32]) -> Option<HWND> {
        top_windows()
            .into_iter()
            .filter(|&h| pid_of(h).map(|p| pids.contains(&p)).unwrap_or(false) && showing(h))
            .map(|h| (area(h), h))
            .filter(|(a, _)| *a > 10_000)
            .max_by_key(|(a, _)| *a)
            .map(|(_, h)| h)
    }

    fn foreground() -> Option<HWND> {
        let h = unsafe { GetForegroundWindow() };
        (!h.is_invalid()).then_some(h)
    }

    /// `true` si la ventana en primer plano es del propio visor.
    pub fn own_window_focused() -> bool {
        foreground().and_then(pid_of) == Some(std::process::id())
    }

    /* ---------------- estrategias ---------------- */

    /// Va probando metodos y se queda con el primero que da una imagen con contenido. Si todos dan negro,
    /// devuelve la primera imagen negra (la pantalla puede estar de verdad en negro, p. ej. en una carga).
    struct Attempts {
        log: Vec<String>,
        black: Option<(Img, String)>,
    }

    impl Attempts {
        fn try_img(&mut self, method: &str, r: Result<Img, String>) -> Option<(Img, String)> {
            match r {
                Ok(img) if !img.blank() => return Some((img, method.to_string())),
                Ok(img) => {
                    self.log.push(format!("{method}: negra"));
                    if self.black.is_none() {
                        self.black = Some((img, format!("{method} (negra)")));
                    }
                }
                Err(e) => self.log.push(format!("{method}: {e}")),
            }
            None
        }
    }

    /// Ventana por WGC recortada a su zona cliente.
    fn wgc_window(hwnd: HWND) -> Result<Img, String> {
        let img = wgc(Window::from_raw_hwnd(hwnd.0))?;
        Ok(match (frame_rect(hwnd), client_rect(hwnd)) {
            (Some(f), Some(c)) => img.cropped(rect_t(f), rect_t(c)),
            _ => img,
        })
    }

    /// Monitor por WGC (o por GDI si `gdi_fallback`) recortado opcionalmente a `inner`.
    fn monitor_img(m: HMONITOR, inner: Option<RECT>, use_gdi: bool) -> Result<Img, String> {
        let mr = monitor_rect(m).ok_or("monitor desconocido")?;
        let img = if use_gdi { gdi(mr, None)? } else { wgc(Monitor::from_raw_hmonitor(m.0))? };
        Ok(match inner {
            Some(i) => img.cropped(rect_t(mr), rect_t(i)),
            None => img,
        })
    }

    fn print_window(hwnd: HWND) -> Result<Img, String> {
        let wr = window_rect(hwnd).ok_or("ventana sin tamano")?;
        let img = gdi(wr, Some(hwnd))?;
        Ok(match client_rect(hwnd) {
            Some(c) => img.cropped(rect_t(wr), rect_t(c)),
            None => img,
        })
    }

    fn done(a: (Img, String), fg_pid: Option<u32>) -> Grabbed {
        let (img, method) = a;
        Grabbed { width: img.w, height: img.h, rgba: img.px, method, fg_pid }
    }

    /// Ventana en primer plano «util» (ni el visor ni Magpie) y si ocupa su monitor entero. Devuelve el pid
    /// de esa ventana y sus datos; si la ventana es del visor, `own` y pid propio.
    pub fn foreground_info() -> Option<(u32, visor_core::capture::ForegroundInfo)> {
        use visor_core::capture::{covers_monitor, ForegroundInfo};
        let own = std::process::id();
        let fg = foreground()?;
        let fg_pid = pid_of(fg)?;
        if fg_pid == own {
            return Some((own, ForegroundInfo { own: true, ..Default::default() }));
        }
        let magpie = find_magpie();
        let magpie_pid = magpie.and_then(pid_of);
        // Si el primer plano es la ventana de Magpie, el juego es la que esta debajo.
        let (win, via_magpie) = if Some(fg_pid) == magpie_pid {
            (window_below(magpie?, &[own, fg_pid])?, true)
        } else {
            (fg, false)
        };
        let pid = pid_of(win)?;
        let covers = |h: HWND| {
            let mon = unsafe { MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST) };
            match (frame_rect(h), monitor_rect(mon)) {
                (Some(w), Some(m)) => covers_monitor(rect_t(w), rect_t(m)),
                _ => false,
            }
        };
        let fullscreen = covers(win) || (via_magpie && magpie.map(covers).unwrap_or(false));
        Some((pid, ForegroundInfo { own: false, exe_name: exe_name(pid), class: class_name(win), fullscreen, known_game: false }))
    }

    /// Captura el juego. `target_pids`: procesos del juego lanzado desde el visor (vacio = el del primer
    /// plano). `magpie_scaled`: con Magpie activo, guardar lo que se ve escalado en vez de la imagen original.
    pub fn grab(target_pids: &[u32], magpie_scaled: bool) -> Result<Grabbed, String> {
        let own = std::process::id();
        let magpie = find_magpie();
        let magpie_pid = magpie.and_then(pid_of);
        let skip: Vec<u32> = std::iter::once(own).chain(magpie_pid).collect();

        // Ventana en primer plano «util»: ni el visor ni Magpie. Si el primer plano es la ventana de Magpie,
        // la que esta debajo (la que escala).
        let fg = foreground();
        let fg_ok = fg
            .filter(|&h| pid_of(h).map(|p| !skip.contains(&p)).unwrap_or(false))
            .or_else(|| magpie.filter(|_| fg.and_then(pid_of) == magpie_pid).and_then(|m| window_below(m, &skip)));

        // Ventana a capturar: la del primer plano si es el juego (o si no hay juego rastreado); si el usuario
        // esta en otra aplicacion, la ventana del juego; en ultimo caso, la del primer plano o la pantalla.
        let fg_is_game = fg_ok.and_then(pid_of).map(|p| target_pids.contains(&p)).unwrap_or(false);
        let win: Option<HWND> = if target_pids.is_empty() || fg_is_game {
            fg_ok
        } else {
            game_window(target_pids).or(fg_ok)
        };
        let fg_pid = win.and_then(pid_of);
        let mut at = Attempts { log: Vec::new(), black: None };

        // Magpie: su ventana tapa el juego con la imagen escalada.
        if let Some(m) = magpie {
            if magpie_scaled {
                let mut affinity = 0u32;
                let _ = unsafe { GetWindowDisplayAffinity(m, &mut affinity) };
                if affinity == 0 {
                    if let Some(a) = at.try_img("Magpie escalado", wgc_window(m)) {
                        return Ok(done(a, fg_pid));
                    }
                    let mon = unsafe { MonitorFromWindow(m, MONITOR_DEFAULTTONEAREST) };
                    if let Some(a) = at.try_img("Magpie escalado (pantalla)", monitor_img(mon, client_rect(m), false)) {
                        return Ok(done(a, fg_pid));
                    }
                } else {
                    at.log.push("Magpie no deja capturar su ventana: se guarda la imagen original".into());
                }
            }
        }

        if let Some(w) = win {
            if showing(w) {
                let tag = if magpie.is_some() { "ventana original (Magpie)" } else { "ventana" };
                if let Some(a) = at.try_img(tag, wgc_window(w)) {
                    return Ok(done(a, fg_pid));
                }
                // Juegos que no se dejan capturar como ventana (pantalla completa exclusiva, superposiciones):
                // la pantalla recortada a la ventana, si esta delante. Con Magpie delante no sirve (saldria
                // la imagen escalada recortada).
                if Some(w) == fg_ok && magpie.is_none() {
                    let mon = unsafe { MonitorFromWindow(w, MONITOR_DEFAULTTONEAREST) };
                    if let Some(a) = at.try_img("pantalla recortada a la ventana", monitor_img(mon, client_rect(w), false)) {
                        return Ok(done(a, fg_pid));
                    }
                }
                if let Some(a) = at.try_img("PrintWindow", print_window(w)) {
                    return Ok(done(a, fg_pid));
                }
            } else {
                at.log.push("la ventana del juego esta minimizada u oculta".into());
            }
        }

        // Ultimo recurso: el monitor entero (del juego, de Magpie o el principal).
        let anchor = win.filter(|&w| showing(w)).or(magpie);
        let mon = match anchor {
            Some(h) => unsafe { MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST) },
            None => unsafe { MonitorFromPoint(POINT::default(), MONITOR_DEFAULTTOPRIMARY) },
        };
        let crop = magpie.filter(|_| magpie_scaled).and_then(client_rect);
        if let Some(a) = at.try_img("pantalla completa", monitor_img(mon, crop, false)) {
            return Ok(done(a, fg_pid));
        }
        if let Some(a) = at.try_img("pantalla completa (GDI)", monitor_img(mon, crop, true)) {
            return Ok(done(a, fg_pid));
        }
        if let Some(a) = at.black.take() {
            eprintln!("captura: todo salio negro: {}", at.log.join("; "));
            return Ok(done(a, fg_pid));
        }
        Err(format!("ningun metodo funciono ({})", at.log.join("; ")))
    }

    pub fn play_shutter() {
        static WAV: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
        let wav = WAV.get_or_init(shutter_wav);
        unsafe {
            let _ = PlaySoundW(PCWSTR(wav.as_ptr() as *const u16), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT);
        }
    }
}

#[cfg(windows)]
pub use imp::{foreground_info, grab, own_window_focused, play_shutter};

#[cfg(not(windows))]
pub fn grab(_target_pids: &[u32], _magpie_scaled: bool) -> Result<Grabbed, String> {
    Err("La captura propia solo esta disponible en Windows".into())
}
#[cfg(not(windows))]
pub fn play_shutter() {}
#[cfg(not(windows))]
pub fn own_window_focused() -> bool {
    false
}
#[cfg(not(windows))]
pub fn foreground_info() -> Option<(u32, visor_core::capture::ForegroundInfo)> {
    None
}
