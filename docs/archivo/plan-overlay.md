# Plan: overlay sobre el juego («menú rápido»)

> **Estado:** propuestas listas para elegir. **Recomendación: A «Plaza»** (§3). Las decisiones abiertas para el usuario están en el §7; los pasos de implementación (§5) valen para cualquier propuesta salvo el paso 12 (la interfaz).

Mockups (1280×720, autocontenidos, datos falsos): `docs/mockups-overlay/overlay-a.html`, `-b.html`, `-c.html`, `-d.html` y `docs/mockups-overlay/comparativa.html` (las cuatro lado a lado en el mismo estado). Cada página tiene botones para todos sus estados, acepta `#estado` en la URL (`#abierto`, `#logros`, `#audio`, `#rendimiento`, `#salir`…), un conmutador de **fondo de juego claro/oscuro** y se maneja con teclado (Espacio = botón de silencio/F24, flechas = cruceta, Intro = A, mantener Intro = forzar cierre, Esc = B) o con el ratón. Las gráficas de CPU/GPU se mueven en vivo.

Decisiones ya tomadas (no se reabren): atajo global de teclado (RegisterHotKey, que es lo que usa `tauri-plugin-global-shortcut` por debajo) con la tecla aprendida en Ajustes —el usuario pone F24, que DS4Windows manda al pulsar el botón de silencio del mando—, y la lista de 8 elementos: logros, últimas capturas, volumen y cambio de entrada/salida, batería del mando, CPU y GPU con gráficas, tiempo de sesión y total, hora, cerrar/forzar cierre.

**Categorías:** el overlay solo enseña datos del juego en marcha (`Active`); no toca plataformas, carpetas ni cómo se detectan.

## 1. Las cuatro propuestas

Estilo común «Wii U glass arcoíris»: cristal ahumado (tinte oscuro translúcido + brillo blanco arriba + borde fino claro), esquinas muy redondeadas, sombras suaves, y el arcoíris solo en acentos: anillo de enfoque arcoíris que gira despacio, trazos de las gráficas, barras de progreso, el anillo de logros. Texto siempre blanco sobre cristal oscuro, para que se lea igual sobre un juego claro que sobre uno oscuro (probarlo con el conmutador de fondo).

### A · «Plaza» — rejilla de baldosas a pantalla completa (`overlay-a.html`)
- **Distribución:** el juego queda desenfocado y oscurecido detrás (con una trama de puntos muy suave). Cabecera: píldora con el nombre del juego y su categoría a la izquierda; hora grande y fecha a la derecha. Debajo, rejilla 4×3 de baldosas de cristal de distintos tamaños, como el menú HOME de Wii U.
- **Elementos:** Logros = baldosa grande 2×2 con un anillo arcoíris (23/48), los 3 últimos conseguidos y «te queda poco: X 70 %». Rendimiento = baldosa ancha con dos **áreas en vivo** (CPU y GPU, trazo de degradado espectral y relleno que se desvanece) y el % grande. Capturas = baldosa ancha con 4 miniaturas. Audio = % de volumen, barra arcoíris y salida actual. Tiempo = sesión grande y total. Mando = pila grande con %. Salir = baldosa teñida de rojo con botón de encendido. Hora = cabecera.
- **Detalle:** A sobre una baldosa la amplía a una hoja de cristal que tapa la rejilla (que queda difuminada detrás).
- **Animación:** entrada = el juego se desenfoca (350 ms) y las baldosas saltan desde 86 % con rebote, escalonadas 40 ms. Salida = se encogen y se funden en 220 ms. Las hojas entran con un zoom elástico.
- **Mando:** cruceta/stick con navegación espacial entre baldosas, A abre, B atrás (en el inicio cierra), LT/RT volumen desde cualquier sitio, silencio cierra.

### B · «Cristal lateral» — panel deslizante (`overlay-b.html`)
- **Distribución:** columna de cristal de 450 px a la derecha, de arriba abajo; el juego sigue nítido a la izquierda (solo un degradado oscuro hacia el panel).
- **Elementos:** cabecera con la hora grande y una píldora de batería; tarjeta del juego con sesión y total y una línea arcoíris; filas de Logros (anillo pequeño + último), Capturas (4 miniaturas), Volumen (barra; ← → lo ajusta sin entrar), Rendimiento (**dos anillos en vivo** CPU/GPU con una **barra de historial** de 22 barras de color según la carga); abajo, Cerrar juego / Forzar….
- **Detalle:** la fila se abre dentro del panel, que se ensancha a 600 px (pila con «‹»).
- **Animación:** entra desde la derecha con rebote (460 ms) y las filas suben escalonadas; sale hacia la derecha en 220 ms.
- **Mando:** ↑ ↓ filas, ← → ajustan, A abre, B atrás/cierra.

### C · «Dock» — barra inferior con cajón (`overlay-c.html`)
- **Distribución:** barra de cristal abajo (1100×118); a la izquierda la hora, la fecha y «1:24 · total 42 h»; a la derecha seis baldosas cuadradas con su dato en vivo. La baldosa enfocada muestra su nombre en una etiqueta blanca encima (guiño a Wii U).
- **Elementos:** Logros (anillo mini), Capturas (pila de 3 miniaturas torcidas), Volumen, CPU y GPU (**mini gráfica con las dos líneas superpuestas**), Mando (pila), Salir (rojo).
- **Detalle:** un cajón de cristal se despliega encima con una flecha que señala su baldosa; con el cajón abierto, ← → en el dock cambian el cajón al momento. Rendimiento en el cajón = «ecualizador» de 60 barras por gráfica, cada barra del color de su carga.
- **Animación:** la barra sube desde abajo con rebote y los iconos saltan escalonados; el cajón se despliega desde abajo (320 ms).
- **Mando:** ← → recorren el dock, A o ↑ abren el cajón, B pliega y luego cierra.

### D · «Anillo» — menú radial (`overlay-d.html`)
- **Distribución:** centro con la hora, la fecha y el juego dentro de un círculo de cristal, rodeado por un aro arcoíris que gira despacio; ocho burbujas de cristal en las ocho direcciones del stick.
- **Elementos:** N Logros, NE Capturas, E Volumen, SE Salida y entrada, S Salir, SO Mando, O Rendimiento, NO Tiempo. El centro amplía la burbuja enfocada; Rendimiento = **dos arcos concéntricos en vivo** (CPU fuera, GPU dentro).
- **Detalle:** el anillo se encoge y se difumina; la burbuja se abre en una tarjeta grande.
- **Animación:** el aro entra girando desde −160° y las burbujas salen disparadas desde el centro; al cerrar vuelven a él.
- **Mando:** el stick apunta directamente a una burbuja (8 direcciones), la cruceta 4 y LB/RB giran; A abre, B atrás/cierra.

Las vistas de detalle (logros con pestañas LB/RB, capturas, audio con salida/entrada, rendimiento 2 min, tiempo con la semana, mando, salir con «mantener para forzar») son las mismas en las cuatro; solo cambia el marco.

## 2. Comparativa

| | A Plaza | B Lateral | C Dock | D Anillo |
|---|---|---|---|---|
| Se ve todo a la vez | Sí, los 8 | 7 de 8 (el detalle de cada uno dentro) | Dato corto de los 8 | Un dato por burbuja |
| Legibilidad sobre juego claro | La mejor (fondo siempre desenfocado) | Buena (degradado lateral) | Buena abajo | Buena (centro oscurecido) |
| Tapa el juego | Todo | ~35 % a la derecha | Franja inferior (todo con cajón) | Casi todo |
| Rapidez con mando | Buena (espacial) | Buena (lista) | Muy buena para lo frecuente | La mejor (apuntar y A) |
| Ratón | Muy bien | Muy bien | Muy bien | Bien |
| Parecido a Wii U | Máximo (baldosas HOME) | Medio | Medio | Bajo (más PS/Steam) |
| Encaja con el cristal real (§4.2) | Perfecto (todo sobre la foto congelada) | Bien si el juego está en pausa | Bien si está en pausa | Perfecto |
| Coste de implementar | Medio | Bajo | Medio-alto (cajón que sigue al dock) | Medio |

## 3. Recomendación: A «Plaza»

1. Es la que mejor cumple el estilo pedido (baldosas Wii U) y la única que enseña los 8 elementos a la vez con espacio para gráficas bonitas.
2. Se lee igual sobre cualquier juego: todo va sobre una foto desenfocada del propio juego, no sobre el juego en movimiento.
3. Técnicamente es la más robusta: una ventana a pantalla completa sobre una foto congelada no depende del efecto de cristal de Windows (§4.2) ni de que el juego esté en pausa para que el cristal cuadre.
4. Navegación espacial simple; se aprovechan dos ideas de las otras: LT/RT para el volumen en cualquier momento (C) y ← → para ajustar el volumen sobre su baldosa sin entrar (B).

B es la alternativa si el usuario prefiere ver el juego mientras mira el menú.

## 4. Decisiones técnicas

### 4.1 Una ventana nueva, no la del indicador REC
Ventana Tauri propia (`quickmenu`, página `quickmenu.html` con Svelte) con los mismos trucos de `overlay.rs`: transparente, sin bordes, siempre encima, `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`, `WDA_EXCLUDEFROMCAPTURE`, se muestra con `show_quiet` y se mantiene arriba con un hilo tipo `keep_on_top` (`overlay.rs:109-128`, `:171-239`). Diferencias: ocupa el monitor del juego entero (`reader::monitor::game_monitor`, como `place` en `overlay.rs:242-256`) y **sí recibe el ratón** (sin `set_ignore_cursor_events` ni `WS_EX_TRANSPARENT`). Se crea oculta al empezar la partida para abrir al instante y se destruye al acabar (WebView2 ocupa memoria).

**No toma el foco.** Quitarle el foco al juego hace que los emuladores a pantalla completa salgan y vuelvan a entrar en ella con un segundo de negro (comentario de `overlay.rs:205-207`), puede parar el escalado de Magpie y cambia el cálculo de tiempo interactivo (`launcher.rs:334-338`). Por eso la entrada llega por otros caminos (§4.3).

### 4.2 El «cristal» de verdad
Un `backdrop-filter` de CSS en una ventana transparente **no desenfoca lo que hay detrás de la ventana** (otras aplicaciones); solo lo de dentro de la página. El acrílico de Windows 11 (efectos de Tauri / `window-vibrancy`) se apaga cuando la ventana no está activa, y aquí nunca lo está. Solución: al abrir, **se captura el juego** con lo que ya existe (`platform::grab(&pids, …)`, `snap.rs:100`), se reduce a ~960 px en JPEG (`capture::rgba_to_jpeg`, `snap.rs:110`) y se pinta de fondo en la página, a tamaño de monitor; el desenfoque y el cristal se hacen con CSS sobre esa foto. Cuesta 30-80 ms al abrir. En A cubre toda la pantalla; en B/C/D solo cuadra perfecto si el juego está en pausa (§4.3).

### 4.3 El juego también recibe el mando
XInput no es exclusivo (`pad.rs:3`): todo lo que se pulse en el menú le llega también al juego. Opciones:
- **Pausar el juego (recomendada, por defecto):** al abrir se suspenden los procesos de `Active.pids` (`NtSuspendProcess`/`NtResumeProcess` de ntdll, con `PROCESS_SUSPEND_RESUME`); al cerrar se reanudan. El juego se queda quieto como con el HOME de una consola: ni recibe el mando ni sigue la partida, y la foto del §4.2 coincide con lo que hay detrás. «Forzar cierre» funciona igual con el proceso suspendido.
  - Riesgos: juegos en línea o con antitrampas (pueden desconectar o quejarse), algunos sonidos en bucle con DirectSound, y Windows puede marcar la ventana como «No responde» si se queda suspendida mucho rato. Por eso hay ajuste para no pausar (global y, más adelante, por juego).
  - Si el visor se cerrara con el juego suspendido, el juego se quedaría congelado: se guardan los pids suspendidos en un fichero y al arrancar se reanudan (comprobando que el exe sigue siendo el mismo).
- **No pausar:** el menú funciona igual, pero el juego recibe los botones. Útil para juegos en línea.
- Descartado: darle el foco al overlay (problemas de arriba) y bloquear el mando desde DS4Windows/HidHide (depende de la configuración de otro programa y además nos quitaría también a nosotros la lectura del mando).

**Navegación con mando:** la webview no recibe el mando sin foco (`pad.rs:1`), así que un hilo en Rust lee XInput cada 16 ms solo mientras el menú está abierto, detecta pulsaciones (con autorrepetición en cruceta/stick: 350 ms y luego cada 90 ms) y las manda a la ventana con `emit_to("quickmenu", "qm-pad", …)`. Se ignoran los botones que ya estaban pulsados al abrir.

**Teclado:** sin foco no llegan teclas a la página. Mientras el menú está abierto se pone un gancho de teclado de bajo nivel (`WH_KEYBOARD_LL`) que se queda las flechas, Intro, Esc, Retroceso y Tab (las convierte en el mismo evento `qm-pad`) y **no las deja pasar al juego**; el resto de teclas pasan. **Ratón:** la ventana cubre el monitor del juego, así que los clics son del menú.

### 4.4 De dónde salen los datos

| Dato | Fuente | Coste / notas |
|---|---|---|
| Logros | `achievements::get_game_achievements(game_id, None)` (`achievements.rs:131-147`); campos `earned`, `unlocked_at`, `stat_current/stat_max` (progreso), `rarity_tier` (`visor-core/src/achievements/mod.rs:82-96`) | Lee de disco; pedirlo al abrir en segundo plano, la baldosa sale con «…» hasta que llega. |
| Últimas capturas | Anillo en memoria con las capturas de la sesión: en `snap.rs` tras guardar se apunta ruta + la miniatura JPEG que ya se hace para el aviso (`snap.rs:110`, hacerla aunque `capture_peek` esté apagado). Para completar con capturas de antes: las más recientes de la carpeta del juego con su miniatura de la caché (`visor_core::thumbs::thumb_path`), leídas **al empezar la partida** en segundo plano (la biblioteca está en un HDD lento). | Gratis al abrir. |
| Volumen y silencio | `IAudioEndpointVolume` del dispositivo de salida predeterminado (y del de entrada para silenciar el micro) | Nuevo; feature `Win32_Media_Audio_Endpoints`. |
| Salida / entrada | `outputs::list` y `outputs::set_default` (`outputs.rs:56-82`), generalizados con `Direction::Render` / `Direction::Capture`; `IPolicyConfig` vale igual para micrófonos | Ya probado. |
| Batería del mando | 1) `XInputGetBatteryInformation`. 2) **Con DS4Windows el juego ve un mando de Xbox virtual (ViGEm) que no informa de batería**, así que hace falta leer el DualSense/DS4 por HID: VID `054C`, PID `0CE6` (DualSense), `0DF2` (Edge), `05C4`/`09CC` (DS4); se abre compartido y se lee un informe de entrada: DualSense USB `0x01` byte 53 / BT `0x31` byte 54 (nibble bajo = nivel 0-10, alto = cargando); DS4 USB `0x01` byte 30 / BT `0x11` byte 32 (nibble bajo nivel, bit 4 cable). **Verificar los desplazamientos con un volcado real.** Crate `hidapi` con `windows-native`. | Si DS4Windows oculta el mando con HidHide, el visor no lo ve: habría que añadirlo a la lista blanca de HidHide; si no, se muestra «—». |
| CPU | `sysinfo` (ya está, `Cargo.toml:23`): uso global; nombre del procesador | Muy barato. |
| GPU uso | Contadores de rendimiento de Windows (PDH, los del Administrador de tareas): `\GPU Engine(*engtype_3D)\Utilization Percentage`, sumando por adaptador (LUID en el nombre de la instancia) y quedándose con el mayor. Vale para NVIDIA, AMD e Intel (WDDM 2+). | Pocos ms por muestra. Feature `Win32_System_Performance`. |
| GPU VRAM | PDH `\GPU Adapter Memory(*)\Dedicated Usage`; total y nombre con DXGI (`IDXGIAdapter1::GetDesc1`) | Feature `Win32_Graphics_Dxgi`. |
| GPU temperatura | NVIDIA: crate `nvml-wrapper` (carga `nvml.dll` si está). AMD/Intel: no hay API común sencilla → «—». | Opcional. Temperatura de CPU: no (pide un driver). |
| Muestreo | Hilo que arranca con la partida (`launcher.rs:203-206`, junto a `achievements::on_launch`) y para al acabar: una muestra cada 2 s en un anillo de 60 (2 min), así al abrir ya hay historial. Mientras el menú está abierto se emite `qm-perf` con cada muestra nueva. | Con el juego suspendido, la GPU baja casi a 0: es real, se deja así. |
| Tiempo | Sesión = ahora − `Active.since`; total = estadísticas del juego (`st.stats`) + sesión; hoy/semana si las sesiones guardadas lo permiten. | |
| Hora | La del sistema, en la página. | |
| Cerrar | Reanudar si estaba pausado, ocultar el menú y mandar `WM_CLOSE` a las ventanas visibles de nivel superior de cada pid (`EnumWindows` + `GetWindowThreadProcessId`). | El juego puede preguntar si guardar. |
| Forzar cierre | `OpenProcess(PROCESS_TERMINATE)` + `TerminateProcess` de todos los `Active.pids`, como `magpie.rs:390-395`. La confirmación es de la interfaz: **mantener A 1,5 s** (se va llenando; soltar antes no hace nada). | El vigilante (`launcher.rs:316-321`) ve que no quedan procesos y guarda la sesión. |

### 4.5 Pantalla completa exclusiva
Ninguna ventana se ve encima de un juego en pantalla completa exclusiva de verdad (igual que el indicador REC, `overlay.rs:4`). En Windows 10/11 la mayoría de juegos DX11/12 «a pantalla completa» van en realidad sin bordes (optimizaciones de pantalla completa) y los emuladores van sin bordes, así que debería verse. Si se detecta exclusiva (`SHQueryUserNotificationState` = `QUNS_RUNNING_D3D_FULL_SCREEN`), el atajo no abre nada y el mando vibra dos veces (`pad::rumble_n`).

## 5. Implementación (orden recomendado)

Ningún paso es trabajo masivo: no hay nada que delegar en `agy`, salvo si se quieren tests en cantidad del parser de HID (paso 9).

**Fase 1 · Esqueleto (abrir, cerrar, navegar, salir del juego)**
1. **Config** — `src-tauri/visor-core/src/config.rs:111`: añadir `overlay_hotkey: String` (atajo del menú rápido, "" = desactivado) y `overlay_pause: bool` (pausar el juego mientras está abierto); en `Default` (`:215`): `overlay_hotkey: "F24".into()` (ninguna tecla física es F24, no estorba) y `overlay_pause: true`. Test como `record_hotkey_default` (`:469`). Frontend: `src/lib/types.ts:67`, `src/lib/store.svelte.ts:51`, `src/lib/mock.ts:325` (campos `overlayHotkey`, `overlayPause`).
2. **Atajo** — `src-tauri/src/lib.rs`: `apply_hotkeys` (`:59-84`) recibe un cuarto atajo («menú del juego»); `save_config` (`:103-128`) lo compara, comprueba que no choque con los otros tres (`hotkey_clash`, `:87`) y lo restaura si falla; `hotkey_suspend` (`:865-875`) lo vuelve a registrar; el manejador (`:934-953`) llama a `quickmenu::toggle(app)` cuando coincide.
3. **Módulo nuevo `src-tauri/src/quickmenu.rs`** (registrar en `lib.rs`, `init` junto a `overlay::init` en `:1098`):
   - `prewarm(app)` (crear oculta) y `destroy(app)`; llamarlos en `launcher.rs:186` (tras `game-started`) y al acabar la partida (antes de `game-exited`, `launcher.rs:391`).
   - `toggle(app)`: si no hay `Active` → no abre (ver §7); si es exclusiva → vibrar 2 veces; si no: foto (§4.2) → si `overlay_pause`, suspender pids y apuntarlos en `config_dir/quickmenu-suspended.json` → colocar a pantalla completa del monitor del juego → `show_quiet` + hilo keep-on-top → arrancar hilo de mando y gancho de teclado → `emit_to("quickmenu","qm-open", estado)`.
   - `close(app)`: lo contrario, en orden inverso; reanudar siempre (también si algo falla por el camino: guardia con `Drop`).
   - `is_open() -> bool` (atómico).
   - Al arrancar la app: reanudar los pids del fichero si siguen vivos con el mismo exe, y borrarlo.
   - Reutilizar `overlay::no_activate` / `to_top` / `show_quiet` / `hide_quiet` (ya son `pub(crate)`), pero sin `WS_EX_TRANSPARENT` ni ignorar el cursor: añadir en `overlay.rs:174` una variante `no_activate_clickable` o un parámetro.
4. **Mando en el menú** — `src-tauri/src/pad.rs`: hacer `pub(crate)` el lector `xinput::Reader` (`:22-98`); en `quickmenu.rs` el hilo que lee cada 16 ms mientras `is_open()` y emite `qm-pad` con `{btn: "up"|"down"|"left"|"right"|"a"|"b"|"x"|"y"|"lb"|"rb"|"lt"|"rt"|"start"|"back", angle?: f32}` (el ángulo del stick, por si se elige D). En `pad.rs:289` añadir `&& !crate::quickmenu::is_open()` para que las combinaciones de captura y audio no salten dentro del menú.
5. **Teclado en el menú** — gancho `WH_KEYBOARD_LL` en un hilo con bucle de mensajes (el patrón de ventana de mensajes de `input.rs:149-255` sirve de modelo) que traduce flechas/Intro/Esc/Retroceso/Tab a `qm-pad` y devuelve 1 para que no lleguen al juego. Solo mientras está abierto.
6. **Cerrar y forzar** — en `quickmenu.rs`: `close_game(app)` y `force_close_game(app)` (§4.4). Comandos `quickmenu_close`, `quickmenu_close_game`, `quickmenu_force_close_game` en `generate_handler!` (`lib.rs:1132`).
7. **Tiempo de juego honesto** — `launcher.rs:253`: no contar pulsaciones si `quickmenu::is_open()`; `launcher.rs:336`: no sumar tiempo interactivo mientras está abierto.
8. **Ajustes** — `src/lib/controller.svelte.ts`: `SettingId` (`:1180`) + `'qmhotkey'` y `'qmpause'`; sección General (`:1195`, no en tablet) tras `'audiohotkey'`; `CYCLE` (`:1248`); acción en `:1456` (A aprende con `startLearn('key','overlay')`, ‹ › recorre `OVERLAY_HOTKEY_PRESETS = ['', 'F24', 'Ctrl+Alt+O', 'Ctrl+Shift+O']`). `src/lib/learn.svelte.ts`: tipo de `target` (`:32`) con `'overlay'`, rama en `:121-123` → `finish({ overlayHotkey: combo }, combo, 'Abrir el menú del juego con')`, presets junto a `:26`. `src/screens/Settings.svelte:48`: filas «Menú del juego con el teclado» (texto: «Atajo global que abre el menú sobre el juego. Con DS4Windows, asigna el botón de silencio del mando a F24 y pulsa aquí A y luego ese botón.») y «Pausar el juego con el menú abierto» (Sí/No, con la advertencia de juegos en línea).

**Fase 2 · Datos**
9. **Batería** — `pad.rs`: `battery() -> Option<PadBattery { name, level_pct, charging, source }>`: primero XInput (`XInputGetBatteryInformation`), si es cable/desconocido, HID de Sony (§4.4). Dependencia `hidapi` (`default-features = false, features = ["windows-native"]`). Tests del parser con informes de ejemplo (aptos para `agy` si se quieren muchos casos).
10. **Audio** — `outputs.rs`: `list(dir)`/`set_default(id)` para entrada y salida (`:56-82`), y funciones nuevas `volume() / set_volume(f32) / set_mute(bool) / set_mic_mute(bool)` con `IAudioEndpointVolume`; feature `Win32_Media_Audio_Endpoints` en `Cargo.toml:49`. Comandos `quickmenu_set_volume`, `quickmenu_set_mute`, `quickmenu_select_output` (reusa `outputs::select`, `:103`), `quickmenu_select_input`, `quickmenu_set_mic_mute`.
11. **Capturas y rendimiento** — `snap.rs:109-114` y `:150-156`: guardar en el anillo de la sesión (`quickmenu::push_shot(path, jpeg)`); prefetch de las anteriores al empezar la partida. Módulo nuevo `src-tauri/src/perf.rs` (CPU con `sysinfo`, GPU con PDH, VRAM/nombre con DXGI, temperatura con `nvml-wrapper` opcional), arrancado en `launcher.rs:203-206`; features `Win32_System_Performance`, `Win32_Graphics_Dxgi`. Comando `quickmenu_state()` que junta todo (juego, sesión/total/semana, logros, capturas, audio, mando, rendimiento, foto, pausado) y evento `qm-perf`.

**Fase 3 · Interfaz**
12. **Página** — `quickmenu.html` (raíz, como `rec.html`) + `src/quickmenu/main.ts` + `src/quickmenu/QuickMenu.svelte` (y un componente por baldosa/vista); entrada en `vite.config.ts:25-30`; `src-tauri/capabilities/quickmenu.json` con `"windows": ["quickmenu"]` y `core:default` (eventos). Modelo visual: el mockup elegido (A: `docs/mockups-overlay/overlay-a.html`; las piezas comunes —cristal, anillo de enfoque, `ring`, `chart`, `hbars`, vistas de detalle— están al principio de cada mockup y se pueden pasar casi tal cual). Navegación espacial igual que `pick()` del mockup. Escuchar `qm-open` (reinicia el estado y lanza la animación de entrada), `qm-pad`, `qm-perf`; al cerrar, animación de salida y luego `quickmenu_close`. La foto del juego de fondo con `filter: blur(18px) brightness(.78)`.

## 6. Comprobación

- Con un juego lanzado desde el visor, en ventana y sin bordes: F24 (o el botón de silencio con DS4Windows) abre en < 150 ms; otra vez, B o Esc cierran. El juego no pierde el foco (un emulador a pantalla completa no parpadea) y con «Pausar» activado no se mueve ni reacciona a la cruceta mientras el menú está abierto; al cerrar sigue donde estaba.
- Con «Pausar» desactivado, el menú funciona igual (el juego recibe los botones, es lo esperado).
- Matar el visor desde el Administrador de tareas con el menú abierto y pausado → al volver a abrir el visor, el juego se reanuda.
- Las flechas e Intro no llegan al juego mientras el menú está abierto; el ratón funciona.
- El menú no sale en las capturas (F12 dentro del menú captura el juego) ni en una grabación de pantalla.
- Volumen y salida/entrada cambian lo mismo que la configuración de sonido de Windows. Batería: comparar con la que enseña DS4Windows.
- CPU/GPU parecidas al Administrador de tareas (pestaña Rendimiento) en un PC NVIDIA; en AMD/Intel, uso sí y temperatura «—».
- «Cerrar juego» en un juego que pregunta al salir → sale su diálogo. «Forzar cierre» soltando antes de 1,5 s → no pasa nada; manteniendo → el juego muere y la sesión se guarda con su tiempo.
- Con Magpie escalando, el menú sale encima y al cerrarlo Magpie sigue escalando.
- `cargo test -p visor-core` (config) y los tests del parser HID.

## 7. Dudas abiertas para el usuario

1. **¿Qué propuesta?** Recomendada A. ¿O B (ver el juego mientras se mira el menú)?
2. **¿Pausar el juego por defecto?** Recomendado sí (es lo único que impide de verdad que el juego reciba el mando sin quitarle el foco). ¿Hay juegos en línea o con antitrampas que se lancen desde el visor? Para esos, ajuste por juego más adelante.
3. **El mando:** ¿es un DualSense (el botón de silencio solo lo tiene el DualSense)? ¿DS4Windows tiene activado «ocultar el mando» con HidHide? Si sí, para leer la batería hay que añadir el visor a la lista blanca de HidHide.
4. **Sin juego en marcha** (o con un juego abierto fuera del visor): ¿F24 no hace nada, o abre el menú sin logros, tiempo ni salir (solo hora, audio, batería y rendimiento)?
5. **«Abrir en el visor»** desde una captura del menú: ¿hace falta? Implica cerrar el menú y traer el visor delante del juego. Si no, las capturas solo se ven en el menú.
6. **Volumen del juego aparte** del volumen general (mezclador por aplicación): ¿lo quieres? Es fácil de añadir más tarde.
