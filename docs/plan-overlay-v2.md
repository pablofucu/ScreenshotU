# Plan: overlay sobre el juego · versión 2 de la propuesta A «Plaza»

> **Estado:** diseño cerrado salvo lo que queda abierto en el §3 (el usuario ya contestó el resto el 3-nov-2026). Aún no hay nada implementado (no existe `quickmenu.rs`). Este documento **sustituye** a los §5 y §7 de [plan-overlay.md](plan-overlay.md) y **amplía** sus §1 (A) y §4. Lo que no se nombra aquí sigue igual que en el plan v1: ventana `quickmenu` sin foco (§4.1), foto congelada como fondo (§4.2), pausa por suspensión y entrada por hilo XInput + gancho de teclado (§4.3), fuentes de datos de rendimiento, batería y audio (§4.4) y comprobación general (§6).

Mockup: `docs/mockups-overlay/overlay-a2.html` (1280×720, autocontenido, datos falsos). Barra de arriba: estados, **Datos** (novela con hucha / novela sin hucha / juego sin racha), **fondo claro/oscuro** y **modo Normal/Exclusiva**. Teclado: `Espacio` = atajo del menú (pulsar o **mantener 1,5 s**), flechas = cruceta, `Intro` = A, `Esc` = B, `X` = captura, `Y` = favorita, `Q`/`E` = LT/RT. Ratón: pasar enfoca, clic acciona, rueda sobre Audio = volumen, clic derecho = B. Estado `#ajustes`: las filas nuevas de Ajustes y su aprendizaje.

**Categorías:** nada cambia en plataformas, carpetas ni detección. El overlay solo enseña el juego en marcha y, si su categoría es una de las de lectura (`vn` o `jvn`), la racha **de esa categoría**; nunca mezcla datos de otra.

## 1. Qué cambia respecto a la v1

| Tema | v1 (`overlay-a.html`) | v2 (`overlay-a2.html`) |
|---|---|---|
| Rejilla | 4×3 filas iguales; Logros 2×2 | 4×3 con filas `1fr 1fr .78fr`. Fila 1: Rendimiento (2) · Tiempo y racha (2). Fila 2: Capturas (2) · Audio (2). Fila 3: Logros (1) · Mando (1) · Salir (2) |
| Logros | Baldosa 2×2 con anillo grande y 3 últimos | **1×1 compacta**: anillo pequeño 23/48, último conseguido y el más cercano. A abre la hoja (sin cambios) |
| Tiempo | 1×1: sesión y total | **2×1** con sesión/hoy/total + **racha y hucha** en novelas, o **semana y sesión récord** en el resto (§2.2) |
| Capturas | Tira de 4 miniaturas; A abre la hoja | Botón **Capturar** (o X en cualquier sitio) + 4 miniaturas enfocables: A = ver en la hoja, **Y = favorita** |
| Audio | 1×1 informativa; todo en la hoja | **2×1 interactiva**: silencio, barra de volumen (← → / arrastrar / rueda), chips de salida, silencio del micro, «Más…» |
| Rendimiento | 2×1 con dos gráficas | **Igual** (decisión del usuario) |
| Mando | 1×1 | Igual |
| Salir | Baldosa que abre una hoja con Cerrar / Forzar | **2×1 con los dos botones directos**: Cerrar = un clic + 3 s para deshacer; Forzar = mantener 1,5 s. Se quita la hoja «Salir» |
| Cabecera | Juego + hora | + píldora «Juego en pausa» si está suspendido |
| Atajo | Una tecla (F24) | **Tres disparadores a la vez**: teclado, mando (botón o combinación XInput, también el botón Xbox) y ratón. Pulsar = abrir/cerrar; **mantener 1,5 s = forzar cierre** |
| Pantalla completa exclusiva | El atajo vibra 2 veces y ya | Pulsación corta = 2 toques; **mantener 1,5 s = forzar cierre** con toques crecientes y una vibración larga |
| Ajustes | 2 filas en General | 5 filas en Ajustes › Mando (§4) |
| Datos de tiempo | `st.stats` en Rust | **Contexto del juego** que manda la ventana principal al empezar la partida y la página del menú calcula con `readgoals.ts` (§5.3) |

## 2. Decisiones cerradas (propuesta)

### 2.1 Logros compactos
1×1 (≈266×143): `ring` arcoíris de 62 px con «23 de 48», nombre y hora del último conseguido y «Cerca: Cartógrafo 70 %». Etiqueta «+3 hoy». Sin logros: baldosa atenuada, no enfocable, texto «Este juego no tiene logros». La hoja de logros no cambia.

### 2.2 Tiempo de juego y racha
Mismo lenguaje que `ReadRings.svelte`, `HuchaMini.svelte`, `HuchaRescate.svelte` y `Streak.svelte`, pasado a cristal oscuro (superficies translúcidas en lugar de `--face`, texto blanco; **los colores no cambian**: `GOAL_COL` azul `#00A0D8` / rosa `#E5487A` para el anillo y la llama, `RESCUE_COL` ámbar `#E9A100` / violeta `#6E56CF` para la hucha, `readgoals.ts:7-15`).

Siempre, a la izquierda: «Esta sesión» grande, «Hoy» y «Total».

**a) Juego de una categoría de lectura (`vn`/`jvn`) con rescate pendiente** (`GoalTrack.rescue != null`):
- Anillo de hoy (104 px, trazo 11) en el color de la categoría: minutos de hoy / meta («38m de 1 h»).
- Medalla como `.rmed` de ReadRings: llama del color de la categoría, número de racha y debajo `pendiente` (en el color del rescate).
- Hucha mini idéntica a HuchaMini: «RESCATE 14/25 min», casillas de 1, 5 o 10 min (misma fórmula, `HuchaMini.svelte:27-34`), macizo = días anteriores, claro = hoy, pie «quedan 9 h · mié 7:00» (`rescueLeft` / `rescueClose`).
- Etiqueta de la baldosa con el texto de Streak (`Streak.svelte:74-81`): «Faltan 22 min para seguir la racha», o «✓ Cumplida a las 21:12 · +5 min a la hucha».
- Hoja «Tiempo y racha»: la hucha completa como HuchaRescate `wide`: una casilla por minuto, «Domingo 18: 35 de 60 min · 1 casilla = 1 min que faltó», «Recuperado 14 de 25 min», partidas de la ventana (las de menos de 10 min tachadas con «no cuenta»), «Cierra en 9 h · miércoles 22 a las 7:00 · Hoy: 33 min más en partidas de 10 min o más», «Rescates · 30 días 1 de 2».

**b) Categoría de lectura sin rescate pendiente (la hucha no aplica):** el hueco de la hucha lo ocupa el **mapa de semanas** (10 columnas de 8 px, como `ReadRings.svelte:79-88`: días de racha con borde, hoy marcado) y «13 días seguidos · sin rescates pendientes». Medalla con `récord` o `mejor N`. Es exactamente lo que hace ReadRings (`{#if t.rescue} HuchaMini {:else} rmap`). Hoja: anillo grande, medalla y mapa de 14 semanas.

**c) Juego de cualquier otra categoría (Juegos, Switch…):** no hay racha (las rachas son solo de lectura; no se inventa otra). En su lugar: barras de los 7 días (`Title.week`, hoy resaltado, sumando la sesión en curso) y tarjeta «Sesión récord 3 h 12 · te falta 1 h 48 para batirla» (`titleStats(t).longest`, `stats.ts:148`); si la sesión la supera, «¡Nueva sesión récord!». Hoja: lo de la v1 (semana) + récord.

**d) Rescate pagado hoy** (`paidDay` = hoy): la etiqueta dice «¡Hucha llena! El domingo 18 cuenta · 13 días» (texto del aviso de Streak, `Streak.svelte:55`). El aviso sobre el juego ya lo da `goal_notice`; el overlay no repite sonido.

Se recalcula cada minuto mientras está abierto (la partida en curso cuenta hasta «ahora», como en Inicio).

### 2.3 Controles directos en la pantalla principal
Regla: lo que se hace a menudo y con un solo gesto va en la baldosa; las hojas quedan para listas largas y ajustes poco frecuentes.

| Baldosa | Directo en la baldosa | En la hoja |
|---|---|---|
| Rendimiento | — (A abre) | Gráficas de 2 min (igual) |
| Tiempo y racha | — (A abre) | Detalle §2.2 |
| Capturas | **Capturar**; miniaturas: A = abrir en la hoja con ella elegida, **Y = favorita** (estrella) | Vista grande, 6 miniaturas, favorita, «Abrir en el visor» (si se aprueba, v1 §7.5) |
| Audio | **Silencio**, **volumen** (barra), **salida** (un chip por dispositivo, hasta 3; si hay más, «Más…»), **silenciar micro** | Entrada (micrófono), resto de salidas, opciones |
| Logros | — (A abre) | Lista con pestañas (igual) |
| Mando | — (A abre) | Batería y «Vibrar para reconocerlo» (igual) |
| Salir | **Cerrar juego** (un clic) y **Forzar cierre** (mantener) | Ya no hay hoja |

Manejo:
- **Mando:** la cruceta/stick se mueve entre todo lo enfocable (baldosas y controles de dentro; mismo `pick()` espacial del mockup). En la barra de volumen ← → ajustan ±2 % y ↑ ↓ salen. Globales en el inicio: **LT/RT volumen** (±5 %), **X captura**, B vuelve al juego. Y solo sobre una miniatura. La barra de ayuda de abajo cambia según el control enfocado.
- **Teclado** (gancho `WH_KEYBOARD_LL`, solo con el menú abierto): flechas, Intro (A), Esc/Retroceso (B), Tab como en la v1, y además `X` (captura), `Y` (favorita), `Q`/`E` y `+`/`−` del teclado numérico (volumen). Todas se tragan mientras el menú está abierto; el resto pasan.
- **Ratón:** pasar enfoca, clic acciona, clic/arrastre en la barra fija el volumen, rueda sobre la baldosa de audio cambia el volumen, **clic derecho = B**, clic fuera de una hoja = volver.
- Capturar desde el menú **no cierra el menú**: destello blanco y la miniatura nueva entra la primera con un rebote.

### 2.4 Cerrar con un clic
- «Cerrar juego» (A o un clic) **no actúa al momento**: el botón pasa a una cuenta atrás de 3 s (anillo que se vacía) y arriba sale «Cerrando *juego* en 3 s · Deshacer». Mientras tanto el juego sigue en pausa: deshacer es real porque aún no se ha enviado nada. Cancelan: B/Esc, clic en «Deshacer», A/clic sobre el propio botón, o cerrar el menú con el atajo. A los 3 s: reanudar, ocultar y `WM_CLOSE` (v1 §4.4).
- Por qué así y no un botón suelto sin confirmación: con navegación espacial es fácil pulsar A en la baldosa equivocada; los 3 s con el juego congelado protegen sin pedir un segundo gesto. El botón vive en la esquina inferior derecha, lejos de lo que se usa a menudo, y «Forzar» va al lado pero **solo funciona manteniendo** 1,5 s (gesto de la v1).
- Descartado: «clic + diálogo de confirmación» (es la confirmación larga que el usuario no quiere) y «doble clic» (con mando no existe).

### 2.5 Atajo configurable, mantener y pantalla completa exclusiva
**Disparadores** (modelo en §4): teclado, mando y ratón, los tres activos a la vez, como ya pasa con la captura (`capture_hotkey` / `capture_combo` / `capture_mouse`). Cualquiera abre y cierra el mismo menú.

**Una máquina de estados para los tres** (en Rust, pura y con tests, §5.1). Tiempos: `TAP_MS = 400`, `HOLD_MS = 1500`.

| Situación al pulsar | Al pulsar | Soltar antes de 400 ms | Mantener 400 ms–1,5 s | Llegar a 1,5 s |
|---|---|---|---|---|
| Menú cerrado y puede salir | **Abre al instante** | Nada más (queda abierto) | Barra arriba «Forzando el cierre… suelta para cancelar» que se llena (y el botón Forzar se llena a la vez); soltar = la barra desaparece, el menú sigue abierto | **Forzar cierre** |
| Menú abierto | Nada todavía | **Cierra** el menú | Igual: barra que se llena | **Forzar cierre** |
| Pantalla completa exclusiva (no puede salir) | Nada | **2 toques** de vibración («aquí no se puede abrir») | Toques suaves cada vez más seguidos (de cada 260 ms a cada 90 ms) | **Forzar cierre** + vibración larga (400 ms, los dos motores) |
| Sin juego del visor en marcha | Lo que se decida en v1 §7.4 | — | — | Nada (no hay pids) |

- «Mantener = forzar cierre» es el mismo gesto siempre (como mantener el botón de encendido de una consola), así funciona aunque la detección de exclusiva falle y el menú no se vea. Se puede desactivar (`overlay_hold_force`).
- Vibración: si disparó el mando, solo ese mando (máscara `pressing` de `pad.rs:249-261`); si fue el teclado o el ratón, todos los mandos conectados (con DS4Windows la «tecla» viene de un mando). Sin mando, `MessageBeep` en el aviso y al forzar.
- Mientras el disparador del mando está pulsado, el hilo de navegación ignora sus botones (para que una combinación como `back+start` no navegue).

## 3. Decisiones del usuario (3-nov-2026) y lo que sigue abierto

**Cerradas** (mandan sobre el resto del documento si hay contradicción):

| Tema | Decisión |
|---|---|
| Cerrar con un clic | Es el cierre **forzado** (`TerminateProcess` de los pids del juego), no `WM_CLOSE`. Se mantiene la cuenta atrás de 3 s con «Deshacer» del §2.4 como protección (el juego sigue en pausa), salvo que el usuario pida quitarla. «Forzar» mantenido 1,5 s sigue como versión inmediata. |
| Mantener el atajo | **Fuerza el cierre siempre** (menú visible o no, y en exclusiva). |
| Segundos de mantener | **Ajustable**, por defecto **1,5 s** (opciones 1 / 1,5 / 2 / 3 s). `overlay_hold_ms` sustituye a la constante `HOLD_MS` (se pasa a `TriggerFsm`). |
| «Marcar» en capturas | **Borrar**, no favorita. Y = borrar la captura enfocada, por el mismo camino que el visor (`trash_shots`, `lib.rs:668`, a la papelera), con un «Deshacer» de unos segundos mientras no se haya vaciado. Desaparece `favorites-changed` y `set_favorite` de este plan. |
| Juegos sin racha | Se queda **semana + sesión récord**, como en el mockup. |
| Pausa | **Se pausa el juego** al abrir y se reanuda al cerrar el menú (por defecto sí, `overlay_pause`). Los juegos que no se puedan pausar los gestiona el usuario por su cuenta (no hace falta ajuste por juego). |
| Sin juego en marcha | **No hace nada** (sin menú reducido). |
| Volumen por aplicación | **Sí, en la hoja ampliada de Audio**: volumen del juego y de otras aplicaciones por separado (mezclador por aplicación). La baldosa principal mantiene el volumen general. |

**Siguen abiertas** (no bloquean la Fase 1):
1. ¿Ocultar la hucha pendiente de la otra categoría de lectura (propuesto: sí, no mezclar categorías)?
2. Botón Xbox como atajo: choca con Game Bar y Steam (por defecto el disparador del mando va desactivado).
3. ¿Teclas multimedia como atajo? (ampliar `keyName`).
4. De la v1: §7.3 (¿DualSense y HidHide? afecta a la batería) y §7.5 («Abrir en el visor» desde una captura).
5. ¿Se quita la cuenta atrás de 3 s ahora que cerrar es forzado?

## 4. Modelo de ajustes y de atajos

### 4.1 Config (`src-tauri/visor-core/src/config.rs`)
Junto a `audio_hotkey` (`:107-111`), con `#[serde(rename_all = "camelCase", default)]` ya puesto:

| Campo Rust | Campo TS | Tipo | Por defecto | Notas |
|---|---|---|---|---|
| `overlay_hotkey` | `overlayHotkey` | String | `"F24"` | "" = desactivado. Mismo formato que `capture_hotkey` |
| `overlay_combo` | `overlayCombo` | String | `"off"` | Ids de `capture::PAD_BUTTONS` (`capture.rs:226-244`), incluido `guide` |
| `overlay_mouse` | `overlayMouse` | String | `""` | `middle` / `x1` / `x2` (`capture.rs:354`) |
| `overlay_hold_force` | `overlayHoldForce` | bool | `true` | Mantener 1,5 s = forzar cierre |
| `overlay_pause` | `overlayPause` | bool | `true` | De la v1 |

Se descarta una lista genérica de «N atajos con tipo»: el patrón de un campo por tipo ya existe para la captura, la interfaz de Ajustes es de una fila por valor y tres disparadores cubren todo lo pedido. DS4Windows → tecla entra por `overlay_hotkey` (recomendado F13–F24, que ningún teclado tiene).

Reglas al guardar (`save_config`, `lib.rs:98-128`), con el mismo mecanismo de «restaurar el anterior y avisar»:
- `overlay_hotkey` no puede repetir captura, grabación ni audio (`hotkey_clash`, `lib.rs:87`).
- `overlay_combo` no puede ser igual, ni subconjunto ni superconjunto, de `capture_combo` o `audio_combo` (se disparan juntas). Mensaje: «La combinación del menú no puede compartir botones con la de captura ni con la de audio».
- `overlay_mouse` no puede ser el de `capture_mouse`.
- Normalización en `Config::load` (`config.rs:276-281`): `overlay_combo` desconocida → `"off"`.

### 4.2 Pantalla de Ajustes
Sección **Mando** (`controller.svelte.ts:1211-1216`), justo después de `'hotkey'`. No se crea una sección nueva para no cambiar las pestañas LB/RB.

| Id | Etiqueta | Valor | A | ‹ › recorre |
|---|---|---|---|---|
| `qmkey` | Abrir el menú del juego con el teclado | `hotkeyLabel` | aprender tecla | `['F24','F13','Ctrl+Alt+O','']` |
| `qmpad` | Abrir el menú del juego con el mando | `comboLabel` | aprender combinación | `['off','guide','back+start','l3+r3']` |
| `qmmouse` | Abrir el menú del juego con el ratón | `mouseLabel` | aprender botón | `MOUSE_PRESETS` |
| `qmhold` | Mantener el atajo para forzar el cierre | Sí · 1,5 s / No | alterna | — |
| `qmpause` | Pausar el juego con el menú abierto | Sí / No | alterna | — |

Descripciones (textos completos en el estado `#ajustes` del mockup, listos para copiar): la de teclado explica DS4Windows («asigna el botón a F24 y púlsalo aquí»); la de mando avisa de Game Bar/Steam; la de mantener explica la barra, la vibración y la exclusiva.

Aprendizaje: el mismo diálogo `Learn.svelte` con `target: 'overlay'`. Con el botón Xbox: `ComboLearner` ya lo ve porque `pad.rs` lee con `XInputGetStateEx` (`pad.rs:32-52`).

## 5. Cambios técnicos por módulo

Base: los pasos 1-12 de plan-overlay.md §5 siguen valiendo con estas diferencias. Donde aquí se dice «v1 paso N», hacerlo como allí.

### 5.1 visor-core
- **Nuevo `src-tauri/visor-core/src/trigger.rs`** (registrar en `visor-core/src/lib.rs`, lista de `pub mod` de las líneas 2-25):
  - `pub const TAP_MS: u64 = 400; pub const HOLD_MS: u64 = 1500;`
  - `pub struct TriggerCtx { pub can_show: bool, pub is_open: bool, pub hold_force: bool }`
  - `pub enum TriggerAct { None, Open, Close, Warn, HoldStart, HoldProgress(f32), HoldTick, Force, HoldCancel }`
  - `pub struct TriggerFsm` con `down(now, ctx) -> TriggerAct`, `tick(now) -> TriggerAct` (se llama cada ~30 ms mientras está pulsado; da `HoldStart` a los 400 ms, `HoldProgress` después, `HoldTick` en el ritmo creciente si `!can_show`, y `Force` una sola vez a los 1500 ms), `up(now) -> TriggerAct` (`Close` si estaba abierto y fue corto, `Warn` si no podía mostrarse y fue corto, `HoldCancel` si se soltó entre 400 y 1500 ms). Un segundo `down` mientras hay uno activo se ignora (teclado y mando a la vez).
  - Tests (tabla del §2.5 completa, `hold_force=false`, doble disparador).
- **`capture.rs`**: tras `ComboDetector` (`:285-313`) añadir `EdgeDetector { mask, was_down }` con `update(buttons) -> Option<bool>` (Some(true) al completar la combinación, Some(false) al romperla) y su test. Función `combos_overlap(a, b) -> bool` (`a & b == a || a & b == b`, con a,b ≠ 0) para la regla del §4.1, con test.
- **`config.rs`**: campos del §4.1 en el struct (`:107-111`), `Default` (`:213-215`), normalización (`:276-281`) y test como `record_hotkey_default` (`:469`).

### 5.2 src-tauri
- **`state.rs:78-85`**: `overlay_mask: AtomicU32` (como `audio_mask`), `overlay_mouse: AtomicU8`. Inicializarlos en `lib.rs:1050-1077` y actualizarlos en `save_config` (`lib.rs:140-142`).
- **`lib.rs` atajos:**
  - `apply_hotkeys` (`:59-84`): quinto parámetro `overlay` («menú del juego»); llamarlo así en `save_config` (`:119`, `:122`), `hotkey_suspend` (`:870-874`) y el arranque (`:1089`).
  - `save_config` (`:103-128`): comparar también `overlay_hotkey` y aplicar las reglas del §4.1.
  - Manejador (`:935-953`): hoy solo mira `Pressed`. Si el atajo es `overlay_hotkey`: `Pressed` → `quickmenu::trigger_down(app, Src::Key, 0)`, `Released` → `quickmenu::trigger_up(app, Src::Key)`, y **no** caer al `snap::trigger` del final. **Verificar en Fase 1** que `global-hotkey 0.8` entrega `Released` en Windows; si no, tras `Pressed` sondear `GetAsyncKeyState(vk)` cada 15 ms en un hilo hasta que se suelte (tabla propia `Code → VK` solo para las teclas que acepta `keyName`, `learn.svelte.ts:92-100`).
- **`pad.rs`** (bucle `:206-299`): leer `overlay_mask`; incluirlo en la condición de reposo (`:233`) y en `pressing` (`:259`). Con `EdgeDetector`: al completar la combinación → `quickmenu::trigger_down(app, Src::Pad, pressing)`; al soltarla → `trigger_up(app, Src::Pad)`. Funciona también con el menú abierto (es lo que lo cierra); como las otras combinaciones, no actúa si el visor tiene el foco (`own_window_focused`, `:289`; la ventana del menú nunca lo tiene). Con el menú abierto, la captura y el audio por combinación no saltan (v1 paso 4). Añadir `rumble_long(pads, ms)` (los dos motores) junto a `rumble_n` (`:182-197`).
- **`mouse.rs`**: el gancho (`:21-41`) también manda las subidas (`WM_MBUTTONUP`, `WM_XBUTTONUP`) con un bit de «suelto»; el consumidor (`:45-53`) llama a `trigger_down/up(Src::Mouse)` si coincide con `overlay_mouse` y sigue capturando con `mouse_button` como hoy.
- **`quickmenu.rs`** (nuevo; todo lo de v1 paso 3, más):
  - `trigger_down(app, src, pads)` / `trigger_up(app, src)`: dueños de la `TriggerFsm`. En `down` calculan `TriggerCtx` (`can_show` = hay `Active` y `SHQueryUserNotificationState` ≠ `QUNS_RUNNING_D3D_FULL_SCREEN`; feature `Win32_UI_Shell` si falta) y arrancan un hilo que llama a `tick` cada 30 ms hasta el `up` o el `Force`. Traducción de actos: `Open` → `open(app)` (el antiguo `toggle`), `Close` → `close(app)`, `Warn` → `pad::rumble_n(p, 2)`, `HoldStart`/`HoldProgress` → `emit_to("quickmenu","qm-hold",{p})` si está abierto, `HoldTick` → pulso corto, `HoldCancel` → `qm-hold {p:-1}`, `Force` → `force_close_game(app)` + `rumble_long(p, 400)`.
  - **Foto a resolución completa:** al abrir, guardar en memoria el `Grabbed` completo (además del JPEG reducido del fondo). `quickmenu_capture`: si el juego está pausado, guarda esa imagen (es idéntica a lo que hay detrás y no depende de capturar un proceso suspendido); si no está pausado, `snap::trigger(app)`. Reutilizar el guardado de `snap.rs:149-159` sacándolo a una función `save_grab(app, grabbed)`. Sin aviso `peek` con el menú abierto (`snap.rs:109`); en su lugar `emit_to("quickmenu","qm-shot",{path, jpeg})` y `push_shot`.
  - **Contexto del juego** (§5.3): `quickmenu_set_context(ctx: serde_json::Value)` lo guarda en un `Mutex<Option<Value>>`; se borra al acabar la partida; `quickmenu_state()` lo incluye tal cual.
  - Comandos nuevos en `generate_handler!` (`lib.rs:1132`): `quickmenu_capture`, `quickmenu_set_context`, más los de v1 pasos 6 y 10. Favorita: la página llama al `set_favorite` existente (`lib.rs:327-335`) y Rust emite `favorites-changed` a la ventana principal.
- **Gancho de teclado** (v1 paso 5): añadir `X`, `Y`, `Q`, `E`, `Add`, `Subtract` a las teclas que se traducen a `qm-pad` y se tragan.

### 5.3 Frontend
- **Contexto para la racha:** la página del menú no tiene la biblioteca (`app.lib`) y la biblioteca es grande; los anillos se calculan en el navegador (`readgoals.ts`). Por eso, como ya hace `scheduleGoalNotice` (`controller.svelte.ts:651-675`), la ventana principal manda al núcleo lo justo al empezar la partida: en `onBackendEvent` `game-started` (`controller.svelte.ts:1635-1637`) llamar a `pushQuickmenuContext(e.id)`:
  `{ gameId, key: goalKeyOf(gameId) ?? null, goal, titles: key ? títulos de esa plataforma → {gameId, name, plays} : [], title: {playSeconds, sessions, week, plays} }` (`goalKeyOf` en `:629`). Nuevo método `quickmenuSetContext` en `backend.ts` / `types.ts` (no-op en `mock.ts`). Volver a mandarlo si cambian `goalVn`/`goalJvn` con la partida en marcha.
- **Funciones puras compartidas:** `readrings.svelte.ts` importa el store y no se puede usar en la página del menú. Mover `rescueDay`, `rescueLeft`, `rescueClose`, `rescueFailed`, `fmtMin` (`readrings.svelte.ts:14-36`) a un `src/lib/readfmt.ts` nuevo sin dependencias, y reexportarlos desde `readrings.svelte.ts` para no tocar a quien ya los importa. `readgoals.ts` (`goalTrack` `:175`, `readDetail` `:285`) y `stats.ts` (`titleStats` `:148`) ya son puros.
- **Página del menú** (v1 paso 12) con los componentes del mockup v2: `TileTime.svelte` (usa `goalTrack(key, goal, [{id:key, titles}] as Platform[], {id: gameId, since}, Date.now())` y `readDetail(...).doneAt`; casos a/b/c/d del §2.2), `TileAudio.svelte`, `TileShots.svelte`, `TileExit.svelte` (cuenta atrás en la interfaz; al cerrar el menú se cancela), `TileAch.svelte`; banner `qm-hold`. Las piezas visuales de racha (anillo SVG, `.rmed`, hucha mini, mapa) se copian del CSS del mockup, que ya está adaptado a cristal oscuro.
- **Config TS:** campos del §4.1 en `types.ts` (junto a `audioHotkey`, `:67`), `store.svelte.ts` (`:51`) y `mock.ts` (`:325`).
- **Aprendizaje** (`learn.svelte.ts`): `target` (`:32` y `store.svelte.ts:132`) admite `'overlay'`; presets nuevos junto a `:13-26` (`OVERLAY_HOTKEY_PRESETS`, `OVERLAY_COMBO_PRESETS`); ramas en `onLearnPad` (`:76-77` → `overlayCombo`, «Abrir el menú con»), `learnKeydown` (`:121-123` → `overlayHotkey`) y `learnMouse` (`:145`, hoy siempre `captureMouse` → según `target`). `Learn.svelte:8-12`: si `target === 'overlay'` y `kind === 'key'`, añadir al subtítulo «Si usas DS4Windows, pulsa ahora el botón del mando que convierte en tecla».
- **Ajustes:** `SettingId` (`controller.svelte.ts:1178-1186`) + los 5 ids; filas en Mando (`:1211-1216`); `CYCLE` (`:1248`) + `qmkey`, `qmpad`, `qmmouse`; acciones junto a `:1455-1458` (A aprende con `startLearn(kind,'overlay')`, ‹ › recorre presets; `qmhold`/`qmpause` alternan); textos en `Settings.svelte` junto a `:47-48`.
- **Favoritas:** en `controller.svelte.ts` escuchar `favorites-changed` y recargar `app.favs` como en `:1663`.

## 6. Orden de implementación

Ninguna fase es trabajo masivo; no hay nada que delegar en `agy` salvo, si se quiere, la batería de tests de `TriggerFsm` y del parser HID.

**Fase 1 · Disparadores y cierre (sin interfaz bonita)**
1. visor-core: `trigger.rs`, `EdgeDetector`, `combos_overlap`, campos de config + tests.
2. src-tauri: atómicos, `apply_hotkeys`/`save_config`/manejador (con `Released`), `pad.rs`, `mouse.rs`.
3. `quickmenu.rs` esqueleto (v1 paso 3) con `trigger_down/up`, exclusiva, `close_game`/`force_close_game`, vibraciones. Página mínima que solo pinta «abierto» y el banner `qm-hold`.
4. Ajustes (§4.2) y aprendizaje.
*Verificar:* el atajo de teclado, el del mando (incluido Xbox) y el del ratón abren y cierran; mantener 1,5 s fuerza el cierre con y sin menú; con un juego en exclusiva, pulsación corta = 2 toques y mantener = cierre + vibración larga; soltar a 1 s no hace nada; las reglas de choque avisan y conservan el valor anterior. `cargo test -p visor-core`.

**Fase 2 · Datos** (v1 pasos 9-11) + contexto del juego (§5.3) + `quickmenu_capture` con la foto completa + favoritas.
*Verificar:* `quickmenu_state` trae el contexto correcto en una novela visual, una japonesa y un juego normal; la captura desde el menú con el juego pausado guarda la imagen del juego (no el menú) en la carpeta del juego; marcar favorita se ve en el visor.

**Fase 3 · Interfaz v2** (v1 paso 12 con el mockup `overlay-a2.html`).
*Verificar con el mockup al lado:* las tres variantes de Tiempo (con hucha, sin hucha, juego) muestran las mismas cifras que Inicio y la pantalla de racha en ese momento; volumen/silencio/salida/micro cambian lo mismo que Windows; LT/RT, X, Y y el teclado (X, Y, Q, E, +, −) funcionan; Cerrar → 3 s → se puede deshacer con B, clic o el atajo; al acabar, el juego recibe `WM_CLOSE`.

## 7. Riesgos

- **`Released` del atajo de teclado:** si `global-hotkey` no lo da en Windows, plan B del §5.2. Con DS4Windows funciona porque mantiene la tecla pulsada mientras se mantiene el botón; si el usuario lo configuró como «toque» (macro), mantener no servirá: se dice en la descripción de Ajustes.
- **Botón Xbox:** Game Bar o Steam pueden abrirse a la vez. Por eso el disparador del mando viene desactivado.
- **El juego también ve el mando** (XInput no es exclusivo): una combinación de disparador la recibe el juego antes de que se pause. Recomendar combinaciones que el juego no use (L3+R3, Xbox).
- **Detección de exclusiva:** `SHQueryUserNotificationState` puede fallar en algunos juegos; el gesto de mantener funciona igual aunque el menú no se vea.
- **Capturar un proceso suspendido:** se evita guardando la foto completa al abrir; con «Pausar» desactivado se captura de verdad.
- **Sin contexto del juego:** si la ventana principal no estuviera cargada al empezar la partida, el overlay enseñaría solo sesión, hoy y total, y la baldosa de Tiempo caería al caso c (sin racha).
- **Forzar con el juego pausado:** `TerminateProcess` funciona igual con procesos suspendidos (v1 §4.3); además se borra el fichero de pids suspendidos.
