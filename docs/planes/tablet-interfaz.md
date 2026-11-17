# Tablet con la interfaz nueva

Fecha: 2026-11-17. Rama TCL en Unificador. Sustituye lo pendiente de «adaptación táctil» de `archivo/plan-tablet.md`.

## Punto de partida

- La tablet monta `src/App.svelte` tal cual (`IS_TABLET`, `mobile/src/tablet.ts`): todo lo nuevo del PC (Estadísticas en una pantalla, Rachas, Mi lista, Inicio) ya llega solo, pero pensado para mando.
- Lienzo de 1280×720 escalado: en la POCO Pad (2560×1600, 16:10) quedan dos franjas vacías.
- Tocar = clic sobre un `[data-k]` (`onScreenClick`); arrastrar = rueda (`startTouch`). Lo que no tiene `data-k` no responde (Terminados, la pantalla de racha y partes de Estadísticas).
- Las pistas de mando se ven y se pueden pulsar (`data-hact`): hoy son la única forma de hacer algunas acciones con el dedo.

## Decisiones (17-nov, con el usuario)

| Tema | Decisión |
|---|---|
| Lienzo | 1280×800 solo en la tablet, sin franjas. |
| Navegación | La del PC (Inicio con su dock, entrar y volver). Sin barra fija nueva. |
| Volver | Flecha visible arriba a la izquierda en cada pantalla que no es Inicio, y el gesto de atrás de Android. |
| Mando | Solo dedo: fuera las pistas y la banda inferior; las acciones pasan a botones táctiles visibles. Sin foco visible: el estado activo es el pulsado (0,97). |
| Alcance | Misma interfaz con todo pulsable (44 px mínimo) y diseño propio donde compense. |
| Pantallas con maqueta propia | Todas las grandes: Visor; Biblioteca y ficha; Estadísticas y Rachas; Inicio, Mi lista y Colecciones. |
| Orientación | Solo horizontal. |
| Menú sobre el juego | Entra: 16:10, sin pistas, todo pulsable. |

## Cómo se hace

1. **Lienzo.** `App.svelte` usa alto 800 cuando `IS_TABLET`; los tokens `--band` y `--content-end` pasan a depender del lienzo (en la tablet sin banda: el contenido acaba a 24 del borde). Repasar las pantallas que cuentan con 720 o 644 fijos.
2. **Toques.** Barrido de pantallas para que todo lo que se elige con el mando tenga `data-k` o su botón; objetivos de 44 px en `.tablet`; listas largas con desplazamiento natural en vez de rueda emulada donde se note.
3. **Acciones sin pistas.** Las pistas de cada pantalla se convierten en botones táctiles (el sitio y la forma salen de las maquetas: barra de acciones arriba, botones junto al elemento o pulsación larga para el menú de acciones).
4. **Volver.** Flecha en la barra superior (`TopBar`), que llama a lo mismo que B; el atrás de Android ya va a `back()`.
5. **Pantallas propias** (según maqueta elegida): Visor (pellizcar para ampliar, doble toque, tira de miniaturas), Biblioteca y ficha, Estadísticas y Rachas (pestañas tocables, carriles con más alto), Inicio, Mi lista y Colecciones (Colecciones sigue a la propuesta P-13 que se elija).
6. **Menú sobre el juego** (`src/quickmenu/`): mismo trato.

Todo vive tras `.tablet` o `IS_TABLET`: el PC no cambia. Las categorías no se tocan.

## Maquetas

En `docs/maquetas/tablet/`, a 1280×800, una por grupo de pantallas, con dos o tres variantes donde haya duda real (sobre todo dónde van las acciones). Se proponen por Unificador.

**Elegida (17-nov, P-15): B «En su sitio»** ([maquetas/tablet/b-en-su-sitio.html](../maquetas/tablet/b-en-su-sitio.html)). Cada acción va junto a lo que toca (Jugar en la ficha, barra sobre la imagen en el visor, Ordenar junto a los filtros) y el menú de una portada o captura sale manteniendo pulsado. La maqueta solo vale para **dónde** va cada cosa: el aspecto es el del PC tal cual (mismo fondo, mismos iconos de `src/lib/icons`, franjas y degradados arcoíris, la racha con sus velas y animaciones). Por eso no se hacen más maquetas: se trabaja sobre los componentes reales con `.tablet`, cambiando solo sitio, orden y tamaño de toque.

## Hecho (17-nov, TCL-4 a TCL-9 y TCL-11)

- Lienzo de 1280×800 en la tablet. La banda de abajo (64 px) se queda: ahí van las acciones como botones táctiles («‹ etiqueta ›» para los pares LB/RB y LT/RT), y lo que ya está en su sitio no se repite (pestañas, flechas del periodo, Jugar en la ficha, Buscar en el catálogo, la flecha de volver).
- Pulsación larga = menú de acciones. Objetivos de 44 px en pestañas, flechas, selectores de Rachas, Ordenar y Buscar.
- Visor: marco más grande, pellizco para ampliar, arrastrar con un dedo al estar ampliado, deslizar para pasar captura. Como no tiene barra arriba, «Volver» se queda abajo.
- Menú sobre el juego (`quickmenu.html?tablet` en desarrollo): lienzo de 1280×800, sin glifos del mando, botones de 44 px.

Falta: probarlo en la POCO Pad (TCL-10).

## Comprobación

`npm run check`; la app de prueba en el navegador a 1280×800 con toque emulado; la APK en la POCO Pad la prueba el usuario (TCL-10 y lo que queda de ODD-8).
