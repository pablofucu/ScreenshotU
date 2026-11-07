# Plan: volver a la partida desde el visor (integración con Artemis, opción A)

Fecha: 2026-11-05. Origen: `docs/plan-tablet.md` §2 y `docs/plan-t4.md`, que ya están hechos y **no se rehacen**.

**Objetivo:** con una partida en marcha, moverse entre el visor y el juego sin abrir Artemis a mano y casi sin ver sus
pantallas. Artemis sigue instalado; integrar el streaming dentro de la app (opción B) queda fuera de este plan.

Las categorías de la biblioteca no se tocan. El escritorio no cambia: todo lo nuevo solo actúa con `IS_TABLET`.

## Situación actual

- **No se puede volver:** `playTitle` responde «Ya estás jugando a X» y no hace nada más
  (`src/lib/controller.svelte.ts:386`). La opción de jugar desaparece del menú (`controller.svelte.ts:989`).
- **El PC ya admite la reconexión:** `session_start` acepta una conexión con la partida en `Starting` o `Playing`, y
  también en `Idle` si el juego sigue abierto (`src-tauri/src/stream.rs:317-340`). El ayudante de «Juego actual» solo
  termina cuando la sesión pasa a reposo. **En principio no hay que tocar ese flujo.**
- **Si la app de la tablet se reinicia, olvida la partida:** `syncPlaying` sale si `a.playing` está vacío
  (`mobile/src/tablet.ts:70`), así que nunca lo rellena a partir del PC.
- **El vigilante se para pronto:** `startWatch` deja de preguntar al PC en cuanto Artemis sale de primer plano
  (`CaptureKeyService.java:208-244`).
- **`GET /api/v1/session` se queda corto:** no dice qué app de Apollo se usa ni cuánto lleva la partida
  (`stream.rs:486`).

## Decisiones

1. **Volver nunca usa `launch_game`.** Daría «Ya hay un juego en marcha». Se abre el stream directamente con
   `Streaming.stream`, y Apollo retoma la sesión.
2. **El PC decide qué hay en marcha y con qué app de Apollo.** `session_json` añade `app` y `elapsedSeconds`. La tablet
   no depende de lo que recordara antes de reiniciarse, y calcula `since` desde `elapsedSeconds` para no depender de
   que los relojes coincidan.
3. **Atrás solo devuelve al visor si se viene del stream.** El servicio se arma al ver `com.limelight.Game` y solo actúa
   si después aparece la lista de Artemis (`PcView` o `AppView`). Abrir Artemis a mano para cambiar sus ajustes no te
   saca de él.
4. **El botón Inicio no te arrastra de vuelta.** Si sales al escritorio de Android o a otra app, es a propósito; para
   volver está la notificación.
5. **Al acabar la partida, solo se trae el visor al frente si Artemis estaba delante o acababa de salir.** Si estás en
   otra app, solo se avisa con la notificación de fin de sesión.
6. **Compatibilidad:** con un visor de PC antiguo, sin `app`, se usa el mismo respaldo de hoy: el ajuste de la app o
   «Desktop» (`mobile/src/tablet.ts:42`).

## Pasos

### P0. Prueba previa en la tablet (Pablo, 10 min)
Hace falta para fijar P4; no se necesita compilar nada. Durante una partida, anotar:

| Acción | Qué mirar |
|---|---|
| Atrás y desconectar en el menú de Artemis | Qué pantalla sale: visor, lista de Artemis o escritorio de Android |
| Atrás y salir del stream («quit») | Lo mismo, y si el juego sigue abierto en el PC |
| Volver a entrar desde Artemis | Cuánto tarda, si el juego sigue en la pantalla virtual y si cambia de resolución o de tamaño |
| Tocar «Volver a la app» en el menú de volumen | Si el stream se corta, y lo mismo al volver a entrar |

Si la pantalla virtual desaparece y el juego queda mal colocado al volver, **parar y revisarlo antes de seguir con el
resto**: habría que resolverlo en el PC.

### P1. PC: ampliar el estado de la sesión
- `src-tauri/src/stream.rs`: añadir `stream_app: Option<String>` a `Session`. Se rellena en `launch` (el mismo valor que
  ya devuelve, `:252` y `:266`) y se limpia en `reset_idle` (`:279`). Comprobar también dónde se limpia la sesión al
  salir el juego (`:72-100`).
- `session_json` (`:486`): añadir `app` y `elapsedSeconds` (0 si `started_ms` es 0 o la fase es `Pending`).
- Reconexión desde `Idle`: `stream_app` estará vacío; la tablet usa el respaldo de la decisión 6.
- Tests: ampliar los de la ruta en `src-tauri/visor-core/src/remote.rs:1350-1356` si el estado es comprobable allí.

### P2. Tablet: recuperar la partida
En `mobile/src/tablet.ts:69-85`:
- Quitar `!a.playing` de la condición de salida de `syncPlaying`.
- Si el PC dice `starting` o `playing` con un juego, poner `a.playing = { id, since: Date.now() - elapsedSeconds * 1000 }`.
- Guardar `app` en una variable para P3.
- `pending` no rellena nada nuevo: no pisa lo que ya puso `playTitle`.
- Mantener el caso actual: `idle` o sin juego limpia `a.playing` y refresca.
- Llamarla también al arrancar, además de con `visibilitychange` y cada 5 s. Atención: `game-started` también rellena
  `a.playing` (`controller.svelte.ts:1672`), y un `idle` del PC lo limpia; el comportamiento con un juego lanzado en el
  propio PC debe seguir siendo el de ahora.

### P3. Interfaz: el botón de volver
**Punto de enganche:** `setResumeHandler(fn)` en `src/lib/controller.svelte.ts`, parecido a `setAfterLaunch`. Solo lo
registra `mobile/src/tablet.ts`, así que en el escritorio sigue vacío y no cambia nada. El handler llama a
`Streaming.stream` con el `app` de P2 y muestra un aviso si falla.

- **Ficha:** en `playTitle` (`:386`), si es el mismo juego y hay handler, volver en vez de mostrar el aviso. En
  `src/screens/Detail.svelte`: `:207` pasa a «Volver a la partida» con «Sesión {sess}» debajo, y la pista de `:261`
  cambia «En marcha» por «Volver». Se puede tocar sin cambios porque `data-act="play"` ya llega a `playTitle` (`:551`).
- **Menú de acciones:** `:989` muestra `play` también si hay handler y es el mismo juego; en
  `src/screens/ActionMenu.svelte:73` la etiqueta pasa a «Volver a la partida».
- **Píldora en toda la app:** nuevo `src/components/ResumePill.svelte`, montado en `src/App.svelte` junto a `offpill`
  (`:204`), con el mismo estilo de cristal. Texto: «{juego} · {tiempo}», con botones **Volver** y **Cerrar**.
  - Solo con `IS_TABLET`, `app.playing` y conexión. Sin conexión no se muestra y queda `offpill`.
  - No debe tapar el visor de capturas a pantalla completa, ni molestar en la ficha del propio juego, que ya tiene su
    botón. Decidir la posición viéndola en la tablet o en una captura antes de cerrarla.
  - **Cerrar** usa `POST /api/v1/close` con el mismo flujo de dos pasos que `closeGame`
    (`CaptureKeyService.java:157-175`): cerrar con normalidad y, si el juego no se cierra, ofrecer forzarlo. Se pide
    confirmación antes de cerrar, porque se puede perder lo no guardado.

### P4. Android: atrás lleva al visor
Según el resultado de P0:

| Tras salir del stream aparece | Qué se hace |
|---|---|
| El visor | Nada |
| La lista de Artemis | Aplicar la decisión 3 en `onAccessibilityEvent` con `event.getClassName()` y llamar a `returnToApp()` |
| El escritorio de Android | Nada automático (decisión 4); queda la notificación de P5 |

Los nombres `com.limelight.Game`, `PcView` y `AppView` salen de lo que se recuerda del código de Moonlight: **confirmarlos
en la tablet** antes de darlos por buenos. Los diálogos, como el menú de atrás de Artemis, no deben disparar nada, por eso
se usa una lista de pantallas permitidas y no «cualquier cosa que no sea `Game`».

Si la conexión falla, Artemis puede volver a su lista y el visor se traerá delante: es aceptable, pero que se vea el motivo
con un aviso si es fácil.

### P5. Android: notificación «Partida en curso»
- **Vigilante:** reformar `startWatch` para que siga vivo mientras la sesión no esté en reposo: cada 2 s con Artemis
  delante y cada 5 s fuera. Si el PC no responde durante unos 2,5 min seguidos, se retira la notificación y se para. Volver
  a Artemis lo arranca de nuevo.
- **Fin de partida:** aplicar la decisión 5 a `sessionEnded` (`:246`).
- **Notificación:** canal nuevo `partida`, importancia baja y sin sonido, con `setOngoing(true)`. Se muestra con la partida
  activa y Artemis fuera de primer plano; se quita al volver a Artemis y al acabar. Texto: «{juego} · Partida en curso».
  El permiso ya se pide en `MainActivity.java:16`.
- **Acciones:** tocar la notificación abre el visor. El botón **Volver a la partida** abre el stream directamente mediante
  una `ResumeActivity` nueva, invisible y no exportada, que lanza el `ShortcutTrampoline` y termina.
- **Datos para volver:** sacar el lanzamiento de `StreamingPlugin.stream` (`:138-166`) a un método estático reutilizable, y
  guardar `uuid`, `pc` y `app` en las preferencias, porque `sunshineInfo()` usa la red y `ResumeActivity` debe ser inmediata.
  Declarar `ResumeActivity` en `mobile/android/app/src/main/AndroidManifest.xml`.
- **Vigilante y `app`:** actualizar las preferencias cuando la sesión traiga un `app` nuevo.
- Actualizar `capture_service_desc` en `strings.xml` si la descripción deja de ser exacta.

### P6. Publicar
Subir la versión del visor del PC y compilar la APK nueva. Al compilar, comprobar que la APK contiene el cambio (ya salió
una vez sin regenerar). Tras cada paso: commit y push según `CLAUDE.md`, y solo con los ficheros de ese paso.

## Orden y dependencias

P0 → P1 → P2 → P3 → P4 → P5 → P6. P3 aporta lo principal y se puede probar en cuanto esté: si P4 o P5 se complican, se
puede publicar sin ellos. P5 es la parte más grande.

## Pruebas (de principio a fin en la tablet)

1. Jugar, atrás, desconectar → se llega al visor → «Volver» en la píldora → mismo juego, sin relanzarlo y con la sesión y
   el contador de tiempo intactos.
2. Lo mismo desde la ficha y desde el menú de acciones.
3. Botón Inicio durante el stream → no te devuelve al visor → «Volver» en la notificación → stream.
4. Cerrar la app de la tablet del todo con la partida abierta y reabrirla → aparece la píldora con el tiempo correcto.
5. Abrir Artemis a mano sin partida, y también tras volver al visor → no te saca de Artemis.
6. Cerrar desde la píldora, cerrar desde el menú de volumen y cerrar el juego en el PC → resumen correcto una sola vez,
   píldora y notificación retiradas, cursor del PC restaurado.
7. Volver con el PC apagado o sin red → aviso claro, sin bloqueos y sin notificación colgada para siempre.
8. Terminar la partida estando en otra app → no te saca de ella y llega el aviso de fin de sesión.
9. Moverse varias veces seguidas entre visor y juego, para detectar pérdidas de sesión y descontar bien el tiempo.
10. Ordenador de escritorio: jugar y el botón «Jugando» se comportan como antes.

## Riesgos

- **Pantalla virtual:** si Apollo la quita al desconectar, el juego puede moverse o cambiar de tamaño al volver, y Magpie
  podría verse afectado. Lo aclara P0; es el único riesgo que puede cambiar el plan.
- **Tiempo de reconexión:** son unos segundos que este plan no puede eliminar; si molestan, es el motivo para pasar a la
  opción B.
- **Nombres internos de Artemis:** pueden cambiar con sus versiones. Si cambian, se pierde solo la vuelta automática de P4;
  la píldora y la notificación siguen funcionando.
- **Android:** puede bloquear abrir la app desde segundo plano. Ya existe el mismo riesgo al acabar la partida, y la
  notificación cubre el fallo.

## Fuera de alcance

- Integrar el streaming dentro de la app (opción B) o mantener una copia modificada de Artemis.
- Eliminar el emparejamiento aparte de Artemis/Apollo.
- El móvil (pantalla < 600 dp).
