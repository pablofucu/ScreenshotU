//! Rendimiento para el menu sobre el juego: una muestra de CPU y GPU cada 2 s durante la partida, en un anillo de 2 min
//! (asi al abrir ya hay historial). Con el juego en pausa la GPU baja casi a 0: es real y se deja asi.
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, RefreshKind, System};
use tauri::{AppHandle, Emitter};

const SAMPLES: usize = 60;
const EVERY: Duration = Duration::from_secs(2);

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfState {
    pub cpu_name: String,
    pub cpu_threads: usize,
    pub gpu_name: String,
    /// Uso en % (0..100), de la muestra mas antigua a la mas reciente.
    pub cpu: Vec<f32>,
    pub gpu: Vec<f32>,
    /// VRAM en bytes; `vram_used` es `None` si no hay contador.
    pub vram_used: Option<u64>,
    pub vram_total: Option<u64>,
    /// Temperatura de la GPU (solo NVIDIA).
    pub gpu_temp: Option<u32>,
}

static STATE: Mutex<Option<PerfState>> = Mutex::new(None);
/// Cada partida arranca su hilo; el de la anterior termina al ver otro numero.
static GEN: AtomicU64 = AtomicU64::new(0);

pub fn snapshot() -> Option<PerfState> {
    STATE.lock().unwrap().clone()
}

pub fn stop() {
    GEN.fetch_add(1, Ordering::SeqCst);
    *STATE.lock().unwrap() = None;
}

/// Arranca el muestreo de la partida.
pub fn start(app: AppHandle) {
    let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = std::thread::Builder::new().name("perf".into()).spawn(move || {
        let mut sys = System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_cpu_usage()));
        let (gpu_name, vram_total) = gpu::adapter();
        let temp = gpu::Temperature::new();
        let mut counters = gpu::Counters::new();
        *STATE.lock().unwrap() = Some(PerfState {
            cpu_name: sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
            cpu_threads: sys.cpus().len(),
            gpu_name,
            vram_total,
            ..Default::default()
        });
        while GEN.load(Ordering::SeqCst) == gen {
            std::thread::sleep(EVERY);
            sys.refresh_cpu_usage();
            let sample = counters.as_mut().map(|c| c.sample()).unwrap_or_default();
            let snap = {
                let mut g = STATE.lock().unwrap();
                let Some(s) = g.as_mut() else { return };
                push(&mut s.cpu, sys.global_cpu_usage());
                push(&mut s.gpu, sample.usage.unwrap_or(0.0));
                s.vram_used = sample.vram;
                s.gpu_temp = temp.as_ref().and_then(|t| t.read());
                s.clone()
            };
            if crate::quickmenu::is_open() {
                let _ = app.emit_to("quickmenu", "qm-perf", snap);
            }
        }
    });
}

fn push(a: &mut Vec<f32>, v: f32) {
    if a.len() >= SAMPLES {
        a.remove(0);
    }
    a.push(v.clamp(0.0, 100.0));
}

#[derive(Default)]
struct GpuSample {
    usage: Option<f32>,
    vram: Option<u64>,
}

#[cfg(windows)]
mod gpu {
    use super::GpuSample;
    use std::collections::HashMap;
    use windows::core::w;
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG, DXGI_ADAPTER_FLAG_SOFTWARE};
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW, PDH_CSTATUS_INVALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
    };

    /// Nombre y VRAM de la grafica con mas memoria dedicada (asi no se elige la integrada).
    pub fn adapter() -> (String, Option<u64>) {
        unsafe {
            let Ok(f) = CreateDXGIFactory1::<IDXGIFactory1>() else { return (String::new(), None) };
            let mut best: Option<(String, u64)> = None;
            let mut i = 0;
            while let Ok(a) = f.EnumAdapters1(i) {
                i += 1;
                let Ok(d) = a.GetDesc1() else { continue };
                if DXGI_ADAPTER_FLAG(d.Flags as i32) & DXGI_ADAPTER_FLAG_SOFTWARE != DXGI_ADAPTER_FLAG(0) {
                    continue;
                }
                let mem = d.DedicatedVideoMemory as u64;
                if best.as_ref().map(|b| mem > b.1).unwrap_or(true) {
                    let n = d.Description.iter().position(|c| *c == 0).unwrap_or(d.Description.len());
                    best = Some((String::from_utf16_lossy(&d.Description[..n]).trim().to_string(), mem));
                }
            }
            best.map(|(n, m)| (n, Some(m))).unwrap_or_default()
        }
    }

    /// Contadores de rendimiento de Windows, los del Administrador de tareas. Valen para NVIDIA, AMD e Intel.
    pub struct Counters {
        query: PDH_HQUERY,
        usage: PDH_HCOUNTER,
        vram: PDH_HCOUNTER,
    }

    // Los manejadores de PDH solo se usan desde el hilo que los crea, pero sus tipos son punteros.
    unsafe impl Send for Counters {}

    impl Counters {
        pub fn new() -> Option<Self> {
            unsafe {
                let mut query = PDH_HQUERY::default();
                if PdhOpenQueryW(None, 0, &mut query) != 0 {
                    return None;
                }
                let (mut usage, mut vram) = (PDH_HCOUNTER::default(), PDH_HCOUNTER::default());
                let a = PdhAddEnglishCounterW(query, w!("\\GPU Engine(*engtype_3D)\\Utilization Percentage"), 0, &mut usage);
                let b = PdhAddEnglishCounterW(query, w!("\\GPU Adapter Memory(*)\\Dedicated Usage"), 0, &mut vram);
                if a != 0 && b != 0 {
                    let _ = PdhCloseQuery(query);
                    return None;
                }
                // La utilizacion es un contador de frecuencia: necesita una primera lectura de referencia.
                let _ = PdhCollectQueryData(query);
                Some(Counters { query, usage, vram })
            }
        }

        pub fn sample(&mut self) -> GpuSample {
            unsafe {
                let _ = PdhCollectQueryData(self.query);
            }
            // El motor 3D de cada proceso va aparte: se suma por adaptador y se queda el que mas uso tiene.
            let mut per_adapter: HashMap<String, f64> = HashMap::new();
            let usage_items = read(self.usage);
            let any_usage = usage_items.is_some();
            for (name, v) in usage_items.unwrap_or_default() {
                *per_adapter.entry(luid_of(&name)).or_default() += v;
            }
            let usage = any_usage.then(|| per_adapter.values().cloned().fold(0.0, f64::max).min(100.0) as f32);
            let vram = read(self.vram).map(|items| items.iter().map(|(_, v)| *v).fold(0.0, f64::max) as u64);
            GpuSample { usage, vram }
        }
    }

    impl Drop for Counters {
        fn drop(&mut self) {
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }

    /// «pid_12_luid_0x0_0xABC_phys_0_eng_0_engtype_3D» -> «0x0_0xABC».
    fn luid_of(instance: &str) -> String {
        let Some(i) = instance.find("luid_") else { return String::new() };
        let rest = &instance[i + 5..];
        rest.split("_phys").next().unwrap_or(rest).to_string()
    }

    fn read(counter: PDH_HCOUNTER) -> Option<Vec<(String, f64)>> {
        unsafe {
            let (mut bytes, mut count) = (0u32, 0u32);
            let r = PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None);
            if r != PDH_MORE_DATA || bytes == 0 {
                return None;
            }
            // Alineado para la estructura; los nombres van detras, en el mismo bloque.
            let mut buf = vec![0u64; (bytes as usize).div_ceil(8)];
            let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, Some(items)) != 0 {
                return None;
            }
            let mut out = Vec::with_capacity(count as usize);
            for it in std::slice::from_raw_parts(items, count as usize) {
                // Instancias que aparecen entre dos lecturas vienen sin dato todavia.
                if it.FmtValue.CStatus == 0 || it.FmtValue.CStatus == PDH_CSTATUS_INVALID_DATA {
                    out.push((it.szName.to_string().unwrap_or_default(), it.FmtValue.Anonymous.doubleValue));
                }
            }
            Some(out)
        }
    }

    /// Temperatura con `nvml.dll` (la instala el controlador de NVIDIA). AMD e Intel no tienen API comun sencilla.
    pub struct Temperature {
        lib: libloading::Library,
        device: *mut std::ffi::c_void,
    }

    unsafe impl Send for Temperature {}

    impl Temperature {
        pub fn new() -> Option<Self> {
            unsafe {
                let lib = libloading::Library::new("nvml.dll").ok()?;
                let init: libloading::Symbol<unsafe extern "C" fn() -> i32> = lib.get(b"nvmlInit_v2").ok()?;
                if init() != 0 {
                    return None;
                }
                let by_index: libloading::Symbol<unsafe extern "C" fn(u32, *mut *mut std::ffi::c_void) -> i32> = lib.get(b"nvmlDeviceGetHandleByIndex_v2").ok()?;
                let mut device = std::ptr::null_mut();
                if by_index(0, &mut device) != 0 {
                    return None;
                }
                drop(by_index);
                drop(init);
                Some(Temperature { lib, device })
            }
        }

        pub fn read(&self) -> Option<u32> {
            unsafe {
                let f: libloading::Symbol<unsafe extern "C" fn(*mut std::ffi::c_void, i32, *mut u32) -> i32> = self.lib.get(b"nvmlDeviceGetTemperature").ok()?;
                let mut t = 0u32;
                (f(self.device, 0, &mut t) == 0).then_some(t)
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::luid_of;

        #[test]
        fn luid_from_instance() {
            assert_eq!(luid_of("pid_1234_luid_0x00000000_0x0000C5F2_phys_0_eng_0_engtype_3D"), "0x00000000_0x0000C5F2");
            assert_eq!(luid_of("raro"), "");
        }
    }
}

#[cfg(not(windows))]
mod gpu {
    use super::GpuSample;

    pub fn adapter() -> (String, Option<u64>) {
        (String::new(), None)
    }

    pub struct Counters;
    impl Counters {
        pub fn new() -> Option<Self> {
            None
        }
        pub fn sample(&mut self) -> GpuSample {
            GpuSample::default()
        }
    }

    pub struct Temperature;
    impl Temperature {
        pub fn new() -> Option<Self> {
            None
        }
        pub fn read(&self) -> Option<u32> {
            None
        }
    }
}
