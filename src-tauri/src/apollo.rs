//! App «Juego actual» en Apollo/Sunshine: una entrada de `apps.json` que lanza `visor.exe --stream-session` para
//! que la sesion de streaming dure lo que dure el juego (ver `stream.rs`). Escribir en `Program Files` pide
//! permisos de administrador: lo hace el propio visor relanzado con `runas` (`--apollo-write`).
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "Juego actual";
const WRITE_ARG: &str = "--apollo-write";

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApolloStatus {
    pub found: bool,
    pub path: String,
    pub installed: bool,
    pub outdated: bool,
}

struct Found {
    apps_json: PathBuf,
    service: String,
}

fn run(cmd: &str, args: &[&str]) -> String {
    let mut c = std::process::Command::new(cmd);
    c.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c.output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default()
}

/// Nombre del servicio de Windows de Apollo (o Sunshine).
fn service_name() -> Option<String> {
    let out = run("sc", &["query", "type=", "service", "state=", "all"]);
    let names: Vec<String> = out.lines().filter_map(|l| l.trim().strip_prefix("SERVICE_NAME:")).map(|s| s.trim().to_string()).collect();
    let pick = |k: &str| names.iter().find(|n| n.to_lowercase().contains(k)).cloned();
    pick("apollo").or_else(|| pick("sunshine"))
}

/// Carpeta de instalacion: sube desde el ejecutable del servicio hasta una que tenga `config`.
fn service_root(service: &str) -> Option<PathBuf> {
    let out = run("sc", &["qc", service]);
    let line = out.lines().find(|l| l.contains("BINARY_PATH_NAME"))?;
    let raw = line.split_once(':')?.1.trim();
    let exe = if let Some(r) = raw.strip_prefix('"') { r.split('"').next()?.to_string() } else { raw.split(" -").next()?.trim().to_string() };
    let mut p = Path::new(&exe).parent()?.to_path_buf();
    for _ in 0..4 {
        if p.join("config").is_dir() {
            return Some(p);
        }
        p = p.parent()?.to_path_buf();
    }
    None
}

fn locate() -> Option<Found> {
    let service = service_name();
    let mut roots: Vec<PathBuf> = service.as_deref().and_then(service_root).into_iter().collect();
    roots.extend(["C:\\Program Files\\Apollo", "C:\\Program Files\\Sunshine"].map(PathBuf::from));
    for root in roots {
        let conf = root.join("config");
        if !conf.is_dir() {
            continue;
        }
        let from_conf = std::fs::read_to_string(conf.join("sunshine.conf")).ok().and_then(|t| {
            t.lines().find_map(|l| {
                let (k, v) = l.split_once('=')?;
                (k.trim() == "file_apps").then(|| v.trim().to_string())
            })
        });
        let apps = match from_conf.filter(|v| !v.is_empty()) {
            Some(v) if Path::new(&v).is_absolute() => PathBuf::from(v),
            Some(v) => conf.join(v),
            None => conf.join("apps.json"),
        };
        if apps.is_file() {
            let service = service.clone().unwrap_or_else(|| if root.to_string_lossy().to_lowercase().contains("apollo") { "ApolloService".into() } else { "SunshineService".into() });
            return Some(Found { apps_json: apps, service });
        }
    }
    None
}

fn expected_cmd(port: u16) -> String {
    let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
    format!("\"{exe}\" --stream-session --port {port}")
}

fn read_apps(path: &Path) -> Result<Value, String> {
    let t = std::fs::read_to_string(path).map_err(|e| format!("No se pudo leer apps.json: {e}"))?;
    serde_json::from_str(t.trim_start_matches('\u{feff}')).map_err(|e| format!("apps.json no es válido: {e}"))
}

pub fn status(port: u16) -> ApolloStatus {
    let Some(f) = locate() else { return ApolloStatus::default() };
    let mut st = ApolloStatus { found: true, path: f.apps_json.display().to_string(), ..Default::default() };
    if let Ok(v) = read_apps(&f.apps_json) {
        if let Some(app) = v["apps"].as_array().and_then(|a| a.iter().find(|x| x["name"] == APP_NAME)) {
            st.installed = true;
            st.outdated = app["cmd"].as_str() != Some(expected_cmd(port).as_str());
        }
    }
    st
}

/// `true` si Apollo ya tiene «Juego actual» apuntando a este visor y puerto.
pub fn app_installed(port: u16) -> bool {
    let s = status(port);
    s.installed && !s.outdated
}

fn new_uuid() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    let a = h.finish();
    h.write_u64(a);
    let b = h.finish();
    format!("{:08x}-{:04x}-4{:03x}-a{:03x}-{:012x}", (a >> 32) as u32, (a >> 16) as u16, (a as u16) & 0xFFF, (b >> 48) as u16 & 0xFFF, b & 0xFFFF_FFFF_FFFF)
}

/// Crea o actualiza «Juego actual» y reinicia el servicio (pide permisos de administrador).
pub fn install(port: u16) -> Result<ApolloStatus, String> {
    let f = locate().ok_or("No se encontró Apollo ni Sunshine")?;
    let mut v = read_apps(&f.apps_json)?;
    let apps = v["apps"].as_array_mut().ok_or("apps.json no tiene lista de apps")?;
    let mut entry = apps.iter().find(|a| a["name"] == "Desktop").cloned().unwrap_or_else(|| json!({}));
    let had_uuid = apps.iter().any(|a| a.get("uuid").is_some());
    let o = entry.as_object_mut().ok_or("La app «Desktop» de apps.json no es válida")?;
    o.insert("name".into(), json!(APP_NAME));
    o.insert("cmd".into(), json!(expected_cmd(port)));
    o.insert("auto-detach".into(), json!("false"));
    o.insert("wait-all".into(), json!("true"));
    o.insert("exit-timeout".into(), json!("5"));
    o.remove("detached");
    if had_uuid {
        o.insert("uuid".into(), json!(new_uuid()));
    }
    apps.retain(|a| a["name"] != APP_NAME);
    apps.push(entry);
    let tmp = std::env::temp_dir().join("visor-apps.json");
    std::fs::write(&tmp, serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    imp::write_elevated(&f.apps_json, &tmp, &f.service)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(status(port))
}

/// Modo auxiliar `--apollo-write <destino> <temporal> <servicio>` (elevado): copia de seguridad, reemplazo y
/// reinicio del servicio. Devuelve el codigo de salida.
pub fn helper_main() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some(WRITE_ARG) {
        return None;
    }
    let (Some(dest), Some(tmp), Some(service)) = (args.get(2), args.get(3), args.get(4)) else { return Some(2) };
    let dest = PathBuf::from(dest);
    let mut bak = dest.clone().into_os_string();
    bak.push(".bak-visor");
    if !Path::new(&bak).exists() {
        let _ = std::fs::copy(&dest, &bak);
    }
    if std::fs::copy(tmp, &dest).is_err() {
        return Some(3);
    }
    run("sc", &["stop", service]);
    for _ in 0..30 {
        if run("sc", &["query", service]).contains("STOPPED") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    run("sc", &["start", service]);
    Some(0)
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn write_elevated(dest: &Path, tmp: &Path, service: &str) -> Result<(), String> {
        let me = std::env::current_exe().map_err(|e| e.to_string())?;
        let file = wide(&me.to_string_lossy());
        let params = wide(&format!("{} \"{}\" \"{}\" \"{}\"", super::WRITE_ARG, dest.display(), tmp.display(), service));
        let verb = wide("runas");
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(params.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };
        unsafe {
            if let Err(e) = ShellExecuteExW(&mut info) {
                return Err(if e.code() == ERROR_CANCELLED.to_hresult() { "No se dio permiso para modificar Apollo".into() } else { format!("No se pudo modificar Apollo: {e}") });
            }
            if info.hProcess.is_invalid() {
                return Ok(());
            }
            let waited = WaitForSingleObject(info.hProcess, 60_000);
            let mut code = 0u32;
            let _ = GetExitCodeProcess(info.hProcess, &mut code);
            let _ = CloseHandle(info.hProcess);
            if waited != WAIT_OBJECT_0 {
                return Err("Apollo tarda demasiado en reiniciarse".into());
            }
            match code {
                0 => Ok(()),
                c => Err(format!("No se pudo escribir apps.json (código {c})")),
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;
    pub fn write_elevated(_: &Path, _: &Path, _: &str) -> Result<(), String> {
        Err("Solo disponible en Windows".into())
    }
}
