//! Lado cliente de la API de `remote` (un PC que se conecta a otro visor, el «PC principal»): descubrirlo en la red,
//! emparejarse, llamar a `/call` y despertarlo con Wake-on-LAN. Sin estado: lo guardado vive en la app.
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::remote::{local_ipv4s, API_REV, DEFAULT_PORT, DISCOVERY_PORT, DISCOVERY_QUERY};

const TIMEOUT: Duration = Duration::from_secs(4);

/// Un visor que ha contestado al descubrimiento.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub id: String,
    pub name: String,
    /// `ip:puerto` de su API.
    pub addr: String,
}

/// Respuesta de `GET /api/v1/hello`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Hello {
    pub app: String,
    pub api: u32,
    pub api_rev: u32,
    pub api_min: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    /// Direcciones con las que llegar al PC (`ip:puerto`).
    pub addrs: Vec<String>,
}

impl Default for Hello {
    fn default() -> Self {
        Hello { app: String::new(), api: 0, api_rev: 1, api_min: 1, id: String::new(), name: String::new(), version: String::new(), features: vec![], addrs: vec![] }
    }
}

/// Credenciales que da el PC principal al aprobar (`pair/wait`) o con un billete (`pair/ticket`).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Paired {
    pub device_id: String,
    pub token: String,
    pub host_id: String,
    pub host_name: String,
    /// Direcciones del PC principal (`ip:puerto`), de casa primero y Tailscale despues.
    #[serde(default)]
    pub addrs: Vec<String>,
}

/// Lo que devuelve `POST /pair/request`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PairRequested {
    /// Secreto de la solicitud, para esperar o cancelar.
    pub request_id: String,
    /// Numero de 4 cifras que el PC muestra igual.
    pub digits: String,
    /// Milisegundos que le quedan a la solicitud.
    #[serde(default)]
    pub expires_ms: u64,
}

/// Estado de una solicitud segun `pair/wait`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairStatus {
    Pending,
    Approved(Paired),
    Rejected,
    Expired,
}

/// `192.168.1.10`, `http://pc:47811/` ... -> `192.168.1.10:47811`.
pub fn normalize_addr(s: &str) -> String {
    let mut a = s.trim();
    for p in ["http://", "https://"] {
        if let Some(r) = a.strip_prefix(p) {
            a = r;
        }
    }
    let a = a.split('/').next().unwrap_or("").trim();
    if a.is_empty() {
        return String::new();
    }
    // `[::1]:47811`, `host:47811` o sin puerto.
    let has_port = if a.starts_with('[') { a.contains("]:") } else { a.matches(':').count() == 1 };
    if has_port {
        a.to_string()
    } else {
        format!("{a}:{DEFAULT_PORT}")
    }
}

/// Parte de la direccion sin el puerto (para Moonlight).
pub fn addr_host(addr: &str) -> String {
    let a = normalize_addr(addr);
    match a.rfind(':') {
        Some(i) => a[..i].trim_start_matches('[').trim_end_matches(']').to_string(),
        None => a,
    }
}

/// Busca visores en la red local (difusion UDP) durante `wait`.
pub fn discover(wait: Duration) -> Result<Vec<Found>> {
    discover_to(&broadcast_targets(&local_ipv4s()), wait)
}

/// Destinos de la difusion: la general (`255.255.255.255`) y la de cada red privada de este PC (`a.b.c.255`).
/// Windows manda la general por una sola interfaz (la de la ruta por defecto), asi que con Tailscale o adaptadores
/// virtuales puede no llegar a la red de casa; la de cada subred sale por su propio adaptador. Se asume /24, que es
/// lo habitual en redes domesticas; una mascara distinta solo hace que esa difusion no llegue, sin dar error.
pub fn broadcast_targets(ips: &[String]) -> Vec<SocketAddr> {
    let mut out = vec![SocketAddr::from((Ipv4Addr::BROADCAST, DISCOVERY_PORT))];
    for ip in ips {
        let Ok(v4) = ip.parse::<Ipv4Addr>() else { continue };
        if !v4.is_private() {
            continue;
        }
        let [a, b, c, _] = v4.octets();
        let bc = SocketAddr::from((Ipv4Addr::new(a, b, c, 255), DISCOVERY_PORT));
        if !out.contains(&bc) {
            out.push(bc);
        }
    }
    out
}

/// Como `discover`, preguntando a esas direcciones (en las pruebas, al respondedor en loopback).
pub fn discover_to(targets: &[SocketAddr], wait: Duration) -> Result<Vec<Found>> {
    let sock = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).context("abriendo el puerto UDP para buscar")?;
    sock.set_broadcast(true).ok();
    for t in targets {
        // Una red sin difusion no es un error: simplemente no aparece nadie.
        let _ = sock.send_to(DISCOVERY_QUERY, t);
    }
    let end = Instant::now() + wait;
    let mut out: Vec<Found> = vec![];
    let mut buf = [0u8; 1024];
    loop {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        sock.set_read_timeout(Some(left.min(Duration::from_millis(200)))).ok();
        let Ok((n, from)) = sock.recv_from(&mut buf) else { continue };
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(&buf[..n]) else { continue };
        if v["app"] != "visor-capturas" {
            continue;
        }
        let port = v["port"].as_u64().unwrap_or(DEFAULT_PORT as u64);
        let f = Found {
            id: v["id"].as_str().unwrap_or("").into(),
            name: v["name"].as_str().unwrap_or("").into(),
            addr: SocketAddr::new(from.ip(), port as u16).to_string(),
        };
        if !out.iter().any(|o| o.addr == f.addr) {
            out.push(f);
        }
    }
    Ok(out)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(TIMEOUT).build()
}

fn url(addr: &str, path: &str) -> String {
    format!("http://{}{path}", normalize_addr(addr))
}

/// Pasa un error de `ureq` a un mensaje para el usuario.
fn http_err(addr: &str, e: ureq::Error) -> anyhow::Error {
    match e {
        ureq::Error::Status(code, r) => {
            let msg = r.into_json::<serde_json::Value>().ok().and_then(|v| v["error"].as_str().map(String::from)).unwrap_or_default();
            match code {
                401 => anyhow!("{}", if msg.is_empty() { "Este dispositivo no está emparejado con el PC principal".into() } else { msg }),
                426 => anyhow!("El PC principal tiene una versión más nueva: actualiza el visor de este equipo"),
                429 => anyhow!("{}", if msg.is_empty() { "Demasiados intentos. Espera un minuto.".into() } else { msg }),
                _ if !msg.is_empty() => anyhow!("{msg}"),
                _ => anyhow!("El PC principal respondió {code}"),
            }
        }
        ureq::Error::Transport(_) => anyhow::Error::new(Unreachable(addr.to_string())),
    }
}

/// No se pudo hablar con el host en esa direccion (apagado, otra red...), no un error que haya devuelto el host.
#[derive(Debug)]
pub struct Unreachable(pub String);

impl std::fmt::Display for Unreachable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No se pudo conectar con {}. ¿Está encendido y con el visor abierto?", self.0)
    }
}

impl std::error::Error for Unreachable {}

/// Cabeceras que identifican a este cliente.
fn with_headers(r: ureq::Request, version: &str) -> ureq::Request {
    r.set("X-Visor-Api", &API_REV.to_string()).set("X-Visor-Version", version)
}

pub fn hello(addr: &str) -> Result<Hello> {
    let r = agent().get(&url(addr, "/api/v1/hello")).call().map_err(|e| http_err(addr, e))?;
    let h: Hello = r.into_json().context("respuesta de hello")?;
    if h.app != "visor-capturas" {
        bail!("En {addr} no hay un visor de capturas");
    }
    Ok(h)
}

/// `hello` y comprueba que el visor admite el emparejamiento nuevo y que este cliente no es demasiado viejo.
fn hello_pair2(addr: &str) -> Result<Hello> {
    let h = hello(addr)?;
    if !h.features.iter().any(|f| f == "pair2") {
        bail!("El visor de {} es antiguo y no admite este emparejamiento: actualízalo", if h.name.is_empty() { addr } else { &h.name });
    }
    if h.api_min > API_REV {
        bail!("El PC principal tiene una versión más nueva: actualiza el visor de este equipo");
    }
    Ok(h)
}

/// Pide entrar en el PC principal (`POST /pair/request`). El usuario lo aprueba en el PC.
pub fn pair_request(addr: &str, name: &str, kind: &str, version: &str) -> Result<(Hello, PairRequested)> {
    let h = hello_pair2(addr)?;
    let body = serde_json::json!({ "name": name, "kind": kind, "appVersion": version, "api": API_REV });
    let r = with_headers(agent().post(&url(addr, "/api/v1/pair/request")), version).send_json(body).map_err(|e| http_err(addr, e))?;
    Ok((h, r.into_json().context("respuesta de la solicitud")?))
}

/// Espera (long-poll de hasta ~25 s) a que el PC resuelva la solicitud. Llamar de nuevo mientras sea `Pending`.
pub fn pair_wait(addr: &str, request_id: &str, version: &str) -> Result<PairStatus> {
    let a = ureq::AgentBuilder::new().timeout_connect(TIMEOUT).timeout(Duration::from_secs(40)).build();
    let path = format!("/api/v1/pair/wait?id={}", urlenc(request_id));
    let r = with_headers(a.get(&url(addr, &path)), version).call().map_err(|e| http_err(addr, e))?;
    let v: serde_json::Value = r.into_json().context("respuesta de la espera")?;
    Ok(match v["status"].as_str().unwrap_or("") {
        "approved" => PairStatus::Approved(serde_json::from_value(v).context("respuesta de la aprobación")?),
        "rejected" => PairStatus::Rejected,
        "pending" => PairStatus::Pending,
        _ => PairStatus::Expired,
    })
}

/// Retira la solicitud pendiente (`POST /pair/cancel`).
pub fn pair_cancel(addr: &str, request_id: &str, version: &str) -> Result<()> {
    let path = format!("/api/v1/pair/cancel?id={}", urlenc(request_id));
    with_headers(agent().post(&url(addr, &path)), version).send_string("").map_err(|e| http_err(addr, e))?;
    Ok(())
}

/// Se empareja con un billete de un solo uso (`POST /pair/ticket`): da el token directamente.
pub fn pair_with_ticket(addr: &str, ticket: &str, name: &str, kind: &str, version: &str) -> Result<(Hello, Paired)> {
    let h = hello_pair2(addr)?;
    let body = serde_json::json!({ "ticket": ticket, "name": name, "kind": kind, "appVersion": version, "api": API_REV });
    let r = with_headers(agent().post(&url(addr, "/api/v1/pair/ticket")), version).send_json(body).map_err(|e| http_err(addr, e))?;
    Ok((h, r.into_json().context("respuesta del emparejamiento")?))
}

/// Empareja el streaming (`POST /stream/pair`). Con `pin` el PC lo mete en Apollo (`{"ok":true}`); sin `pin` pide un
/// OTP a Apollo y devuelve `{"pin","passphrase","port"}`.
pub fn stream_pair(addr: &str, token: &str, version: &str, pin: Option<&str>) -> Result<serde_json::Value> {
    let a = ureq::AgentBuilder::new().timeout_connect(TIMEOUT).timeout(Duration::from_secs(30)).build();
    let body = match pin {
        Some(p) => serde_json::json!({ "pin": p }),
        None => serde_json::json!({}),
    };
    let r = with_headers(a.post(&url(addr, "/api/v1/stream/pair")), version)
        .set("Authorization", &format!("Bearer {token}"))
        .send_json(body)
        .map_err(|e| http_err(addr, e))?;
    Ok(r.into_json().context("respuesta del emparejamiento del streaming")?)
}

/// Codifica un valor para la query (los `requestId` son hexadecimales, pero no se da nada por supuesto).
fn urlenc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// `POST /call` con el token del dispositivo.
pub fn call(addr: &str, token: &str, version: &str, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value> {
    let r = with_headers(agent().post(&url(addr, "/api/v1/call")), version)
        .set("Authorization", &format!("Bearer {token}"))
        .send_json(serde_json::json!({ "cmd": cmd, "args": args }))
        .map_err(|e| http_err(addr, e))?;
    Ok(r.into_json().context("respuesta del PC principal")?)
}

/// Como `call`, con mas tiempo (subidas grandes, p. ej. el audio de una frase).
pub fn call_long(addr: &str, token: &str, version: &str, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value> {
    let a = ureq::AgentBuilder::new().timeout_connect(TIMEOUT).timeout(Duration::from_secs(120)).build();
    let r = with_headers(a.post(&url(addr, "/api/v1/call")), version)
        .set("Authorization", &format!("Bearer {token}"))
        .send_json(serde_json::json!({ "cmd": cmd, "args": args }))
        .map_err(|e| http_err(addr, e))?;
    Ok(r.into_json().context("respuesta del PC principal")?)
}

/// Deja de estar emparejado (el token deja de valer en el host).
pub fn unpair(addr: &str, token: &str, version: &str) -> Result<()> {
    with_headers(agent().post(&url(addr, "/api/v1/unpair")), version)
        .set("Authorization", &format!("Bearer {token}"))
        .send_string("")
        .map_err(|e| http_err(addr, e))?;
    Ok(())
}

/// Paquete magico de Wake-on-LAN: 6 x `FF` y la MAC 16 veces.
pub fn magic_packet(mac: &str) -> Result<Vec<u8>> {
    let bytes: Vec<u8> = mac
        .split(|c| c == ':' || c == '-')
        .map(|h| u8::from_str_radix(h.trim(), 16))
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| anyhow!("MAC no válida: {mac}"))?;
    if bytes.len() != 6 {
        bail!("MAC no válida: {mac}");
    }
    let mut p = vec![0xFF; 6];
    for _ in 0..16 {
        p.extend_from_slice(&bytes);
    }
    Ok(p)
}

/// Despierta el equipo con esa MAC (difusion al puerto 9; solo en la red local).
pub fn wake(mac: &str) -> Result<()> {
    let p = magic_packet(mac)?;
    let sock = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).context("abriendo el puerto UDP")?;
    sock.set_broadcast(true).context("activando la difusion")?;
    sock.send_to(&p, (Ipv4Addr::BROADCAST, 9)).context("enviando el paquete de encendido")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::{Caller, Discovery, Provider, RemoteServer, Snapshot};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[test]
    fn broadcast_targets_incluye_cada_subred_privada() {
        let ips = vec!["192.168.0.80".to_string(), "100.124.5.117".to_string(), "192.168.0.80".to_string(), "10.1.2.3".to_string()];
        let t = broadcast_targets(&ips);
        let want: Vec<SocketAddr> = vec![
            SocketAddr::from((Ipv4Addr::BROADCAST, DISCOVERY_PORT)),
            SocketAddr::from((Ipv4Addr::new(192, 168, 0, 255), DISCOVERY_PORT)),
            SocketAddr::from((Ipv4Addr::new(10, 1, 2, 255), DISCOVERY_PORT)),
        ];
        assert_eq!(t, want);
    }

    #[test]
    fn broadcast_targets_sin_ips_solo_la_general() {
        assert_eq!(broadcast_targets(&[]), vec![SocketAddr::from((Ipv4Addr::BROADCAST, DISCOVERY_PORT))]);
    }

    #[test]
    fn broadcast_targets_seguro() {
        // Cadenas que no son IPv4 se ignoran sin fallar.
        assert_eq!(broadcast_targets(&["no-es-ip".to_string()]).len(), 1);
    }

    #[test]
    fn addresses() {
        assert_eq!(normalize_addr(" 192.168.1.10 "), "192.168.1.10:47811");
        assert_eq!(normalize_addr("http://pc-salon:5000/api"), "pc-salon:5000");
        assert_eq!(normalize_addr("[::1]"), "[::1]:47811");
        assert_eq!(normalize_addr(""), "");
        assert_eq!(addr_host("192.168.1.10:47811"), "192.168.1.10");
        assert_eq!(addr_host("[::1]:5"), "::1");
    }

    #[test]
    fn magic() {
        let p = magic_packet("aa:bb:cc:dd:ee:0f").unwrap();
        assert_eq!(p.len(), 102);
        assert_eq!(&p[..6], &[0xFF; 6]);
        assert_eq!(&p[96..], &[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x0f]);
        assert!(magic_packet("aa:bb").is_err());
        assert!(magic_packet("zz-bb-cc-dd-ee-ff").is_err());
    }

    #[test]
    fn finds_the_responder() {
        let d = Discovery::start("127.0.0.1:0".parse().unwrap(), "host-1".into(), "SALON".into(), 47900).unwrap();
        let found = discover_to(&[d.local_addr()], Duration::from_millis(600)).unwrap();
        assert_eq!(found, vec![Found { id: "host-1".into(), name: "SALON".into(), addr: "127.0.0.1:47900".into() }]);
    }

    struct Host {
        dir: PathBuf,
        devs: Arc<Mutex<Vec<(String, Caller)>>>,
    }
    impl Provider for Host {
        fn name(&self) -> String {
            "SALON".into()
        }
        fn id(&self) -> String {
            "host-1".into()
        }
        fn snapshot(&self) -> Result<Snapshot> {
            bail!("no hace falta")
        }
        fn thumbs_dir(&self) -> PathBuf {
            self.dir.clone()
        }
        fn previews_dir(&self) -> PathBuf {
            self.dir.clone()
        }
        fn device_by_token(&self, token: &str) -> Option<Caller> {
            self.devs.lock().unwrap().iter().find(|(t, _)| t == token).map(|(_, c)| c.clone())
        }
        fn pair(&self, name: &str, kind: &str, _v: &str, _api: u32) -> Result<(String, String)> {
            let mut d = self.devs.lock().unwrap();
            let n = d.len() + 1;
            d.push((format!("tok{n}"), Caller { device_id: format!("dev{n}"), name: name.into(), kind: kind.into() }));
            Ok((format!("dev{n}"), format!("tok{n}")))
        }
        fn stream_pair(&self, _c: &Caller, pin: Option<&str>) -> Result<serde_json::Value> {
            Ok(match pin {
                Some(p) => serde_json::json!({ "ok": true, "pin": p }),
                None => serde_json::json!({ "pin": "4321", "passphrase": "abc", "port": 47989 }),
            })
        }
        fn unpair(&self, _c: &Caller) -> Result<()> {
            self.devs.lock().unwrap().clear();
            Ok(())
        }
        fn call(&self, caller: &Caller, cmd: &str, _a: serde_json::Value) -> std::result::Result<serde_json::Value, String> {
            match cmd {
                "pc_mac" => Ok(serde_json::json!("aa:bb:cc:dd:ee:ff")),
                "whoami" => Ok(serde_json::json!(caller.name)),
                _ => Err("no".into()),
            }
        }
    }

    fn host() -> (tempfile::TempDir, RemoteServer, String) {
        let d = tempfile::tempdir().unwrap();
        let srv = RemoteServer::start("127.0.0.1:0".parse().unwrap(), Host { dir: d.path().into(), devs: Default::default() }).unwrap();
        let a = srv.local_addr().to_string();
        (d, srv, a)
    }

    /// Espera (poco) a que haya una solicitud pendiente en el servidor.
    fn first_pending(srv: &RemoteServer) -> crate::remote::PairRequest {
        for _ in 0..100 {
            if let Some(r) = srv.pending_requests().into_iter().next() {
                return r;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("no llego la solicitud");
    }

    #[test]
    fn request_approve_call_unpair_against_the_server() {
        let (_d, srv, a) = host();
        let (h, rq) = pair_request(&a, "Portátil", "pc", "1.0").unwrap();
        assert_eq!((h.name.as_str(), h.id.as_str()), ("SALON", "host-1"));
        assert!(!h.addrs.is_empty(), "hello debe llevar addrs");
        assert_eq!(rq.digits.len(), 4);
        let waiter = {
            let (a, id) = (a.clone(), rq.request_id.clone());
            std::thread::spawn(move || pair_wait(&a, &id, "1.0").unwrap())
        };
        let pend = first_pending(&srv);
        assert_eq!((pend.name.as_str(), pend.kind.as_str(), pend.digits.as_str()), ("Portátil", "pc", rq.digits.as_str()));
        srv.approve(&pend.id_public).unwrap();
        let PairStatus::Approved(p) = waiter.join().unwrap() else { panic!("debia estar aprobada") };
        assert_eq!((p.device_id.as_str(), p.token.as_str(), p.host_id.as_str(), p.host_name.as_str()), ("dev1", "tok1", "host-1", "SALON"));
        assert!(!p.addrs.is_empty());
        // se recoge una sola vez
        assert_eq!(pair_wait(&a, &rq.request_id, "1.0").unwrap(), PairStatus::Expired);
        assert_eq!(call(&a, &p.token, "1.0", "whoami", serde_json::json!({})).unwrap(), "Portátil");
        assert_eq!(call(&a, &p.token, "1.0", "pc_mac", serde_json::Value::Null).unwrap(), "aa:bb:cc:dd:ee:ff");
        // streaming: con PIN y sin PIN
        assert_eq!(stream_pair(&a, &p.token, "1.0", Some("1234")).unwrap()["ok"], true);
        let otp = stream_pair(&a, &p.token, "1.0", None).unwrap();
        assert_eq!((otp["pin"].as_str(), otp["passphrase"].as_str(), otp["port"].as_u64()), (Some("4321"), Some("abc"), Some(47989)));
        unpair(&a, &p.token, "1.0").unwrap();
        assert!(call(&a, &p.token, "1.0", "whoami", serde_json::json!({})).is_err());
        let e = hello("127.0.0.1:1").unwrap_err();
        assert!(format!("{e}").contains("No se pudo conectar"), "{e}");
    }

    #[test]
    fn request_rejected_and_cancelled() {
        let (_d, srv, a) = host();
        let (_, rq) = pair_request(&a, "Tablet", "tablet", "1.0").unwrap();
        let pend = first_pending(&srv);
        assert_eq!(pend.kind, "tablet");
        srv.reject(&pend.id_public).unwrap();
        assert_eq!(pair_wait(&a, &rq.request_id, "1.0").unwrap(), PairStatus::Rejected);
        // pendiente: con la espera corta del servidor no se queda colgado y cancelar la borra
        let (_, rq2) = pair_request(&a, "Tablet 2", "tablet", "1.0").unwrap();
        pair_cancel(&a, &rq2.request_id, "1.0").unwrap();
        assert!(srv.pending_requests().is_empty());
        assert_eq!(pair_wait(&a, &rq2.request_id, "1.0").unwrap(), PairStatus::Expired);
    }

    #[test]
    fn ticket_pairs_once_and_wrong_ticket_fails() {
        let (_d, srv, a) = host();
        let t = srv.new_ticket();
        let e = pair_with_ticket(&a, "ZZZZ-ZZZZ", "Portátil", "pc", "1.0").unwrap_err();
        assert!(format!("{e}").contains("billete"), "{e}");
        let (h, p) = pair_with_ticket(&a, &t.code.to_lowercase(), "Portátil", "pc", "1.0").unwrap();
        assert_eq!((h.id.as_str(), p.token.as_str(), p.host_name.as_str()), ("host-1", "tok1", "SALON"));
        assert!(!p.addrs.is_empty());
        assert_eq!(call(&a, &p.token, "1.0", "whoami", serde_json::json!({})).unwrap(), "Portátil");
        assert!(pair_with_ticket(&a, &t.code, "Otro", "pc", "1.0").is_err(), "el billete vale una vez");
    }
}
