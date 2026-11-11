# Propuestas de mejora (11 nov 2026, v2.0.1)

Propuestas nuevas para decidir qué hacer después de la reorganización. Ninguna repite lo que ya está en marcha: app de tablet y Magpie, «Volver a la partida», tiempo de Switch, Multi-PC, resumen semanal y pestañas de Estadísticas, revisión del 9-nov y overlay del juego. Tampoco repite lo de `docs/propuestas-antigravity.md`.

**Regla de `CLAUDE.md`:** las categorías no se tocan. Todo lo de aquí vive dentro de una categoría. Las listas, los paneles y los resultados van agrupados por categoría y nunca mezclan títulos de categorías distintas.

Las líneas citadas son del commit `cd7b6d9` y solo sirven para orientarse: busca siempre por el nombre de la función.

Esfuerzo: S = menos de un día · M = de 1 a 3 días · L = más de 3 días.

---

## A. Funciones

| # | Propuesta | Valor para el usuario | Esfuerzo | Riesgo |
|---|---|---|---|---|
| A1 | La captura del logro | Alto | S | Bajo |
| A2 | Buscador que también encuentra colecciones y frases | Medio | S | Bajo |
| A3 | Exportar la selección para compartir | Medio | S | Bajo |
| A4 | Capturas repetidas, por su huella | Alto | M | Medio (borra a la Papelera) |
| A5 | Repaso rápido al terminar la partida | Alto | M | Bajo |
| A6 | Vaciar «Sin clasificar» con sugerencias | Medio | M | Bajo |
| A7 | Revisión de la biblioteca por categoría | Medio | M | Bajo |
| A8 | Bandeja de importación (Steam, Xbox Game Bar, NVIDIA) | Alto | L | Medio |

**A1. La captura del logro.**
- **Qué es:** al desbloquear un logro ya se hace una captura automática (`src-tauri/src/achievements.rs:455`, `snap::trigger`), pero no queda enlazada al logro. El esquema de logros no guarda ninguna ruta (`visor-core/src/achievements/mod.rs` no menciona capturas) y `AchievementsModal.svelte:292` solo enseña la fecha.
- **Propuesta:** en el detalle del logro, mostrar la captura del juego con `date` entre `unlocked_at − 5 s` y `+30 s`. En el visor, una marca «Logro: …» sobre esa captura.
- **Por qué:** es la foto del momento y ya está en disco. No hace falta tocar la base de datos: basta con cruzar las fechas.
- **Primer paso:** una función pura `shotForUnlock(title, ts)` en `src/lib/achievements.svelte.ts`, con su prueba en `tests/`.

**A2. Buscador que también encuentra colecciones y frases.**
- **Qué es:** el buscador global solo compara nombres de títulos (`src/lib/search.svelte.ts:14-26`).
- **Propuesta:** dentro de cada fila de categoría, añadir las colecciones de esa categoría (`collections.ts`) y, en las de lectura, las frases guardadas del glosario que contengan el texto.
- **Por qué:** con varias decenas de colecciones y cientos de frases, hoy solo se llega a ellas recorriendo pestañas.
- **Primer paso:** ampliar `SearchGroup` con `colls: string[]` y pintarlas como baldosas al final de la fila. Las frases van en una segunda fase.

**A3. Exportar la selección para compartir.**
- **Qué es:** ya existen el modo de selección y «Copiar captura» (`ActionMenu.svelte:52-58, 75`), pero copiar va de una en una y las capturas 4K en PNG pesan mucho.
- **Propuesta:** añadir «Exportar N para compartir…» al menú Start. Copia las capturas a una carpeta elegida, en JPEG de 1920 px, con nombres `Juego_fecha.jpg`, y nunca toca los originales (como manda el README).
- **Primer paso:** un comando `export_shots(paths, dir, max_w)` que reutilice el redimensionado de `visor-core/src/thumbs.rs`.

**A4. Capturas repetidas, por su huella.**
- **Qué es:** cada captura ya tiene su SHA-256 calculado en segundo plano, con índice propio (`visor-core/src/catalog.rs:32, 36`), y nadie lo consulta. Ningún código busca duplicados.
- **Por qué:** con unas 14.000 capturas, importadas a mano desde la SD y desde varias herramientas, seguro que hay repetidas. Ocupan sitio en el HDD y ensucian la cuadrícula.
- **Propuesta:** en Ajustes del juego › Capturas y datos (y para toda la categoría desde Start), un panel «Repetidas: N grupos». Muestra cada grupo y deja mandar las sobrantes a la Papelera con `trash.rs`. Solo se comparan capturas de la misma categoría.
- **Primer paso:** `catalog::duplicates(c, platform_prefix)`, con `GROUP BY sha256 HAVING count(*) > 1` sobre las rutas `Plataforma/…`, y su prueba junto a `sync_hash_rename_and_remove`.

**A5. Repaso rápido al terminar la partida.**
- **Qué es:** al llegar `game-exited` (`controller.svelte.ts:1860-1866`) solo sale un aviso y se reescanea. Las capturas de esa partida ya se pueden localizar (el Calendario las cruza con las partidas).
- **Propuesta:** si en la partida se hicieron 3 capturas o más, ofrecer «Repasar 12 capturas de esta partida»: el visor solo con esas, X = favorita, Y = Papelera con deshacer y A = siguiente.
- **Por qué:** es el momento en que se recuerda qué merecía la pena. Más tarde nadie limpia 14.000 capturas.
- **Primer paso:** filtrar `t.shots` por `[start, end]` de la última partida, en `app.gain`, y abrir el visor con esa lista, igual que hace la vista de una colección.

**A6. Vaciar «Sin clasificar» con sugerencias.**
- **Qué es:** las capturas de un juego desconocido van a `PC/Sin clasificar` (`snap.rs:162-171`). En ese momento se conoce el proceso en primer plano (`g.fg_pid` → `exe_of`), pero ese dato se pierde.
- **Propuesta:** guardar el exe y el título de la ventana junto a la captura, en una tabla `shot_origin`. Luego, un panel en esa carpeta: «14 capturas de `game.exe`». Desde ahí se mueven a un título de **Juegos PC** o se pulsa «Añadir como juego», que abre `GameForm` con el exe ya puesto.
- **Primer paso:** la tabla y la escritura en `save_grabbed`. Sin interfaz, ya sirve para no perder el dato.

**A7. Revisión de la biblioteca por categoría.**
- **Qué es:** un ejecutable que ya no existe solo se descubre al pulsar Jugar (`launcher.rs:97`). Lo mismo pasa con una carpeta de trabajo movida o con un juego que no tiene carpeta (`hasFolder` es false).
- **Propuesta:** una hoja «Revisar Juegos PC» en el menú Start de la lista, con una línea por problema y la acción que lo arregla. Las acciones ya existen: «Cambiar ejecutable…», «Cambiar carpeta de trabajo…», SteamGridDB y Ocultar.
- **Primer paso:** un comando `library_issues(platform)` que haga solo comprobaciones `is_file` e `is_dir`, sin abrir imágenes, para no castigar el HDD.

**A8. Bandeja de importación (Steam, Xbox Game Bar, NVIDIA).**
- **Qué es:** sigue pendiente en el README (`README.md:281`). Los patrones de nombre de esas herramientas ya se reconocen (`visor-core/src/dates.rs`), e importar copiando con fecha EXIF ya existe (`importer.rs`).
- **Propuesta:** en Ajustes, elegir las carpetas de origen. La app propone «23 capturas nuevas de Elden Ring», usando el appid de Steam o el nombre de la ventana, y las copia a ese título dentro de su categoría. Nunca mueve ni borra el origen.
- **Riesgo:** acertar el título. Si no está claro, que pregunte en vez de adivinar.
- **Primer paso:** solo Steam (`userdata/<id>/760/remote/<appid>/screenshots`), con el appid enlazado a `games.rs`.

---

## B. Interfaz y experiencia (ratón, teclado y mando)

| # | Propuesta | Valor para el usuario | Esfuerzo | Riesgo |
|---|---|---|---|---|
| B1 | Pistas que siguen al último dispositivo | Alto | S | Bajo |
| B2 | Atajos de teclado reales | Medio | S | Bajo |
| B3 | Arrastrar con el ratón la imagen ampliada | Medio | S | Bajo |
| B4 | L3, R3 y stick derecho en Capturas | Medio | S | Bajo |
| B5 | Selección con Ctrl/Mayús + clic y por rango con mando | Medio | S | Bajo |
| B6 | «Deshacer» en vez de «Pulsa A otra vez» | Alto | M | Medio |
| B7 | Hoja «Controles de esta pantalla» | Medio | M | Bajo (depende de C7) |

**B1. Pistas que siguen al último dispositivo.**
- **Qué es:** `Hints.svelte:19` siempre pinta glifos de mando (A, B, LB…). Con teclado, A es Intro, B es Retroceso y LB es Q (`controller.svelte.ts:1989-1994`), y nada en pantalla lo dice.
- **Propuesta:** guardar `app.input = 'pad' | 'kbd' | 'mouse'` en `poll`, `onKeyDown` y `onScreenMove`. Con teclado, las pistas muestran la tecla en el mismo `.btn` (guía §7). Con ratón se quedan como están, que ya son clicables.
- **Primer paso:** un mapa `KEY_LABEL` invertido a partir de `KEYS` y una rama en `Hints.svelte`.

**B2. Atajos de teclado reales.**
- **Qué es:** `onKeyDown` descarta cualquier tecla con Ctrl o Alt (`controller.svelte.ts:2000`), y `KEYS` no incluye Inicio, Fin, RePág ni AvPág.
- **Propuesta:**
  - Ctrl+F o `/` abren el buscador (`openSearch`).
  - Inicio y Fin van a la primera y la última captura o portada.
  - RePág y AvPág equivalen a LB y RB.
  - F1 abre B7.
- **Primer paso:** tratar estos casos antes del `return` de la línea 2000.

**B3. Arrastrar con el ratón la imagen ampliada.**
- **Qué es:** en el visor, la rueda amplía hacia el puntero (`controller.svelte.ts:2110-2118`), pero luego no hay forma de mover la imagen con el ratón. `Viewer.svelte` no tiene `pointerdown`, y la panorámica solo existe con stick (`panViewContinuous`) o con el dedo en la tablet.
- **Propuesta:** con zoom mayor que 1, arrastrar con el botón izquierdo mueve `app.vz`. El clic sin arrastre sigue haciendo lo de ahora, y el doble clic restablece el zoom.
- **Primer paso:** `onpointerdown`, `onpointermove` y `onpointerup` en `.vframe`, que llamen a una variante de `panViewContinuous` en píxeles.

**B4. L3, R3 y stick derecho en Capturas.**
- **Qué es:** `PAD_KEYS` (`controller.svelte.ts:2217`) no lee los botones 10 y 11 (L3 y R3). El stick derecho solo hace algo en el visor y en la lista de juegos (l.2318-2331).
- **Propuesta:**
  - R3 abre el buscador desde cualquier pantalla.
  - L3 lleva a Inicio (`cmd('home')` ya existe, l.1937).
  - En la cuadrícula de Capturas, el stick derecho desplaza `app.goff` de forma continua, para recorrer miles de capturas sin pasar fila a fila.
- **Primer paso:** añadir `l3` y `r3` a `padState` y `PAD_KEYS`, más su rama en `cmd`.

**B5. Selección con Ctrl/Mayús + clic y por rango con mando.**
- **Qué es:** hoy solo se marca de una en una (`toggleSel`, `controller.svelte.ts:596`) y hay que entrar antes en «Seleccionar» desde Start.
- **Propuesta:**
  - Ctrl+clic entra en la selección y marca la captura.
  - Mayús+clic marca el rango desde la última marcada.
  - Con mando, mantener X mientras se mueve marca todo lo que se recorre.
- **Primer paso:** leer `e.ctrlKey` y `e.shiftKey` en `onScreenClick` y guardar `lastSel` en `app`.

**B6. «Deshacer» en vez de «Pulsa A otra vez».**
- **Qué es:** el menú pide doble pulsación para quitar y para borrar (`ActionMenu.svelte:51, 57-58`). El menú del juego ya resuelve lo mismo con un aviso «Captura borrada · LB para deshacer» (`QuickMenu.svelte:204-215`).
- **Propuesta:** usar ese patrón en la app principal para borrar, mover, ocultar, cambiar el estado y quitar de una colección. La acción se aplica al momento en la interfaz y se envía al backend pasados unos 4 s, salvo que se deshaga.
- **Riesgo:** cerrar la app durante la espera. Al salir hay que ejecutar lo pendiente.
- **Primer paso:** un `pendingAction` con temporizador en `controller.svelte.ts` y el aviso con un botón clicable.

**B7. Hoja «Controles de esta pantalla».**
- **Qué es:** con 9 vistas, 24 tipos de panel (`store.svelte.ts:11, 90`) y botones que cambian de sentido según el foco (`cmd`, l.1940-1985), la barra inferior no da para explicarlo todo.
- **Propuesta:** mantener View o pulsar F1 abre una hoja con todas las acciones de la pantalla y su tecla y botón.
- **Primer paso:** hacer C7 antes. Sin una tabla única, esta hoja sería otra lista a mano que acabaría desfasada.

---

## C. Técnicas (rendimiento, arquitectura, pruebas, mantenimiento)

| # | Propuesta | Valor para el usuario | Esfuerzo | Riesgo |
|---|---|---|---|---|
| C1 | `npm test` y pruebas del frontend en CI | Alto | S | Bajo |
| C2 | `.gitattributes` para los finales de línea | Alto | S | Bajo |
| C3 | Registro a fichero en la versión publicada | Alto | S | Bajo |
| C4 | Refrescos baratos: nada de reescaneo completo al volver a la ventana | Alto | M | Medio |
| C5 | Prueba de contrato entre la tablet y `remote_call` | Medio | S | Bajo |
| C6 | CHANGELOG y README al día | Medio | S | Bajo |
| C7 | Tabla única de acciones por pantalla y división de `controller.svelte.ts` | Alto | L | Medio |
| C8 | Tipos de TypeScript generados desde Rust | Medio | M | Bajo |
| C9 | Capturas bajo demanda en vez de la biblioteca entera | Medio | L | Alto |

**C1. `npm test` y pruebas del frontend en CI.**
- **Qué es:** `tests/weekreview.test.ts` y `tests/readgoals.test.ts` se lanzan a mano con `node --test`. `package.json` no tiene un script `test`, y el CI solo ejecuta `cargo test` y `svelte-check` (`.github/workflows/build-windows.yml:38-43`).
- **Por qué:** `stats.ts` (600 líneas), `milista.svelte.ts` y `readgoals.ts` son lógica pura que alimenta Estadísticas y Mi lista, justo lo que se está rehaciendo.
- **Primer paso:**
  - Añadir `"test": "node --test tests/"` a `package.json`.
  - Añadir ese paso al CI con Node 22.18 o posterior. Hoy el CI usa Node 20, que no ejecuta TypeScript.

**C2. `.gitattributes` para los finales de línea.**
- **Qué es:** el repositorio mezcla LF y CRLF, y cada plan exige comprobarlos a mano antes de cada commit (`plan-interfaz-pendiente.md:13`). No existe `.gitattributes`.
- **Primer paso:**
  - Un `.gitattributes` con `* text=auto`, más reglas explícitas para `*.rs`, `*.ts`, `*.svelte` y `*.md`.
  - Un único commit de normalización (`git add --renormalize .`), hecho a solas para que el diff sea revisable.

**C3. Registro a fichero en la versión publicada.**
- **Qué es:** el código tiene 81 `eprintln!` (por ejemplo, los tiempos de escaneo en `src-tauri/src/lib.rs:254`), pero la versión publicada no tiene consola (`main.rs:2`). Precisamente los diagnósticos del HDD, de Magpie y de la tablet se pierden donde más falta hacen.
- **Primer paso:**
  - Usar `tauri-plugin-log`, o un escritor propio con rotación a 1 MB, en `%LOCALAPPDATA%\app.visorcapturas.desktop\logs`.
  - Cambiar los `eprintln!` por `log::info!` o `log::warn!`. Este cambio es masivo y repetitivo: apto para `agy`.
  - Añadir «Abrir registro» en Ajustes › Sistema.

**C4. Refrescos baratos.**
- **Qué es:** cada vez que la ventana recupera el foco se lanza un reescaneo completo (`controller.svelte.ts:2352`), y también tras cada captura (l.1838) y tras cada partida (l.1865). `scan_full` (`lib.rs:221-256`) vuelve a detectar todos los juegos, lista todas las carpetas, sincroniza el catálogo y manda por IPC la biblioteca entera. Al salir de un juego suelen coincidir dos escaneos: el del foco y el de `game-exited`.
- **Propuesta:**
  - Al recuperar el foco, comprobar solo el arte del título visible. Ese es el motivo que da el comentario de la l.2351.
  - Tras `capture-saved`, añadir un comando `scan_title(platform, folder)` que devuelva un solo `Title` y lo sustituya en `app.lib`.
  - Más adelante, un vigilante `notify` sobre la raíz que use ese mismo refresco por título.
- **Primer paso:** quitar el reescaneo de la l.2352 y medir con `examples/bench_scan.rs` en la biblioteca real (ver la memoria `library-on-hdd`).

**C5. Prueba de contrato entre la tablet y `remote_call`.**
- **Qué es:** `src-tauri/src/remote_call.rs` reparte a mano unos 80 comandos y no tiene ninguna prueba. Un comando que falte da 403 en la tablet en tiempo de ejecución.
- **Primer paso:** una prueba en Rust que extraiga con regex los nombres de `invoke('…')` de `src/` y compruebe que cada uno está en `dispatch` o en una lista explícita `PC_ONLY`. Así, un comando nuevo obliga a decidir si la tablet puede usarlo.

**C6. CHANGELOG y README al día.**
- **Qué es:**
  - La release dice «Novedades en CHANGELOG.md» (`build-windows.yml:56`), pero ese fichero no existe.
  - El README se contradice: en `README.md:94` dice que todo sigue en JSON y en la l.66 que vive en `visor.db`.
  - El README da como pendientes la búsqueda y la bandeja (l.280 y 283), que ya existen (`Search.svelte`, `tray.rs`).
- **Primer paso:**
  - Un `CHANGELOG.md` que empiece en 2.0.1, con las entradas sacadas de `git log`.
  - Podar las secciones «Pendiente» y «Verificación» del README.

**C7. Tabla única de acciones por pantalla y división de `controller.svelte.ts`.**
- **Qué es:** `controller.svelte.ts` tiene 2.377 líneas. El significado de cada botón vive en `cmd` (l.1909-1987), y las pistas se escriben a mano en 28 ficheros que usan `<Hints>`. Los fallos 2, 3 y 9 de la revisión del 9-nov son justo esas dos fuentes desincronizadas.
- **Propuesta:**
  - Un `actions.ts` por vista con `{ boton, etiqueta, cuando(), hacer() }`, del que salen a la vez `cmd` y las pistas.
  - Con eso, dividir el controlador por pantalla (`ctl/titles.ts`, `ctl/viewer.ts`…).
  - Esto complementa el contrato de botones de la revisión: no lo sustituye.
- **Primer paso:** migrar solo el visor, como piloto. Después, el resto de vistas, que es trabajo repetitivo apto para `agy`.

**C8. Tipos de TypeScript generados desde Rust.**
- **Qué es:** `src/lib/types.ts` (467 líneas) copia a mano las estructuras de Rust, como `Title` y `Shot` de `visor-core/src/scan.rs`. Cada campo nuevo se escribe dos veces, y la tablet depende de los mismos nombres.
- **Primer paso:**
  - Añadir `ts-rs`, que respeta `serde(rename_all = "camelCase")`, a `visor-core`, empezando por `Library`, `Platform`, `Title` y `Shot`.
  - Generar `src/lib/gen/*.ts` desde `cargo test` y hacer que `types.ts` los reexporte.

**C9. Capturas bajo demanda.**
- **Qué es:** cada escaneo manda todos los `Title` con su lista `shots` completa (`types.ts:215`). Son unas 14.000 entradas por refresco, que se recorren en 111 sitios con `.shots` repartidos en 27 ficheros.
- **Propuesta:** que la biblioteca lleve solo un resumen por título (número, primera, última y portada) y que las capturas se pidan al entrar en el título.
- **Riesgo:** alto, porque toca Estadísticas, Colecciones e Inicio.
- **Primer paso:** medir el tamaño del JSON y el tiempo de deserialización en la biblioteca real antes de decidir. Si C4 basta, no hacer C9.

---

## Las 5 que haría primero

1. **C4, refrescos baratos.** Es lo que más se nota a diario con el HDD: deja de reescanearse todo con cada Alt+Tab y cada captura.
2. **C1 y C2, `npm test` en CI y `.gitattributes`.** Son media jornada en total y protegen la reorganización y las fases de Estadísticas y Mi lista.
3. **A4, capturas repetidas.** La huella ya está calculada y la Papelera ya existe: solo falta la consulta y el panel.
4. **B1 y B2, pistas por dispositivo y atajos de teclado.** Son baratas y dejan teclado y mando al mismo nivel.
5. **A5, repaso al terminar la partida.** Reutiliza el visor y la Papelera, y evita que la biblioteca siga creciendo sin control.
