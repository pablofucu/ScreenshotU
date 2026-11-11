# Plan de resolución: fallo del overlay del visor (menú sobre el juego)

> **Estado:** Documento de diseño técnico y plan de resolución detallado para el agente principal (Sonnet).  
> **Fecha:** 3 de noviembre de 2026.  
> **Problema reportado:** Al pulsar el atajo del overlay sobre un juego (con Magpie escalando), la pantalla del juego se congela (pausa), pero no aparece el menú sobre la pantalla («no sale nada más»).

---

## 1. Diagnóstico técnico en profundidad

### 1.1 Evidencia del registro real (`quickmenu.log`)

En una ejecución real sobre `STEINS.GATE.REBOOT` (`sgre_steam.exe`) con escalado Magpie activo:
```text
17:01:17.319 partida empezada
17:01:22.449 atajo pulsado (menu abierto: false)
17:01:22.450 pantalla completa D3D: true; Magpie escalando: true; exclusiva: false
17:01:30.450 atajo pulsado (menu abierto: true)
17:02:07.986 cierre forzado pedido; pids: [16508]
17:02:07.994 terminate 16508 (P:\Visual Novels\STEINS.GATE.REBOOT\SGRE\sgre_steam.exe): ok
```

De este registro se desprenden hechos concluyentes:
1. **La pantalla se congela intencionadamente:** A las 17:01:22 se ejecuta `open(app)`. Al estar activo `overlay_pause` (por defecto `true`), se invoca `proc::suspend(pid)` (`NtSuspendProcess` sobre el PID del juego). Los hilos del juego se suspenden y este deja de pintar fotogramas. La pantalla se congela para pausar la partida mientras el menú está abierto.
2. **El estado del menú cambia a `open: true`:** A las 17:01:30, la segunda pulsación registra `menu abierto: true`. Para Rust el menú estaba abierto y en marcha.
3. **El usuario no vio nada:** A las 17:02:07 el usuario mantuvo el atajo para forzar el cierre de la partida porque la pantalla quedó congelada sin mostrar el menú.

---

### 1.2 Análisis causa-raíz: ¿Por qué no sale nada en pantalla?

El fallo no se debe a un único factor, sino a la convergencia de cuatro problemas en la arquitectura de ventanas y ciclo de vida de Tauri/WebView2:

#### Causa A: Incompatibilidad de `WS_EX_NOACTIVATE` y `SWP_NOACTIVATE` con Magpie y DirectFlip
1. **La ventana de Magpie (`Magpie_ScalingWindow`):**
   - Se ejecuta normalmente con privilegios elevados de Administrador (High Integrity Level). El visor corre sin elevar (Medium Integrity Level).
   - Crea una ventana a pantalla completa con Direct3D 11 swapchain configurada para presentación directa (`DirectFlip` / `Independent Flip` / `MPO`). Bajo este modelo, el compositor DWM de Windows entrega el escaneo directamente al controlador de pantalla, saltándose la composición tradicional del escritorio.
2. **El error de diseño con `WS_EX_NOACTIVATE`:**
   - La ventana de Tauri se configuró con `crate::overlay::no_activate_clickable`, que le añade el estilo Win32 `WS_EX_NOACTIVATE`.
   - Se muestra con `crate::overlay::show_quiet(&win)` pasando la bandera `SWP_NOACTIVATE`.
   - **Regla de oro de Z-Order en Windows:** Entre dos ventanas con `WS_EX_TOPMOST`, la ventana activa que ostenta el foco en primer plano (`Foreground Window`) se sitúa **siempre** por encima de las ventanas inactivas.
   - Al estar Magpie activa en primer plano a pantalla completa y `quickmenu` marcada como inactiva sin foco (`WS_EX_NOACTIVATE`), Windows DWM sitúa a `quickmenu` **por detrás** de la ventana de Magpie.
   - El bucle `keep_on_top` llama a `SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE)` cada 250 ms, pero al pasar `SWP_NOACTIVATE`, Windows no altera la precedencia de la ventana activa en primer plano.
   - Además, por aislamiento de privilegios de interfaz (UIPI), una ventana no elevada inactiva no puede imponerse sobre una ventana elevada activa en primer plano.

#### Causa B: Supresión del renderizado de WebView2 por eludir la API de Tauri (`show_quiet`)
1. En `quickmenu.rs`, la ventana se crea con `.visible(false)`.
2. En Tauri 2 / Wry (Windows), cuando una ventana se crea con visibilidad falsa, el controlador COM interno de WebView2 se inicializa en estado oculto (`ICoreWebView2Controller::put_IsVisible(FALSE)`).
3. Cuando `show_window` quiere mostrarla, invoca `crate::overlay::show_quiet(&win)` (un `SetWindowPos(SWP_SHOWWINDOW)` directo sobre el HWND del contenedor de Windows), saltándose el método oficial `win.show()` de Tauri.
4. Al no llamarse `win.show()`, Wry nunca llama a `put_IsVisible(TRUE)`.
5. En WebView2, mientras `put_IsVisible` sea `FALSE`, **el pipeline de rasterizado y pintado por GPU está suspendido**. No genera fotogramas.
6. Como la ventana de Tauri tiene `.transparent(true)`, el contenedor Win32 es transparente y WebView2 no dibuja nada. La ventana resultante es **100% invisible y transparente**.

#### Causa C: Detección errónea del monitor cuando Magpie está escalando (`game_monitor`)
1. `show_window` calcula el monitor mediante `crate::reader::monitor::game_monitor(pid)` usando el PID del juego original (`sgre_steam.exe`).
2. Cuando Magpie está escalando, la ventana que ve el usuario cubriendo la pantalla es la de `Magpie.exe`. La ventana del juego original suele ser movida, minimizada o relegada a segundo plano por Magpie.
3. Si `game_window(pid)` no encuentra la ventana del juego o no detecta su visibilidad, recurre al monitor principal de Windows (`MONITOR_DEFAULTTOPRIMARY`).
4. En configuraciones multimonitor (muy frecuentes en juegos en TV/segunda pantalla), el menú se dimensiona y coloca en el Monitor 1 mientras el juego y Magpie están en el Monitor 2.

#### Causa D: Asincronía y pérdida de eventos (`qm-open`) vs carga de WebView2
1. `open(app)` emite `app.emit_to("quickmenu", "qm-open", state(app))` en un hilo en segundo plano al mismo tiempo que despacha `show_window` al hilo principal.
2. En `data.ts`, `setupQuickMenuListeners` utilizaba una importación dinámica asíncrona (`import('@tauri-apps/api/event').then(...)`).
3. Si la ventana tarda en cargarse o el import dinámico se resuelve después de la emisión, el evento `qm-open` se pierde y la interfaz nunca recibe los datos del juego ni la captura congelada de fondo.

---

## 2. Decisiones de diseño y arquitectura

### 2.1 ¿Se debe ocultar, minimizar o suspender Magpie? NO.
- **Inviabilidad técnica por elevación (UIPI):** `Magpie.exe` suele ejecutarse como Administrador. El visor se ejecuta sin elevar. Las llamadas Win32 como `ShowWindow(magpie_hwnd, SW_HIDE)`, `ShowWindow(magpie_hwnd, SW_MINIMIZE)` o `NtSuspendProcess(magpie_pid)` fallan con `ERROR_ACCESS_DENIED` desde un proceso no elevado.
- **Destrucción de la cadena de intercambio (swapchain):** Minimizar la ventana de escalado de Magpie destruye su swapchain Direct3D y reinicia shaders y hooks, provocando parpadeos o fallos al restaurar.
- **La solución limpia:** Magpie debe permanecer intacto debajo. Antes de pausar el juego, el visor ya toma la captura escalada con `platform::grab(&pids, scaled)` y la coloca como fondo visual en `BACKGROUND` / `.qm-bg`. Al superponer el menú con foco propio, Magpie queda cubierto sin alterarse.

### 2.2 ¿`quickmenu` debe tomar foco y activarse (`SetForegroundWindow`)? SÍ.
- `quickmenu` no es un indicador pasivo como `recindicator` (que no debe robar el foco mientras se juega). Es un **menú de pausa modal a pantalla completa**.
- El juego ya está suspendido (`NtSuspendProcess`), por lo que perder el foco no le afecta: no puede procesar mensajes de pérdida de foco, no desactiva audio y no minimiza emuladores mientras esté suspendido.
- Al otorgar a `quickmenu` la activación en primer plano (`SetForegroundWindow`):
  1. Windows DWM suspende el modo DirectFlip de Magpie y compone el escritorio de forma fluida.
  2. Al ser una ventana `HWND_TOPMOST` activa y en primer plano, la jerarquía de Z-order de Windows la sitúa garantizadamente por encima de Magpie y de cualquier juego a pantalla completa sin bordes.
  3. WebView2 recibe el foco nativo y activa el pintado por GPU y los eventos de ratón/teclado.
- Para garantizar que `SetForegroundWindow` funcione desde un hilo en segundo plano (cuando el atajo proviene de un mando XInput), se utiliza la técnica canónica Win32 de acoplamiento de hilos (`AttachThreadInput` con el hilo de `GetForegroundWindow()`).

### 2.3 Recordar y restaurar la ventana previa al cerrar
- Al abrir el menú, se guarda el identificador de la ventana que estaba activa (`prev_fg = GetForegroundWindow()`).
- Al cerrar el menú (`close` / `teardown`), se reanudan los procesos del juego (`NtResumeProcess`), se oculta la ventana del menú con `win.hide()` y se devuelve el foco inmediatamente a `prev_fg` con `SetForegroundWindow`. El juego o Magpie recuperan el control al instante sin clics adicionales.

### 2.4 Detección del monitor prioritario
- En la función que calcula las dimensiones y posición del monitor para el menú (`target_monitor` / `game_monitor`), se comprueba si Magpie está escalando:
  - Si `platform::find_magpie()` devuelve un `HWND` visible, se obtiene el monitor a partir de esa ventana (`MonitorFromWindow`).
  - Si no, se utiliza la ventana principal del juego.
  - Si ninguna está disponible, se recurre al monitor principal.

### 2.5 Secuencia de suspensión con Magpie (`ScalingWindow.cpp` / `IsWindowHung`)
- **Mecanismo de detección de cuelgue en Magpie:** Al detectar un cambio en la ventana en primer plano (`_UpdateFocusStateAsync`), Magpie comprueba internamente `Win32Helper::IsWindowHung` enviando un `SendMessageTimeout(WM_NULL, 500ms)` a la ventana del juego que está escalando.
- **Riesgo de suspensión anticipada:** Si el proceso del juego se suspende con `NtSuspendProcess` antes de que Magpie procese el cambio de foco, el hilo del juego no responderá a `WM_NULL`, expirando el tiempo límite de 500 ms y provocando que Magpie considere la ventana colgada o detenga su escalado.
- **Secuencia estricta de apertura:**
  1. Tomar captura escalada del juego/Magpie (`platform::grab`).
  2. Primero se muestra y da foco a `quickmenu` (`win.show()`, `win.set_focus()`, `force_foreground`).
  3. Se espera hasta 200 ms a que Magpie quite `WS_EX_TOPMOST` de su ventana de escalado (pasando a `HWND_NOTOPMOST` de forma limpia y sin detener el escalado). Durante esta transición, el juego aún responde con normalidad a `WM_NULL`.
  4. **Después** se suspende el proceso del juego (`NtSuspendProcess`).
- **Secuencia estricta de cierre:**
  1. En `teardown`, se reanuda primero el proceso del juego (`NtResumeProcess`).
  2. En `hide_window`, se oculta la ventana del menú (`win.hide()`) y se devuelve el foco a `PREV_FOREGROUND` (`SetForegroundWindow`).
  3. De este modo, el juego ya responde a `WM_NULL` cuando Magpie recupera el primer plano.

### 2.6 Guardia `own_window_focused()` en `pad.rs` y `mouse.rs`
- Tradicionalmente, `platform::own_window_focused()` evitaba que los atajos de mando o ratón se dispararan cuando una ventana del propio visor tenía el foco (para no interferir mientras se navega por la biblioteca del visor).
- Como ahora `quickmenu` tiene el foco en primer plano (y pertenece al PID del visor), `own_window_focused()` devuelve `true` mientras el menú está abierto.
- Por tanto, `pad.rs:430` y `mouse.rs:62` deben permitir `trigger_down` cuando:
  ```rust
  !platform::own_window_focused() || crate::quickmenu::is_open()
  ```
  Esto asegura que el atajo de mando o ratón pueda cerrar el menú o forzar el cierre de la partida manteniendo pulsado el atajo durante 1,5 s.

---

## 3. Plan detallado de implementación

### Fase 1: Monitor y Magpie (`src-tauri/src/platform.rs` y `src-tauri/src/reader/monitor.rs`)

1. **Exponer la ventana de escalado de Magpie:**
   - En `src-tauri/src/platform.rs`, hacer pública la función que obtiene el `HWND` de Magpie:
     ```rust
     #[cfg(windows)]
     pub fn magpie_window() -> Option<windows::Win32::Foundation::HWND> {
         imp::find_magpie()
     }
     #[cfg(not(windows))]
     pub fn magpie_window() -> Option<isize> {
         None
     }
     ```
2. **Ajustar la resolución de monitor para el menú:**
   - En `src-tauri/src/reader/monitor.rs`, modificar `game_monitor` (o crear `target_monitor(game_pid: Option<u32>) -> Rect`) para que:
     - Primero consulte `crate::platform::magpie_window()`. Si existe, obtenga su `MonitorFromWindow`.
     - Si no hay Magpie, busque la ventana del proceso `game_pid`.
     - Devolver el `Rect` del monitor completo correspondiente.

### Fase 2: Ventana, Activación y Foco (`src-tauri/src/quickmenu.rs`)

1. **Eliminar `WS_EX_NOACTIVATE` del menú:**
   - En `build(app)`:
     - Dejar de llamar a `crate::overlay::no_activate_clickable(&w)`.
     - Configurar estilos limpios para el menú: aplicar `WS_EX_TOOLWINDOW` (para no salir en Alt+Tab) y `SetWindowDisplayAffinity(..., WDA_EXCLUDEFROMCAPTURE)` (para que no salga en capturas de pantalla completas). Nunca añadir `WS_EX_NOACTIVATE`.
2. **Gestión de la ventana previa en primer plano:**
   - Crear un estático seguro:
     ```rust
     static PREV_FOREGROUND: Mutex<Option<windows::Win32::Foundation::HWND>> = Mutex::new(None);
     ```
   - En `open(app)`, antes de suspender y mostrar, capturar la ventana actual:
     ```rust
     #[cfg(windows)]
     {
         let fg = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
         if !fg.is_invalid() {
             *lock(&PREV_FOREGROUND) = Some(fg);
         }
     }
     ```
3. **Muestra y activación fiable (`show_window` y `force_foreground`):**
   - Sustituir `crate::overlay::show_quiet(&win)` por:
     ```rust
     let _ = win.show();
     let _ = win.set_focus();
     force_foreground(&win);
     ```
   - Implementar `force_foreground(win: &tauri::WebviewWindow)`:
     ```rust
     #[cfg(windows)]
     fn force_foreground(win: &tauri::WebviewWindow) {
         use windows::Win32::Foundation::HWND;
         use windows::Win32::UI::WindowsAndMessaging::{
             BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
             SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
         };
         use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};

         if let Ok(h) = win.hwnd() {
             let hwnd = HWND(h.0);
             unsafe {
                 let fg = GetForegroundWindow();
                 let fg_thread = GetWindowThreadProcessId(fg, None);
                 let cur_thread = GetCurrentThreadId();
                 if fg_thread != 0 && fg_thread != cur_thread {
                     let _ = AttachThreadInput(cur_thread, fg_thread, true);
                     let _ = SetForegroundWindow(hwnd);
                     let _ = BringWindowToTop(hwnd);
                     let _ = AttachThreadInput(cur_thread, fg_thread, false);
                 } else {
                     let _ = SetForegroundWindow(hwnd);
                     let _ = BringWindowToTop(hwnd);
                 }
                 let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
             }
         }
     }
     ```
4. **Secuencia de apertura y suspensión compatible con Magpie (`open`):**
   - Magpie comprueba `Win32Helper::IsWindowHung` (`SendMessageTimeout(WM_NULL, 500ms)`) justo al detectar el cambio de ventana en primer plano (`_UpdateFocusStateAsync`).
   - Por tanto, con Magpie escalando, el orden en `open(app)` es:
     1. Capturar ventana previa: `*lock(&PREV_FOREGROUND) = Some(GetForegroundWindow())`.
     2. Mostrar ventana y otorgar foco en primer plano: `win.show()`, `win.set_focus()`, `force_foreground(&win)`.
     3. Esperar hasta 200 ms a que Magpie quite `WS_EX_TOPMOST` de su ventana de escalado (pasando a `HWND_NOTOPMOST` sin detener el escalado). Durante esta espera, el juego sigue activo y responde a `WM_NULL`.
     4. **Después** suspender el proceso del juego (`proc::suspend(pid)` / `NtSuspendProcess`).
     5. Emitir `qm-open` desde el hilo principal con WebView2 activo.
5. **Secuencia de cierre y restauración (`hide_window` y `teardown`):**
   - En `teardown`: reanudar **primero** el proceso del juego (`proc::resume(pid)` / `NtResumeProcess`) para que el juego ya responda a `WM_NULL` cuando Magpie recupere el primer plano.
   - En `hide_window`: llamar a `win.hide()` de Tauri y restaurar el foco a `PREV_FOREGROUND`:
     ```rust
     #[cfg(windows)]
     if let Some(prev) = lock(&PREV_FOREGROUND).take() {
         unsafe {
             let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(prev);
         }
     }
     ```

### Fase 2b: Guardia `own_window_focused()` en mando y ratón (`src-tauri/src/reader/pad.rs` y `src-tauri/src/reader/mouse.rs`)

1. **Atajo en mando (`src-tauri/src/reader/pad.rs:430`):**
   - Como ahora `quickmenu` tiene el foco en primer plano (y pertenece al PID del visor), `own_window_focused()` devuelve `true`.
   - Ajustar la comprobación para permitir `trigger_down` cuando `!platform::own_window_focused() || crate::quickmenu::is_open()`.
   - Esto permite que el atajo de mando pueda cerrar el menú o forzar el cierre manteniendo pulsado 1,5 s.
2. **Atajo en ratón (`src-tauri/src/reader/mouse.rs:62`):**
   - Aplicar la misma condición `!platform::own_window_focused() || crate::quickmenu::is_open()` para que el atajo de ratón funcione cuando el menú tiene el foco.

### Fase 3: Robustez en el Frontend (`src/quickmenu/data.ts` y `src/quickmenu/QuickMenu.svelte`)

1. **Importación estática en `src/quickmenu/data.ts`:**
   - Añadir `import { listen } from '@tauri-apps/api/event';` al inicio del módulo.
   - En `setupQuickMenuListeners`, sustituir la importación dinámica `import('@tauri-apps/api/event').then(...)` por llamadas directas a `listen(...)` registrando un helper `const reg = (p: Promise<() => void>) => { p.then((u) => { if (cleaned) u(); else unlistens.push(u); }); };` para los 6 eventos (`qm-open`, `qm-close`, `qm-pad`, `qm-hold`, `qm-shot`, `qm-perf`).
2. **Sincronización activa y prevención de doble entrada en `src/quickmenu/QuickMenu.svelte`:**
   - En `handleKeydown`: añadir al inicio `if (isTauri()) return;` porque en Tauri el gancho `WH_KEYBOARD_LL` de Rust (`keyboard::start`) ya intercepta las teclas y emite `qm-pad`, evitando que ahora que la ventana tiene foco se procesen dos veces en la webview.
   - En `onMount` y `onDestroy`: añadir una función `syncIfOpen()` que llame a `fetchQuickmenuState().then((init) => { if (init.open) onOpenState(init); })`. Invocarla en `onMount`, registrarla en `window.addEventListener('focus', syncIfOpen)` y eliminarla en `onDestroy` con `window.removeEventListener('focus', syncIfOpen)`.

---

## 4. Matriz de verificación y pruebas

| Escenario | Comportamiento esperado |
|---|---|
| **Juego con Magpie activo (D3D fullscreen simulado)** | Al pulsar el atajo, el juego se pausa y el menú aparece inmediatamente en primer plano con el fotograma escalado de fondo. La navegación con cruceta/teclado responde al momento. Al pulsar B o cerrar el menú, el menú se oculta y Magpie recupera el foco sin reiniciarse. |
| **Juego en ventana / sin bordes sin Magpie** | El menú se abre en primer plano en el monitor del juego. Al cerrar, el juego recupera el foco sin retrasos. |
| **Configuración multimonitor** | Si el juego o Magpie están en el Monitor 2, el menú se posiciona y dimensiona en el Monitor 2. |
| **Cierre forzado manteniendo el atajo (1,5 s)** | Sigue funcionando exactamente igual: vibración creciente, `proc::terminate`, log de cierre y guardado de sesión sin bloqueos. |
| **Pausa desactivada (`overlay_pause = false`)** | El juego no se suspende; el menú se abre en primer plano sobre el juego en ejecución. |

### Comprobaciones de compilación:
```bash
cd src-tauri && cargo check && cargo test -p visor-core
node node_modules/svelte-check/bin/svelte-check --tsconfig ./tsconfig.json
node node_modules/vite/bin/vite.js build
```

---

## 5. Resumen de límites y reglas del proyecto

- **Las categorías no se tocan:** Juegos, Novelas visuales, Switch, Novelas japonesas y sus carpetas se mantienen exactamente iguales.
- **Finales de línea:** Conservar finales de línea LF en los ficheros modificados.
- **Sin commit ni push por parte del planificador:** La ejecución y commits corresponden al agente principal tras revisar el plan.
- **Sin parchear dependencias:** Todo se resuelve dentro de las APIs oficiales de Tauri 2 y Win32.
