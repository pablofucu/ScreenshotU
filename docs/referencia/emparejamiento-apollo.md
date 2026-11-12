# Emparejamiento automático con Apollo (Moonlight-qt y Artemis)

Investigación del 2026-11-12. Marcas: **[V]** verificado en código fuente o en este PC; **[S]** supuesto o inferido.
Fuentes principales (código en GitHub):
- Apollo: [confighttp.cpp](https://github.com/ClassicOldSong/Apollo/blob/master/src/confighttp.cpp), [nvhttp.cpp](https://github.com/ClassicOldSong/Apollo/blob/master/src/nvhttp.cpp), [pin.html](https://github.com/ClassicOldSong/Apollo/blob/master/src_assets/common/assets/web/pin.html) (última release consultada: v0.4.6; se leyó `master`, que coincide en las rutas).
- Artemis: [moonlight-android, rama `moonlight-noir`](https://github.com/ClassicOldSong/moonlight-android/tree/moonlight-noir) (la rama por defecto; `master` es el Moonlight original sin OTP). Última release v20.2.6.
- Moonlight-qt: [pair.cpp](https://github.com/moonlight-stream/moonlight-qt/blob/master/app/cli/pair.cpp), [CliPair.qml](https://github.com/moonlight-stream/moonlight-qt/blob/master/app/gui/CliPair.qml), [commandlineparser.cpp](https://github.com/moonlight-stream/moonlight-qt/blob/master/app/cli/commandlineparser.cpp). Última release v6.2.0.

## 1. API web de Apollo

Servidor HTTPS autofirmado en el puerto base + 1 = **47990** [V: `PORT_HTTPS = 1` en confighttp.cpp; el HTTP de nvhttp es 47989]. Solo acepta orígenes dentro de `origin_web_ui_allowed` (por defecto LAN); desde la propia máquina (127.0.0.1) siempre vale [V la comprobación `checkIPOrigin`; S que localhost pase el filtro, es lo habitual].

**Autenticación (diferencia importante con Sunshine).** Apollo **no admite Basic auth** [V: ni `Authorization` ni `Basic` en `authenticate()` de Apollo v0.4.6/master]. Usa cookie de sesión:
1. `POST /api/login`, `Content-Type: application/json`, cuerpo `{"username":"..","password":".."}`. Respuesta 200 con `Set-Cookie: auth=<64 chars>; Secure; SameSite=Strict; Max-Age=2592000` (401 si falla).
2. Resto de llamadas con la cabecera `Cookie: auth=<valor>` (en Rust, `reqwest` con `cookie_store(true)` y `danger_accept_invalid_certs(true)`, solo para este host local).
3. Solo hay **una** sesión viva en el servidor: otro login (p. ej. el navegador) invalida la anterior y hay que volver a hacer login ante un 401. Caduca por `SESSION_EXPIRE_DURATION` [V que existe; valor no mirado].
4. Si Apollo aún no tiene usuario, `authenticate` redirige a `/welcome`: hay que crear usuario/contraseña una vez. Las credenciales se guardan en `sunshine_state.json` con hash y sal, así que **no se pueden leer del disco**: la app tiene que pedirlas una vez y guardarlas cifradas (apartado 6). [V en código; el fichero existe en `C:\Program Files\Apollo\config\`, no se ha leído su contenido.]

Sunshine upstream (LizardByte) sí acepta `Authorization: Basic` y además tokens de API; no es lo que usa Apollo [V en sun.cpp master].

**Endpoints** (todos los POST exigen `Content-Type: application/json`):

| Endpoint | Cuerpo | Respuesta |
|---|---|---|
| `POST /api/pin` | `{"pin":"1234","name":"Nombre"}` | `{"status":true}` |
| `GET /api/clients/list` | - | `{"named_certs":[{name, uuid, display_mode, perm, enable_legacy_ordering, allow_client_commands, always_use_virtual_display, do, undo, connected...}], "platform":"windows","status":true}` |
| `POST /api/clients/unpair` | `{"uuid":"<uuid>"}` | `{"status":true}` si lo quitó (y corta su sesión si estaba conectado) |
| `POST /api/clients/unpair-all` | vacío | - |
| `POST /api/clients/update` | `{"uuid","name","display_mode","perm",...}` | `{"status":bool}` |
| `POST /api/clients/disconnect` | (uuid) | - |
| `POST /api/otp` | `{"passphrase":"abcd","deviceName":"Tablet"}` | `{"otp":"1234","ip":"..","name":"<sunshine_name>","status":true,"message":"OTP created, effective within 3 minutes."}` |

Detalles de `/api/pin` [V en `nvhttp::pin`]:
- El PIN son **exactamente 4 dígitos**; si no, devuelve `status:false`.
- Solo funciona si ya hay un emparejamiento **en curso**: el cliente debe haber enviado antes `phrase=getservercert` (la petición queda colgada esperando el PIN). Si no hay ninguno devuelve `{"status":false}`. Por tanto: lanzar primero el cliente y reintentar `POST /api/pin` cada ~500 ms durante unos segundos.
- Usa la primera sesión pendiente del mapa (`std::begin(map_id_sess)`); hay que emparejar de uno en uno.
- `name` pone el nombre del dispositivo en la lista (útil para localizarlo luego). Sunshine upstream es distinto: ahí hay `GET /api/pin` (pendientes) y `pairing_id` en el POST [V sun.cpp]; Apollo no tiene `pairing_id`.

Identificador devuelto: `uuid` generado **por el host** al emparejar (no es el uniqueid del cliente) [V: `uuid_t::generate()` en nvhttp.cpp] y `name`. Para saber cuál es el cliente recién emparejado: comparar la lista antes y después, o buscar por el `name` pasado a `/api/pin`/`deviceName`.

Permisos (README de Apollo y `crypto.h`): el **primer** cliente emparejado recibe todos los permisos; los siguientes solo `view | list` (`_default`). Para que un dispositivo nuevo pueda lanzar apps y usar ratón/teclado hay que subirle `perm` con `/api/clients/update` (`perm` es una máscara: entrada = bits 8-12, operaciones = bits 16-20, `list`=1<<24, `view`=1<<25, `launch`=1<<26 [V en crypto.h]; lo más simple es copiar el `perm` de un cliente completo de `/api/clients/list`). Si no, el usuario verá «Permission Denied».

## 2. OTP en Apollo y en Artemis

**Apollo** [V]: el host genera un OTP de 4 dígitos con `POST /api/otp` (passphrase propia, mínimo 4 caracteres; en el código el comentario dice GET pero la ruta registrada es POST). Es válido **3 minutos**, de **un solo uso**, y solo hay uno activo a la vez (global). El cliente envía en `getservercert` el parámetro `otpauth = SHA256(pin + salt + passphrase)` en hexadecimal mayúsculas; si coincide, Apollo empareja sin ningún paso más. El `deviceName` pasa a ser el nombre del cliente en la lista. Si no hay OTP o caducó devuelve 503 «OTP auth not available».

**Artemis** [V en moonlight-noir]: lo admite y se puede disparar desde otra app.
- `AndroidManifest.xml`: la actividad `preferences.AddComputerManually` está `exported="true"` con un `intent-filter` `ACTION_VIEW` + `DEFAULT` + `BROWSABLE`, esquema `art`. También está en la release v20.2.6.
- URI: `art://<host>:<puerto>?pin=<otp>&passphrase=<passphrase>` (el puerto es el HTTP de nvhttp, 47989; el `name` que añade la web de Apollo no lo lee el cliente). Flujo: `AddComputerManually` añade el PC y, si hay `pin` y `passphrase`, arranca `PcView` con extras `hostname`, `port`, `pin`, `passphrase`; `PcView` espera a que el PC aparezca ONLINE con esa dirección y llama a `doPair` sin teclear nada. Si el PC ya estaba emparejado, `doPair` lo detecta y no hace nada (sin error).
- Pega: `PcView` compara la dirección con `activeAddress.equals(AddressTuple(hostname, port))`; conviene pasar **siempre el puerto explícito** (si falta, `getPort()` devuelve -1) [S sobre el efecto exacto del -1].
- Paquete de Artemis: `com.limelight.noir` (variantes `com.limelight.root.noir`, `com.limelight.noirdebug`); lo que ya busca `StreamingPlugin.java` [V en gradle: `applicationIdSuffix ".noir"`]. Desde Capacitor/Java: `Intent(ACTION_VIEW, Uri.parse(url))` con `setPackage(pkg)` para no abrir un navegador, `FLAG_ACTIVITY_NEW_TASK`. Codificar la passphrase con `Uri.encode`.
- Alternativa sin la tablet tocando nada: la web de Apollo (`/pin#OTP`) genera exactamente este enlace y un QR (`deepLink` en pin.html) [V].

Moonlight-qt y Moonlight Android original **no** tienen OTP: solo PIN.

## 3. Moonlight-qt: `Moonlight.exe pair <host> --pin <pin>`

[V en código] Existe `pair`, con opción `--pin` (4 dígitos; si no son 4 aborta con «PIN must be 4 digits»). Sintaxis: `Moonlight.exe pair <host> --pin 1234`, donde `<host>` es nombre, UUID o IP. Presente desde **v4.2.0** (no existe en v4.1.0). Sin `--pin` genera uno aleatorio y lo muestra.

Matices importantes:
- **No es sin interfaz.** Abre una ventana QML («Pairing... Please enter '1234' on PC») y al terminar muestra un diálogo «Pairing completed successfully» con botón OK; el proceso solo sale al cerrar ese diálogo. Un error también sale en un diálogo.
- **Código de salida**: siempre 0 (se sale con `Qt.quit()` en éxito, error y Escape) [V]. No sirve para detectar el resultado; hay que comprobarlo con `GET /api/clients/list` (aparece el cliente nuevo) o con el `PairStatus` del `serverinfo`.
- Si el host ya está emparejado: diálogo «<nombre> is already paired» [V].
- Si no encuentra el host a tiempo: «Failed to connect to <host>» [V]. Tras el PIN erróneo el error llega del `PairingCompleted` con mensaje de error.
- El PIN lo elegimos nosotros, así que el flujo es: la app del PC principal genera un PIN, lanza `Moonlight.exe pair <ip-del-principal> --pin <pin>` en el PC cliente y llama a `POST /api/pin` en el host con ese PIN (reintentando hasta que Moonlight conecte).
- **En este PC no está instalado** Moonlight.exe: no existe en `Program Files`, `Program Files (x86)`, `%LOCALAPPDATA%\Programs` ni en Descargas (solo «Moonlight Internet Hosting Tool», que es otra cosa). Por tanto no se ha podido ejecutar `--help`. Ruta típica del instalador: `C:\Program Files\Moonlight Game Streaming\Moonlight.exe` [S, ruta típica del instalador oficial]; también existe la versión portable y la de Microsoft Store/winget (`MoonlightGameStreamingProject.Moonlight`).
- Cómo lanzar el cliente desde el PC principal: como el cliente es **otro PC**, la app del cliente (misma app Tauri en modo cliente) es la que ejecuta Moonlight; el host solo envía el PIN.

## 4. Notificación de Windows con Aprobar/Rechazar

`tauri-plugin-notification` **sí admite acciones en escritorio desde 2.6.0** (PR #3671; el proyecto usa `"2"`, la última es 2.8.1) [V changelog y `desktop.rs`]:
- Registrar tipos con `app.notification().register_action_types(...)` (en escritorio solo se usan id y título de cada acción, que pasan a ser los botones), referenciarlos con `.action_type_id(..)` y escuchar con `app.notification().on_action(|a| ...)` en Rust o `onAction` en JS. Sin un manejador registrado antes de mostrar la notificación, los botones **no se muestran**.
- Pegas en Windows: la pulsación solo se notifica mientras el toast está en pantalla; si pasa al Centro de actividades, no llega nada [V en la documentación del código]. Para una aprobación, avisar con un banner persistente (duración larga) y tener la misma decisión disponible dentro de la app (lista de solicitudes pendientes) como respaldo.
- AppUserModelID: el plugin pone el `identifier` de `tauri.conf.json` como AUMID **solo si el exe no está en `...\target\debug` o `...\target\release`**; en desarrollo usa el AUMID de PowerShell, y entonces el toast se atribuye a PowerShell y la activación puede no volver a la app [V]. Con la app instalada (NSIS/MSI crea el acceso directo con AUMID) funciona. Una app no empaquetada y sin acceso directo en el menú Inicio con ese AUMID puede mostrar el toast, pero la activación por clic fiable requiere el acceso directo.
- Alternativa en Rust puro: crate [`tauri-winrt-notification`](https://crates.io/crates/tauri-winrt-notification) 0.8.x (es lo que usa `notify-rust` por debajo): `Toast::new(app_id).title(..).text1(..).add_button("Aprobar","ok").add_button("Rechazar","no").on_activated(|accion| ...).show()`. Mismas condiciones de AUMID; el callback se ejecuta en un hilo del toast y solo mientras el proceso vive. No aporta nada que el plugin no dé ya; solo tendría sentido si el plugin se queda corto.
- Si falla lo anterior, plan simple: ventana propia pequeña (Tauri, siempre visible) con Aprobar/Rechazar; sin dependencia de AUMID.

## 5. Detectar la IP de Tailscale en Windows sin `tailscale.exe`

- Rango **100.64.0.0/10** (CGNAT) en IPv4, y `fd7a:115c:a1e0::/48` en IPv6 [conocimiento general de Tailscale, S no re-verificado en docs].
- En este PC [V]: adaptador de nombre `Tailscale`, descripción `Tailscale Tunnel`, IP 100.124.5.117. `tailscale.exe` existe en `C:\Program Files\Tailscale\` aunque puede no estar en PATH.
- Método recomendado: enumerar adaptadores con `GetAdaptersAddresses` (la app ya usa el crate `windows`; también vale `if-addrs`/`local-ip-address`) y quedarse con la IPv4 que esté en 100.64/10 **y** cuyo alias o descripción contenga «Tailscale». Con solo el rango hay falsos positivos (otras VPN, CGNAT del operador); con solo el nombre falla si el usuario renombra el adaptador. Si no se encuentra, probar `C:\Program Files\Tailscale\tailscale.exe ip -4` como respaldo.

## 6. Guardar una contraseña cifrada por usuario

- **Administrador de credenciales** (`CredWriteW`/`CredReadW`): **ya lo usa el proyecto** en `src-tauri/src/nintendo/secrets.rs` con el crate `windows` (feature `Win32_Security_Credentials` ya activada) [V]. Cifrado por usuario de Windows, visible/borrable por el usuario en el Panel de credenciales, límite de blob 2560 bytes (sobra para una contraseña). `CRED_PERSIST_LOCAL_MACHINE` ya usado.
- **DPAPI** (`CryptProtectData`): cifra con la clave del usuario, pero tú guardas el blob (fichero o base de datos) y necesita la feature `Win32_Security_Cryptography` y gestionar memoria; no aporta nada frente a lo anterior para una sola contraseña.
- Crate `keyring`: envoltorio multiplataforma del Administrador de credenciales; innecesario al existir ya `secrets.rs`.
- **Recomendación:** reutilizar el patrón de `secrets.rs` (un objetivo como `visor-capturas/apollo-web`), sin dependencias nuevas.

## Recomendación para el plan

**PC cliente (Windows, Moonlight-qt)** — flujo con PIN generado por nuestra app:
1. El principal (con Apollo) hace login en `https://127.0.0.1:47990/api/login` con las credenciales guardadas (se piden una sola vez al configurar y se guardan con `secrets.rs`).
2. Genera un PIN de 4 dígitos y se lo manda al cliente por nuestro propio canal (el emparejamiento de dispositivos de la app).
3. El cliente ejecuta `Moonlight.exe pair <ip-principal> --pin <pin>` (ventana visible, hay que dejar que el usuario pulse OK al final; salida siempre 0).
4. El principal llama a `POST /api/pin {"pin","name"}` reintentando cada 500 ms hasta ~15 s (hasta que `status:true`).
5. Confirmar con `GET /api/clients/list` (cliente nuevo, por `name`), y si hace falta `POST /api/clients/update` para darle permisos de lanzar apps/entrada. Desemparejar = `POST /api/clients/unpair` con el `uuid` guardado.
6. Aprobación manual: notificación del plugin con botones Aprobar/Rechazar y respaldo en la UI de la app.
Tailscale: usar la IP 100.x detectada como `<ip-principal>` cuando el cliente no esté en la LAN.

**Tablet (Artemis)** — flujo OTP, **no** hace falta teclear nada:
1. El principal hace `POST /api/otp {"passphrase":"<aleatoria>","deviceName":"<nombre>"}` y guarda `uuid` luego desde `/api/clients/list`.
2. Envía a la tablet `otp`, `passphrase`, IP y puerto 47989.
3. `StreamingPlugin` lanza un `Intent(ACTION_VIEW, art://<ip>:47989?pin=<otp>&passphrase=<pass>)` con `setPackage("com.limelight.noir")` (probar las demás variantes de `MOONLIGHT`). Artemis añade el PC y empareja solo.
4. La ventana de 3 minutos y el OTP único exigen generar el OTP justo antes de lanzar el intent.

**Plan B si Artemis no se automatiza** (versión sin `art://`, Moonlight Android original, o no se resuelve la dirección):
- Mostrar en la tablet el PIN que enseña Artemis y pegarlo en nuestra app, que llama al principal para `POST /api/pin`; o bien mostrar el enlace/QR `art://...` para abrirlo a mano (la web de Apollo en `/pin#OTP` ya lo genera).
- Para Moonlight Android sin OTP: el cliente muestra su PIN y nuestra app lo reenvía al principal; o pegar el PIN en la web de Apollo.
- Verificación común: tras emparejar, `GET /api/clients/list` y comprobar el `name`.

Riesgos a probar en real antes de dar el flujo por bueno: (a) que `127.0.0.1` pase `origin_web_ui_allowed` con la configuración actual de Apollo, (b) que el cierre de sesión del navegador por el login único no moleste, (c) la entrega del puerto en el `art://`, (d) la activación de los botones del toast con la app instalada.
