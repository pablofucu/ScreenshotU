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
    conf: PathBuf,
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
            return Some(Found { apps_json: apps, service, conf });
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
    o.insert("auto-detach".into(), json!(false));
    o.insert("wait-all".into(), json!(true));
    o.insert("exit-timeout".into(), json!(5));
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

/// Cliente de la API web local de Apollo (`https://127.0.0.1:47990`): login por cookie de sesion, emparejamiento
/// por PIN u OTP y lista de clientes. Ver `docs/referencia/emparejamiento-apollo.md`. Las credenciales viven en el
/// Administrador de credenciales de Windows; nunca se devuelven al frontend ni van a `config.json` o a los registros.
pub mod web {
    use serde::Serialize;
    use serde_json::{json, Value};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    const BASE_PORT_DEFAULT: u16 = 47989;

    // ---------- funciones puras ----------

    /// Un cliente emparejado en Apollo.
    #[derive(Serialize, Debug, Clone, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Client {
        pub uuid: String,
        pub name: String,
        pub perm: Option<u64>,
        pub connected: Option<bool>,
    }

    /// Resultado de `web_check`, para ensenarlo en Ajustes.
    #[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub enum WebState {
        NoCredentials,
        BadCredentials,
        ApolloOff,
        Incompatible,
        Ok,
    }

    /// Error interno: distingue los casos que la interfaz explica de forma distinta.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum ApiErr {
        NoCreds,
        BadCreds,
        Off,
        Incompatible,
        Other(String),
    }

    impl ApiErr {
        fn text(&self) -> String {
            match self {
                ApiErr::NoCreds => "Faltan el usuario y la contraseña de Apollo. Guárdalos en Ajustes.".into(),
                ApiErr::BadCreds => "Apollo no acepta el usuario o la contraseña guardados.".into(),
                ApiErr::Off => "No se puede hablar con Apollo: está apagado o no se encuentra.".into(),
                ApiErr::Incompatible => "Esta versión de Apollo no ofrece la API esperada.".into(),
                ApiErr::Other(m) => m.clone(),
            }
        }
    }

    pub fn login_body(user: &str, pass: &str) -> Value {
        json!({ "username": user, "password": pass })
    }

    pub fn pin_body(pin: &str, name: &str) -> Value {
        json!({ "pin": pin, "name": name })
    }

    pub fn otp_body(passphrase: &str, device_name: &str) -> Value {
        json!({ "passphrase": passphrase, "deviceName": device_name })
    }

    pub fn unpair_body(uuid: &str) -> Value {
        json!({ "uuid": uuid })
    }

    pub fn disconnect_body(uuid: &str) -> Value {
        json!({ "uuid": uuid })
    }

    /// `current_app` de `GET /api/apps`: cadena no vacía o id positivo si hay una app de streaming abierta.
    pub fn parse_current_app(v: &Value) -> Option<String> {
        match &v["current_app"] {
            Value::String(s) if !s.is_empty() => Some(s.clone()),
            Value::Number(n) if n.as_i64().is_some_and(|x| x > 0) => Some(n.to_string()),
            _ => None,
        }
    }

    /// El PIN de Apollo son exactamente 4 digitos.
    pub fn valid_pin(pin: &str) -> bool {
        pin.len() == 4 && pin.bytes().all(|b| b.is_ascii_digit())
    }

    /// Valor de la cookie `auth` a partir de las cabeceras `Set-Cookie`.
    pub fn parse_auth_cookie(set_cookies: &[String]) -> Option<String> {
        set_cookies.iter().find_map(|h| {
            let first = h.split(';').next()?.trim();
            let v = first.strip_prefix("auth=")?;
            (!v.is_empty()).then(|| v.to_string())
        })
    }

    /// `{"status":true}` -> true. Cualquier otra cosa -> false.
    pub fn parse_status(v: &Value) -> bool {
        v["status"].as_bool().unwrap_or(false)
    }

    /// Permisos completos de un cliente de Apollo: ver, listar y lanzar apps (bits 24-26) mas todas las operaciones
    /// (16-20) y entradas (8-12). Es el `perm` del primer cliente emparejado; los demas nacen solo con ver y listar,
    /// y Moonlight dice entonces «Permission denied» (503) al lanzar.
    pub const PERM_ALL: u64 = 0x071F_1F00;

    /// Cuerpo de `/api/clients/update` para un cliente: su ficha tal cual la dio la lista, con `perm` cambiado
    /// (Apollo sustituye la ficha entera, asi que no se pueden mandar solo los campos que cambian).
    pub fn update_body(client: &Value, perm: u64) -> Option<Value> {
        let mut o = client.as_object()?.clone();
        o.get("uuid")?.as_str()?;
        o.insert("perm".into(), json!(perm));
        Some(Value::Object(o))
    }

    pub fn parse_clients(v: &Value) -> Vec<Client> {
        v["named_certs"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|c| {
                        let uuid = c["uuid"].as_str()?.to_string();
                        Some(Client {
                            uuid,
                            name: c["name"].as_str().unwrap_or("").to_string(),
                            perm: c["perm"].as_u64(),
                            connected: c["connected"].as_bool(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clientes a los que conviene pedir desconexión al acabar la partida.
    /// Si se conoce el `uuid` del dispositivo que jugaba, se desconecta solo si sigue conectado (o si Apollo no
    /// informa `connected`); si no se conoce, se desconectan los que aparezcan conectados.
    pub fn disconnect_targets(clients: &[Client], target_uuid: Option<&str>) -> Vec<String> {
        let target = target_uuid.map(str::trim).filter(|s| !s.is_empty());
        if let Some(u) = target {
            return clients.iter().filter(|c| c.uuid == u && c.connected != Some(false)).map(|c| c.uuid.clone()).collect();
        }
        clients.iter().filter(|c| c.connected == Some(true)).map(|c| c.uuid.clone()).collect()
    }

    /// PIN del OTP: Apollo lo devuelve como texto, pero se admite tambien un numero.
    pub fn parse_otp(v: &Value) -> Option<String> {
        if !parse_status(v) {
            return None;
        }
        match &v["otp"] {
            Value::String(s) if !s.is_empty() => Some(s.clone()),
            Value::Number(n) => Some(format!("{:04}", n.as_u64()?)),
            _ => None,
        }
    }

    /// Cliente con ese nombre (el ultimo de la lista si hay varios, que es el mas reciente).
    pub fn client_by_name<'a>(clients: &'a [Client], name: &str) -> Option<&'a Client> {
        clients.iter().rev().find(|c| c.name == name)
    }

    /// Passphrase aleatoria de `len` caracteres alfanumericos a partir de una fuente de u64.
    pub fn passphrase_from(mut next: impl FnMut() -> u64, len: usize) -> String {
        const ABC: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        (0..len).map(|_| ABC[(next() % ABC.len() as u64) as usize] as char).collect()
    }

    fn random_u64() -> u64 {
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        h.finish()
    }

    /// Puerto de la API web: puerto base (`port` de sunshine.conf, 47989 por defecto) + 1.
    pub fn web_port_from_conf(conf: &str) -> u16 {
        let base = conf
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once('=')?;
                if k.trim() == "port" {
                    v.trim().parse::<u16>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(BASE_PORT_DEFAULT);
        base.saturating_add(1)
    }

    fn web_port() -> u16 {
        super::locate()
            .and_then(|f| std::fs::read_to_string(f.conf.join("sunshine.conf")).ok())
            .map(|t| web_port_from_conf(&t))
            .unwrap_or(BASE_PORT_DEFAULT + 1)
    }

    /// Puerto base de streaming de Apollo (47989 por defecto): el que usa Artemis para emparejar y conectar.
    pub fn stream_port() -> u16 {
        web_port().saturating_sub(1)
    }

    // ---------- credenciales ----------

    const CRED_TARGET: &str = "visor-capturas/apollo-web";

    pub fn creds_set(user: &str, pass: &str) -> Result<(), String> {
        if user.trim().is_empty() || pass.is_empty() {
            return Err("Indica el usuario y la contraseña de Apollo.".into());
        }
        cred::write(user.trim(), pass)?;
        *SESSION.lock().unwrap() = None;
        Ok(())
    }

    pub fn creds_clear() -> Result<(), String> {
        *SESSION.lock().unwrap() = None;
        cred::delete()
    }

    pub fn creds_present() -> bool {
        matches!(cred::read(), Ok(Some(_)))
    }

    #[cfg(windows)]
    mod cred {
        use super::CRED_TARGET;
        use windows::core::{PCWSTR, PWSTR};
        use windows::Win32::Foundation::{ERROR_NOT_FOUND, FILETIME};
        use windows::Win32::Security::Credentials::{CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC};

        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        pub fn write(user: &str, pass: &str) -> Result<(), String> {
            let mut target = wide(CRED_TARGET);
            let mut u = wide(user);
            let mut blob = pass.as_bytes().to_vec();
            let cred = CREDENTIALW {
                Flags: CRED_FLAGS(0),
                Type: CRED_TYPE_GENERIC,
                TargetName: PWSTR(target.as_mut_ptr()),
                Comment: PWSTR::null(),
                LastWritten: FILETIME::default(),
                CredentialBlobSize: blob.len() as u32,
                CredentialBlob: blob.as_mut_ptr(),
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                AttributeCount: 0,
                Attributes: std::ptr::null_mut(),
                TargetAlias: PWSTR::null(),
                UserName: PWSTR(u.as_mut_ptr()),
            };
            // SAFETY: `cred` y los buffers a los que apunta viven hasta que vuelve la llamada.
            unsafe { CredWriteW(&cred, 0) }.map_err(|e| format!("No se pudieron guardar las credenciales de Apollo: {e}"))
        }

        pub fn read() -> Result<Option<(String, String)>, String> {
            let target = wide(CRED_TARGET);
            let mut out: *mut CREDENTIALW = std::ptr::null_mut();
            // SAFETY: `target` termina en 0; `out` recibe un bloque que se libera con `CredFree`.
            match unsafe { CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None, &mut out) } {
                Ok(()) => {
                    // SAFETY: tras un Ok, `out` apunta a una CREDENTIALW valida con su blob.
                    let r = unsafe {
                        let c = &*out;
                        let bytes = if c.CredentialBlob.is_null() || c.CredentialBlobSize == 0 { &[][..] } else { std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize) };
                        let pass = String::from_utf8_lossy(bytes).into_owned();
                        let user = if c.UserName.is_null() { String::new() } else { c.UserName.to_string().unwrap_or_default() };
                        CredFree(out as *const _);
                        (user, pass)
                    };
                    Ok(Some(r))
                }
                Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(None),
                Err(e) => Err(format!("No se pudieron leer las credenciales de Apollo: {e}")),
            }
        }

        pub fn delete() -> Result<(), String> {
            let target = wide(CRED_TARGET);
            // SAFETY: `target` termina en 0.
            match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
                Ok(()) => Ok(()),
                Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
                Err(e) => Err(format!("No se pudieron borrar las credenciales de Apollo: {e}")),
            }
        }
    }

    #[cfg(not(windows))]
    mod cred {
        pub fn write(_: &str, _: &str) -> Result<(), String> {
            Err("Solo disponible en Windows".into())
        }
        pub fn read() -> Result<Option<(String, String)>, String> {
            Ok(None)
        }
        pub fn delete() -> Result<(), String> {
            Ok(())
        }
    }

    // ---------- TLS: certificado autofirmado, solo en 127.0.0.1 ----------

    mod tls {
        use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
        use rustls::crypto::{ring, verify_tls12_signature, verify_tls13_signature, CryptoProvider};
        use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
        use rustls::{DigitallySignedStruct, SignatureScheme};
        use std::net::{IpAddr, Ipv4Addr};
        use std::sync::Arc;

        /// Acepta cualquier certificado, pero solo si el servidor es 127.0.0.1; la firma se sigue comprobando.
        #[derive(Debug)]
        struct LoopbackOnly(Arc<CryptoProvider>);

        impl ServerCertVerifier for LoopbackOnly {
            fn verify_server_cert(&self, _: &CertificateDer<'_>, _: &[CertificateDer<'_>], name: &ServerName<'_>, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
                match name {
                    ServerName::IpAddress(ip) if IpAddr::from(*ip) == IpAddr::V4(Ipv4Addr::LOCALHOST) => Ok(ServerCertVerified::assertion()),
                    _ => Err(rustls::Error::General("solo se acepta el certificado de 127.0.0.1".into())),
                }
            }
            fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
                verify_tls12_signature(m, c, d, &self.0.signature_verification_algorithms)
            }
            fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
                verify_tls13_signature(m, c, d, &self.0.signature_verification_algorithms)
            }
            fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
                self.0.signature_verification_algorithms.supported_schemes()
            }
        }

        pub fn agent() -> Result<ureq::Agent, String> {
            let provider = Arc::new(ring::default_provider());
            let cfg = rustls::ClientConfig::builder_with_provider(provider.clone())
                .with_safe_default_protocol_versions()
                .map_err(|e| format!("No se pudo preparar la conexión segura: {e}"))?
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(LoopbackOnly(provider)))
                .with_no_client_auth();
            Ok(ureq::AgentBuilder::new().tls_config(Arc::new(cfg)).timeout(std::time::Duration::from_secs(10)).redirects(0).build())
        }
    }

    // ---------- sesion y peticiones ----------

    /// Cookie `auth` viva (una sola sesion; otro login la invalida y se renueva ante un 401).
    static SESSION: Mutex<Option<String>> = Mutex::new(None);

    fn url(path: &str) -> String {
        format!("https://127.0.0.1:{}{}", web_port(), path)
    }

    fn map_transport(e: ureq::Error) -> ApiErr {
        match e {
            ureq::Error::Status(code, _) => ApiErr::Other(format!("Apollo respondió con el error {code}.")),
            ureq::Error::Transport(t) => match t.kind() {
                ureq::ErrorKind::ConnectionFailed | ureq::ErrorKind::Dns | ureq::ErrorKind::Io => ApiErr::Off,
                _ => ApiErr::Other(format!("No se pudo hablar con Apollo: {t}")),
            },
        }
    }

    fn login(agent: &ureq::Agent) -> Result<String, ApiErr> {
        let (user, pass) = cred::read().map_err(ApiErr::Other)?.ok_or(ApiErr::NoCreds)?;
        match agent.post(&url("/api/login")).send_json(login_body(&user, &pass)) {
            Ok(resp) if resp.status() == 200 => {
                let cookies: Vec<String> = resp.all("set-cookie").iter().map(|s| s.to_string()).collect();
                let c = parse_auth_cookie(&cookies).ok_or(ApiErr::Incompatible)?;
                *SESSION.lock().unwrap() = Some(c.clone());
                Ok(c)
            }
            Ok(_) => Err(ApiErr::Other("Apollo todavía no tiene usuario: créalo primero en su página web.".into())),
            Err(ureq::Error::Status(401, _)) => Err(ApiErr::BadCreds),
            Err(ureq::Error::Status(404, _)) => Err(ApiErr::Incompatible),
            Err(e) => Err(map_transport(e)),
        }
    }

    fn send(agent: &ureq::Agent, method: &str, path: &str, body: &Option<Value>, cookie: &str) -> Result<ureq::Response, ureq::Error> {
        let req = agent.request(method, &url(path)).set("Cookie", &format!("auth={cookie}"));
        match body {
            Some(b) => req.send_json(b.clone()),
            None => req.call(),
        }
    }

    /// Peticion con sesion: reutiliza la cookie y, si Apollo responde 401, vuelve a entrar una vez.
    fn call(method: &str, path: &str, body: Option<Value>) -> Result<Value, ApiErr> {
        let agent = tls::agent().map_err(ApiErr::Other)?;
        let cached = SESSION.lock().unwrap().clone();
        let mut cookie = match cached {
            Some(c) => c,
            None => login(&agent)?,
        };
        let mut retried = false;
        loop {
            match send(&agent, method, path, &body, &cookie) {
                Ok(resp) if resp.status() == 200 => return resp.into_json::<Value>().map_err(|e| ApiErr::Other(format!("Apollo devolvió una respuesta ilegible: {e}"))),
                Ok(resp) if resp.status() == 301 || resp.status() == 302 => {
                    // Redirige a /login o /welcome: sesion caducada o sin usuario.
                    if retried {
                        return Err(ApiErr::BadCreds);
                    }
                    retried = true;
                    cookie = login(&agent)?;
                }
                Ok(resp) => return Err(ApiErr::Other(format!("Apollo respondió con el código {}.", resp.status()))),
                Err(ureq::Error::Status(401, _)) | Err(ureq::Error::Status(403, _)) if !retried => {
                    retried = true;
                    *SESSION.lock().unwrap() = None;
                    cookie = login(&agent)?;
                }
                Err(ureq::Error::Status(401, _)) | Err(ureq::Error::Status(403, _)) => return Err(ApiErr::BadCreds),
                Err(ureq::Error::Status(404, _)) => return Err(ApiErr::Incompatible),
                Err(e) => return Err(map_transport(e)),
            }
        }
    }

    // ---------- API publica ----------

    /// Comprueba credenciales, Apollo y version; para ensenarlo en Ajustes.
    pub fn web_check() -> WebState {
        match call("GET", "/api/clients/list", None) {
            Ok(v) if v.get("named_certs").is_some() => WebState::Ok,
            Ok(_) => WebState::Incompatible,
            Err(ApiErr::NoCreds) => WebState::NoCredentials,
            Err(ApiErr::BadCreds) => WebState::BadCredentials,
            Err(ApiErr::Off) | Err(ApiErr::Other(_)) => WebState::ApolloOff,
            Err(ApiErr::Incompatible) => WebState::Incompatible,
        }
    }

    /// Envia el PIN a Apollo. Necesita un emparejamiento en curso: reintenta cada 500 ms durante 60 s (Moonlight tarda en abrirse y llegar a ese punto).
    pub fn pair_pin(pin: &str, name: &str) -> Result<(), String> {
        if !valid_pin(pin) {
            return Err("El PIN debe tener exactamente 4 cifras.".into());
        }
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            match call("POST", "/api/pin", Some(pin_body(pin, name))) {
                Ok(v) if parse_status(&v) => return Ok(()),
                Ok(_) => {}
                Err(e) => return Err(e.text()),
            }
            if Instant::now() >= deadline {
                return Err("Apollo no recibió ninguna solicitud de emparejamiento a tiempo. Prueba de nuevo.".into());
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// Crea un OTP (valido 3 minutos, un solo uso). Devuelve `(pin, passphrase)`.
    pub fn otp_new(device_name: &str) -> Result<(String, String), String> {
        let passphrase = passphrase_from(random_u64, 12);
        let v = call("POST", "/api/otp", Some(otp_body(&passphrase, device_name))).map_err(|e| e.text())?;
        let pin = parse_otp(&v).ok_or("Apollo no ha creado el código de emparejamiento.")?;
        Ok((pin, passphrase))
    }

    pub fn clients() -> Result<Vec<Client>, String> {
        call("GET", "/api/clients/list", None).map(|v| parse_clients(&v)).map_err(|e| e.text())
    }

    pub fn unpair_client(uuid: &str) -> Result<(), String> {
        let v = call("POST", "/api/clients/unpair", Some(unpair_body(uuid))).map_err(|e| e.text())?;
        if parse_status(&v) {
            Ok(())
        } else {
            Err("Apollo no ha podido quitar el dispositivo.".into())
        }
    }

    /// Da permisos completos al cliente (si no los tiene ya), para que pueda lanzar apps y usar el mando.
    pub fn grant_full(uuid: &str) -> Result<(), String> {
        let v = call("GET", "/api/clients/list", None).map_err(|e| e.text())?;
        let raw = v["named_certs"].as_array().and_then(|a| a.iter().find(|c| c["uuid"].as_str() == Some(uuid))).ok_or("Apollo no conoce ese dispositivo.")?;
        if raw["perm"].as_u64().is_some_and(|p| p & PERM_ALL == PERM_ALL) {
            return Ok(());
        }
        let body = update_body(raw, PERM_ALL).ok_or("Ficha de dispositivo ilegible.")?;
        let r = call("POST", "/api/clients/update", Some(body)).map_err(|e| e.text())?;
        if parse_status(&r) {
            Ok(())
        } else {
            Err("Apollo no ha cambiado los permisos del dispositivo.".into())
        }
    }

    /// uuid del cliente con ese nombre (para guardar el id del recien emparejado).
    pub fn client_uuid_by_name(name: &str) -> Result<Option<String>, String> {
        Ok(client_by_name(&clients()?, name).map(|c| c.uuid.clone()))
    }

    /// Cierra cualquier sesión abierta en Apollo al acabar el juego: primero cierra la app en marcha (si hay una,
    /// p. ej. «Desktop» o «Juego actual» si no cerró sola) y después desconecta al cliente si sigue conectado.
    pub fn close_session(apollo_uuid: Option<&str>) -> Result<(), String> {
        if !creds_present() {
            return Ok(());
        }
        let mut first_err: Option<String> = None;
        match call("GET", "/api/apps", None) {
            Ok(v) if parse_current_app(&v).is_some() => {
                if let Err(e) = call("POST", "/api/apps/close", Some(json!({}))) {
                    first_err = Some(e.text());
                }
            }
            Ok(_) => {}
            Err(ApiErr::NoCreds) => return Ok(()),
            Err(e) => first_err = Some(e.text()),
        }
        match call("GET", "/api/clients/list", None) {
            Ok(v) => {
                for uuid in disconnect_targets(&parse_clients(&v), apollo_uuid) {
                    if let Err(e) = call("POST", "/api/clients/disconnect", Some(disconnect_body(&uuid))) {
                        if first_err.is_none() {
                            first_err = Some(e.text());
                        }
                    }
                }
            }
            Err(ApiErr::NoCreds) => return Ok(()),
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e.text());
                }
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const LIST: &str = r#"{"named_certs":[
            {"name":"PC","uuid":"aaa-1","perm":234881024,"connected":true,"do":[],"undo":[]},
            {"name":"Tablet","uuid":"bbb-2","perm":100663296,"connected":false},
            {"name":"Tablet","uuid":"ccc-3"},
            {"name":"sin uuid"}],"platform":"windows","status":true}"#;

        #[test]
        fn cuerpos() {
            assert_eq!(login_body("u", "p"), json!({"username":"u","password":"p"}));
            assert_eq!(pin_body("1234", "T"), json!({"pin":"1234","name":"T"}));
            assert_eq!(otp_body("abcd", "T"), json!({"passphrase":"abcd","deviceName":"T"}));
            assert_eq!(unpair_body("x"), json!({"uuid":"x"}));
            assert_eq!(disconnect_body("x"), json!({"uuid":"x"}));
        }

        #[test]
        fn update_conserva_la_ficha_y_cambia_perm() {
            let c = json!({"name":"T","uuid":"u1","perm":50331648,"do":[{"cmd":"x"}],"display_mode":"1920x1080x60"});
            let b = update_body(&c, PERM_ALL).unwrap();
            assert_eq!(b["perm"], json!(119480064));
            assert_eq!(b["do"], c["do"]);
            assert_eq!(b["name"], "T");
            assert!(update_body(&json!({"name":"x"}), PERM_ALL).is_none());
        }

        #[test]
        fn pin_valido() {
            assert!(valid_pin("0123"));
            assert!(!valid_pin("123"));
            assert!(!valid_pin("12345"));
            assert!(!valid_pin("12a4"));
        }

        #[test]
        fn cookie() {
            let h = vec!["other=1; Path=/".to_string(), "auth=abc123; Secure; SameSite=Strict; Max-Age=2592000".to_string()];
            assert_eq!(parse_auth_cookie(&h).as_deref(), Some("abc123"));
            assert_eq!(parse_auth_cookie(&["auth=; Max-Age=0".to_string()]), None);
            assert_eq!(parse_auth_cookie(&[]), None);
        }

        #[test]
        fn estado_y_otp() {
            assert!(parse_status(&json!({"status":true})));
            assert!(!parse_status(&json!({"status":false})));
            assert!(!parse_status(&json!({})));
            let ok = json!({"otp":"1234","ip":"1.2.3.4","name":"PC","status":true,"message":"OTP created, effective within 3 minutes."});
            assert_eq!(parse_otp(&ok).as_deref(), Some("1234"));
            assert_eq!(parse_otp(&json!({"otp":42,"status":true})).as_deref(), Some("0042"));
            assert_eq!(parse_otp(&json!({"otp":"1234","status":false})), None);
            assert_eq!(parse_otp(&json!({"status":true})), None);
        }

        #[test]
        fn lista_de_clientes() {
            let v: Value = serde_json::from_str(LIST).unwrap();
            let cs = parse_clients(&v);
            assert_eq!(cs.len(), 3);
            assert_eq!(cs[0], Client { uuid: "aaa-1".into(), name: "PC".into(), perm: Some(234881024), connected: Some(true) });
            assert_eq!(cs[1].connected, Some(false));
            assert_eq!(cs[2].perm, None);
            assert_eq!(cs[2].connected, None);
            assert_eq!(client_by_name(&cs, "Tablet").unwrap().uuid, "ccc-3");
            assert!(client_by_name(&cs, "nadie").is_none());
            assert!(parse_clients(&json!({})).is_empty());
        }

        #[test]
        fn app_y_objetivos_desconexion() {
            assert_eq!(parse_current_app(&json!({})), None);
            assert_eq!(parse_current_app(&json!({"current_app": ""})), None);
            assert_eq!(parse_current_app(&json!({"current_app": 0})), None);
            assert_eq!(parse_current_app(&json!({"current_app": "app-uuid"})).as_deref(), Some("app-uuid"));
            assert_eq!(parse_current_app(&json!({"current_app": 42})).as_deref(), Some("42"));

            let v: Value = serde_json::from_str(LIST).unwrap();
            let cs = parse_clients(&v);
            assert_eq!(disconnect_targets(&cs, Some("aaa-1")), vec!["aaa-1"]);
            assert!(disconnect_targets(&cs, Some("bbb-2")).is_empty());
            assert_eq!(disconnect_targets(&cs, Some("ccc-3")), vec!["ccc-3"]);
            assert!(disconnect_targets(&cs, Some("inexistente")).is_empty());
            assert_eq!(disconnect_targets(&cs, None), vec!["aaa-1"]);
            assert_eq!(disconnect_targets(&cs, Some("")), vec!["aaa-1"]);
        }

        #[test]
        fn passphrase() {
            let mut n = 0u64;
            let p = passphrase_from(
                || {
                    n += 7;
                    n
                },
                12,
            );
            assert_eq!(p.len(), 12);
            assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
            assert_eq!(passphrase_from(random_u64, 12).len(), 12);
        }

        #[test]
        fn puerto_web() {
            assert_eq!(web_port_from_conf(""), 47990);
            assert_eq!(web_port_from_conf("sunshine_name = PC\nport = 48000\n"), 48001);
            assert_eq!(web_port_from_conf("port = nada"), 47990);
        }

        #[test]
        fn estados_serializan() {
            assert_eq!(serde_json::to_string(&WebState::BadCredentials).unwrap(), "\"badCredentials\"");
            assert_eq!(serde_json::to_string(&WebState::Ok).unwrap(), "\"ok\"");
        }
    }
}
