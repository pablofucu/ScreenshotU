# Plan: la app de tablet

Fecha: 2026-11-01. Solo planificación: aún no hay nada programado de esto. Sustituye a los «Próximos pasos» de
`docs/estudio-streaming-tablet.md` (ese informe sigue valiendo como referencia técnica).

Tres bloques: **interfaz igual que el escritorio**, **jugar en el PC (Apollo/Artemis)** y **conexión PC‑tablet
más sencilla**. El móvil (pantalla < 600 dp) se queda como está por ahora.

---

## 1. Interfaz: la tablet usa la interfaz del escritorio, no una copia

### Situación
- `mobile/src/` tiene pantallas propias (Home, Games, Detail, Viewer, Stats…), unas 3.600 líneas, hechas antes del
  rediseño v5: no tienen atmósfera, degradado, Mi lista en franjas, colecciones nuevas, logros por rareza, etc.
- El escritorio (`src/`) ya habla con el PC a través de una sola interfaz, `Backend` (`src/lib/types.ts`), que hoy
  tiene dos versiones: la de Tauri (`invoke(...)`) y la simulada (`src/lib/mock.ts`).

### Propuesta
En vez de redibujar las pantallas del móvil para que «se parezcan», **la tablet carga la misma interfaz del
escritorio** con una tercera versión del `Backend`: **`RemoteBackend`**, que hace lo mismo que `invoke` pero por
HTTP contra el visor del PC. Resultado:
- Organización, colores, menús y pantallas **idénticos** al escritorio, y cada cambio gráfico futuro llega a la
  tablet sin hacer nada.
- Una sola interfaz que mantener en lugar de dos que se separan.

Al arrancar, la APK mira el tamaño de pantalla: tablet (≥ 600 dp) → interfaz del escritorio; móvil → la de ahora.

### Qué hay que hacer
1. **`RemoteBackend`** (`src/lib/backend-remote.ts`): cada método llama a una ruta del PC. Como la versión de Tauri
   ya es una capa fina sobre `invoke`, en el PC basta **una ruta genérica** `POST /api/v1/call {método, args}` con
   una **lista blanca** de métodos permitidos que reenvía a los mismos comandos de Rust. Imágenes: las rutas
   `thumb`/`preview`/`file` que ya existen.
2. **Eventos** (`backend.onEvent`: juego cerrado, capturas nuevas…): ruta `GET /api/v1/events` (espera larga o
   SSE). La misma ruta sirve después para «volver a la app» y el resumen de la sesión (bloque 2).
3. **Funciones solo de PC**, ocultas en la tablet con un indicador `backend.caps`: elegir carpetas/ficheros,
   añadir juegos, copias de seguridad, monitores, salida de audio, mando, atajos de teclado, Anki, abrir carpeta.
   Se quedan en el PC; en la tablet no aparecen (no se ven deshabilitadas, simplemente no están).
4. **Qué sí se puede cambiar desde la tablet** (propuesta): favoritos, estado/nota del juego, orden de Mi lista,
   aplazar, colecciones, color de acento, ocultar, mover capturas, mandar a la papelera.
5. **Adaptación táctil** (la parte de diseño de verdad):
   - Toques en lugar de ratón/mando: objetivos ≥ 44 px, pulsación larga = menú contextual (el `ActionMenu`
     del mando sirve tal cual), deslizar en el visor de capturas y en las franjas de Mi lista.
   - Pocas cosas dependen de pasar el ratón por encima (≈ 17 sitios): revisarlas una a una.
   - Horizontal como orientación principal; vertical con menos columnas.
   - Márgenes seguros de Android, botón atrás del sistema = atrás de la interfaz.
6. **Rendimiento por Wi‑Fi**: la biblioteca se guarda en la tablet (como hace hoy el móvil con ETag) para abrir al
   instante; miniaturas en caché.

### Riesgos
- Algunas pantallas del escritorio pueden leer ficheros o usar Tauri directamente fuera de `backend` (hay ~18
  usos sueltos de `@tauri-apps`/`listen`): hay que pasarlos por `backend`.
- Rendimiento de Svelte + animaciones de atmósfera en la tablet: probar pronto (Fase T1).

---

## 2. Jugar en el PC: Apollo + Artemis

### Recomendación
**Apollo en el PC y Artemis en la tablet.** La pantalla virtual de Apollo, con la resolución exacta de la tablet,
resuelve de una vez la calidad, el estirado y probablemente los toques desajustados con Magpie (la imagen y la
pantalla tienen la misma proporción). Artemis añade resolución/bitrate a medida y salir bien de la sesión. Los dos
mantienen el `ShortcutTrampoline` que ya usamos.

Todo se diseña **agnóstico** (vale también con Sunshine + Moonlight), por si Apollo da problemas.

### Antes de programar: prueba de 15 minutos (la haces tú)
1. Desinstalar Sunshine, instalar Apollo (v0.4.6 estable) e instalar Artemis en la tablet.
2. Emparejar, abrir «Desktop» y comprobar: nitidez, si los toques caen donde deben **con y sin Magpie**, y si se
   ve el cursor.
3. Decirme el nombre de paquete que muestra Android para Artemis (Ajustes → Aplicaciones), que no está claro.

Si la prueba sale mal → seguimos con Sunshine y cambiamos la resolución del monitor real al conectar.

### Qué se construye (orden de arranque nuevo, del estudio)
**PC (nueva versión del visor):**
1. «Juego pendiente»: la tablet pide jugar; el visor apunta el juego, aún no lo lanza.
2. Ayudante `visor.exe --stream-session` configurado como aplicación «Juego actual» en Apollo/Sunshine. Al
   conectar: avisa al visor de la resolución (`SUNSHINE_CLIENT_WIDTH/HEIGHT/FPS`), el visor lanza el juego en la
   pantalla correcta; el ayudante espera mientras el juego vive y sale al cerrarse → la sesión termina sola.
   **El visor crea esa aplicación en Apollo/Sunshine él mismo** (botón en Ajustes), para que no tengas que
   configurarla a mano.
3. `POST /api/v1/close`: cerrar el juego desde la tablet (cierre normal y, si no responde, forzado).
4. Ocultar el cursor durante la sesión (si Apollo no lo hace ya) y restaurarlo siempre al terminar o al arrancar.
5. La última sesión (juego, minutos, capturas, progreso de metas) en los eventos.

**Tablet:**
1. Soporte del paquete de Artemis (además de Moonlight).
2. Volumen abajo = captura (ya hecho). **Volumen arriba = menú** superpuesto que desaparece solo: capturar, cerrar
   el juego, volver a la app. Sin botones fijos.
3. Al cerrarse el juego: vuelta a la app y **resumen de la sesión** con la misma animación de estadísticas que el
   escritorio. Como la tablet usa la interfaz del escritorio (bloque 1), **la animación es la misma pieza**, no
   hay que reescribirla.
   - Si Android no deja abrir la app desde segundo plano: notificación «Sesión terminada · ver resumen».

---

## 3. Conexión PC‑tablet: menos lío y que no se rompa

### Cómo funciona hoy y por qué es liosa
| Qué | Hoy | Problema |
|---|---|---|
| Encontrar el PC | Escribir la IP a mano | Si el router le da otra IP al PC, la tablet deja de conectar |
| Autorización | Código de 8 letras escrito a mano | Engorroso; se guarda en los dos lados (eso sí persiste) |
| PC encendido | El servidor solo existe con el visor **abierto** | Tras reiniciar el PC, la tablet no conecta hasta abrir el visor |
| Streaming | Emparejar Moonlight con Sunshine aparte (PIN) | Segundo emparejamiento, otra app, otro concepto |

Lo que sí es permanente hoy: el código (guardado en la config del PC y en la tablet) y el emparejamiento de
Moonlight/Sunshine. Lo que se rompe: la IP y que el visor esté cerrado.

### Propuesta
1. **Emparejar con QR**: en Ajustes del visor → «Conectar tablet» enseña un QR (dirección + código). En la tablet,
   «Escanear» y listo. Se acaba escribir IP y código. (Queda «escribir a mano» como alternativa escondida.)
2. **Encontrar el PC solo**: el visor se anuncia en la red local (mDNS o difusión UDP) con un **identificador fijo
   del PC**. La tablet guarda ese identificador, no la IP: si la IP cambia, lo vuelve a encontrar sin preguntar.
3. **Servidor siempre disponible**: opción «Arrancar con Windows en segundo plano» (icono en la bandeja, sin
   ventana). Así, tras reiniciar el PC, la tablet conecta sin tocar nada. Apollo ya arranca como servicio de
   Windows.
4. **Un solo emparejamiento**: el visor guía el de Apollo/Artemis desde la misma pantalla («Paso 2: emparejar el
   streaming»). Apollo tiene emparejamiento con contraseña de un solo uso; **por verificar** si el visor puede
   pedirla él y si Artemis la acepta por enlace, para que sea un toque. Si no, el visor al menos explica el paso y
   comprueba que quedó hecho.
5. **Estado claro** en la tablet: un punto «PC conectado / PC dormido / visor cerrado» en lugar de errores sueltos.
   (Despertar el PC por red —Wake‑on‑LAN— es posible pero opcional: depende de la BIOS y el router.)

---

## 4. Fases

| Fase | Qué | Quién | Se prueba |
|---|---|---|---|
| **T0** | Prueba de Apollo + Artemis (15 min) | Tú | Nitidez, toques con/sin Magpie, cursor, nombre de paquete |
| **T1** | `RemoteBackend` + ruta `/api/v1/call` + la interfaz del escritorio en la APK en modo solo lectura | Yo (+ agy flash para el barrido de métodos) | La tablet enseña Inicio, Biblioteca, Mi lista, Estadísticas iguales que el PC |
| **T2** | Adaptación táctil, ocultar funciones de PC, escrituras permitidas, eventos | Yo | Uso real en la tablet |
| **T3** | Conexión: QR, descubrimiento por identificador, arranque con Windows en bandeja | Yo | Reiniciar PC y tablet, cambiar IP: sigue conectando |
| **T4** | Streaming: juego pendiente, ayudante, cerrar, cursor, menú de volumen arriba, vuelta a la app + resumen | Yo | Sesión completa de lectura de principio a fin |
| **T5** | Publicar cliente de PC y APK | Yo | — |

T1–T2 y T3 son independientes y pueden ir en paralelo. T4 depende de T0 y aprovecha los eventos de T1.

## 5. Decisiones que necesito de ti
1. **¿Te parece bien que la tablet use directamente la interfaz del escritorio** (bloque 1) en lugar de rehacer
   las pantallas del móvil? Es la recomendación.
2. **Qué se puede cambiar desde la tablet**: ¿vale la lista del punto 1.4, o la tablet solo mira?
3. **Orientación**: ¿solo horizontal, o también vertical?
4. **Arrancar el visor con Windows en la bandeja**: ¿sí?
5. Hacer la **prueba T0** cuando puedas y pasarme el resultado.

### Respuestas (2026-11-01)
- Sí: la tablet usa la interfaz del escritorio **tal cual** (sin rediseño táctil; basta con que funcione al tocar).
- Mismas funciones que el escritorio: mirar, favoritos, capturas, estados, Mi lista, etc.
- Solo horizontal.
- Apollo + Artemis: adelante.
- **La app debe funcionar con el PC apagado**: en el futuro capturas y partidas guardadas estarán en un servidor en
  la nube. Por eso el `RemoteBackend` habla con «un servidor» (hoy el PC, mañana la nube, misma API) y guarda en la
  tablet lo último sincronizado: datos en IndexedDB, imágenes en la caché HTTP, y los cambios hechos sin conexión
  (favoritos, estados, orden…) en una cola que se envía al volver.
- El PC tiene IP fija, pero se quiere igualmente que la app lo encuentre sola (fase T3).

## Fuera de alcance
- Rediseño del móvil (pantalla < 600 dp): más adelante.
- Las categorías de la biblioteca no se tocan (`CLAUDE.md`).
