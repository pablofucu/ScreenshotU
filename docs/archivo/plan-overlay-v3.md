# Overlay v3: el menú sobre el juego nunca congela sin verse

Sustituye a la parte de apertura/foco de `plan-overlay-resolucion.md`. No cambia ninguna decisión del usuario:
pausar al abrir, cierre forzado inmediato, mantener = forzar cierre, Magpie sigue escalando por debajo (ni se oculta ni se
suspende). Las categorías de la biblioteca no se tocan (nada de esto les afecta).

## 1. Diagnóstico (verificado en el código)

### 1.1 «Magpie no cede» es un síntoma: la ventana del menú nunca llegó a mostrarse

- En el log falta la línea `ventana mostrada en (...)`, que `show_window` (`quickmenu.rs:328`) escribe **dentro** del
  cierre que corre en el hilo principal, y tampoco aparece `no se pudo obtener ni crear la ventana`. Ese cierre no terminó.
- Tiempos: captura 40.623 → «Magpie cedio» 41.401 = 778 ms = 500 ms de `recv_timeout` (`quickmenu.rs:334`, se agotó)
  + 203 ms de espera a Magpie + recorrer ventanas. `open()` siguió adelante sin saber si la ventana existía y
  **suspendió el juego** (`quickmenu.rs:400-401`).
- Magpie solo quita su `WS_EX_TOPMOST` cuando el primer plano deja de ser el juego. Si nuestra ventana no tomó el primer
  plano, Magpie no tenía nada que ceder. Esperar más no arregla nada.
- Elevación/UIPI (la «Causa A» del plan anterior) no es la explicación: con Magpie, el primer plano sigue siendo la ventana
  del juego (sin elevar), no la de Magpie. UIPI no impide subir una ventana propia a `HWND_TOPMOST` ni
  `SetForegroundWindow` sobre una ventana propia. Lo único que bloquearía es `AttachThreadInput` si el primer plano fuera
  de Magpie, y en ese caso fallaría sin colgar nada.
- Sospechoso principal del bloqueo del hilo principal: la cadena de foco en `show_window`. `win.set_focus()` (tao
  fuerza la activación) y `overlay::force_foreground` (`overlay.rs:237-248`) hacen `AttachThreadInput` del hilo
  principal **con el hilo del juego** y luego `SetForegroundWindow` + `SetFocus`. Con las colas de entrada acopladas, el
  cambio de foco envía mensajes **síncronos** al hilo del juego. Además, WebView2 tiene ventanas hijas de otro proceso
  que comparten cola de entrada con nuestra ventana. Si el juego tarda (está gestionando su pérdida de foco) o está
  **suspendido**, el hilo principal se queda esperando. Si el juego se suspende con el acople a medias, la espera ya no
  termina.
- **Bug latente sin Magpie:** en esa ruta `open()` suspende **antes** de `show_window` (`quickmenu.rs:384-389`), así que
  `force_foreground` se acopla a un hilo ya suspendido. Mismo cuelgue, también en juegos sin Magpie.
- Un hilo principal colgado lo explica todo: no se pinta nada; `hide_window`/`restore_foreground` (que van al hilo
  principal) no se ejecutan al cerrar, así que queda una ventana nuestra, transparente y «siempre encima», tapando el
  juego y quedándose los clics. Eso da «el juego no responde». A los 5 s Windows la marca como «No responde».
- Matiz: las pulsaciones de 46.920 y 48.493 sí se registraron. Si venían del atajo de teclado (el plugin
  global-shortcut las entrega en el hilo principal), el hilo principal seguía procesando mensajes publicados. Eso encaja
  con que esté atascado dentro de una llamada que bombea mensajes (llamada COM a WebView2 o espera de un mensaje
  síncrono) y no con un bloqueo seco. La telemetría de la fase 0 lo resuelve.

### 1.2 La carrera del final: existe, pero no como la describe el brief

- La línea `captura de fondo ... (GDI)` de 50.729 **no viene de una pulsación nueva**. `trigger_down` escribe
  «atajo pulsado» siempre que hay partida (`quickmenu.rs:120-129`), y a las 50.7 seguía habiéndola.
- Viene de la pulsación de 48.493. `TriggerFsm::down` devuelve `Open` **al bajar** (`visor-core/src/trigger.rs:75-76`)
  y la misma pulsación, mantenida 1,5 s, da `Force` (48.493 + 1500 ≈ 50.007). Así que **cada cierre forzado con el menú
  cerrado lanza antes un `open()`**.
- Ese `open()` tiene `LIFE` cogido **durante toda la captura** (`quickmenu.rs:354-379`). Con los intentos WGC fallando
  tardó 2,2 s, y cada intento puede esperar hasta 2,5 s (`platform.rs:130`). Mientras tanto:
  - `force_close_game` mata el juego y su `close()` (`quickmenu.rs:535`) espera a `LIFE`.
  - Luego `open()` termina sobre un juego muerto: `suspend` de un pid muerto, que reescribe
    `quickmenu-suspended.json` cuando `force_close_game` ya lo había borrado. Después pone `IS_OPEN`, `show_window`
    (otra vez con acople), `keep_on_top`, `menu_nav` y `keyboard::start`.
  - En cuanto suelta `LIFE`, `close()` hace `teardown` y el vigilante hace `destroy`, así que la ventana se muestra, se
    oculta y se destruye casi seguido.
- **Bug nuevo encontrado: fuga del gancho de teclado.** `keyboard::start` (`quickmenu.rs:679-695`) lanza un hilo que
  guarda su id en `THREAD` **después** de `SetWindowsHookExW`. Si `teardown` → `keyboard::stop` (`:701-708`) llega
  antes, ve `THREAD == 0` y no hace nada. El gancho se queda para siempre y se traga en **todo el sistema** flechas,
  Enter, Esc, Tab, Retroceso, Q, E, X, Y, + y −. Con `close()` esperando justo detrás de `open()`, en este escenario es
  casi seguro que pasa. Es otra vía de «al salir el juego no responde».
- ¿Por qué se cierra la app? No se puede afirmar con lo que hay:
  - `Cargo.toml:40` tiene `panic = "abort"`, no hay `panic::set_hook` y la app usa `windows_subsystem = "windows"`. Un
    pánico en cualquier hilo (incluido dentro de tauri/wry) cierra la app **en silencio**.
  - También puede ser un fallo nativo de WebView2 al hacer show → hide → destroy seguidos sobre un hilo principal medio
    atascado.
  - O que el usuario/Windows cierren una app «No responde».
  - `WindowEvent::Destroyed` solo sale para `main` (`lib.rs:1184`), así que destruir el menú no llama a `app.exit`.
  - Hace falta la telemetría de la fase 0 y el Visor de eventos para confirmarlo.

## 2. Decisión

1. **No pausar nunca a ciegas.** La pausa se mantiene solo si hay prueba de que el menú se ve:
   - el cierre del hilo principal terminó;
   - la página avisó de que ha pintado esta apertura (`quickmenu_painted(gen)` tras dos `requestAnimationFrame`);
   - nuestra ventana está por encima de la de Magpie y de la del juego en el orden Z.

   Si no se confirma en ~1 s (con un reintento), se deshace todo (reanudar, ocultar, devolver el primer plano) y se avisa
   con dos vibraciones o un pitido, como en pantalla completa exclusiva. El estilo `WS_EX_TOPMOST` de Magpie deja de ser
   la señal: solo se registra en el log.
2. **El hilo principal no se acopla a hilos ajenos.** Fuera `AttachThreadInput`, `SetFocus` y `win.set_focus()`. Se
   queda `win.show()` (necesario para que WebView2 pinte) + `SetWindowPos(HWND_TOPMOST)` + `SetForegroundWindow` sin
   acople (si Windows lo niega, no pasa nada). El teclado ya va por el gancho de bajo nivel y el mando por `menu_nav`:
   el menú no necesita el foco para ser interactivo.
3. **La captura sale de `LIFE`** y `open()` se puede cancelar: comprueba `ENDING`/`FORCED` antes de cada efecto.
   Mientras termina la partida no se aceptan atajos.
4. **Gancho de teclado sin fugas:** no se traga nada si el menú no está abierto, y `start`/`stop` sin carrera.

Descartado: abrir al **soltar** en vez de al pulsar (evitaría el `open()` de un cierre forzado). Cambia la experiencia
acordada, porque el progreso de mantener se dibuja dentro del menú abierto, y con `open()` cancelable no hace falta.
Descartado: subir el tiempo de espera a Magpie, por lo dicho en 1.1.

## 3. Pasos

### Fase 0 — Telemetría (va en el mismo commit; sin ella no se sabrá por qué se cerró la app)

0.1 `lib.rs`, en el `setup` junto a `quickmenu::recover` (`lib.rs:1144`): `std::panic::set_hook` que añada a
    `config_dir/panic.log` (ruta en un `OnceLock<PathBuf>`) y a `quickmenu::log`: nombre del hilo
    (`std::thread::current().name()`), mensaje, `location()` y `std::backtrace::Backtrace::force_capture()`. Con
    `panic = "abort"` el gancho corre antes de abortar.
0.2 `quickmenu.rs:129` `trigger_down`: registrar `src` y el hilo (`atajo pulsado (src: Key, hilo: main, menu abierto: …)`).
0.3 `quickmenu.rs:174` `run_act`: registrar `Open`, `Close`, `Force` y `Warn` (los `Hold*` no, que son muchos).
0.4 `show_window` (`quickmenu.rs:316`), marcas con ms desde que se pide:
    - antes de `run_on_main_thread`: `pido mostrar`;
    - dentro del cierre: `hp: empieza`, `hp: show hecho`, `hp: encima hecho (SetForegroundWindow=…, primer plano=pid exe)`;
    - si `recv_timeout` se agota: `el hilo principal no respondio en N ms`.
0.5 `teardown`/`hide_window`/`destroy_now`: una línea cada uno con el motivo (`cerrar`, `cerrar ya`, `fin de partida`),
    además de `ventana oculta` y `ventana destruida` dentro de sus cierres del hilo principal.
0.6 `keyboard::start/stop`: `gancho teclado puesto (tid)` / `quitado (tid)` / `stop sin gancho que quitar`.
0.7 `launcher.rs:298` bucle del vigilante: si una vuelta tarda más de 1 s, registrarlo (`vigilante: vuelta lenta N ms`).

### Fase 1 — Hilo principal sin acoples (`overlay.rs`, `quickmenu.rs`)

1.1 `overlay.rs:225-250` `force_foreground` → renombrar a `raise_menu(w, x, y, width, height) -> (bool, Option<u32>)`:
    - `SetWindowPos(HWND_TOPMOST, x, y, w, h, SWP_NOOWNERZORDER | SWP_SHOWWINDOW)`;
    - `let fg_ok = SetForegroundWindow(hwnd).as_bool()`;
    - `BringWindowToTop(hwnd)`;
    - devuelve `fg_ok` y el pid del primer plano resultante (para el log).

    Sin `AttachThreadInput`, sin `SetFocus`. Actualizar el `#[cfg(not(windows))]` (`:252-256`).
1.2 `overlay.rs:259-280` `restore_foreground`: quitar el acople y dejar solo `IsWindow` + `SetForegroundWindow`. Al
    cerrar, el primer plano es nuestro y Windows lo permite.
1.3 `quickmenu.rs:316-335` `show_window(app, pid, gen) -> bool`:
    - dentro del cierre, primero `if !is_current(gen) { return }`, porque un cierre que llega tarde no debe mostrar nada;
    - `win.show()` (sí), **quitar `win.set_focus()`**, y `raise_menu`;
    - devuelve `true` solo si `recv_timeout(800 ms)` recibió.
1.4 `keep_on_top` (`quickmenu.rs:338-351`):
    - dentro del cierre comprobar `is_current(gen)`;
    - no encolar otro `to_top` si el anterior aún no corrió: `static TOP_PENDING: AtomicBool`, que se pone a `true` al
      encolar y a `false` dentro del cierre.

### Fase 2 — Abrir con confirmación (`quickmenu.rs`, `platform.rs`, página)

2.1 Nuevos estáticos en `quickmenu.rs` (junto a `:40-57`):
    - `PAINTED: AtomicU64` (último `gen` pintado por la página);
    - `ENDING: AtomicBool` (la partida está terminando);
    - `SESSION: AtomicU64` (sube en cada `prewarm`).
2.2 `platform.rs` (junto a `magpie_topmost`, `:337-342`):
    - nuevo `pub fn menu_on_top(own: isize, game_pids: &[u32]) -> (bool, String)`;
    - usa `top_windows()`, que va en orden Z de arriba abajo;
    - es `true` si `own` es visible y su índice es menor que el de la ventana de escalado de Magpie (si la hay) y que el
      de cada ventana visible de más de 10 000 px² de los `game_pids`;
    - el `String` describe el porqué, para el log;
    - exportarlo en el `pub use imp::{…}` de `:585` y añadir la versión `not(windows)` que devuelva `(true, …)`.
2.3 Comando `quickmenu_painted(gen: u64)` en `quickmenu.rs` (sección de comandos, `:768+`) que haga
    `PAINTED.store(gen)`. Registrarlo en `lib.rs:1283`, junto a `quickmenu_close_game`.
2.4 `state()` (`quickmenu.rs:753`): añadir `"gen": GEN.load(SeqCst)`.
2.5 Página:
    - `src/quickmenu/data.ts`:
      - `QuickMenuState` gana `gen?: number`;
      - `markPainted(gen)` → `invoke('quickmenu_painted', { gen })` (solo con `isTauri()`);
      - en `setupQuickMenuListeners` (`:512-535`), escuchar `qm-paused` (`{ paused: boolean }`) con un nuevo
        `events.onPaused`.
    - `src/quickmenu/QuickMenu.svelte`:
      - en `onOpenState` (`:206`), dentro del `tick().then(...)`, añadir
        `requestAnimationFrame(() => requestAnimationFrame(() => markPainted(newState.gen)))` si hay `gen`;
      - el manejador `onPaused` hace `qmState = { ...qmState, paused }`;
      - registrarlo en el objeto de `:393-399`.
2.6 Reescribir `open()` (`quickmenu.rs:353-415`). Orden:
    1. Sin `LIFE`: si `ENDING`, `FORCED` o `is_open()` → `log("apertura cancelada: …")` y salir. Sacar `pids`.
    2. Sin `LIFE`: `platform::grab(&pids, scaled)` a una variable local (no tocar `FROZEN`/`BACKGROUND` todavía).
    3. `let _life = lock(&LIFE)`. Repetir las comprobaciones del paso 1 y que `active_pids` siga existiendo; si falla,
       salir. Ahora sí: `FROZEN`, `BACKGROUND`, `PREV_FG`.
    4. `magpie = magpie_scaling()`. `gen = GEN+1`; `PAINTED` no se toca (vale el `gen` viejo); `IS_OPEN = true`.
    5. Si `pause && !magpie`: `suspend` (como hoy, por los emuladores a pantalla completa).
    6. `emit_to(LABEL, "qm-open", state(app))` **antes** de mostrar, para que la página tenga el contenido listo.
    7. `let shown = show_window(app, pid, gen)`.
    8. `confirm(gen, pids, deadline)`: sondeo cada 20 ms hasta 1000 ms. Sale con `false` si `ENDING`, `FORCED` o
       `!is_current(gen)`. Éxito = `shown && PAINTED == gen && menu_on_top(...).0`. Si falla, **un reintento**: encolar
       `raise_menu` en el hilo principal y volver a sondear 300 ms.
    9. Si no se confirma:
       - log con los tres datos (`shown`, `painted`, `on_top` + motivo);
       - `teardown(app)` (reanuda si se pausó en el paso 5);
       - `emit qm-close`, `hide_window(app)`;
       - aviso: `let fb = feedback_pads(0); if fb == 0 { beep() } else { pad::rumble_n(fb, 2) }`;
       - salir. **Nunca queda el juego en pausa sin menú.**
    10. Si se confirma, `log("menu visible tras N ms (primer plano propio: …)")`. Si `pause && magpie`:
        - si el primer plano es nuestro (`platform::own_window_focused()`), esperar como mucho 300 ms a que
          `!magpie_topmost()` (solo para que Magpie termine su `IsWindowHung` con el juego vivo) y registrar el
          resultado;
        - volver a mirar `ENDING`/`FORCED`/`is_current`;
        - `suspend`;
        - `emit_to(LABEL, "qm-paused", json!({"paused": true}))`.
    11. `keep_on_top(app, gen)`, `pad::menu_nav(app.clone(), gen)`, `keyboard::start(app, gen)`.

    `LIFE` queda cogido como mucho ~2,2 s (0,8 + 1,3 + 0,3), siempre acotado. Ningún cierre del hilo principal toma
    `LIFE`, así que no hay interbloqueo nuevo.
2.7 Quitar el comentario muerto «Re-actualiza el estado de pausa…» (`:402`) y la línea de log «Magpie cedio» (la
    sustituye la del paso 10).

### Fase 3 — Fin de partida sin carreras (`quickmenu.rs`)

3.1 `prewarm` (`:261`): `ENDING.store(false)`, `SESSION.fetch_add(1)`.
3.2 `destroy` (`:275`): `ENDING.store(true)` **síncrono, antes** de lanzar el hilo.
3.3 `trigger_down` (`:119`): justo después de `active_pids`, `if ENDING || FORCED { log("atajo ignorado: la partida
    termina"); return }`. Así una pulsación que llega con el juego ya muerto no abre nada.
3.4 `destroy_now` (`:285-296`):
    - `let s = SESSION.load()`;
    - `close_now(app)`, limpiar `CONTEXT`/`SHOTS`;
    - `sleep(CLOSE_ANIM_MS + 1500 ms)`, para no destruir encima del show/hide;
    - encolar `w.destroy()` solo si `!is_open() && ENDING && SESSION == s`, porque una partida nueva reutiliza la
      ventana y no debe perderla;
    - log `ventana destruida`.
3.5 `force_close_game` (`:516-536`): sin cambios de lógica (ya pone `FORCED` lo primero). El `close()` final espera a
    `LIFE` como mucho ~2,2 s.
3.6 `suspend` (`:481`): no escribir el fichero ni suspender si `FORCED` (cinturón por si el paso 2.6.3 se escapa).

### Fase 4 — Gancho de teclado (`quickmenu.rs:599-709`)

4.1 `hook()`: primera línea dentro de `if code == HC_ACTION`: `if !super::is_open() { return CallNextHookEx(...) }`.
    Un gancho que se fugue ya no se traga teclas.
4.2 `start(app, gen)`:
    - empieza llamando a `stop()`;
    - guarda `WANT: AtomicU64 = gen`;
    - en el hilo: `PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE)` (crea la cola para que `PostThreadMessageW` funcione),
      `SetWindowsHookExW`, `THREAD.store(tid)` y log;
    - **luego** `if WANT.load() != gen { UnhookWindowsHookEx; THREAD.compare_exchange(tid, 0); return }`;
    - después, el bucle de mensajes de siempre.

    `stop()`: `WANT.store(0)` primero, luego el `swap` de `THREAD` y `PostThreadMessageW(WM_QUIT)`. Actualizar la
    versión `not(windows)` (`:711-716`) con la firma nueva.

## 4. Comprobación

Compilación (sin compilar la app entera no hace falta, pero sí esto):
```
cd src-tauri && cargo check && cargo test -p visor-core
node node_modules/svelte-check/bin/svelte-check --tsconfig ./tsconfig.json
node node_modules/vite/bin/vite.js build
```
En juego real (tsukihime_global con Magpie; un juego sin Magpie; un emulador a pantalla completa), mirando `quickmenu.log`:
1. Pulsar con Magpie: `hp: empieza/… hecho`, `menu visible tras N ms`, **después** la pausa. El menú se ve y responde a
   la cruceta y a las flechas. Pulsar otra vez: se cierra, el juego sigue y Magpie no se reinicia.
2. Si no se confirma: `no confirmado (shown/painted/on_top …)`, dos vibraciones, el juego **no** queda congelado.
3. Mantener con el menú cerrado hasta forzar: tras `cierre forzado pedido` no sale ninguna `captura de fondo` sin un
   `apertura cancelada` o un cierre inmediato. La app sigue viva. Al acabar, Enter/Esc/flechas funcionan en otras apps
   (gancho quitado: `gancho teclado quitado` en el log).
4. Juego sin Magpie / emulador: abre, pausa y cierra; el juego responde al volver (antes podía colgar el hilo principal).
5. Pulsar justo mientras el juego se cierra solo: `atajo ignorado: la partida termina`.
6. Si la app vuelve a cerrarse: leer `panic.log` y el Visor de eventos (ver dudas).

## 5. Riesgos y dudas abiertas

- **`requestAnimationFrame` puede no llegar** si WebView2 considera la ventana tapada. Saldría siempre «no confirmado»
  con `on_top: true`. El log lo dirá. Si pasa, se acepta `shown && on_top` cuando `painted` no llega pero la página
  respondió a `qm-open` (decisión a tomar con datos, no ahora).
- **Si Magpie se vuelve a subir encima sin parar** (cuando el juego conserva el primer plano porque Windows negó
  `SetForegroundWindow`, típico si el atajo viene del mando XInput y no del teclado), `menu_on_top` fallará y el menú no
  abrirá: aviso, sin congelar. Siguiente paso, si el log lo muestra: hacer el `AttachThreadInput` + `SetForegroundWindow`
  en un **hilo desechable** (nunca en el principal) con tiempo límite, y no suspender hasta que vuelva.
- Clic de ratón en el menú con Magpie y el juego ya pausado: el primer plano pasa a ser nuestro, Magpie hace
  `IsWindowHung` sobre un juego suspendido y podría dejar de escalar. Ya existía; vigilar.
- Con el menú cerrado, pulsar sigue tardando lo que tarde la captura (hasta varios segundos si WGC falla) antes de que
  salga nada. No se toca aquí.
- **Dudas para el usuario** (no deducibles del código):
  1. ¿El log pegado está completo? Si de verdad no hay ninguna línea `ventana mostrada` ni `vigilante: …`, el hilo
     principal estaba atascado (y el vigilante también, o la app murió antes). Pedir el fichero tal cual.
  2. ¿La app se cerró sola o Windows mostró «No responde» y se cerró desde ahí? Mirar Visor de eventos → Registros de
     Windows → Aplicación hacia las 18:30:50: evento **1000** (fallo, con módulo culpable) o **1002** (cuelgue).
  3. ¿El atajo se pulsó con el teclado o con una tecla F mandada por DS4Windows (`Src::Key`, entra por el hilo principal)
     o con el mando XInput (`Src::Pad`)? Lo dirá la línea nueva de 0.2.
  4. Versión de Magpie y si el juego se lanza elevado (cambia lo que cuenta el punto «Elevación/UIPI»).
