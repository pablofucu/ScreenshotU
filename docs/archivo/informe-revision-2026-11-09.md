# Revisión de interfaces y flujos (v1.10.0, 9 nov 2026)

Recorrido completo en modo mock (`?mock`) a 1280×720, usando el teclado como mando (Enter = A, Retroceso = B, m = Start, q/e = LB/RB, z/c = LT/RT, v = View).

Pantallas revisadas:
- **Inicio:** dock, «Seguir jugando», «Juego de hoy» y la racha.
- **Por categoría:** Mi lista, Colecciones, Catálogo (cuadrícula y carrusel), ficha del juego, Ajustes del juego, Capturas, visor y menú Start de una captura.
- **Globales:** Favoritas, Estadísticas (Clasificación, Calendario y Resumen anual), Añadir juego, Ajustes globales en los dos temas.
- **Ventanas aparte:** menú dentro del juego (`quickmenu.html`) y lector (`reader.html`).

Las categorías no se tocan en ninguna propuesta: todo lo que sigue vive dentro de cada categoría.

---

## 1. Fallos (corregir)

### Prioridad alta

| # | Dónde | Qué pasa | Arreglo |
|---|---|---|---|
| 1 | Tema «Noche ahumada»: [app.css:88-90](../src/app.css) | `.pill` tiene el fondo blanco fijo (`rgba(255,255,255,.80→.52)`), y el bloque oscuro ([app.css:28-35](../src/app.css)) no lo redefine. El texto pasa a `#b4bccb` sobre blanco y no se lee. Afecta al dock de Inicio, a las pistas de mando, a las migas de pan y a las tarjetas de racha. Hay otros 41 blancos fijos en `app.css` y aparecen en 9 pantallas más. | Crear los tokens `--pill-top` / `--pill-bot` (y uno para el blanco de los chips) en `:root` y en `#stage[data-theme=dark]`. Repasar las pantallas en oscuro una por una. |
| 2 | Catálogo: [Titles.svelte:355](../src/screens/Titles.svelte) | La pista dice **«X Jugar»**, pero X llama a `cycleTitleOrder()` (toast «Orden: A-Z»). Además, el chip «X Por color/A-Z» de arriba contradice la pista. | X = Jugar, igual que en la ficha y en Colecciones. El orden pasa al chip, o a View/Start. |
| 3 | Catálogo: [Titles.svelte:356](../src/screens/Titles.svelte) y [:202-215](../src/screens/Titles.svelte) | La pista dice **«Y Buscar / Vista»**, pero Y solo cambia entre cuadrícula y carrusel. El campo «Buscar juego…» no tiene `fx` ni `data-k`, así que con el mando no se puede llegar a él. Ya estaba en informe-flujo-navegacion §2.3.4 y sigue pendiente. | Hacer enfocable el campo de búsqueda y separar las funciones: Y = Buscar, View = Vista. |
| 4 | Inicio, dock «Buscar»: [controller.svelte.ts:631](../src/lib/controller.svelte.ts) | Es el primer botón del dock y solo saca el toast «Buscar: próximamente». | Implementarlo (ver diseño B) u ocultarlo hasta que exista. |
| 5 | Visor | La píldora de información («1/50 Elden Ring 07/09/2022 00:00 3840x2160») se monta sobre la de pistas («LB/RB ±10»), y la resolución sale cortada. | Ver diseño D (pistas abajo, información bajo demanda). |
| 6 | Estadísticas | Resumen anual 2026: **513 h, 11 juegos**. Clasificación «Siempre»: **449 h, 6 juegos**. Parece que el año es mayor que el total histórico. La causa es que la Clasificación filtra por la categoría (la etiqueta pequeña «Juegos PC») y el resumen anual no. | Mostrar el ámbito bien visible en las dos vistas («Juegos PC · Siempre» frente a «Todas · 2026»), o hacer que el resumen respete la categoría (LT/RT). |
| 7 | Mi lista: [MiListaDual.svelte:16](../src/screens/MiListaDual.svelte) | «hace **1 meses**». Además, `Math.round(d/30)` da «hace 12 meses» entre 345 y 364 días. | Usar el singular «mes» y pasar a «hace 1 año» desde 345 días. |
| 8 | Mi lista | El carril «A medias» se desborda: Outer Wilds sale cortado y «Se aparca d…» truncado. | Ver diseño E. |

### Prioridad media

| # | Dónde | Qué pasa | Arreglo |
|---|---|---|---|
| 9 | Racha: [Streak.svelte:293](../src/screens/Streak.svelte) | La pista dice «LB/RB **Cambiar meta**», pero lo que cambia es la categoría del mapa (chip Novelas → PC → Switch). | Cambiar la pista a «Categoría». |
| 10 | Inicio → tarjeta «Juego de hoy» | La tarjeta rota sola entre «Juego de hoy» y «Lectura de hoy». Al pulsarla abrió «Racha de lectura» (textos de [readgoals.ts:31](../src/lib/readgoals.ts): «A qué hora lees», «Qué has leído este mes») aunque mostraba «Juego de hoy». | Comprobarlo. La rotación automática ya es suficiente motivo para quitarla (ver diseño A). |
| 11 | Avatar arriba a la izquierda: [TopBar.svelte:56](../src/components/TopBar.svelte) | Por su cristal y su tamaño parece un botón, pero es `aria-hidden` y no hace nada. | Darle una función (perfil, racha, perfil de tablet) o reducirlo a un adorno discreto. |
| 12 | Barra superior ⏻ y dock «Salir» | Llevan iconos parecidos y hacen cosas muy distintas: el ⏻ de arriba **suspende el PC** (pide dos pulsaciones) y «Salir» cierra la app. | Usar una luna o «Suspender» para el de arriba y reservar ⏻ para «Salir». |
| 13 | Añadir juego (.exe) | «Guardar» está activo con el formulario vacío y solo avisa con el toast «Elige primero el ejecutable». Hero, Logo y Portada son tres filas de «Ninguno». | Desactivar «Guardar» hasta tener el ejecutable y marcar los campos obligatorios. Ofrecer SteamGridDB (ya existe `SgdbPicker`) en vez de tres selectores vacíos. |

### Prioridad baja (aspecto)

| Pantalla | Problemas y propuesta |
|---|---|
| Ficha | <ul><li>El odómetro («0 0 9 h 1 2 min») muestra ceros a la izquierda y se reanima desde 0 cada vez que vuelves. Propuesta: quitar los ceros y animar solo en la primera visita.</li><li>El chip «Fecha de lanzamiento···» parece un marcador de posición en pleno hero.</li><li>El nombre del logro sale truncado y «Y · cambiar» es diminuto.</li></ul> |
| Ajustes del juego | «Capturas y datos» va pegado al contador («datos3»). |
| Ajustes globales | <ul><li>Solo «Sonido de navegación» muestra las flechas ‹ ›; las demás filas no indican que se pueden cambiar.</li><li>El cuadro de descripción queda medio vacío.</li></ul> |
| Toasts | Aparecen encima del contenido: «Vista: carrusel» tapa la línea de datos. Propuesta: un sitio fijo, arriba al centro, bajo la barra superior. |
| Capturas «Por días» | Si hay una captura por día, la agrupación solo añade ruido. Propuesta: agrupar por semana o mes cuando los grupos sean de 1. |
| Favoritas vacía | Solo sale un toast. Propuesta: una pantalla vacía que explique cómo marcar favoritas. |
| Menú del juego (quickmenu) | <ul><li>«Esta sesión **20:08**», junto al reloj «20:32», se lee como una hora. Mejor «20 min».</li><li>«13 novelas RÉCORD» y «13 días seguidos sin rescates pendientes» no se entienden a la primera.</li><li>En mock tardó unos 2-3 s en pintarse. Comprobarlo en real: el menú pausa el juego.</li></ul> |
| Lector | <ul><li>«Sin conexión…» en rojo junto a «Esperando texto…», sin decir qué hacer (por ejemplo «Abre Texthook»).</li><li>No sigue el tema oscuro.</li></ul> |
| Accesibilidad | El dock ([Home.svelte:121](../src/screens/Home.svelte)) y muchas filas son `div`. Con ratón funcionan por `data-act`, pero no hay rol ni nombre accesible. |

---

## 2. Coherencia de flujos

1. **Start tiene tres nombres.**
   - Situación actual:
     - En Inicio se llama «Ajustes».
     - En Catálogo y Colecciones, «Menú».
     - En la ficha, «Opciones» ([Detail.svelte:265](../src/screens/Detail.svelte)), y abre un panel titulado «Ajustes», que también es el nombre de los ajustes globales.
   - Propuesta:
     - Start = **«Menú»** en todas las pantallas.
     - El panel del juego se titula **«Ajustes del juego»**.
2. **La ficha muestra «A Jugar» y «X Jugar» a la vez**, y A pasa a «Seleccionar» cuando el foco sale de Jugar. Propuesta: X = Jugar siempre y A = la acción del elemento enfocado.
3. **Colecciones: «X Jugar / Capturas»** asigna dos funciones a un mismo botón. Elegir una de las dos.
4. **Pistas de mando arriba en el visor y abajo en el resto.** informe-interfaz §1.5 ya fijaba que las pistas van «solo en la barra inferior».
5. **La barra de pestañas cambia de altura.**
   - En Mi lista está a y≈55 (sin cabecera).
   - En Colecciones y Catálogo, a y≈99.
   - En carrusel se centra.
   - Propuesta: misma posición en las tres pestañas.
6. **Doble indicador de página en el Catálogo:** «Página 2 de 2» arriba y «Página 2/2 · Azules-Grises» a la derecha. Dejar solo uno.
7. **El menú Start mezcla acciones de contexto con acciones globales.**
   - En una captura salen «Salida de audio» ([ActionMenu.svelte:70](../src/screens/ActionMenu.svelte)) y «Añadir capturas», que es una acción del juego.
   - Falta «Favorita».
   - Propuesta: primero las acciones del elemento, luego un separador y después las globales.
8. **Duplicados en Inicio.**
   - «Ajustes» en el dock repite la pista «Start Ajustes».
   - La etiqueta «Últimos 7 días: 15 h · 0 capturas» flota sin quedar claro a qué tarjeta pertenece.
   - Las tarjetas recientes mezclan la fecha de la última partida con el tiempo total («13 oct · 120 h 14 min»).
9. **Estadísticas.**
   - Se abre en el mes actual aunque esté vacío («Sin partidas · Noviembre 2026»).
   - Las vistas quedan escondidas tras «•••».
   - Muestra «A Ver juego» cuando no hay nada que abrir.

---

## 3. Diseños alternativos

### A. Inicio más limpio
- **Dock de 4 elementos:** Buscar (de verdad), Favoritas, Estadísticas y Añadir juego.
  - «Ajustes» se queda en Start.
  - «Salir» pasa a un elemento del menú Start o a mantener ⏻ pulsado.
- **«Juego de hoy» sin rotación automática.**
  - Una tarjeta fija con el anillo de la categoría que toca.
  - LB/RB dentro de la tarjeta para pasar de una categoría a otra.
  - Al abrirla, la racha de esa misma categoría.
- **El avatar muestra la racha** (llama + días) y abre la pantalla de Racha.
- **«Últimos 7 días»** baja a la línea de datos de «Seguir jugando», o se integra en la tarjeta de racha.

### B. Buscador global (el «Buscar» del dock)
- Una capa a pantalla completa con un solo campo, que se escribe con el teclado o con el teclado en pantalla.
- Los resultados salen **agrupados por categoría**, en una fila por categoría, sin mezclar títulos.
- A abre la ficha y X juega.
- Se puede reutilizar `catalogSearch` y el filtrado del Catálogo.

### C. Contrato de botones (una sola gramática)

| Botón | Siempre significa |
|---|---|
| A | Abrir o confirmar lo enfocado |
| B | Volver |
| X | Jugar (si el título se puede lanzar) |
| Y | Acción secundaria del contexto (Buscar en el Catálogo, Editar en Colecciones, cambiar logro en la ficha) |
| View | Vista, orden o filtro, y modo desnudo en el visor |
| Start | Menú contextual (siempre «Menú») |
| LB/RB · LT/RT | Página o fecha · pestaña o categoría |

Con esto se resuelven los fallos 2 y 3 y los puntos 1-3 de §2.

### D. Visor a pantalla completa
- Por defecto, el modo desnudo (el de `v`): ahora el marco de cristal con márgenes desperdicia un tercio de la pantalla.
- Al pulsar A o mover el stick, aparece la información **abajo** durante 2 s (juego, fecha, n/N), con las pistas en la barra inferior como en el resto de la app.
- La resolución pasa a la hoja de información (Y).

### E. Mi lista con carriles adaptativos
- La altura de cada carril depende de cuántos títulos tiene.
- Los carriles vacíos se pliegan a una línea («Pendiente · 0 · A para añadir»).
- «A medias» deja de desbordarse y «Pendiente» y «Aparcado» ya no quedan vacíos.
- Alternativa: estantes horizontales, uno por estado, con desplazamiento lateral (ya hay soporte de rueda horizontal).

### F. Estadísticas con pestañas visibles
- Un segmentado **Clasificación · Calendario · Resumen anual** en lugar de «•••».
- Se abre en el último periodo con datos.
- El ámbito siempre visible: «Juegos PC · Siempre».
- Sin «A Ver juego» cuando no hay ninguna fila.

### G. Pasada de tokens para el tema oscuro
- Inventario de los blancos fijos: 41 en `app.css` y 9 pantallas más.
- Convertirlos en tokens.
- Revisar cada pantalla en «Noche ahumada»: Inicio, Racha (las tarjetas pastel quedan claras sobre fondo oscuro), migas de pan, pistas y lector.

---

## 4. Orden propuesto

1. **Rápidos** (menos de 1 h cada uno): fallos 1 (`.pill` oscuro), 2-3 (pistas X/Y del Catálogo), 4 (ocultar «Buscar»), 7 (plural), 9 (pista de Racha) y 13 (Guardar desactivado).
2. **Coherencia:** contrato de botones (C), Start = «Menú» siempre, pistas abajo en el visor (D), ámbito visible en Estadísticas (fallo 6, diseño F), carriles de Mi lista (E).
3. **Diseño:** Inicio de 4 elementos y tarjeta fija (A), buscador global (B), repaso completo del tema oscuro (G).

## 5. Estado de los informes anteriores

| Informe | Punto | Estado |
|---|---|---|
| informe-flujo-navegacion §2.1 | «Seguir jugando» volvía al Catálogo | **Resuelto**: `openRecent` llama a `openTitle(r.ti, 'home')` |
| informe-flujo-navegacion §2.3.1 | Pistas que contradicen al controlador | **Persiste** en las pistas X e Y del Catálogo |
| informe-flujo-navegacion §2.3.4 | Buscador del Catálogo inaccesible con mando | **Persiste** |
| informe-menu-juego | Reorganizar Ajustes del juego | **Resuelto**: 4 secciones |
| informe-interfaz §1.5 | Pistas solo abajo | **Incumplido** en el visor |
| design-handoff | Modo oscuro «Noche ahumada» | **Existe, pero incompleto** (fallo 1, diseño G) |

## 6. Lo que funciona bien (mantener)

- Las migas de pan clicables, la atmósfera de color de cada juego y la barra arcoíris del Catálogo, que sirve de minimapa.
- La línea temporal por años en Colecciones y la Clasificación con barras hechas de portadas.
- El Calendario de Estadísticas (mapa de calor, barras mensuales y el día elegido) y el Resumen anual.
- El menú dentro del juego («Plaza»): está claro, los mosaicos tienen tamaños bien jerarquizados y «Cerrar juego» va aparte.
- Los Ajustes globales en dos columnas, con la descripción de cada fila.
- El asistente «Añadir juego», con sus cuatro orígenes bien explicados.
