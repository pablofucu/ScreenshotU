# Plan de la versión 1.4.0

Planificación cerrada con Pablo. Los bocetos de referencia están en `diseno/` (sobre todo `diseno/titulos-color.html`, variantes 3 «Aurora» y 4 «Pestaña multicolor armonizada», y `diseno/titulos-mezcla.html`, variante M1).

## Reglas que no se rompen

- **Las categorías de la biblioteca no se tocan** (ver `CLAUDE.md`). Juegos, Novelas visuales, Switch, Novelas japonesas… siguen igual. Todo lo nuevo vive **dentro** de cada categoría y nunca mezcla títulos de categorías distintas.
- Migraciones de `visor-core/src/db.rs`: nunca se edita una ya publicada, se añade una nueva al final (ver `docs/publicar.md`). Hay que añadir pruebas en `visor-core` para cada migración nueva.
- Todo pensado primero para mando; el ratón, que funcione donde ya funciona (clic, rueda).
- Textos de la interfaz en español, mismo tono que el resto de la app.
- El archivo mezcla finales de línea LF/CRLF: conservar los de cada archivo (comprobar el número de `\r` antes del commit).

## Fase 1 · Controles, capturas y visor

1. **Mantener pulsado repite, con aceleración.** LB, RB, LT y RT se repiten igual que las flechas (`REPEAT` en `controller.svelte.ts`). La repetición acelera cuanto más tiempo se mantiene: empieza como ahora (300 ms y luego cada 85 ms) y baja de forma progresiva hasta unos 30 ms. Aplica a todas las pantallas, sobre todo a la lista de capturas.
2. **Orden por defecto de las capturas, para toda la app.** Nuevo ajuste en Ajustes: «Orden de las capturas: más recientes primero / más antiguas primero». Es el orden con el que se abre cualquier lista de capturas (juego, saga/colección, favoritas, por días). Si un juego tiene un orden propio elegido con Y, ese manda para ese juego. Se guarda en la config.
3. **Borrar capturas, a la Papelera de Windows** (nunca borrado definitivo):
   - En el visor y en la lista de capturas: opción «Borrar captura» en el menú de Start, con confirmación.
   - En el modo selección: «Borrar las N seleccionadas», con confirmación.
   - Backend: mover a la papelera con la API de Windows (`SHFileOperationW` con `FOF_ALLOWUNDO`, o `IFileOperation`). En el modo navegador (`mock.ts`) simplemente quitarla de la lista.
   - Después de borrar se actualiza la biblioteca en memoria, las favoritas, la caché del catálogo y la miniatura, sin volver a escanear todo. El foco pasa a la captura siguiente.
4. **Zoom en el visor.**
   - Stick derecho arriba/abajo: acercar/alejar. Rueda del ratón: acercar/alejar hacia el puntero.
   - Con zoom, el stick izquierdo y las flechas mueven la imagen en lugar de cambiar de captura. B o volver al 100 % quita el zoom.
   - Se lee el stick derecho (`axes[2]`, `axes[3]`) en `padState`/`poll`, con zona muerta.
5. **Modo «solo imagen» en el visor.** El botón View oculta toda la interfaz (barra superior, pistas, datos) y deja solo la imagen. Se vuelve a pulsar View o B para salir. Es independiente de la pantalla completa de la ventana (F11), que ya existe.

## Fase 2 · Lista de juegos: orden, color y tamaño

1. **Orden de los juegos, separado de los filtros.** Ahora «A–Z» está mezclado con los filtros de LT/RT; se saca de ahí. En la lista de juegos, el botón **X** cambia el orden y se recuerda por categoría:
   - Última partida (por defecto): lo último jugado primero. Si no hay partidas, se usa la fecha de la última captura.
   - A–Z.
   - Por color: por el tono (hue) del color de acento automático de la portada, como un arcoíris. Los grises/sin color al final.
   - La cabecera muestra el orden activo, como ya hace la lista de capturas.
2. **Tamaño de las portadas (zoom de la cuadrícula).** El tamaño actual (5 por fila) es el de por defecto. Se puede reducir para ver más: 5 → 6 → 7 → 8 por fila (y volver). Controles: stick derecho arriba/abajo en la lista de juegos, teclas `+`/`-` y Ctrl+rueda. Se guarda en la config. La paginación con LB/RB y la ventana virtual se adaptan al nuevo tamaño de página.
3. **Pestañas dentro de cada categoría.** Arriba de la lista: **Todos · Mi lista · Colecciones**; el botón **View** pasa de una a otra. En esta fase solo existe «Todos»; las otras dos se rellenan en las fases 3 y 4. Hay que dejar preparada la estructura para que encajen.

## Fase 3 · Mi lista (estados de los juegos)

Estados existentes: Sin estado, Pendiente, Jugando, Terminado, Abandonado (`STATUS_ORDER`, tabla `title_status`).

1. **Pestaña «Mi lista»** dentro de cada categoría, con estanterías horizontales en el estilo de las colecciones (pestaña sobre balda, ver fase 4):
   - Jugando ahora
   - Pendientes, en el orden que elija el usuario (LB/RB mueve el juego enfocado dentro de Pendientes)
   - Terminados AAAA: una estantería por año, la más reciente primero, ordenada por fecha de terminado
   - Abandonados
   - Sin estado (al final)
   - ↑↓ cambia de estantería, ←→ de juego, A abre el juego, X lo lanza si se puede, Y abre la barra de estado.
2. **Barra rápida de estado.** Con Y en «Mi lista», y desde el menú de Start en «Todos» (la opción «Estado» que ya existe pasa a abrir esta barra): Sin estado · Pendiente · Jugando · Terminado · Abandonado.
   - Al marcar **Terminado**: se guarda la fecha de hoy (editable), una nota opcional de 1 a 5 estrellas y un atajo para añadirlo a una colección.
   - La nota y el orden de pendientes se guardan en la base (migración nueva).
3. **Asistencia, todo opcional y configurable en Ajustes** (sección nueva «Mi lista»):
   - «Juegos parados»: si un juego lleva N días (30 por defecto, editable) en «Jugando» sin jugarse, la app **pregunta** si pasarlo a Pendiente o Abandonado (o lo deja). Ajustes: activado/desactivado, días, «preguntar» o «mover solo a Pendiente/Abandonado».
   - «Al lanzar un pendiente, pasa a Jugando»: activado por defecto, se puede quitar.
   - «Jugar» cuenta como la última partida registrada; si el juego no tiene partidas (novelas sin lanzador), la fecha de la última captura.
   - Las preguntas salen como mucho una vez por juego y no se repiten si el usuario dice que lo deje.

## Fase 4 · Colecciones (sustituyen a las sagas)

1. **Modelo.** Una colección pertenece a **una sola categoría** (plataforma) y solo puede tener juegos de esa categoría. Campos: id, categoría, nombre, juegos (en orden), color (`null` = automático, o uno de los 7 colores de `RAINBOW`), orden manual entre colecciones. Migración nueva en la base:
   - Cada saga existente pasa a ser una colección de la categoría donde tenga más juegos.
   - Si una saga mezcla juegos de varias categorías, se divide en una colección por categoría con el mismo nombre.
2. **Fuera la plataforma virtual «Sagas».** Se quitan la baldosa de Sagas de la fila de Inicio y el botón «Sagas» del dock. `sagas.ts`/`withSagas` se sustituyen por el nuevo modelo.
3. **Pestaña «Colecciones»** dentro de cada categoría: estanterías verticales, una por colección (diseño M1 de `diseno/titulos-mezcla.html`):
   - **Pestaña** de cristal con el título grande, «N juegos · N capturas», la **línea de tiempo** (un punto por juego, según el año de su primera captura o partida; el punto del juego enfocado se ilumina) y el botón redondo **«Ver todas las capturas» (X)** al lado.
   - **Balda** con las portadas ordenadas por ese año y, en el borde, el año bajo cada portada (el del juego enfocado, resaltado).
   - «Ver todas las capturas» abre la vista de capturas combinada que ya existe para las sagas (con el filtro por juego en LB/RB).
   - A sobre una portada abre ese juego. Y edita la colección (nombre, juegos, color, borrar). Al final, una estantería/baldosa «Nueva colección».
   - **Sugerencias** al crear: grupos con prefijo común dentro de la categoría (`suggestName` ya existe), p. ej. Trails, Kingdom Hearts…
4. **Dos estilos de pestaña, elegibles en Ajustes** («Estilo de las colecciones»), ambos dentro de la misma interfaz:
   - **Aurora** (`diseno/titulos-color.html`, variante 3): la pestaña tiene el color de la colección y por dentro flotan manchas difuminadas con los colores de las portadas.
   - **Multicolor armonizada** (variante 4): la pestaña es un degradado horizontal con los colores de las portadas.
   - En los dos, los colores de las portadas se **armonizan**: mismo tono, pero con luz e intensidad fijas (OKLCH, ya hay `oklch.ts` y `palette.rs`), para que combinen aunque las portadas sean muy distintas. El texto debe leerse siempre (sombra suave si hace falta).
   - Color de la colección: **automático** (sacado de las portadas, como el acento de cada juego) o **manual** (los 7 colores de la app), desde «Editar colección › Color».
5. **Selector de juegos nuevo** (crear y editar), a pantalla completa:
   - Buscador arriba con el teclado en pantalla (`Osk`), que filtra en vivo.
   - Cuadrícula de portadas con check sobre las elegidas; solo juegos de esa categoría.
   - Filtros «Todos / Seleccionados (N)».
   - X: marcar todos los resultados de la búsqueda. A: marcar/desmarcar. Y: escribir. Start: guardar. B: atrás (pregunta si hay cambios sin guardar).
   - El ratón funciona con clic.

## Cierre

- Cada fase: `npm run check` sin errores nuevos; si se toca `visor-core`, `cargo test -p visor-core` en verde. Commit y push al terminar cada fase.
- Al final: versión **1.4.0** en `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` y `src-tauri/tauri.conf.json`; commit, etiqueta `v1.4.0` y `git push --tags` (el workflow compila y publica la release).
