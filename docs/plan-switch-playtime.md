# Plan: tiempo de juego y lista de juegos de Nintendo Switch (Switch 1 y Switch 2)

Fecha: 2026-11-07. Origen: el documento de investigación de Pablo (APIs de historial de juego y Control Parental
«Moon», login OAuth PKCE). Aquí se adapta a esta app; lo que el documento proponía y aquí cambia está en «Decisiones».

**Objetivo:** que los juegos de la categoría **Switch** tengan tiempo de juego automático (histórico + día a día) y que
se pueda importar la lista de juegos jugados en la consola como fichas de esa categoría, con sus grafismos de
SteamGridDB. Sirve para Switch 1 y Switch 2.

**Las categorías no se tocan.** Todo vive dentro de la plataforma `switch` que ya existe (`visor-core/src/config.rs:179`).
No se crea ninguna categoría, no se cambia cómo se detectan y no se mezclan títulos con otras. Si la configuración no
tiene la plataforma `switch`, la importación se desactiva con un aviso; nunca se crea.

## Situación actual

- **El tiempo se guarda por juego, no por título.** `play_stats(game_id…)` y `sessions(game_id, start_ms, end_ms…)`
  (`visor-core/src/db.rs:54-66`, ampliadas en la migración 11, `:218-225`). Se cargan en memoria con `stats_load`
  (`visor-core/src/db/data.rs:154`) y se guardan en `AppState.stats` al arrancar (`src-tauri/src/lib.rs:1130`).
- **Solo los títulos con juego vinculado reciben tiempo.** `link_game` copia `seconds`, `week`, `plays`… del `Stats` por
  `g.id` (`visor-core/src/scan.rs:290-309`). Una carpeta suelta `Switch/Zelda` sin fila en `games` no tiene tiempo.
- **El id del título cambia al vincular un juego:** `title_id` devuelve el id del juego si lo hay y si no
  `plataforma/carpeta` (`visor-core/src/userdata.rs:10`). No existe ninguna función que traslade los datos de un id a
  otro (estado, colecciones, sgdb, fecha…). Esto condiciona cómo se enlazan los juegos de Nintendo (decisión 4).
- **Juegos «solo capturas»:** `games.kind = 'captures'`, `platform = 'switch'`, id `manual:<n>`
  (`visor-core/src/games.rs:314-319`). `add_game` crea además la carpeta (`src-tauri/src/lib.rs:471-524`); el escaneo ya
  admite títulos sin carpeta (`scan.rs:348-364`, `has_folder: false`).
- **El orden «jugado recientemente» se calcula antes de fijar ids** (`scan.rs:367-374`): el tiempo de consola tiene que
  aplicarse antes de ordenar.
- **Grafismos:** `sgdb_auto` busca por nombre y baja portada, hero y logo de lo que falte (`src-tauri/src/sgdb.rs:564-624`,
  `:643`); el frontend lo lanza tras cada escaneo (`src/lib/controller.svelte.ts:1618`, `:1656`). Solo hay tres tipos de
  arte (`visor-core/src/art.rs:7`), no hay «icono».
- **Red:** `ureq` 2 bloqueante (`src-tauri/Cargo.toml:28`); sin tokio/reqwest ni `keyring`. `windows` 0.62 ya está
  (`Cargo.toml:50`) sin la feature `Win32_Security_Credentials`. `sha2` y `getrandom` están en `visor-core`
  (`visor-core/Cargo.toml:21,26`); no hay `base64`.
- **Copias de seguridad:** el .zip lleva `visor.db` y `config.json` (`visor-core/src/backup.rs:47-67`). La clave de
  SteamGridDB va en la configuración, así que **un secreto en config.json acaba en las copias**.
- **Ventanas webview propias:** se crean con `WebviewWindowBuilder` (p. ej. `src-tauri/src/quickmenu.rs:338`, que hay que
  llamar desde un hilo secundario, `:286`). Las capacidades solo dan IPC a `main`, `quickmenu` y `texthook`
  (`src-tauri/capabilities/*.json`).
- **Trabajos en segundo plano:** hilos sueltos con `std::thread::spawn` desde `setup` (copia automática,
  `src-tauri/src/lib.rs:1195-1200`) o bucles con `sleep` (`goal_notice`, `:410-424`). No hay un planificador común.
- **Dónde se pinta el tiempo:** ficha, panel de horas (`src/screens/Detail.svelte:43-54`, `:214-227`) y panel de la
  semana, que solo sale si el título es lanzable (`Detail.svelte:229`; foco en `controller.svelte.ts:120`). También
  `Titles.svelte:227,261,313`, `Home.svelte:42,53`, `MiListaDual.svelte:74,78`, `lib/stats.ts` (usa `plays`) y el menú
  rápido (`quickmenu/TileTime.svelte:41`). Todos leen `playSeconds`, `week` y `plays` de `Title`
  (`src/lib/types.ts:188-198`).

## Decisiones

1. **Todo en Rust con `ureq`, sin reqwest ni sidecar.** Las llamadas son pocas, bloqueantes y en un hilo propio; es el
   mismo patrón que `sgdb.rs`. La lógica pura (PKCE, parseo, cálculo de tiempos, tablas) va en
   `visor-core/src/nintendo/` para poder probarla sin Tauri; el HTTP, el llavero y los comandos en
   `src-tauri/src/nintendo/`. Se añade `base64 = "0.22"` a `visor-core` (PKCE y JWT). Descartado: sidecar de Python
   con `pynintendoparental` (otro runtime que instalar y empaquetar solo para tres peticiones).
2. **Secretos en el Administrador de credenciales de Windows**, con la feature `Win32_Security_Credentials` del crate
   `windows` (`CredWriteW` / `CredReadW` / `CredDeleteW`, tipo `CRED_TYPE_GENERIC`, persistencia `LOCAL_MACHINE`,
   destinos `VisorCapturas/nintendo/history` y `VisorCapturas/nintendo/parental`). Nunca en `visor.db`, ni en
   `config.json`, ni en el servidor del móvil. Los access tokens solo en memoria. Al restaurar una copia en otro PC hay
   que volver a conectar, y es lo correcto. Descartados `keyring` (dependencia nueva para lo mismo) y `stronghold`
   (pide contraseña).
3. **El historial de juego es la fuente principal; el Control Parental es opcional.** El historial ya da el total por
   juego, primera y última vez, nombre, imagen y, según la extensión de Raycast, los **últimos días desglosados**
   (`recentPlayHistories`, *a verificar en la Fase 0*). Si se confirma, se tiene tiempo diario sin vincular la consola al
   Control Parental. Moon añade el desglose por consola y usuario, el tiempo de hoy en curso y una ventana más larga.
   La Fase 0 decide si Moon entra en la v1 o pasa a una fase posterior.
4. **Enlace título de Nintendo → ficha sin cambiar ids ni crear categorías.** Tabla propia `switch_titles` con
   `game_id` **o** `folder`:
   - Ya hay juego en la categoría Switch (emulador o «solo capturas») → `game_id = manual:<n>`.
   - Solo hay carpeta de capturas, sin juego → `folder = <nombre de la carpeta>`. **No se crea fila en `games`**, porque
     cambiaría el id del título (`switch/zelda` → `manual:<n>`) y dejaría huérfanos su estado, sus colecciones y su
     arte. El escaneo busca el tiempo por clave normalizada de carpeta (`norm_key`, como ya hace `scan.rs:340-343`).
   - No existe nada → se crea un juego `kind = 'captures'`, `platform = 'switch'`, `folder = NULL` (se deriva del
     nombre), **sin crear la carpeta** en el disco. La carpeta aparecerá cuando lleguen capturas.
   - El emparejamiento automático es por nombre normalizado (minúsculas, sin ™/®/©, sin «:», «-» ni espacios dobles), y
     solo contra títulos de la categoría Switch. Lo que no cuadra se enseña para enlazarlo a mano.
5. **Importar la lista es una acción explícita con vista previa; la sincronización periódica nunca crea fichas.** La
   sincronización solo actualiza tiempos y apunta «N juegos nuevos de Switch sin importar». Así, borrar una ficha
   importada no hace que vuelva sola. Las apps que no son juegos (YouTube, Netflix…) se pueden desmarcar; se guarda
   `ignored = 1`.
6. **El tiempo de Nintendo no entra en `play_stats` ni en `sessions`.** Son datos de otra naturaleza (minutos por día, no
   intervalos) y meterlos como partidas sintéticas falsearía `run_count`, `active_ms`, la fusión de partidas
   (`MERGE_GAP_MS`), `MAX_SESSIONS`, el % activo y los gráficos de `stats.ts`. Se guardan en `switch_*` y se **suman al
   leer**, en Rust, al construir el `Title`:
   - `play_seconds = segundos del PC + segundos de consola`; `last_played = max(...)`; `first_played = min(...)`;
     `week = semana del PC + minutos de consola por día`.
   - `sessions` y `plays` siguen siendo **solo del PC**: no se inventan partidas.
   - Campo nuevo `console: Option<ConsoleTime>` con `seconds`, `firstPlayed`, `lastPlayed`, `days` (últimos 28 días,
   `[inicio_del_día_ms, minutos]`) y `source` (`"history"` / `"parental"`), para el desglose en la ficha.
7. **No hay doble conteo con el emulador.** Lo que mide la app (emulador en el PC) y lo que da Nintendo (consola real)
   son conjuntos disjuntos: se suman. Entre las dos fuentes de Nintendo sí se solapan, así que por día y juego cuenta el
   **máximo** de las dos (historial del día frente a la suma de Moon de los usuarios marcados), no la suma.
8. **Total de consola:** `total = total_del_historial + Σ minutos_día` de los días **posteriores** al día en que se leyó
   el historial (`history_ms`). Como el historial se vuelve a pedir en cada sincronización, el segundo término solo
   cubre huecos (historial caído o retrasado). El día de la lectura no se suma (ya va en el total).
9. **Metadatos y arte:** nombre e imagen salen de las propias respuestas de Nintendo (historial y Moon traen
   `titleName`/`imageUrl`, *a verificar*). Los grafismos (portada, hero, logo) los pone el `sgdb_auto` de siempre, que
   ya busca por nombre los títulos sin arte: **no hay que tocar `sgdb.rs`** para la v1. La imagen cuadrada de Nintendo se
   guarda como portada de respaldo solo si SteamGridDB no tiene portada (fase posterior). `blawar/titledb` se descarta
   para la v1: son JSON de decenas de MB por región, va por `nsuId` y no está claro que cubra Switch 2; solo haría falta si
   alguna respuesta llegara sin nombre.
10. **Ids de título:** se guardan como texto, 16 hexadecimales en mayúsculas, **sin suponer el prefijo 0100** (Switch 2
    puede usar otros, *a verificar*). Se normalizan al id base (`id & ~0x1FFF`: las actualizaciones acaban en `800` y los
    DLC en `1xxx`).
11. **Login con una ventana webview propia que captura la redirección**, con el pegado del enlace como respaldo.
    `WebviewWindowBuilder` + `on_navigation`: cuando la URL empieza por `npf<client_id>://auth`, se lee el
    `session_token_code` del fragmento, se cancela la navegación (`false`) y se cierra la ventana. La ventana **no**
    aparece en ninguna capability (sin IPC: carga una web ajena). Con las cookies conservadas, el segundo login (el
    otro `client_id`) es un solo clic en «Elegir a esta persona». El respaldo (campo de pegar) existe siempre, por si
    Nintendo o WebView2 bloquean el esquema.
12. **Sincronización:** un hilo `nintendo-sync` creado en `setup` (junto a la copia automática, `lib.rs:1195`). Primera
    pasada a los 60 s del arranque, después cada 30 min; duerme en tramos de 1 min mirando una bandera de «sincronizar
    ya» y de apagado. Sin cuenta conectada no hace nada. Al terminar emite `nintendo-synced { changed }`; el frontend, si
    `changed > 0`, pide `nintendo_title_times` (Rust devuelve los campos de tiempo ya mezclados por título) y parchea los
    títulos en memoria **sin reescanear el disco** (la biblioteca está en un HDD lento). La instantánea
    `library-snapshot.json` se actualiza en el siguiente escaneo normal.
13. **Solo escritorio.** La sección «Nintendo» de Ajustes no sale con `IS_TABLET`; el módulo Rust va con
    `#[cfg(windows)]`. La tablet ve el tiempo porque viaja en el `Title`.

## Modelo de datos (migración 15, al final de `MIGRATIONS`, `visor-core/src/db.rs:244`)

```sql
-- Juegos de la cuenta Nintendo. Enlace con la ficha: game_id (juego de la categoria Switch) o folder (carpeta suelta).
CREATE TABLE switch_titles (
    nx_id           TEXT PRIMARY KEY,          -- id base, 16 hex en mayusculas
    name            TEXT NOT NULL,
    image_url       TEXT,
    device_type     TEXT,                      -- Switch / Switch 2, si la API lo da
    first_played_ms INTEGER,
    last_played_ms  INTEGER,
    total_minutes   INTEGER NOT NULL DEFAULT 0, -- del historial
    history_ms      INTEGER,                   -- cuando se leyo ese total
    game_id         TEXT,
    folder          TEXT,
    ignored         INTEGER NOT NULL DEFAULT 0,
    seen_ms         INTEGER NOT NULL DEFAULT 0  -- primera vez que aparecio (para «nuevos sin importar»)
);
-- Minutos por dia. Idempotente: se hace upsert con la clave completa.
CREATE TABLE switch_daily (
    day        TEXT NOT NULL,                  -- YYYY-MM-DD en la zona local
    nx_id      TEXT NOT NULL,
    source     TEXT NOT NULL,                  -- 'history' | 'parental'
    device_id  TEXT NOT NULL DEFAULT '',
    player_id  TEXT NOT NULL DEFAULT '',
    minutes    INTEGER NOT NULL,
    updated_ms INTEGER NOT NULL,
    PRIMARY KEY (day, nx_id, source, device_id, player_id)
);
CREATE INDEX switch_daily_title ON switch_daily(nx_id, day);
-- Usuarios de las consolas vinculadas al Control Parental y si cuentan como Pablo.
CREATE TABLE switch_players (
    device_id   TEXT NOT NULL,
    player_id   TEXT NOT NULL,
    nickname    TEXT NOT NULL DEFAULT '',
    device_name TEXT NOT NULL DEFAULT '',
    counted     INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (device_id, player_id)
);
```

El estado no secreto (apodo de la cuenta, última sincronización, último error, consolas) va en `meta` con claves
`nintendo.*`. Estas tablas sí entran en las copias de seguridad: son datos, no secretos.

## Fases

### Fase 0. Investigación con respuestas reales (Pablo + script, antes de programar)
Script fuera del repo (en `.claude-scratch/nintendo/`, que no se sube) con `pynintendoparental` para Moon y un script
pequeño para el historial, portado de la extensión de Raycast. Hay que anotar en `docs/nintendo-api.md`:
- [ ] `client_id`, scopes, `redirect_uri`, URLs, cabeceras obligatorias y User-Agent/versión de app, de **las dos** APIs.
- [ ] Respuesta del historial: si trae `titleId`, `titleName`, `imageUrl`, `totalPlayedMinutes`, `firstPlayedAt`,
      `lastPlayedAt`, `deviceType`, y si `recentPlayHistories` da minutos por día y cuántos días.
- [ ] Si el historial incluye **juegos de Switch 2** y cómo se distinguen; formato de sus ids de título.
- [ ] Moon: si funciona con Switch 2, si la versión de la API (`/moon/v1/`) sigue siendo esa, cuántos días atrás dan
      los resúmenes diarios y los mensuales, y qué devuelve una consola recién vinculada.
- [ ] Zona horaria de los días (¿la de la consola, la de la cuenta, un parámetro?).
- [ ] Duración real del session token y qué error da al caducar o revocarse.
- [ ] En Tauri: que `on_navigation` ve la URL `npf…://auth` en WebView2 y que el login de Nintendo carga en una
      webview (captcha, bloqueo de webviews embebidas). Prueba de 20 líneas en una rama aparte.
- [ ] Si SteamGridDB permite buscar por id de Nintendo además de por nombre (si no, se queda la búsqueda por nombre).
- [ ] Guardar 4-6 respuestas reales **anonimizadas** (ids de cuenta/consola cambiados, sin tokens) como fixtures:
      historial completo, historial vacío, Moon dispositivos, Moon resumen diario, Moon «summary does not exist».

**Puerta:** si el historial no da desglose diario y Moon no funciona con Switch 2, se replantea el alcance con Pablo
antes de seguir.

### Fase 1. Núcleo puro y tablas (visor-core, con tests)
- [ ] Migración 15 en `visor-core/src/db.rs` (al final de `MIGRATIONS`) y la tabla nueva en la lista de tablas
      esperadas del test (`db.rs:425`).
- [ ] `visor-core/src/nintendo/mod.rs`: constantes centralizadas (client_ids, hosts, versión de app, User-Agent) en un
      solo sitio; normalización de ids (decisión 10) y de nombres (decisión 4).
- [ ] `nintendo/pkce.rs`: `state`, `code_verifier`, `code_challenge` S256 (`sha2` + `getrandom` + `base64`), URL de
      login, parseo de la URL de vuelta (`session_token_code`, comprobar `state`).
- [ ] `nintendo/models.rs`: structs serde de las respuestas, tolerantes (campos opcionales y tipos raros, como en
      `sgdb.rs:845-886`).
- [ ] `nintendo/store.rs`: upserts de `switch_titles`, `switch_daily`, `switch_players`; lectura de títulos sin enlazar.
- [ ] `nintendo/merge.rs`: `console_times(conn, now, tz) -> ConsoleMap` (por `game_id` y por carpeta normalizada) con
      las reglas 7 y 8, filtro de usuarios marcados y los 28 días recientes.
- [ ] `nintendo/link.rs`: emparejamiento automático (decisión 4) → propuesta `{nuevos, enlazados, dudosos}`.

### Fase 2. Mezcla en la biblioteca
- [ ] `visor-core/src/scan.rs`: campo `console: Option<ConsoleTime>` en `Title` (`:34-83`, con `#[serde(default)]`);
      `ScanOpts` recibe `console: Option<&ConsoleMap>`; se aplica **solo en la plataforma `switch`** y **antes de
      ordenar** (`:367`), por `game_id` (dentro de `link_game`, `:290-309`) o por carpeta normalizada para las carpetas
      sueltas. Suma `play_seconds`, `week`, `last_played` y `first_played` (decisión 6).
- [ ] `src-tauri/src/lib.rs:214-236` (`scan_full`): cargar `ConsoleMap` de la base y pasarlo en `ScanOpts`. Lo mismo en
      `visor-core/examples/serve.rs:58` para que compile.
- [ ] `src/lib/types.ts:188-198`: `console?: ConsoleTime | null`; `src/lib/mock.ts` (≈`:172-177` y `:490`) con un
      juego de Switch con tiempo de consola.

### Fase 3. Login y llavero
- [ ] `src-tauri/Cargo.toml:50`: añadir la feature `Win32_Security_Credentials`.
- [ ] `src-tauri/src/nintendo/secrets.rs`: `put/get/delete(kind)` sobre el Administrador de credenciales.
- [ ] `src-tauri/src/nintendo/auth.rs`: intercambio `session_token_code → session_token` y `session_token → access
      token` (en memoria, con caducidad), con `ureq` y las cabeceras de la Fase 0.
- [ ] `src-tauri/src/nintendo/login.rs`: ventana `nintendo-login` (hilo secundario, como `quickmenu.rs:286`), captura en
      `on_navigation`, sin capability. Comandos `nintendo_login_start(kind)`, `nintendo_login_paste(kind, url)`,
      `nintendo_logout(kind)`, `nintendo_status()`.
- [ ] Registrar los comandos en `generate_handler!` (`src-tauri/src/lib.rs:1264`) y `mod nintendo;` (`lib.rs:2-39`).

### Fase 4. Historial e importación de la lista
- [ ] `src-tauri/src/nintendo/history.rs`: cliente del historial; guarda títulos, total y días recientes
      (`source = 'history'`).
- [ ] Comandos `nintendo_import_preview()` (propuesta de `link.rs`) y `nintendo_import_apply(choices)`: enlaza, crea los
      juegos `captures` sin carpeta (usar `games::new_manual_id` y `dbd::manual_add`, **sin** el `create_dir_all` de
      `add_game`) y marca los ignorados. Después, el frontend reescanea (una vez) y `sgdb_auto` pone los grafismos.
- [ ] Comando `nintendo_link(nx_id, title)` para corregir un enlace a mano desde la ficha.

### Fase 5. Sincronización periódica
- [ ] `src-tauri/src/nintendo/sync.rs`: hilo `nintendo-sync` (decisión 12) lanzado en `setup` (`lib.rs:1195-1200`);
      `nintendo_sync_now()` levanta la bandera. 401 → renovar el access token una vez; 401 con el session token →
      estado «caducado» y aviso, sin borrar datos; resto de errores → espera creciente (1, 2, 4… hasta 30 min).
      Nunca más de una petición por API cada pocos minutos.
- [ ] Al acabar, `emit("nintendo-synced", { changed, newTitles })`. Comando `nintendo_title_times()` que devuelve, por
      id de título, `playSeconds`, `week`, `lastPlayed`, `firstPlayed` y `console` ya mezclados con el PC.

### Fase 6. Control Parental (si la Fase 0 lo confirma)
- [ ] El usuario vincula la consola en la app oficial (sin restricciones, sin límite). Ajustes lo explica en una línea.
- [ ] `src-tauri/src/nintendo/parental.rs`: dispositivos, usuarios (`switch_players`) y resúmenes diarios
      (`source = 'parental'`); hoy en curso; «summary does not exist» no es un error.
- [ ] Entra en el mismo hilo de sincronización.

### Fase 7. Interfaz
- [ ] `src/lib/nintendo.svelte.ts`: estado, llamadas y tipo `NintendoId` (mismo patrón que `remote.svelte.ts:8`).
      Filas: `nxaccount` (conectar / «Conectado como X» / «Caducado: vuelve a conectar»), `nxparental` (conectar
      Control Parental y consolas vistas), `nxplayers` (usuarios que cuentan), `nximport` (importar juegos, con «N
      nuevos» si los hay), `nxsync` (sincronizar ahora · última sincronización), `nxoff` (desconectar y borrar tokens).
- [ ] `src/lib/controller.svelte.ts:1194-1263`: `NintendoId` en `SettingId`, sección «Nintendo» (`t: 0.9`, antes de
      «Acerca de»), solo si `!IS_TABLET`; acciones en el despacho de ajustes (junto a `:1520-1522`).
- [ ] `src/screens/Settings.svelte` (junto a `:87-89`): textos de las filas. Un aviso fijo en `nxaccount`: «Usa una API
      no oficial de Nintendo; ver riesgos».
- [ ] Pantalla de importación: lista con casillas en tres grupos (nuevos, se enlazan con…, sin pareja) y un botón
      «Importar». Reutilizar el estilo de la lista de títulos de SteamGridDB (`openSgdbTitles`, `src/lib/sgdb.svelte.ts`).
- [ ] Ficha (`src/screens/Detail.svelte`): en el panel de horas (`:215`) el subtítulo pasa a «Horas jugadas · consola X
      h · PC Y h» cuando hay `T.console`; el panel de la semana sale con `T.launchable || T.console` (`:229`) y su foco
      igual (`controller.svelte.ts:120`).
- [ ] Escuchar `nintendo-synced` y parchear los títulos con `nintendo_title_times` (decisión 12).

### Fase posterior (sin diseñar a fondo)
- **Importar capturas de la Switch** hacia `Raiz/Switch/<Juego>` con el importador que ya existe
  (`visor-core/src/importer.rs`). Vías: USB (MTP, «Copiar al PC» del Álbum en Switch 1 y 2), microSD (`Nintendo/Album/
  AAAA/MM/DD/<fecha>-<hash>.jpg|mp4`) o la app móvil de Nintendo. `dates.rs:9` ya lee la fecha de ese nombre; el
  `<hash>` identifica el juego (se deriva del id de título con un algoritmo conocido por la comunidad, *a verificar*) y,
  con `switch_titles`, dice a qué carpeta va. Al llegar la primera captura de un juego importado sin carpeta, la carpeta
  se crea con su nombre y el escaneo la enlaza sola por `folder`.
- Icono de Nintendo como portada de respaldo cuando SteamGridDB no tiene portada.
- Tiempo de consola en `lib/stats.ts` (gráficos globales y mapa de calor), con los días de `switch_daily`.
- Presencia «jugando ahora» (NSO): necesita f-token de terceros; más riesgo, fuera.
- Importar el historial desde la **exportación de datos personales** de la cuenta Nintendo (si la ofrece en un formato
  legible): alternativa sin API no oficial.

## Pruebas

Tests inline en `visor-core` con `tempfile::tempdir()` y `Db` en disco o en memoria, como `games.rs:550-610`:
- PKCE: el challenge de un verifier conocido (vector de la RFC 7636), parseo de la URL de vuelta, `state` que no cuadra.
- Modelos: cada fixture de la Fase 0 se parsea (`include_str!`); campos nulos o ausentes no rompen.
- Ids: `0100ABCD00000800` → `0100ABCD00000000`; minúsculas → mayúsculas; un id de Switch 2 de la fixture.
- Merge: total = historial + días posteriores; el día de la lectura no se suma; máximo entre fuentes en el mismo día;
  usuario desmarcado no cuenta; PC + consola se suman; semana correcta con DST (como `week_seconds`, `games.rs:374`).
- Enlace: juego existente → `game_id`; carpeta suelta → `folder` y **no** se crea juego; nada → juego `captures` sin
  carpeta en el disco; nunca se enlaza con títulos de otra categoría.
- Escaneo: un título de la categoría Switch recibe `console` y sube en el orden; uno de PC con el mismo nombre no.
- Migración: base de la versión 14 → 15 sin perder datos (patrón `db.rs:441-453`).
- Copias: `create_backup` no contiene nada de `nintendo` salvo las tablas (no hay secretos que buscar; el test comprueba
  que `config.json` no tiene claves `nintendo*`).

Comprobación a mano: conectar la cuenta, importar, ver un juego de Switch con horas en la ficha y en la lista, jugar
30 min en la consola y ver que tras la siguiente sincronización suben sin reescanear.

## Seguridad y privacidad

- Session tokens solo en el Administrador de credenciales; access tokens solo en memoria. Nada en `visor.db`,
  `config.json`, registros (`eprintln!` nunca imprime tokens ni la URL de vuelta) ni respuestas del servidor del móvil.
- La ventana de login no tiene IPC con la app.
- «Desconectar» borra las credenciales y las cookies de la ventana de login; los datos de tiempo se quedan.
- Las fixtures del repo van anonimizadas.

## Riesgos

- **API no oficial y términos de uso.** El acuerdo de usuario de la cuenta Nintendo no autoriza clientes de terceros.
  Lo que hace esta función (leer tu propio historial y el Control Parental imitando las apps oficiales) lo hacen desde
  hace años la integración de Home Assistant y `nxapi`, sin bloqueos conocidos por solo leer, pero **no hay garantía**:
  Nintendo podría suspender la cuenta o, más probable, romper la API con una actualización. Para reducirlo: solo
  lectura, pocas peticiones, User-Agent y versión reales, nada de NSO/f-token, y un interruptor para apagarlo todo. La
  decisión de usarlo con la cuenta principal es de Pablo.
- **Rotura por actualizaciones de las apps de Nintendo** (versión mínima, client_id, formato). Constantes en un solo
  sitio; los errores de formato se enseñan como «la API de Nintendo ha cambiado», sin perder datos.
- **Ventana de días de Moon e historial:** si se sincroniza menos que esa ventana, se pierde el desglose de esos días (el
  total no, porque sale del historial).
- **Nombres que no cuadran** (ediciones, «Switch 2 Edition», ™, idiomas): el enlace automático puede fallar o
  equivocarse; por eso hay vista previa y enlace manual.
- **Una ficha importada se borra y Nintendo la sigue trayendo:** queda como «sin importar», no vuelve sola.
- **WebView2 y el esquema `npf`:** si `on_navigation` no lo ve, solo queda el pegado del enlace (peor, pero funciona).

## Preguntas abiertas para Pablo

1. ¿Aceptas el riesgo de la API no oficial con tu cuenta Nintendo principal?
2. ¿Quieres vincular la consola al Control Parental (pide PIN para cambiar sus ajustes) o te basta con el historial si
   da los últimos días desglosados?
3. Al importar, ¿se crean fichas para todos los juegos jugados, también los de 5 minutos y las demos, o solo a partir de
   un mínimo (p. ej. 30 min)?
4. ¿Hay más usuarios en tus consolas cuyo tiempo no deba contar?
5. En la ficha, ¿basta con el desglose «consola X h · PC Y h» o quieres ver por separado Switch 1 y Switch 2?

## Tareas para Kanvas (grupo «SW»)

`[I]` = investigación, `[D]` = desarrollo. Entre paréntesis, de qué depende.

1. `[I]` SW-1 Script de prueba del historial con la cuenta real; anotar client_id, scopes, URLs, cabeceras y formato en
   `docs/nintendo-api.md`. Ficheros: `.claude-scratch/nintendo/`, `docs/nintendo-api.md`.
2. `[I]` SW-2 Lo mismo para Moon con `pynintendoparental` (incluida Switch 2 y la ventana de días). (SW-1 para el login)
3. `[I]` SW-3 Fixtures anonimizadas de las respuestas. Ficheros: `src-tauri/visor-core/src/nintendo/fixtures/*.json`.
   (SW-1, SW-2)
4. `[I]` SW-4 Prueba de WebView2: `on_navigation` ve `npf…://auth` y el login carga. Rama aparte, no se fusiona.
5. `[I]` SW-5 Decidir con Pablo: alcance de Moon en la v1 y preguntas abiertas 1-5. (SW-1, SW-2)
6. `[D]` SW-6 Migración 15 y test de migración. Ficheros: `visor-core/src/db.rs`. (SW-5)
7. `[D]` SW-7 Constantes, normalización de ids y nombres, PKCE con tests; añadir `base64`. Ficheros:
   `visor-core/Cargo.toml`, `visor-core/src/lib.rs`, `visor-core/src/nintendo/{mod,pkce}.rs`. (SW-1)
8. `[D]` SW-8 Modelos serde y tests con fixtures. Ficheros: `visor-core/src/nintendo/models.rs`. (SW-3)
9. `[D]` SW-9 Store: upserts y lecturas de `switch_*`. Ficheros: `visor-core/src/nintendo/store.rs`. (SW-6)
10. `[D]` SW-10 Merge de tiempos con tests (reglas 7 y 8, DST). Ficheros: `visor-core/src/nintendo/merge.rs`. (SW-9)
11. `[D]` SW-11 Emparejamiento automático con tests. Ficheros: `visor-core/src/nintendo/link.rs`. (SW-9)
12. `[D]` SW-12 `Title.console` y aplicación en el escaneo antes de ordenar; `scan_full` y `serve.rs`. Ficheros:
    `visor-core/src/scan.rs`, `src-tauri/src/lib.rs:214-236`, `visor-core/examples/serve.rs`. (SW-10)
13. `[D]` SW-13 Tipos y mock del frontend. Ficheros: `src/lib/types.ts`, `src/lib/mock.ts`. (SW-12)
14. `[D]` SW-14 Llavero de Windows. Ficheros: `src-tauri/Cargo.toml`, `src-tauri/src/nintendo/secrets.rs`.
15. `[D]` SW-15 Auth (session token y access token). Ficheros: `src-tauri/src/nintendo/auth.rs`. (SW-7, SW-14)
16. `[D]` SW-16 Ventana de login, pegado de respaldo y comandos de estado. Ficheros: `src-tauri/src/nintendo/login.rs`,
    `src-tauri/src/lib.rs` (mod y `generate_handler!`). (SW-4, SW-15)
17. `[D]` SW-17 Cliente del historial y guardado. Ficheros: `src-tauri/src/nintendo/history.rs`. (SW-8, SW-9, SW-15)
18. `[D]` SW-18 Comandos de vista previa, importación y enlace manual. Ficheros: `src-tauri/src/nintendo/mod.rs`. (SW-11,
    SW-17)
19. `[D]` SW-19 Hilo de sincronización, errores y evento; `nintendo_title_times`. Ficheros:
    `src-tauri/src/nintendo/sync.rs`, `src-tauri/src/lib.rs:1195-1200`. (SW-12, SW-17)
20. `[D]` SW-20 Cliente de Moon y usuarios. Ficheros: `src-tauri/src/nintendo/parental.rs`. (SW-5, SW-19) — solo si
    SW-5 lo mete en la v1.
21. `[D]` SW-21 Sección «Nintendo» de Ajustes. Ficheros: `src/lib/nintendo.svelte.ts`, `src/lib/controller.svelte.ts`,
    `src/screens/Settings.svelte`. (SW-16, SW-19)
22. `[D]` SW-22 Pantalla de importación. Ficheros: componente nuevo en `src/components/`, `src/lib/nintendo.svelte.ts`.
    (SW-18, SW-21)
23. `[D]` SW-23 Ficha: desglose consola/PC, panel de semana y foco; parcheo tras `nintendo-synced`. Ficheros:
    `src/screens/Detail.svelte`, `src/lib/controller.svelte.ts:120`. (SW-13, SW-19)
24. `[I]` SW-24 Fase posterior: estudiar la importación de capturas (MTP, microSD, hash → id de título). Sin código.
