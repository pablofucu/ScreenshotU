//! Capa FFI con el host de LunaHook (`LunaHost64.dll` / `LunaHost32.dll`), cargado dinamicamente con `libloading`.
//!
//! ## API usada y fuentes (repositorio https://github.com/HIllya51/LunaTranslator, rama `main`, release v12.1.2.9)
//!
//! * `src/NativeImpl/LunaHook/LunaHost/LunaHostDll.cpp`: exports C (`C_LUNA_API = extern "C" __declspec(dllexport)`,
//!   convencion de llamada por defecto de la plataforma: en x64 solo hay una; en x86 seria cdecl). Exports que se usan:
//!   `Luna_Start`, `Luna_Settings`, `Luna_ConnectProcess`, `Luna_CheckIfNeedInject`, `Luna_DetachProcess`,
//!   `Luna_InsertHookCode`, `Luna_RemoveHook`, `Luna_FindHooks`. (Existen ademas `Luna_SyncThread`, `Luna_SettingsEx`,
//!   `Luna_InsertPCHooks`, `Luna_QueryThreadHistory`, `Luna_Use/CheckIsUsingEmbed`, `Luna_EmbedCallback`, `Luna_ResetLang`,
//!   `Luna_AllocString`; aqui no hacen falta.) Export verificado en la tabla de exportacion de `LunaHost64.dll` del release.
//! * `src/LunaTranslator/textio/textsource/texthook.py`: wrapper ctypes. De ahi salen el orden y los tipos de los callbacks
//!   de `Luna_Start` (10 punteros: Connect(pid), Disconnect(pid), Create(hookcode, hookname, ThreadParam, embedable),
//!   Destroy(hookcode, hookname, ThreadParam), Output(hookcode, hookname, ThreadParam, texto), HostInfo(tipo, texto),
//!   HookInsert(pid, addr, hookcode), Embed(texto, ThreadParam), I18NQuery(texto) -> wchar_t* (o NULL), EmuGameInfo(id, titulo,
//!   version)); la firma de `Luna_Settings(flushDelay, defaultCodepage, maxBufferSize, maxHistorySize, enablePCHooks)`;
//!   `Luna_FindHooks(pid, SearchParam (por valor), callback(hookcode, texto), direcciones)`; y los valores por defecto de
//!   `SearchParam` (patron, longitud, offset, 30 s de busqueda, 100000 registros).
//! * `src/NativeImpl/LunaHook/include/types.h`: `struct ThreadParam { DWORD processId; uint64_t addr, ctx, ctx2; }` (32 bytes,
//!   por valor) y `struct SearchParam` (PATTERN_SIZE = 30, MAX_MODULE_SIZE = 120; los `ALIGNPTR` son uniones con uint64,
//!   asi que el layout es identico en 32 y 64 bits). Enum `HOSTINFO`: Console=0, Warning=1, EmuWarning=2, EmuConnected=3,
//!   EngineType=4.
//! * `src/NativeImpl/LunaHook/LunaHost/host.cpp`: `Luna_ConnectProcess` crea las tuberias con nombre del proceso (hay que
//!   llamarlo UNA vez por pid, antes de inyectar); `Luna_CheckIfNeedInject` devuelve false si ya existe el mutex del hook
//!   (ya inyectado). El host NO inyecta: `ConnectAndInjectProcess` esta bajo `#if 0` en `LunaHostDll.cpp`.
//! * `src/NativeImpl/LunaHook/LunaHost/injector.cpp`: la inyeccion es `LoadLibraryW(<ruta de LunaHook64.dll|LunaHook32.dll>)`
//!   via `CreateRemoteThread`; para un proceso de otra arquitectura usa la direccion de `LoadLibraryW` de su `kernel32`
//!   (WOW64). Aqui se reproduce eso mismo (ver `inject`). LunaTranslator, en cambio, delega en
//!   `LunaSubprocess32/64.exe dllinject <pid..> <dll>` (`src/LunaTranslator/LunaSubProcess.py`); se usa como alternativa si
//!   esta junto a las DLL.
//! * `src/NativeImpl/LunaHook/LunaHost/hookfinder.cc`/`src/LunaTranslator/gui/selecthook.py`: busqueda de hooks por texto
//!   (`SearchParam.text` = texto, 30 caracteres como mucho, minimo 3).
//!
//! ## Carpeta de LunaHook (release `LunaTranslator_x64.zip` -> `LunaTranslator_x64/files/LunaHook/`)
//! `LunaHost64.dll` (host; el visor es de 64 bits), `LunaHook64.dll` (se inyecta en juegos de 64 bits) y `LunaHook32.dll`
//! (juegos de 32 bits). `LunaTmpFontLoader.dll` no hace falta. Las DLL dependen del runtime de VC++ (MSVCP140, VCRUNTIME140,
//! VCRUNTIME140_1). Un host de 64 bits sirve tambien para procesos de 32 bits (la comunicacion es por tuberias y memoria
//! compartida; solo cambia la DLL inyectada).
//!
//! ## Suposiciones (no confirmadas en ejecucion real; no se ha ejecutado nada de lo descargado)
//! * `Luna_Start` se puede llamar una sola vez por proceso y los callbacks se invocan desde hilos del host mientras este
//!   tiene bloqueos internos, por eso aqui solo se encolan eventos y no se llama al host desde ellos.
//! * La inyeccion directa a un proceso de 32 bits desde 64 bits (`kernel32` de SysWOW64 + `CreateRemoteThread`) funciona
//!   como en `injector.cpp` (que usa un llamador YAPI); si falla se prueba `LunaSubprocess32.exe` (su codigo de salida no se
//!   interpreta: se confia en el evento de conexion).
//! * No hay API de cancelacion de busquedas: `th_search_cancel` solo deja de mostrar resultados.
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

/// Parametros de un hilo de texto (`ThreadParam` de LunaHook, 32 bytes, se pasa por valor).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThreadParam {
    pub process_id: u32,
    pub addr: u64,
    pub ctx: u64,
    pub ctx2: u64,
}

/// `SearchParam` de LunaHook (se pasa por valor a `Luna_FindHooks`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SearchParam {
    pub pattern: [u8; 30],
    pub address_method: i32,
    pub search_method: i32,
    pub length: i32,
    pub offset: i32,
    pub search_time: i32,
    pub max_records: i32,
    pub codepage: i32,
    pub padding: i64,
    pub min_address: u64,
    pub max_address: u64,
    pub boundary_module: [u16; 120],
    pub export_module: [u16; 120],
    pub text: [u16; 30],
    pub is_jit_hook: bool,
    pub sharemem_name: [u16; 64],
    pub sharemem_size: u64,
}

impl SearchParam {
    /// Busqueda por texto como la de LunaTranslator (`defaultsp()` + `text`): patron de inicio de funcion, 30 s,
    /// 100000 registros. Con `module` vacio se buscan funciones en todos los modulos (busqueda «profunda»).
    pub fn for_text(text: &str, codepage: u32, is64: bool, module: &str) -> SearchParam {
        let mut sp = SearchParam {
            pattern: [0; 30],
            address_method: 0,
            search_method: 0,
            length: 0,
            offset: 0,
            search_time: 30_000,
            max_records: 100_000,
            codepage: codepage as i32,
            padding: 0,
            min_address: 0,
            max_address: 0,
            boundary_module: [0; 120],
            export_module: [0; 120],
            text: [0; 30],
            is_jit_hook: false,
            sharemem_name: [0; 64],
            sharemem_size: 0,
        };
        let pat: &[u8] = if is64 { &[0xCC, 0xCC, 0x48, 0x89] } else { &[0x55, 0x8B, 0xEC] };
        sp.pattern[..pat.len()].copy_from_slice(pat);
        sp.length = pat.len() as i32;
        sp.offset = if is64 { 2 } else { 0 };
        sp.max_address = if is64 { u64::MAX } else { 0xFFFF_FFFF };
        copy_wide(&mut sp.boundary_module, module);
        copy_wide(&mut sp.text, text);
        sp
    }
}

/// Copia `s` en un buffer UTF-16 fijo dejando siempre el terminador nulo.
fn copy_wide(dst: &mut [u16], s: &str) {
    let max = dst.len().saturating_sub(1);
    for (i, c) in s.encode_utf16().take(max).enumerate() {
        dst[i] = c;
    }
}

/// Evento del host, ya copiado a tipos de Rust (los callbacks solo encolan esto).
#[derive(Debug, Clone)]
pub enum Event {
    Connect(u32),
    Disconnect(u32),
    Create { code: String, name: String, tp: ThreadParam },
    Destroy { tp: ThreadParam },
    Output { code: String, name: String, tp: ThreadParam, text: String },
    /// `HOSTINFO`: 0 consola, 1 aviso, 2 aviso de emulador, 3 emulador conectado, 4 tipo de motor.
    Info(i32, String),
    Found { code: String, text: String },
}

/// Nombre del DLL host segun la arquitectura del visor.
pub fn host_dll_name() -> &'static str {
    if cfg!(target_pointer_width = "64") {
        "LunaHost64.dll"
    } else {
        "LunaHost32.dll"
    }
}

/// Nombre del DLL de hook que se inyecta en un proceso.
pub fn hook_dll_name(is64: bool) -> &'static str {
    if is64 {
        "LunaHook64.dll"
    } else {
        "LunaHook32.dll"
    }
}

/// Busca la carpeta de LunaHook: `<config>/lunahook`, `<carpeta del exe>/lunahook` y `LUNAHOOK_DIR` (en ese orden).
/// En cada una vale tambien la estructura del zip original (`files/LunaHook`) o una subcarpeta `LunaHook`.
pub fn find_dir(config_dir: Option<&Path>) -> Result<PathBuf, String> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(c) = config_dir {
        roots.push(c.join("lunahook"));
    }
    if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("lunahook"))) {
        roots.push(d);
    }
    if let Some(v) = std::env::var_os("LUNAHOOK_DIR").filter(|v| !v.is_empty()) {
        roots.push(PathBuf::from(v));
    }
    let host = host_dll_name();
    for r in &roots {
        for sub in ["", "files/LunaHook", "LunaHook"] {
            let d = if sub.is_empty() { r.clone() } else { r.join(sub) };
            if d.join(host).is_file() {
                return Ok(d);
            }
        }
    }
    let primera = roots.first().map(|p| p.display().to_string()).unwrap_or_else(|| "<carpeta de configuracion>\\lunahook".into());
    Err(format!(
        "No se encuentra {host} de LunaHook. Copia {host}, LunaHook64.dll y LunaHook32.dll (de la carpeta files\\LunaHook de \
         LunaTranslator) en `{primera}`, o junto al ejecutable en `lunahook`, o indica la carpeta con la variable de entorno LUNAHOOK_DIR."
    ))
}

/// Direccion (RVA) de una exportacion por nombre en un PE (PE32 o PE32+). Error si no existe o es un reenvio.
pub fn pe_export_rva(d: &[u8], name: &str) -> Result<u32, String> {
    fn u16at(d: &[u8], o: usize) -> Option<u16> {
        d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32at(d: &[u8], o: usize) -> Option<u32> {
        d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    let malo = || "PE no valido".to_string();
    let pe = u32at(d, 0x3c).ok_or_else(malo)? as usize;
    if d.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err(malo());
    }
    let nsec = u16at(d, pe + 6).ok_or_else(malo)? as usize;
    let opt_size = u16at(d, pe + 20).ok_or_else(malo)? as usize;
    let opt = pe + 24;
    let dd = opt + if u16at(d, opt).ok_or_else(malo)? == 0x10b { 96 } else { 112 };
    let exp_rva = u32at(d, dd).ok_or_else(malo)?;
    let exp_size = u32at(d, dd + 4).ok_or_else(malo)?;
    let sec0 = opt + opt_size;
    let off = |rva: u32| -> Option<usize> {
        for i in 0..nsec {
            let s = sec0 + i * 40;
            let vsize = u32at(d, s + 8)?;
            let va = u32at(d, s + 12)?;
            let rsize = u32at(d, s + 16)?;
            let raw = u32at(d, s + 20)?;
            if rva >= va && rva < va + vsize.max(rsize) {
                return Some((rva - va + raw) as usize);
            }
        }
        None
    };
    if exp_rva == 0 {
        return Err("sin tabla de exportacion".into());
    }
    let e = off(exp_rva).ok_or_else(malo)?;
    let n_names = u32at(d, e + 24).ok_or_else(malo)? as usize;
    let funcs = off(u32at(d, e + 28).ok_or_else(malo)?).ok_or_else(malo)?;
    let names = off(u32at(d, e + 32).ok_or_else(malo)?).ok_or_else(malo)?;
    let ords = off(u32at(d, e + 36).ok_or_else(malo)?).ok_or_else(malo)?;
    for i in 0..n_names {
        let np = off(u32at(d, names + 4 * i).ok_or_else(malo)?).ok_or_else(malo)?;
        let fin = d[np..].iter().position(|&b| b == 0).ok_or_else(malo)?;
        if &d[np..np + fin] == name.as_bytes() {
            let ord = u16at(d, ords + 2 * i).ok_or_else(malo)? as usize;
            let rva = u32at(d, funcs + 4 * ord).ok_or_else(malo)?;
            if rva >= exp_rva && rva < exp_rva + exp_size {
                return Err(format!("{name} es un reenvio"));
            }
            return Ok(rva);
        }
    }
    Err(format!("no se encuentra la exportacion {name}"))
}

#[cfg(windows)]
pub use win::Host;
#[cfg(not(windows))]
pub use other::Host;

#[cfg(not(windows))]
mod other {
    use super::*;

    /// Sin Windows no hay LunaHook.
    pub struct Host;

    const NO_WIN: &str = "LunaHook solo esta disponible en Windows";

    impl Host {
        pub fn load(_dir: &Path, _tx: Sender<Event>) -> Result<Host, String> {
            Err(NO_WIN.into())
        }
        pub fn dir(&self) -> &Path {
            Path::new("")
        }
        pub fn settings(&self, _flush: i32, _codepage: i32, _buf: i32, _hist: i32) {}
        pub fn connect_process(&self, _pid: u32) {}
        pub fn need_inject(&self, _pid: u32) -> bool {
            false
        }
        pub fn detach(&self, _pid: u32) {}
        pub fn insert_code(&self, _pid: u32, _code: &str) {}
        pub fn remove_hook(&self, _pid: u32, _addr: u64) {}
        pub fn find_hooks(&self, _pid: u32, _sp: SearchParam) {}
        pub fn inject(&self, _pid: u32, _is64: bool) -> Result<(), String> {
            Err(NO_WIN.into())
        }
        pub fn process_info(_pid: u32) -> Result<(String, bool), String> {
            Err(NO_WIN.into())
        }
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use libloading::Library;
    use std::ffi::{c_char, c_void, CStr};
    use std::sync::{Mutex, OnceLock};

    type ProcessEvent = extern "C" fn(u32);
    type CreateCb = extern "C" fn(*const u16, *const c_char, ThreadParam, bool);
    type DestroyCb = extern "C" fn(*const u16, *const c_char, ThreadParam);
    type OutputCb = extern "C" fn(*const u16, *const c_char, ThreadParam, *const u16);
    type HostInfoCb = extern "C" fn(i32, *const u16);
    type HookInsertCb = extern "C" fn(u32, u64, *const u16);
    type EmbedCb = extern "C" fn(*const u16, ThreadParam);
    type I18nCb = extern "C" fn(*const u16) -> *mut u16;
    type EmuCb = extern "C" fn(*const u16, *const u16, *const u16);
    type FindCb = extern "C" fn(*const u16, *const u16);

    type StartFn = unsafe extern "C" fn(
        ProcessEvent,
        ProcessEvent,
        CreateCb,
        DestroyCb,
        OutputCb,
        HostInfoCb,
        HookInsertCb,
        EmbedCb,
        I18nCb,
        EmuCb,
    );
    type SettingsFn = unsafe extern "C" fn(i32, i32, i32, i32, bool);
    type PidFn = unsafe extern "C" fn(u32);
    type NeedInjectFn = unsafe extern "C" fn(u32) -> bool;
    type InsertCodeFn = unsafe extern "C" fn(u32, *const u16);
    type RemoveHookFn = unsafe extern "C" fn(u32, u64);
    type FindHooksFn = unsafe extern "C" fn(u32, SearchParam, FindCb, *const u16);

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        fn CloseHandle(h: isize) -> i32;
        fn IsWow64Process(h: isize, wow: *mut i32) -> i32;
        fn QueryFullProcessImageNameW(h: isize, flags: u32, buf: *mut u16, size: *mut u32) -> i32;
        fn VirtualAllocEx(h: isize, addr: *mut c_void, size: usize, ty: u32, prot: u32) -> *mut c_void;
        fn VirtualFreeEx(h: isize, addr: *mut c_void, size: usize, ty: u32) -> i32;
        fn WriteProcessMemory(h: isize, base: *mut c_void, buf: *const c_void, size: usize, written: *mut usize) -> i32;
        #[allow(clippy::too_many_arguments)]
        fn CreateRemoteThread(
            h: isize,
            attrs: *mut c_void,
            stack: usize,
            start: *mut c_void,
            param: *mut c_void,
            flags: u32,
            tid: *mut u32,
        ) -> isize;
        fn WaitForSingleObject(h: isize, ms: u32) -> u32;
        fn GetExitCodeThread(h: isize, code: *mut u32) -> i32;
        fn GetModuleHandleW(name: *const u16) -> isize;
        fn GetProcAddress(h: isize, name: *const u8) -> *mut c_void;
        fn K32EnumProcessModulesEx(h: isize, mods: *mut isize, cb: u32, needed: *mut u32, filter: u32) -> i32;
        fn K32GetModuleBaseNameW(h: isize, module: isize, buf: *mut u16, size: u32) -> u32;
        fn GetLastError() -> u32;
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    // CREATE_THREAD | VM_OPERATION | VM_READ | VM_WRITE | QUERY_INFORMATION
    const PROCESS_INJECT_ACCESS: u32 = 0x0002 | 0x0008 | 0x0010 | 0x0020 | 0x0400;
    const MEM_COMMIT_RESERVE: u32 = 0x3000;
    const MEM_RELEASE: u32 = 0x8000;
    const PAGE_READWRITE: u32 = 0x04;
    const LIST_MODULES_32BIT: u32 = 0x01;
    const ERROR_ACCESS_DENIED: u32 = 5;

    /// Cola hacia el hilo de trabajo; los callbacks `extern "C"` solo escriben aqui.
    static TX: OnceLock<Mutex<Sender<Event>>> = OnceLock::new();

    fn send(ev: Event) {
        if let Some(tx) = TX.get() {
            if let Ok(tx) = tx.lock() {
                let _ = tx.send(ev);
            }
        }
    }

    /// `wchar_t*` terminado en nulo -> `String`.
    unsafe fn from_wide(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    }

    unsafe fn from_c(p: *const c_char) -> String {
        if p.is_null() {
            return String::new();
        }
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    extern "C" fn cb_connect(pid: u32) {
        send(Event::Connect(pid));
    }
    extern "C" fn cb_disconnect(pid: u32) {
        send(Event::Disconnect(pid));
    }
    extern "C" fn cb_create(code: *const u16, name: *const c_char, tp: ThreadParam, _embedable: bool) {
        let (code, name) = unsafe { (from_wide(code), from_c(name)) };
        send(Event::Create { code, name, tp });
    }
    extern "C" fn cb_destroy(_code: *const u16, _name: *const c_char, tp: ThreadParam) {
        send(Event::Destroy { tp });
    }
    extern "C" fn cb_output(code: *const u16, name: *const c_char, tp: ThreadParam, text: *const u16) {
        let (code, name, text) = unsafe { (from_wide(code), from_c(name), from_wide(text)) };
        send(Event::Output { code, name, tp, text });
    }
    extern "C" fn cb_info(kind: i32, text: *const u16) {
        send(Event::Info(kind, unsafe { from_wide(text) }));
    }
    extern "C" fn cb_hook_insert(_pid: u32, _addr: u64, _code: *const u16) {}
    extern "C" fn cb_embed(_text: *const u16, _tp: ThreadParam) {}
    extern "C" fn cb_i18n(_text: *const u16) -> *mut u16 {
        // NULL = sin traduccion (el host usa sus textos por defecto).
        std::ptr::null_mut()
    }
    extern "C" fn cb_emu(_id: *const u16, _title: *const u16, _version: *const u16) {}
    extern "C" fn cb_found(code: *const u16, text: *const u16) {
        let (code, text) = unsafe { (from_wide(code), from_wide(text)) };
        send(Event::Found { code, text });
    }

    /// Host de LunaHook cargado. La DLL no se descarga nunca (los callbacks deben seguir siendo validos).
    pub struct Host {
        dir: PathBuf,
        _lib: Library,
        settings: SettingsFn,
        connect: PidFn,
        need_inject: NeedInjectFn,
        detach: PidFn,
        insert: InsertCodeFn,
        remove: RemoveHookFn,
        find: FindHooksFn,
    }

    // Solo contiene punteros a funciones del host y una `Library`, que son seguros de compartir entre hilos.
    unsafe impl Send for Host {}
    unsafe impl Sync for Host {}

    impl Host {
        /// Carga `LunaHost*.dll` de `dir` y llama a `Luna_Start` con los callbacks que encolan eventos en `tx`.
        pub fn load(dir: &Path, tx: Sender<Event>) -> Result<Host, String> {
            let path = dir.join(host_dll_name());
            unsafe {
                let lib = Library::new(&path).map_err(|e| {
                    format!(
                        "No se pudo cargar {} ({e}). Comprueba que es de la misma arquitectura que el visor y que esta instalado \
                         el runtime de Visual C++ (vc_redist).",
                        path.display()
                    )
                })?;
                macro_rules! sym {
                    ($t:ty, $n:literal) => {
                        *lib.get::<$t>(concat!($n, "\0").as_bytes())
                            .map_err(|e| format!("{} no exporta {} ({e}): version de LunaHook incompatible.", path.display(), $n))?
                    };
                }
                let start: StartFn = sym!(StartFn, "Luna_Start");
                let settings: SettingsFn = sym!(SettingsFn, "Luna_Settings");
                let connect: PidFn = sym!(PidFn, "Luna_ConnectProcess");
                let need_inject: NeedInjectFn = sym!(NeedInjectFn, "Luna_CheckIfNeedInject");
                let detach: PidFn = sym!(PidFn, "Luna_DetachProcess");
                let insert: InsertCodeFn = sym!(InsertCodeFn, "Luna_InsertHookCode");
                let remove: RemoveHookFn = sym!(RemoveHookFn, "Luna_RemoveHook");
                let find: FindHooksFn = sym!(FindHooksFn, "Luna_FindHooks");
                if TX.set(Mutex::new(tx.clone())).is_err() {
                    if let Some(m) = TX.get() {
                        if let Ok(mut g) = m.lock() {
                            *g = tx;
                        }
                    }
                }
                start(cb_connect, cb_disconnect, cb_create, cb_destroy, cb_output, cb_info, cb_hook_insert, cb_embed, cb_i18n, cb_emu);
                Ok(Host { dir: dir.to_path_buf(), _lib: lib, settings, connect, need_inject, detach, insert, remove, find })
            }
        }

        pub fn dir(&self) -> &Path {
            &self.dir
        }

        /// `Luna_Settings`: retardo de volcado (ms), codepage por defecto, buffer maximo, historial maximo. Sin hooks de PC.
        pub fn settings(&self, flush: i32, codepage: i32, buf: i32, hist: i32) {
            unsafe { (self.settings)(flush, codepage, buf, hist, false) }
        }

        pub fn connect_process(&self, pid: u32) {
            unsafe { (self.connect)(pid) }
        }

        pub fn need_inject(&self, pid: u32) -> bool {
            unsafe { (self.need_inject)(pid) }
        }

        pub fn detach(&self, pid: u32) {
            unsafe { (self.detach)(pid) }
        }

        pub fn insert_code(&self, pid: u32, code: &str) {
            let w = wide(code);
            unsafe { (self.insert)(pid, w.as_ptr()) }
        }

        pub fn remove_hook(&self, pid: u32, addr: u64) {
            unsafe { (self.remove)(pid, addr) }
        }

        /// Lanza la busqueda de hooks (asincrona; los resultados llegan como `Event::Found`).
        pub fn find_hooks(&self, pid: u32, sp: SearchParam) {
            // `addresses` solo se usa con search_method == 3.
            unsafe { (self.find)(pid, sp, cb_found, std::ptr::null()) }
        }

        /// Nombre del exe y si el proceso es de 64 bits.
        pub fn process_info(pid: u32) -> Result<(String, bool), String> {
            unsafe {
                let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                if h == 0 {
                    return Err(format!("No se puede abrir el proceso {pid} (error {}). Puede que ya haya terminado.", GetLastError()));
                }
                let mut wow = 0i32;
                let ok = IsWow64Process(h, &mut wow);
                let mut buf = vec![0u16; 1024];
                let mut size = buf.len() as u32;
                let named = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut size);
                CloseHandle(h);
                if ok == 0 {
                    return Err("No se pudo determinar si el proceso es de 32 o 64 bits".into());
                }
                let full = if named != 0 { String::from_utf16_lossy(&buf[..size as usize]) } else { String::new() };
                let exe = full.rsplit(['\\', '/']).next().unwrap_or("").to_string();
                // wow64 = proceso de 32 bits en Windows de 64. (En un visor de 32 bits todo se trata como 32.)
                let is64 = cfg!(target_pointer_width = "64") && wow == 0;
                Ok((exe, is64))
            }
        }

        /// Inyecta `LunaHook64.dll`/`LunaHook32.dll` en el proceso. Reintentable: si el proceso aun no tiene
        /// `kernel32` cargado devuelve un error.
        pub fn inject(&self, pid: u32, is64: bool) -> Result<(), String> {
            let dll = self.dir.join(hook_dll_name(is64));
            if !dll.is_file() {
                return Err(format!(
                    "Falta {} en `{}` (necesaria para juegos de {} bits).",
                    hook_dll_name(is64),
                    self.dir.display(),
                    if is64 { 64 } else { 32 }
                ));
            }
            match inject_direct(pid, is64, &dll) {
                Ok(()) => Ok(()),
                Err(e) if e.starts_with("ACCESO:") => Err(e.trim_start_matches("ACCESO:").to_string()),
                Err(e) => {
                    // Alternativa: el inyector auxiliar de LunaTranslator, si esta junto a las DLL.
                    let exe_name = if is64 { "LunaSubprocess64.exe" } else { "LunaSubprocess32.exe" };
                    let exe = [self.dir.join(exe_name), self.dir.parent().map(|p| p.join(exe_name)).unwrap_or_default()]
                        .into_iter()
                        .find(|p| p.is_file());
                    match exe {
                        Some(exe) => {
                            use std::os::windows::process::CommandExt;
                            std::process::Command::new(exe)
                                .arg("dllinject")
                                .arg(pid.to_string())
                                .arg(&dll)
                                .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
                                .status()
                                .map(|_| ())
                                .map_err(|e2| format!("{e}; ademas fallo {exe_name}: {e2}"))
                        }
                        None => Err(e),
                    }
                }
            }
        }
    }

    /// `LoadLibraryW(dll)` en el proceso mediante `CreateRemoteThread`.
    fn inject_direct(pid: u32, target64: bool, dll: &Path) -> Result<(), String> {
        unsafe {
            let h = OpenProcess(PROCESS_INJECT_ACCESS, 0, pid);
            if h == 0 {
                let e = GetLastError();
                let msg = format!("No se puede abrir el proceso {pid} para inyectar (error {e})");
                return Err(if e == ERROR_ACCESS_DENIED {
                    format!("ACCESO:{msg}: el juego se ejecuta con mas privilegios; abre el visor como administrador.")
                } else {
                    msg
                });
            }
            let r = inject_with_handle(h, target64, dll);
            CloseHandle(h);
            r
        }
    }

    unsafe fn inject_with_handle(h: isize, target64: bool, dll: &Path) -> Result<(), String> {
        let host64 = cfg!(target_pointer_width = "64");
        let load_library: usize = if target64 == host64 {
            let k = GetModuleHandleW(wide("kernel32.dll").as_ptr());
            GetProcAddress(k, b"LoadLibraryW\0".as_ptr()) as usize
        } else {
            // Proceso de 32 bits desde un visor de 64: base de su kernel32 + RVA de LoadLibraryW en SysWOW64.
            let mut mods = vec![0isize; 1024];
            let mut needed = 0u32;
            if K32EnumProcessModulesEx(h, mods.as_mut_ptr(), (mods.len() * std::mem::size_of::<isize>()) as u32, &mut needed, LIST_MODULES_32BIT) == 0 {
                return Err("El proceso aun no esta listo (no se pueden listar sus modulos)".into());
            }
            let n = (needed as usize / std::mem::size_of::<isize>()).min(mods.len());
            let mut base = 0usize;
            for &m in &mods[..n] {
                let mut name = [0u16; 260];
                let len = K32GetModuleBaseNameW(h, m, name.as_mut_ptr(), name.len() as u32) as usize;
                if String::from_utf16_lossy(&name[..len]).eq_ignore_ascii_case("kernel32.dll") {
                    base = m as usize;
                    break;
                }
            }
            if base == 0 {
                return Err("El proceso aun no esta listo (kernel32 sin cargar)".into());
            }
            let windir = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
            let k32 = windir.join("SysWOW64").join("kernel32.dll");
            let bytes = std::fs::read(&k32).map_err(|e| format!("No se pudo leer {}: {e}", k32.display()))?;
            base + pe_export_rva(&bytes, "LoadLibraryW")? as usize
        };
        if load_library == 0 {
            return Err("No se encontro LoadLibraryW".into());
        }
        let w = wide(&dll.to_string_lossy());
        let bytes = w.len() * 2;
        let remote = VirtualAllocEx(h, std::ptr::null_mut(), bytes, MEM_COMMIT_RESERVE, PAGE_READWRITE);
        if remote.is_null() {
            return Err(format!("VirtualAllocEx fallo (error {})", GetLastError()));
        }
        let mut written = 0usize;
        let res = if WriteProcessMemory(h, remote, w.as_ptr() as *const c_void, bytes, &mut written) == 0 {
            Err(format!("WriteProcessMemory fallo (error {})", GetLastError()))
        } else {
            let t = CreateRemoteThread(h, std::ptr::null_mut(), 0, load_library as *mut c_void, remote, 0, std::ptr::null_mut());
            if t == 0 {
                Err(format!("CreateRemoteThread fallo (error {})", GetLastError()))
            } else {
                let w = WaitForSingleObject(t, 15_000);
                let mut code = 0u32;
                let ok = GetExitCodeThread(t, &mut code);
                CloseHandle(t);
                if w != 0 {
                    Err("LoadLibraryW no termino a tiempo en el juego".into())
                } else if ok != 0 && code == 0 {
                    Err("LoadLibraryW devolvio error en el juego (DLL bloqueada por un antivirus o dependencias que faltan)".into())
                } else {
                    Ok(())
                }
            }
        };
        VirtualFreeEx(h, remote, 0, MEM_RELEASE);
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_param_layout() {
        // Layout de C: 30 + 2 (relleno) + 7 enteros + 4 (relleno) + 3 u64 + 3 buffers wchar + bool + nombre + u64.
        assert_eq!(std::mem::size_of::<SearchParam>(), 768);
        assert_eq!(std::mem::size_of::<ThreadParam>(), 32);
    }

    #[test]
    fn search_text_truncado() {
        let sp = SearchParam::for_text(&"あ".repeat(40), 932, true, "game.exe");
        assert_eq!(sp.text[28], 'あ' as u16);
        assert_eq!(sp.text[29], 0);
        assert_eq!(sp.length, 4);
        assert_eq!(sp.boundary_module[0], 'g' as u16);
        assert_eq!(sp.boundary_module[8], 0);
    }

    #[cfg(windows)]
    #[test]
    fn export_de_kernel32_wow64() {
        let p = std::path::Path::new(r"C:\Windows\SysWOW64\kernel32.dll");
        if let Ok(b) = std::fs::read(p) {
            assert!(pe_export_rva(&b, "LoadLibraryW").unwrap() > 0);
        }
    }
}
