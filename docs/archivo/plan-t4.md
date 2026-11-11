# Plan T4: sesión de streaming completa (PC + tablet)

Origen: `docs/plan-tablet.md` §2 y fila T4. Ya hecho y **no se rehace**: `launch_stream` (`src-tauri/src/remote_call.rs:76`)
→ `launcher::start_stream` (`src-tauri/src/launcher.rs:97-137`, espera la pantalla virtual 30 s y lanza allí con
`start_at`), mapeo `launch_game → launch_stream` (`src/lib/remote/core.ts:120`), plugin `StreamingPlugin.java`,
captura con volumen abajo (`CaptureKeyService.java:38-51`).

Las categorías de la biblioteca no se tocan: nada de esto lee ni cambia plataformas, salvo leer `platform` del
juego para la meta diaria (solo lectura).

## Decisiones

1. **Juego pendiente = `start_stream` con estado explícito**, movido a un módulo nuevo `src-tauri/src/stream.rs`.
   La tablet ya no dispara un temporizador ciego de 30 s: el juego queda «pendiente» y se lanza cuando el ayudante
   avisa de que la tablet ha conectado. Sin ayudante (app «Desktop» o Sunshine) se mantiene el comportamiento
   actual (sondeo de la pantalla virtual 30 s y, si no aparece, monitor del PC) como respaldo.
2. **El ayudante sí hace falta**, aunque `start_stream` ya lance tras conectar. Aporta lo que el sondeo no puede:
   (a) la sesión de Apollo termina sola al cerrarse el juego (el proceso de la app sale) y Artemis vuelve; sin él
   la sesión «Desktop» sigue abierta; (b) aviso exacto de conexión, sin límite de 30 s (Artemis puede tardar);
   (c) si la tablet nunca conecta, el juego no se abre en el PC. Va por HTTP a `127.0.0.1` (ruta sin código solo
   desde loopback), sin IPC nueva. El puerto va en la línea de comandos de la app de Apollo.
3. **Crear «Juego actual» en Apollo**: escribir `apps.json` (en `Program Files`, pide UAC con el mismo patrón que
   `magpie::imp::restart_elevated`) y reiniciar el servicio. Descartado: la API web de Apollo (`:47990`) porque
   pide el usuario/contraseña de su web y su login difiere del de Sunshine.
4. **`POST /api/v1/close`**: ruta propia (no `/call`) porque la llama Java desde el servicio de accesibilidad, igual
   que `/capture`. Cierre normal = `WM_CLOSE` con `SendMessageTimeout(SMTO_ABORTIFHUNG)`; si la ventana no
   responde o no hay ventana → forzado (`quickmenu::force_close_game`). Si respondió pero el juego sigue vivo
   (diálogo «¿salir?»), no se fuerza solo: una segunda petición antes de 60 s fuerza.
5. **Cursor**: cursores del sistema transparentes (`SetSystemCursor`) mientras dura un juego lanzado por streaming;
   restaurar con `SystemParametersInfoW(SPI_SETCURSORS)` al acabar, al arrancar el visor (fichero marcador), en el
   gancho de pánico y al perder la pantalla virtual. Interruptor en Ajustes (por defecto sí), porque en modo
   trackpad de Artemis (el apaño del punto 7) el puntero tiene que verse.
6. **Última sesión**: evento nuevo `session-summary` (reenviado en `/events`) + `GET /api/v1/session` (estado ligero
   para el ayudante y para Java). Se construye escuchando `game-started`/`capture-saved`/`game-exited` desde
   `stream.rs`: **no se toca `launcher::watch`**. Las metas se calculan en la tablet con `goalTrack` (la tablet ya
   tiene la interfaz y los datos del escritorio).
7. **Tablet**: el menú de volumen arriba y la vigilancia del fin de sesión viven en `CaptureKeyService` (el servicio
   de accesibilidad puede abrir actividades desde segundo plano —excepción de Android para servicios enlazados por
   el sistema— y pintar `TYPE_ACCESSIBILITY_OVERLAY` sin permiso extra). El resumen es la animación ya existente
   de `Detail.svelte:56-76` (`app.gain`, disparada por `game-exited`), más un aviso con capturas y meta; los
   eventos se retienen mientras la app está oculta para que la animación se vea al volver.
8. **Puntero con Magpie (plan-tablet punto 7) fuera de T4 → T4b.** Necesita investigar `destRect` de Magpie y la
   reentrada de `SetCursorPos` en el gancho; no bloquea el flujo de sesión y hay apaño (trackpad).

## Contratos nuevos (los usan PC, ayudante y tablet)

- `GET /api/v1/session` (código, o sin código desde loopback) →
  `{ state: "idle"|"pending"|"starting"|"playing", game: {id,name}|null, streamed: bool, last: Summary|null }`
- `POST /api/v1/session/start` (solo loopback) body `{width,height,fps}` → `{ok:true,state}` o 409 si no hay
  juego pendiente ni en marcha.
- `POST /api/v1/close` (código) body `{force?:bool}` → `{ closed: "none"|"asked"|"forced" }`.
- Evento `session-summary`: `Summary = { id, name, platform, startedMs, endedMs, seconds, shots, streamed }`.
- `launch_stream` (en `/call`) devuelve `{ app: "Juego actual" | null }` (null = la tablet usa su ajuste/«Desktop»).
- Nuevo `/call` `stream_cancel` (quita el pendiente si Artemis no llegó a abrir).
- `/hello` `features` añade `"close"`, `"session"`.

## Incrementos (cada uno: commit + push por separado)

Antes de cada commit: los cambios ajenos sin commitear de `launcher.rs`, `quickmenu.rs` y
`src/lib/controller.svelte.ts` **no son de T4**. Si un incremento toca uno de esos ficheros, `git add -p` solo
con los trozos propios (o preguntar si ya se subieron). Mirar finales de línea (LF/CRLF mezclados, memoria
`line-endings-mixed.md`). `quickmenu.rs` no se edita en T4.

### I1 — PC: rutas `close` y `session`, resumen de sesión
1. `src-tauri/visor-core/src/remote.rs:103-132` (trait `Provider`): métodos con valor por defecto
   `close(&self, force: bool) -> Result<Value>`, `session(&self) -> Value` (`{"state":"idle"}`),
   `session_start(&self, w: u32, h: u32, fps: u32) -> Result<Value>` (bail).
2. `remote.rs:613-675` (`route`): `/api/v1/close` y `/api/v1/session/start` al grupo POST (línea 628-631),
   `/api/v1/session` al GET. Antes del `authorized` (645): si `ep` es `session`/`session_start` y
   `req.ip.is_loopback()`, sin código. `session_start` solo desde loopback (403 si no). Errores → 409 como
   `launch`. Añadir `"close","session"` a `features` (625).
3. Tests en `remote.rs` junto a `launch_and_capture_over_http` (~1286): 401 sin código en `close`, 200 con código,
   `session` sin código desde 127.0.0.1 = 200, `session/start` GET = 405. Ampliar el proveedor de prueba (~1109).
4. Nuevo `src-tauri/src/stream.rs` (+ `mod stream;` en `lib.rs:2-37`), con estado global
   `Mutex<Session { phase, streamed, started_ms, shots, last: Option<Summary>, close_asked: Option<Instant> }>`:
   - `init(app)`: `listen_any` de `game-started` (fase→playing, `started_ms`, `shots=0`), `capture-saved`
     (`shots+=1` si playing), `game-exited` (arma `Summary` con `platform` de `st.games`, guarda `last`, fase→idle,
     emite `session-summary`). Llamarlo en `lib.rs:1166` antes de `remote::forward_events`.
   - `close(app, force)`: sin `st.active` → `none`. `force` o `close_asked` < 60 s → hilo con
     `quickmenu::force_close_game(&app)` → `forced`. Si no: ventana principal de los pids
     (nuevo `pub fn game_window(pids:&[u32]) -> Option<isize>` en `display.rs` cerca de 111, envolviendo
     `imp::main_window`), `SendMessageTimeoutW(WM_CLOSE, SMTO_ABORTIFHUNG, 3000)`; sin ventana o colgada →
     forzado; si respondió → `close_asked = now`, `asked`.
   - `session_json()` según el contrato.
5. `src-tauri/src/remote.rs:15-96` (`AppProvider`): implementar `close` y `session` llamando a `stream`.
   `FORWARDED_EVENTS` (100-116): añadir `"session-summary"` (orden alfabético).
6. `remote_call.rs:96` aprox.: `"close_game" => out(crate::stream::close(app, arg(a,"force").unwrap_or(false)))`.

### I2 — PC: ocultar y restaurar el cursor
1. `visor-core/src/config.rs:146` y `:252`: `stream_hide_cursor: bool` (por defecto `true`, con `#[serde(default…)]`
   como los vecinos). `src/lib/types.ts:95` `streamHideCursor: boolean`; `src/lib/store.svelte.ts:69` `true`;
   `src/lib/mock.ts:325` el campo.
2. `stream.rs`: `hide_cursor(config_dir)` (crea el marcador `config_dir/cursor-oculto`, luego para cada `OCR_*`
   —NORMAL, IBEAM, WAIT, CROSS, UP, SIZENWSE, SIZENESW, SIZEWE, SIZENS, SIZEALL, NO, HAND, APPSTARTING, HELP—
   `SetSystemCursor(CreateCursor 32×32 transparente, id)`; un cursor nuevo por llamada, la función lo destruye) y
   `restore_cursor(config_dir)` (`SystemParametersInfoW(SPI_SETCURSORS,0,None,0)` y borra el marcador). Features
   ya en `Cargo.toml:50` (`Win32_UI_WindowsAndMessaging`).
   - Ocultar en `game-started` si la fase venía de streaming y `stream_hide_cursor`.
   - Restaurar en `game-exited`; al arrancar si existe el marcador (`lib.rs:1170`, junto a `quickmenu::recover`);
     en `install_panic_hook` (`lib.rs:1168`, buscar su definición); al salir del visor (buscar `exit_app` /
     salir de la bandeja en `tray.rs`).
   - Hilo vigía mientras está oculto: si `display::virtual_monitor()` desaparece > 5 s (Artemis se desconectó
     a mitad), restaurar; si vuelve a conectar (`session_start` con juego en marcha) se vuelve a ocultar.
3. Ajustes: fila `remotecursor` en `src/lib/remote.svelte.ts:8-11` (tipo, `REMOTE_ROWS`, `REMOTE_CYCLE`),
   texto en `remoteRow`, acción `patchConfig({streamHideCursor: !…})` en `remoteAct`. Añadir el id a la lista de
   `src/lib/controller.svelte.ts:1232` (**fichero con cambios ajenos: `git add -p`**). Fila solo de PC (oculta con
   `IS_TABLET` como las demás de PC, comprobar cómo se ocultan `remotewin`/`remoteqr` en la tablet).

### I3 — PC: juego pendiente
1. `launcher.rs:140`: `fn start_at` → `pub(crate) fn start_at`. `launcher.rs:90-137`: quitar `start_stream`,
   `STREAM_WAIT` y `STREAM_WAITING` (pasan a `stream.rs`). **Fichero con cambios ajenos: `git add -p`.**
2. `stream.rs`: `launch(app, game) -> Result<Value,String>`:
   - Error si `st.active` o fase ≠ idle. Fase = `pending{game, at}`, `streamed = true`.
   - Si `apollo::app_installed()` (I5; hasta entonces `false`): hilo que caduca el pendiente a los 120 s
     (aviso `magpie-status` «La tablet no llegó a conectar; no se abre el juego») y devuelve `{app:"Juego actual"}`.
   - Si no: el sondeo actual de `launcher.rs:107-133` (30 s, asentar 1,5 s, `run_on_main_thread(start_at)`),
     tomando el pendiente con `take` bajo el mutex; devuelve `{app:null}`.
   - `launch_now(app, game)`: fase→starting; espera `virtual_monitor()` hasta 10 s + 1,5 s; `start_at` en el
     hilo principal; error → fase idle + aviso.
   - `cancel()`: pendiente → idle.
3. `remote_call.rs:76-87`: llamar a `crate::stream::launch(app, game)`; añadir `"stream_cancel"`.

### I4 — PC: ayudante `visor.exe --stream-session --port N`
1. `stream.rs`: `pub fn helper_main() -> Option<i32>` (sin Tauri): si `args[1] == "--stream-session"`; lee
   `SUNSHINE_CLIENT_WIDTH/HEIGHT/FPS` (0 si faltan); `ureq` (ya en `Cargo.toml:28`) `POST
   http://127.0.0.1:N/api/v1/session/start`; error de conexión 10 s → `Some(2)`; 409 → `Some(1)`. Luego
   `GET /api/v1/session` cada 1 s: sale `Some(0)` cuando ve `idle` tras haber visto `starting/playing`; si sigue
   en `pending/starting` > 120 s → `Some(1)`; visor caído 10 s → `Some(2)`. Registro en `%TEMP%\visor-stream.log`.
2. `lib.rs:989-991`: `stream::helper_main().or_else(magpie::helper_main)`. Imprescindible: no hay plugin de
   instancia única, una segunda instancia normal abriría otro visor.
3. `stream.rs` `session_start(w,h,fps)`: pendiente → `launch_now`; idle con `st.active` → fase playing (reengancha
   y vuelve a ocultar el cursor); nada → `Err`. `AppProvider::session_start` en `remote.rs`.
4. La fase pasa a `idle` en el oyente de `game-exited`, que se emite **después** de `restore_primary`
   (`launcher.rs:453-469`): así el ayudante sale, y Apollo quita la pantalla virtual, cuando el monitor principal ya
   está devuelto.

### I5 — PC: botón «Crear “Juego actual” en Apollo»
1. Nuevo `src-tauri/src/apollo.rs`:
   - `locate()`: `HKLM\SYSTEM\CurrentControlSet\Services\{ApolloService,SunshineService}\ImagePath` (nombres por
     verificar, ver dudas) → raíz = carpeta padre de `tools\`; si no, `C:\Program Files\Apollo` / `…\Sunshine`.
     `apps.json` = `file_apps` de `<raíz>\config\sunshine.conf` o `<raíz>\config\apps.json`.
   - `status() -> {found, path, installed, outdated}`: `installed` si hay una app «Juego actual»; `outdated` si su
     `cmd` no tiene el `current_exe()` o el `remote_port` actuales. `app_installed()` = installed && !outdated.
   - `install()`: lee el JSON, copia la entrada «Desktop» como plantilla si existe (para heredar claves propias de
     Apollo), pone `name:"Juego actual"`, `cmd:"\"<exe>\" --stream-session --port <port>"`, `auto-detach:"false"`,
     `wait-all:"true"`, `exit-timeout:"5"`, `uuid` nuevo si las demás llevan `uuid`; sustituye la existente.
     Escribe en temporal; si se puede escribir el destino, directo; si no, elevado:
     `visor.exe --apollo-write <destino> <temporal> <servicio>` con `runas` (copiar el patrón de
     `magpie.rs:406-443`), que hace copia `apps.json.bak-visor`, reemplaza y reinicia el servicio (`sc stop`/`sc
     start`). Despacharlo en `lib.rs:989` junto a los otros ayudantes.
   - Comandos Tauri `stream_app_status`, `stream_app_install` en `lib.rs:1293`. **No** en `remote_call`.
2. Frontend: `src/lib/backend.ts:166` y `src/lib/types.ts:365` (+ `mock.ts:579`) los dos métodos; fila
   `remotestream` en `src/lib/remote.svelte.ts` («Juego actual en Apollo»: «Crear» / «Creada» / «Actualizar» /
   «Apollo no encontrado»), A = instalar; id en `controller.svelte.ts:1232` (**`git add -p`**). Solo PC.

### I6 — Tablet: volumen arriba = menú
1. `CaptureKeyService.java:38-51`: tratar también `KEYCODE_VOLUME_UP` con su propio `swallowUp`. Solo con
   Moonlight/Artemis delante (`StreamingPlugin.isMoonlight`); fuera, el volumen sigue normal.
2. Menú: `WindowManager.addView` con `TYPE_ACCESSIBILITY_OVERLAY`, `WRAP_CONTENT`, arriba al centro,
   `FLAG_NOT_FOCUSABLE | FLAG_NOT_TOUCH_MODAL` (los toques fuera llegan a Artemis). Vistas creadas en código (sin
   layout XML). Se oculta a los 4 s sin tocar, o con otra pulsación de volumen arriba.
   - «Capturar» → la `capture()` existente.
   - «Cerrar juego» → `POST /api/v1/close {}`; `asked` → el botón pasa a «Forzar cierre» y el menú sigue 5 s;
     `forced`/`none` → se oculta. Vibración como `vibrate(ok)`.
   - «Volver a la app» → `getPackageManager().getLaunchIntentForPackage(getPackageName())` con
     `FLAG_ACTIVITY_NEW_TASK | FLAG_ACTIVITY_REORDER_TO_FRONT`.
   Sacar la petición HTTP a un `post(path, body)` común (reutiliza `base`/`code` de `PREFS`).
3. `res/xml/capture_key_service.xml` y el texto de descripción: comprobar `flagRequestFilterKeyEvents` y mencionar
   volumen arriba.

### I7 — Tablet: vuelta a la app y resumen
1. `CaptureKeyService.onAccessibilityEvent:25-31`: al pasar Moonlight a primer plano arranca un hilo (uno solo) que
   hace `GET /api/v1/session` cada 2 s mientras Moonlight siga delante. Si ve `starting/playing` y después `idle`
   con `last.endedMs` posterior al inicio del hilo → `returnToApp()` (mismo intent que el menú). A los 1,5 s, si
   `foreground` no es nuestro paquete (Android bloquea en silencio), notificación «Sesión terminada · ver resumen»
   que abre la app (canal propio; `POST_NOTIFICATIONS` en `AndroidManifest.xml:63` y petición en
   `MainActivity.onCreate` si API ≥ 33).
2. `src/lib/remote/core.ts:108-122`: `afterLaunch(args, res)` (firma y llamada).
3. `mobile/src/tablet.ts:34-38`: app = `(res as {app?:string|null})?.app || stream.appName.trim() || 'Desktop'`;
   si `Streaming.stream` falla, `invoke('stream_cancel')` y aviso.
4. `src/lib/remote/event.ts:13-19`: si `document.hidden`, encolar `game-exited` y `session-summary` y
   despacharlos en `visibilitychange` a visible (así `app.gain` → la animación de `Detail.svelte:56-76` se ve al
   volver). Solo afecta a la tablet (este fichero sustituye a los eventos de Tauri).
5. `mobile/src/tablet.ts`: `listen('session-summary')` (de `@desk/lib/remote/event`) → `toast` de 5 s
   «Has jugado X a Y · N capturas · Meta de hoy a/b min». Meta: si `platform` es `vn`/`jvn`,
   `goalTrack(platform, goal, a.lib.platforms, null, Date.now())` de `@desk/lib/readgoals` (solo lectura) tras el
   `softRefresh` que ya hace `game-exited`. El `toast` de `game-exited` se sustituye porque este llega después.

### T4b (aparte) — Puntero con Magpie
Plan propio cuando T4 esté probado: gancho `WH_MOUSE_LL` de `src-tauri/src/mouse.rs` solo en sesión de streaming
con Magpie activo; inversa `destRect→srcRect`; distinguir `SetCursorPos` propio (bandera `LLMHF_INJECTED` +
marca temporal). Requiere estudiar `Magpie.Core/CursorManager.cpp` antes.

## Orden y paralelo
I1 → I3 → I4 → I5 en el PC (I2 en cualquier momento tras I1). Tablet: I6 en paralelo con I3-I5 (solo necesita I1
desplegado); I7 tras I1 e I3. Ningún paso es trabajo masivo: nada para `agy`.

## Comprobación
- `cargo test -p visor-core remote` (rutas nuevas) y `cargo check` del crate de la app; `npm run check` del
  frontend; compilar la APK (`node node_modules/vite/bin/vite.js build` si `npm run build` no encuentra node, y
  comprobar que la APK lleva el cambio).
- Manual PC: `curl -X POST -H "Authorization: Bearer <código>" http://IP:47811/api/v1/close` con un juego abierto
  (cierre normal; segunda vez forzado); `curl http://127.0.0.1:47811/api/v1/session` sin código = 200, desde otra
  máquina sin código = 401. Matar el visor con el cursor oculto y volver a abrirlo: el cursor vuelve.
- Sesión completa (la prueba de la fila T4): crear «Juego actual» desde Ajustes → en la tablet «Jugar» una novela
  → Artemis abre «Juego actual» → el juego sale en la pantalla virtual sin cursor → volumen abajo captura, volumen
  arriba menú → «Cerrar juego» → Apollo termina la sesión → la tablet vuelve sola al Detalle con la animación y el
  aviso de minutos/capturas/meta. Repetir con «Desktop» (sin ayudante) para el respaldo, y sin abrir Artemis
  (el pendiente caduca a los 120 s y no se abre nada en el PC).

## Riesgos y dudas abiertas
- **Nombre del servicio y ruta de Apollo** (`ApolloService`? ImagePath en `tools\`?) y si Apollo relee `apps.json`
  sin reiniciar: verificar en el PC del usuario antes de I5 (`sc query type= service | findstr /i apollo`, mirar
  `apps.json`). Claves propias de Apollo en cada app (`uuid`, pantalla virtual por app): se heredan copiando
  «Desktop»; confirmar con su `apps.json` real.
- **Que Apollo lance el ayudante en la sesión del usuario** y le pase `SUNSHINE_CLIENT_*` (fork de Sunshine, se
  espera que sí); el ayudante funciona aunque falten (manda 0).
- **Salir desde Artemis a mitad** (sin cerrar el juego): Apollo mata el ayudante pero el juego sigue en el PC; el
  vigía del cursor lo restaura. Si se vuelve a abrir «Juego actual», `session_start` reengancha.
- **Arranque desde segundo plano en Android**: se cuenta con la excepción del servicio de accesibilidad; si el
  dispositivo la bloquea igualmente, queda la notificación (que requiere permiso concedido).
- ¿Apollo/Artemis ocultan ya el cursor? (T0 no lo dejó escrito.) Si sí, I2 se reduce al interruptor apagado por
  defecto. ¿El cursor transparente oculta también el que pinta Magpie? Se espera que sí (Magpie copia la forma del
  cursor actual); comprobar en la prueba.
- Con Sunshine (sin pantalla virtual) el juego se abre en el monitor del visor sin cambiar resolución: cambiar la
  resolución al conectar queda fuera de T4.
- Hay cambios ajenos sin commitear en `launcher.rs` (I3), `controller.svelte.ts` (I2, I5) y `quickmenu.rs` (no se
  toca): no mezclarlos en los commits de T4.
