# Plan: varios dispositivos (host + PCs cliente + tablet)

Objetivo: un **PC host** (el de la biblioteca en P: y `visor.db`) es la fuente de verdad. Otros PCs y la tablet
se emparejan con él, ven su biblioteca entera, editan en él y juegan en streaming desde él. Más adelante, los
clientes añaden sus propios juegos instalados, se llevan partidas de viaje y todo acaba también en una nube personal.

**Regla dura (CLAUDE.md):** las categorías (plataformas que salen de las carpetas) no se tocan. Un cliente nunca
crea categorías: al añadir un juego elige una **existente del host**. Instalaciones, dispositivos y sesiones viven
dentro del título, que sigue en su categoría de siempre.

---

## 0. Decisiones de arquitectura

1. **El cliente PC es la misma app en «modo cliente»**, con la misma interfaz que la tablet: `src/lib/remote/core.ts`
   ya sustituye `invoke` por `POST /api/v1/call` con caché en IndexedDB. En el PC cliente se monta un *router*
   (`src/lib/ipc.ts`): los comandos de ventana/atajos/diálogos y los nuevos `client_*` van a Tauri local; el resto,
   al host. Nada de duplicar pantallas.
   - *Descartado:* que el cliente abra `P:\...\visor.db` por red (SMB). SQLite en red corrompe la base. Hay que
     impedirlo activamente (ver riesgos).
   - *Descartado:* hacer el proxy en Rust (cada comando de Tauri reenviando al host). Obliga a tocar todos los
     comandos; el JS ya resuelve esto en la tablet.
2. **Identificador global del juego = el `title_id` actual del host** (`steam:123`, `manual:<hex>`,
   `plataforma/carpeta`; `visor-core/src/userdata.rs:10`). Ya es estable dentro del host y todas las tablas lo usan.
   El cliente nunca inventa ids: si un juego no existe, se lo pide crear al host.
   - *Descartado:* un UUID nuevo por juego. Habría que migrar todas las tablas por título (prefs, estados,
     colecciones, logros, frases…) y no aporta nada mientras el host sea la fuente de verdad.
3. **Juego vs instalación:** la fila `games` de cada base es **la instalación de ese dispositivo** (ruta, exe,
   workdir); así el lanzador no cambia. El host guarda además una tabla `installs` con las instalaciones que le
   comunican los clientes (solo metadatos, fase 2).
4. **Dispositivo:** id aleatorio (`remote::new_id`), nombre puesto por el usuario y token propio. En las bases,
   `device_id = ''` significa «el dueño de esta base» (en el host, el host). Así las filas antiguas no hay que
   reescribirlas y una copia de seguridad restaurada sigue siendo correcta.
5. **Protocolo:** se mantienen las rutas `/api/v1/*` (la APK actual sigue funcionando). `API_VERSION` pasa a 2,
   `hello` añade `apiMin` (1) y los clientes mandan `X-Visor-Api` y `X-Visor-Version`. Cada novedad se anuncia en
   `features`.
6. **Pensado para la nube desde ya:** toda fila que viaja entre equipos lleva `uid` global + `device_id`, y todo
   fichero se identifica por SHA-256 (las capturas ya lo tienen: `catalog.rs`, columna `shots.sha256`). La nube será
   «otro sitio donde está el fichero» (sección 7).

---

## Fase 1 (ahora): emparejar, ver/editar el host y jugar en streaming desde otro PC

### 1A. Esquema: migración 16

`src-tauri/visor-core/src/db.rs:280`: añadir **al final** de `MIGRATIONS` (no editar las publicadas):

```sql
-- 16: dispositivos emparejados (solo se usa en el host) y dispositivo de cada partida.
CREATE TABLE devices (
    id           TEXT PRIMARY KEY,          -- remote::new_id()
    name         TEXT NOT NULL,             -- «Portátil», «Tablet»
    kind         TEXT NOT NULL,             -- 'pc' | 'tablet'
    token_sha256 TEXT NOT NULL UNIQUE,      -- nunca se guarda el token en claro
    created_ms   INTEGER NOT NULL,
    last_seen_ms INTEGER NOT NULL DEFAULT 0,
    app_version  TEXT NOT NULL DEFAULT '',
    api          INTEGER NOT NULL DEFAULT 0,
    revoked      INTEGER NOT NULL DEFAULT 0
);
ALTER TABLE sessions ADD COLUMN device_id TEXT NOT NULL DEFAULT '';   -- '' = el dueño de la base
ALTER TABLE sessions ADD COLUMN uid TEXT;                             -- id global (subidas idempotentes, nube)
ALTER TABLE sessions ADD COLUMN synced_ms INTEGER;                    -- cliente: cuándo se subió al host
CREATE UNIQUE INDEX sessions_uid ON sessions(uid) WHERE uid IS NOT NULL;
CREATE INDEX sessions_device ON sessions(device_id, start_ms);
ALTER TABLE read_sessions ADD COLUMN device_id TEXT NOT NULL DEFAULT '';
ALTER TABLE read_sessions ADD COLUMN uid TEXT;
```

- Mejor como módulo nuevo `visor-core/src/devices.rs` (`pub const SCHEMA` + funciones `create`, `by_token_hash`,
  `list`, `rename`, `revoke`, `touch`), igual que `catalog`/`saves`. `touch` escribe `last_seen_ms` como mucho una
  vez por minuto.
- `db.rs:461` (test de tablas): añadir `devices`. Test nuevo: migrar desde `MIGRATIONS[..15]` con partidas → siguen,
  con `device_id = ''`.

### 1B. Sesiones con dispositivo

- `visor-core/src/db/data.rs:279` `stat_add_session`: nuevos parámetros `device_id: &str` y `uid: Option<&str>`
  (si es `None` se genera con `remote::new_id()`).
  - La fusión de partidas (`data.rs:284`) debe buscar la última **del mismo dispositivo**:
    `WHERE game_id = ?1 AND device_id = ?2`. Si no, una partida en streaming se fundiría con una del host.
  - `INSERT` en `data.rs:312` con `device_id` y `uid`; con `ON CONFLICT(uid) DO NOTHING` y sin sumar a
    `play_stats` si no se insertó (deja la fase 2 lista).
  - Actualizar las llamadas de los tests (`db.rs:513` y las que salgan con grep `stat_add_session`).
- `visor-core/src/jp/readstats.rs:342` `session_start`: parámetro `device_id`; insertar también `uid`.
- `src-tauri/src/state.rs:15` `Active`: campo `device: String` ('' = jugado en el host).
- `src-tauri/src/launcher.rs:91` `start_at(app, game, screen, device: String)` → lo guarda en `Active` (`:131`).
  `start` (`:86`) pasa `String::new()`.
- `launcher.rs:396`: leer `device` de `st.active` **antes** de vaciarlo (`:399`) y pasarlo a `stat_add_session`.
- `src-tauri/src/novel.rs:190` `readstats::start`: pasar el dispositivo de `Active`.
- Las capturas en streaming las hace el host como hoy; no cambian.

### 1C. Servidor: quién llama (dispositivo) y emparejamiento

`src-tauri/visor-core/src/remote.rs`:

- Tipo nuevo `pub struct Caller { pub device_id: String, pub name: String, pub kind: String }`.
- `Provider` (`:105`): métodos nuevos `fn device_by_token(&self, token: &str) -> Option<Caller>` y
  `fn pair(&self, name: &str, kind: &str, version: &str, api: u32) -> Result<(String /*id*/, String /*token*/)>`;
  `launch` y `call` reciben `caller: &Caller` como primer argumento.
- `authorized` (`:608`) pasa a devolver `Option<Caller>`:
  1. si el valor coincide con el código de emparejamiento actual (`code_matches`, como hoy) →
     `Caller { device_id: "legacy-tablet", name: "Tablet", kind: "tablet" }` (compatibilidad con la APK actual);
  2. si no, `provider.device_by_token(token)` (SHA-256 del token, `revoked = 0`).
  Guardar `X-Visor-Version`/`X-Visor-Api` en `touch`.
- `route` (`:627`):
  - `hello` (`:639`): `api: 2`, `apiMin: 1`, features `+ "pair", "devices"`.
  - Nuevo `POST /api/v1/pair` (sin cabecera; el código va en el cuerpo, mismo `throttle`): cuerpo
    `{ code, name, kind, appVersion, api }` → `{ deviceId, token, hostId, hostName, api }`. Token: 32 bytes de
    `getrandom` en hex.
  - Nuevo `POST /api/v1/unpair` (con token): se revoca a sí mismo.
  - Si el cliente manda `X-Visor-Api` < `apiMin` → `426` «Actualiza el visor de este equipo».
  - `launch` y `call` pasan el `Caller`.
- `Playing` (`:74`): campo `device` (nombre) para que todos vean «jugando en Portátil».
- `remote::host_name()` como nombre por defecto; el host tiene nombre propio editable (`config.device_name`,
  `visor-core/src/config.rs:144` junto a `remote_*`). `AppProvider::name` (`src-tauri/src/remote.rs:20`) lo usa.
- `visor-core/examples/serve.rs` y el proveedor de los tests de `remote.rs` (`:1300`–`:1450`): adaptar firmas.
- Tests: emparejar con código bueno/malo, token válido, token revocado → 401, el código antiguo sigue entrando,
  `426` con api vieja.

`src-tauri/src/remote.rs` (`AppProvider`, `:19`): implementar `device_by_token`/`pair` con `visor_core::devices`
sobre `st.db`; `launch` (`:51`) recibe el `Caller` y lo ignora (lanzar «en el host» = `device ''`).

`src-tauri/src/remote_call.rs`:
- `dispatch(app, caller, cmd, args)` (`:50`).
- `launch_stream` (`:76`–`:87`): `crate::stream::launch(app, game, caller.device_id.clone())`.
- `src-tauri/src/stream.rs:225` `launch(app, game, device)`: guardar `device` en la sesión junto a `pending`;
  `launch_now` (`:297`) lo pasa a `launcher::start_at` (`:315`). Así la tablet, con la misma llamada, ya deja sus
  partidas con su id.
- Comandos de gestión **solo del escritorio del host** (no se añaden a `dispatch`): `devices_list`,
  `device_rename`, `device_revoke`, `set_device_name`. Registrar en `src-tauri/src/lib.rs:1312`.

### 1D. Cliente PC (Rust, solo local)

Módulo nuevo `src-tauri/src/client.rs`, datos en `<config_dir>/client.json` (local, **no** en `Config`, para que
`save_config` contra el host no lo pise ni lo lleve allí):
`{ hostId, hostName, hostAddrs: ["192.168.1.10:47811"], hostMac, deviceId, deviceName, token, moonlightPath }`.

Comandos (registrar en `lib.rs:1312`, nunca en `remote_call.rs`):
- `client_info()` → modo (`host` | `client`) y lo de arriba sin el token.
- `client_discover()` → envía `remote::DISCOVERY_QUERY` por difusión UDP al `DISCOVERY_PORT` (`remote.rs:956`) y
  junta respuestas 1,5 s. Función nueva `remote::discover(timeout)` en visor-core (el respondedor ya existe, `:968`).
- `client_pair(addr, code, deviceName)` → `GET hello` + `POST /pair` con `ureq` (ya es dependencia,
  `src-tauri/Cargo.toml:28`); guarda `client.json`, pide `pc_mac` al host y lo guarda.
- `client_unpair()` → `POST /unpair` y borra `client.json` (vuelve a modo host/sin biblioteca).
- `client_open_stream(app)` → lanza `Moonlight.exe stream <ip-host> "<app>"` (app = la que devuelve
  `launch_stream`, «Juego actual»). Autodetectar `%ProgramFiles%\Moonlight Game Streaming\Moonlight.exe`.
- `client_wake()` → paquete mágico (UDP 9, difusión) a `hostMac`.
- En modo cliente el arranque no exige biblioteca: `library::resolve` (`src-tauri/src/library.rs:13`) se queda en
  `LibMode::Legacy` (base local en la carpeta de configuración). **Bloquear** elegir como biblioteca una carpeta
  cuyo `visor.db` tenga `meta.host_id` distinto del propio (ver riesgos): el host escribe `meta.host_id` al
  arrancar.

### 1E. Cliente PC (interfaz)

- `src/lib/ipc.ts` (nuevo): exporta `invoke`, `convertFileSrc`, `listen`, `once`.
  - Modo host (o sin `client.json`): reexporta Tauri tal cual.
  - Modo cliente: `LOCAL` = comandos de ventana y PC (`window_monitor`, `next_monitor`, `exit_app`, `close_main`,
    `hotkey_suspend`, `learn_key`, `learn_pad`, `quickmenu_*`, `client_*`, `open_folder`, `reveal_in_explorer`) →
    Tauri; el resto → `remote/core.ts`. `convertFileSrc` → el de `remote/core.ts:190` (rutas del host).
    `listen` → los dos (local + `remote/event.ts`).
  - El modo se decide una vez al arrancar (`client_info`) **antes** de `initBackend` (`src/lib/backend.ts:248`) y se
    fija el `endpoint` con `setEndpointProvider` (`src/lib/remote/config.ts:33`) con la dirección y el token.
- Cambiar los imports de `@tauri-apps/api/core` y `@tauri-apps/api/event` por `ipc` en: `src/lib/backend.ts`
  (`:98`–`:99`), `controller.svelte.ts`, `saves.svelte.ts`, `reading.svelte.ts`, `library.svelte.ts`,
  `phrases.svelte.ts`, `sgdb.svelte.ts`, `nintendo.svelte.ts`, `achievements.svelte.ts`, `src/quickmenu/data.ts`,
  `src/hook/api.ts`. (No `remote/*`). Son ~12 cambios de una línea: no merece `agy`.
- `src/lib/remote/core.ts`:
  - Mandar `X-Visor-Api` y `X-Visor-Version` en `http` (`:69`).
  - `setOfflineWrites(false)` nuevo: en modo cliente PC, sin conexión las escrituras fallan con
    «Sin conexión con <host>: solo lectura» en vez de ir al buzón (`OUTBOX`, `:20`; `offline`, `:138`). La tablet
    sigue como está.
  - `setAfterLaunch` (`:110`): en el cliente PC → `client_open_stream` con el `app` de la respuesta.
  - `401` → mensaje «Este equipo ya no está emparejado» y ofrecer emparejar de nuevo.
- Estado de conexión: reutilizar `remoteStatus` (`src/lib/remote/status.svelte.ts`) y el botón de despertar de
  `src/components/TopBar.svelte` (hoy usa `__VISOR_WAKE__` de la tablet; en el PC → `client_wake`).
- Ajustes (filas por id, como `src/lib/remote.svelte.ts:8`):
  - **Host → «Dispositivos»**: nombre de este equipo, lista (nombre, tipo, última conexión, versión) con Renombrar
    y Quitar. La fila del código (`remotecode`) se queda: es el que se teclea para emparejar.
  - **Cliente → «Este equipo»**: «Conectar a un PC principal» → lista de `client_discover` (o dirección a mano) →
    código → nombre del dispositivo → emparejar → recargar la ventana. Ruta de Moonlight + «Probar». Desconectar.
- Ficha del juego en el cliente: «Jugar» = streaming (fase 1 no hay instalaciones locales). Si el host responde
  409 (ocupado), enseñar el motivo tal cual («Ya hay un juego en marcha: X»).

### 1F. Tablet (misma llamada)

- `mobile/src/lib/cache.ts:5` y su pantalla de conexión: tras validar el código, llamar a `POST /pair`
  (`kind: "tablet"`, nombre que ponga el usuario, por defecto «Tablet») y guardar el **token** en `ServerCfg.code`
  (`src/lib/remote/config.ts:5`; el campo se sigue llamando `code`, ahora lleva el token). Si el host es viejo
  (sin `pair` en `features`) sigue con el código.

### 1G. Comprobación de la fase 1

- `cargo test -p visor-core` (migración 16, fusión por dispositivo, `uid` único, emparejar/tokens/legacy/426).
- Manual, con dos PCs en la LAN: emparejar el portátil («Portátil»), ver la biblioteca con miniaturas, cambiar un
  estado y verlo en el host, apagar el host → la biblioteca sigue visible y editar avisa de «solo lectura».
- Jugar en streaming desde el portátil una novela de 5 min: en `visor.db` del host (copia, ver memoria
  «Library on HDD») `SELECT device_id, uid FROM sessions ORDER BY id DESC LIMIT 3` → el id del portátil. Una
  partida en el host justo después no se funde con ella.
- La tablet empareja con token y sus partidas salen con su id; una APK vieja sigue entrando con el código.

---

## Fase 2: juegos instalados en el cliente y sesiones locales

- Migración 17 (host): `installs(game_id, device_id, exe_name, folder_name, size_bytes, added_ms, seen_ms,
  PRIMARY KEY(game_id, device_id))`.
- Endpoints `call` nuevos en el host: `match_title { platform, name, exeName, folderName, steamAppId }` → candidatos
  **de esa categoría** (appid exacto; luego nombre de exe/carpeta normalizado; luego parecido de nombre);
  `register_install { gameId | null, platform, name, … }` → si `gameId` es null crea el título en esa categoría
  (entrada manual sin ejecutable en el host) y devuelve su id; apunta la fila en `installs`.
- Cliente: «Añadir carpeta» reutiliza `scan_games_dir`/`add_game` locales; antes de guardar, elige categoría de la
  lista del host (nunca una nueva) y confirma «es el mismo que …» o «nuevo». La fila local de `games` usa el id del
  host. Comandos `add_game`, `scan_games_dir`, `set_game_paths`, `remove_game` pasan a `LOCAL` en el router.
- `snapshot`/`scan_library` del host: `Title.installedOn: [{deviceId, name}]`. El cliente marca «instalado también
  aquí» con sus `games` locales. «Jugar» en el cliente: instalado aquí → local; si no → streaming; con las dos,
  botón con opción.
- Subida de sesiones: hilo en `client.rs` (al conectar, tras `game-exited` y cada 5 min) → `POST /api/v1/sessions`
  `{ sessions: [{ uid, gameId, startMs, endMs, activeMs, input }], readSessions: [...] }` con las que tienen
  `synced_ms IS NULL`; el host inserta con `device_id = caller` vía `stat_add_session` (idempotente por `uid`) y
  devuelve los `uid` aceptados; el cliente marca `synced_ms`. Lo jugado en local sin host se ve en el cliente como
  «pendiente de enviar».
- Espejo de miniaturas: descargar en segundo plano todas las miniaturas del host a la caché local (para que «sin
  conexión» no dependa de la caché HTTP del WebView). Por `sha256` de la captura, no por ruta.

## Fase 3: partidas de viaje y copiar el juego

- Ya existe casi todo en `saves.rs`/`visor-core/src/saves/` (rutas Ludusavi + motores + manuales, copias .zip,
  restaurar). Falta:
  - `saves_export(id, destDir)`: crea copia (`saves_backup`) y la copia a la carpeta elegida como
    `<Juego> - AAAA-MM-DD HHmm - <dispositivo>.zip`, con `visor-saves.json` dentro:
    `{ gameId, gameName, deviceId, deviceName, createdMs, newestMs, files: [{ path, size, mtimeMs, sha256 }] }`.
  - `saves_import(id, zip)`: lee el manifiesto, **siempre** hace copia de lo actual primero, compara `newestMs` con
    lo que hay y con la última copia del host (`saves_info` remoto) y avisa si lo existente es más reciente; luego
    restaura con la lógica actual.
  - Rutas manuales dentro de la carpeta del juego se guardan relativas (`{game}/savedata`) para que valgan en
    cualquier dispositivo (KiriKiri/Ren'Py con partidas en la carpeta del juego).
  - Carpeta de destino recordada (USB/OneDrive); la nube llega en la fase 6 con el mismo zip y manifiesto.
- Copiar el juego entero al cliente: `GET /api/v1/install/list?id=` (ficheros relativos con tamaño y fecha de la
  carpeta de instalación del host, solo esa) y `GET /api/v1/install/file?id=&f=` con `Range` (reanudar). Comando
  local `client_copy_game(id, destDir)` con progreso; al acabar, `register_install` como en la fase 2.

## Fase 4: texthooker/Textractor en el cliente

- Juegos instalados en el cliente: el texthook corre local (`th-*` en `LOCAL` del router) y las frases y
  estadísticas de lectura se escriben en el host (`phrases_*` ya están en `dispatch`; `read_sessions` suben como en
  la fase 2).
- Juegos en streaming: el texthook corre en el host; reenviar los eventos `th-*` (hoy excluidos,
  `src-tauri/src/remote.rs:110`) solo a quien está jugando, para ver el texto en la ventana del cliente.

## Fase 5: fuera de casa (Tailscale)

- Tailscale en host y clientes, sin abrir puertos. `client.json.hostAddrs` guarda varias direcciones (LAN primero,
  IP/nombre de Tailscale después); se prueba en orden con `hello`. Moonlight también va por Tailscale.
- El tráfico va cifrado por WireGuard; en la LAN sigue siendo HTTP con token (aceptable en casa).

## Fase 6: nube personal

- La nube expone **la misma API** (como ya prevé `docs/api-movil.md`) y es un dispositivo más.
- Ver sección 7.

---

## 7. Modelo para la nube (se diseña ahora, se implementa en la fase 6)

- **Ficheros por huella:** capturas (`shots.sha256`, ya existe), zips de partidas (sha256 del zip + manifiesto con
  sha256 por fichero), grafismos. Tabla futura `file_places(sha256, place, path, size, seen_ms,
  PRIMARY KEY(sha256, place))`, con `place` = id de dispositivo o `cloud`. Mostrar un fichero = buscar un sitio
  donde esté (disco local → host → nube).
- **Registro de cambios:** tabla futura `changes(seq INTEGER PRIMARY KEY, at_ms, device_id, entity, key, op)` en el
  host, rellenada por los comandos que escriben (estado, Mi lista, colecciones, favoritos, prefs, sesiones,
  partidas). `GET /api/v1/changes?since=<seq>` → lo que cambió. Sirve para refrescar la caché de los clientes y para
  que la nube se ponga al día sin reenviarlo todo.
- **Filas que viajan:** `uid` global + `device_id` + `updated_ms` (sesiones ya en la fase 1; `title_status` ya tiene
  `updated_ms`).

---

## Riesgos y decisiones abiertas

1. **SQLite por red.** Si alguien apunta el cliente a `P:` compartido, dos procesos escriben el mismo `visor.db` y se
   corrompe. Mitigación en 1D (`meta.host_id`), pero **¿el cliente PC tiene acceso a P: por red?** Si sí, cabe
   leer los originales por SMB en vez de por HTTP (más rápido), nunca la base.
2. **Wake-on-LAN.** Depende de BIOS/adaptador/router; solo funciona en la LAN (por Tailscale no llega la difusión).
   ¿Se quiere despertar al pulsar «Jugar» automáticamente o solo con el botón? Además el servidor necesita el visor
   abierto en el host: confirmar que el arranque con Windows en bandeja (plan-tablet T3) está activo.
3. **Host ocupado.** Hoy `stream::launch` rechaza si hay un juego o sesión en marcha (409). Abierto: ¿un cliente
   puede cerrar el juego de otro (`/close`) o «quitar» la sesión? Propuesta: no; solo mostrar quién y qué.
   Apollo solo admite una sesión a la vez.
4. **Conflictos.** Fase 1 sin conflictos: el cliente escribe directo en el host y sin conexión es solo lectura (gana
   el último que escribe). Partidas: se resuelven a mano en el import (copia siempre + aviso por fecha). Sesiones:
   idempotentes por `uid`; dos dispositivos jugando a la vez es legítimo.
5. **Versiones distintas.** `api`/`apiMin` + `426`; los `cmd` de `/call` pueden cambiar de argumentos entre
   versiones. Propuesta: si `version` mayor.menor no coincide, el cliente avisa y se queda en solo lectura hasta
   actualizar (los dos tienen el actualizador). Abierto: ¿bloquear o solo avisar?
6. **Rutas de `Title`/`Shot` son rutas del host.** El cliente las trata como opacas (como la tablet). Cualquier
   código del escritorio que haga algo local con ellas (abrir carpeta, arrastrar, `reveal`) debe ocultarse en modo
   cliente: revisar en la fase 1 con grep de `open_folder`/`reveal_in_explorer`/`move_shots`.
7. **Emparejar Moonlight con Apollo** es un paso aparte (PIN) en cada PC cliente; el visor solo puede explicarlo y
   comprobarlo. Por verificar si Moonlight acepta `stream <host> "<app>"` con el host por IP y por nombre de
   Tailscale.
8. **Stats en memoria** (`games::add_session`, `launcher.rs:389`) siguen sin dispositivo; las estadísticas por
   dispositivo se harán leyendo `sessions.device_id` de la base. ¿Qué vistas quieres por dispositivo (solo total por
   equipo, o filtro en Estadísticas)?
9. **IDs `plataforma/carpeta`**: si se renombra la carpeta en el host, cambia el id (pasa ya hoy); las instalaciones
   del cliente quedarían huérfanas. Fase 2: al subir una instalación con id desconocido, volver a hacer `match_title`.
