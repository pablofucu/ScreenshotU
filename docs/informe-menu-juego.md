# Informe: el menú de opciones de cada juego («Grafismos y opciones»)

Queja del usuario: «es un desastre, poco intuitivo, muchas opciones, categorías difusas».

## Dónde vive

- Componente: `src/screens/ActionMenu.svelte` (145 líneas; sirve también para el menú de capturas y el de la lista de juegos, según `app.menu.kind`).
- Qué acciones salen: `menuItemsFor(t)` en `src/lib/controller.svelte.ts:346`.
- Agrupación en secciones: `MENU_GROUPS` (`controller.svelte.ts:370`) y `menuLayout()` (`:378`, reparte en 2 columnas si hay más de 8).
- Qué hace cada fila: `menuAct()` (`controller.svelte.ts:448`).
- Se abre con `openDetailMenu()` (`:407`) desde la ficha del juego (`src/screens/Detail.svelte`).
- Escenario fijo de 1280×720, manejo con mando (A/B) además de ratón; en tablet (`IS_TABLET`) se ocultan varias acciones.

## Qué contiene hoy

Título del panel: **«Grafismos y opciones»**. Hasta 17 acciones en 3 secciones (según el juego):

| Sección | Acciones |
|---|---|
| Juego y Lanzamiento | Iniciar juego · Magpie · Cambiar ejecutable… · Cambiar carpeta de trabajo… · Abrir carpeta · Añadir capturas · Partidas guardadas… |
| Presentación | Grafismos (SteamGridDB o del PC)… · Color de acento · Recargar grafismos |
| Biblioteca y Estado | Estado: X · Fecha de lanzamiento… · Añadir a una colección… · Frases y repaso… · Cambiar de categoría… · Ocultar/Mostrar juego · Quitar de la lista |

Condiciones: Magpie/ejecutable/carpeta de trabajo solo en juegos `.exe` añadidos a mano; Frases solo en novelas (jvn/vn); Estado/Fecha/Colección solo si el título tiene id y no es agrupación; Cambiar de categoría solo si hay más de una; «Iniciar» desaparece si ya hay un juego en marcha.

## Problemas detectados

1. **Demasiadas acciones de golpe, de naturaleza muy distinta.** Hasta 17 filas en dos columnas mezclan lo que se hace *casi siempre* (jugar), lo que se toca *una vez* (fecha de lanzamiento, estado, colección) y lo que es *mantenimiento/avanzado* (ejecutable, carpeta de trabajo, Magpie, recargar grafismos, quitar). Todo con el mismo peso visual.
2. **El título no describe el contenido.** «Grafismos y opciones» promete dos cosas, pero «grafismos» es solo una fila (y «Presentación» otra sección). Lo demás —estado, colecciones, guardados, lanzamiento— no es ni grafismo ni «opciones» en un sentido útil.
3. **Secciones-cajón de sastre.**
   - «Juego y Lanzamiento» junta *jugar* con *configurar cómo se lanza* (Magpie, ejecutable, carpeta de trabajo), *carpetas del disco* (Abrir carpeta, Añadir capturas) y *datos del juego* (Partidas guardadas).
   - «Biblioteca y Estado» junta *metadatos* (estado, fecha), *organización* (colección, categoría), *estudio* (Frases y repaso) y *acciones destructivas o de visibilidad* (Ocultar, Quitar). Es el 40 % del menú y no tiene una idea común.
   - «Presentación» incluye «Recargar grafismos», que es mantenimiento, no elección de aspecto.
4. **Las filas se comportan de formas distintas sin avisarlo.**
   - Acción inmediata: Iniciar, Abrir carpeta, Recargar, Ocultar.
   - Abre submenú o panel: Grafismos…, Color de acento (sin puntos suspensivos), Estado: X (sin puntos), Fecha…, Colección…, Cambiar de categoría…
   - Cambia en el sitio al pulsar A: **Magpie** (cicla modos).
   - Pide confirmación con una segunda pulsación: Quitar.
   - Abre un diálogo del sistema: Añadir capturas, Cambiar ejecutable… (con y sin «…» de forma inconsistente).
   El usuario no puede predecir qué pasará al pulsar.
5. **Valor actual mostrado de forma inconsistente.** Estado, Fecha y Magpie enseñan su valor en la etiqueta («Estado: Jugando»); Color de acento, Grafismos, Colección o Categoría no. Cuando la etiqueta *es* el valor, la fila no dice que se puede cambiar.
6. **Etiquetas vagas o con jerga.** «Grafismos (SteamGridDB o del PC)…» (portada, fondo, logo, icono… ¿qué es un «grafismo»?), «Recargar grafismos», «Color de acento», «Frases y repaso…», «Cambiar de categoría…», «Quitar de la lista» frente a «Ocultar juego» (la diferencia no es evidente: ambos hacen que el juego deje de verse).
7. **El menú cambia de forma y de longitud de un juego a otro** (de ~5 a 17 filas, a una o dos columnas, con secciones que aparecen y desaparecen). No hay posiciones estables que la memoria muscular o el mando puedan aprender.
8. **Lo avanzado y lo peligroso comparten espacio con lo cotidiano.** Magpie, ejecutable y carpeta de trabajo ocupan la 2ª–4ª fila de la primera columna; «Quitar de la lista» está pegado a «Ocultar». Ni se esconden ni se separan visualmente (solo `danger` en rojo).
9. **Densidad.** Con muchas filas el paso baja (hasta ~49 px en la columna larga) y existen clases `tight/tighter/squeeze`; el panel llena casi toda la altura del escenario de 720 px. En una pantalla pensada para mando a distancia es mucho que escanear.
10. **Posible duplicidad con la ficha.** Estado, fecha de lanzamiento y colecciones son datos del juego que probablemente ya se ven (o podrían verse y editarse) en la propia ficha (`Detail.svelte`); el menú los trata como acciones. El planificador debe comprobar qué enseña ya la ficha.

## Restricciones para cualquier propuesta

- **Las categorías de la biblioteca no se tocan** (ver `CLAUDE.md`): nada de renombrar, fusionar ni reorganizar plataformas. «Cambiar de categoría…» puede cambiar de sitio dentro del menú, pero la función y las categorías siguen igual.
- Todo lo nuevo vive dentro de cada categoría; el menú es por juego.
- Debe seguir funcionando con **mando** (A/B, cruceta) y ratón en el escenario 1280×720, y en **tablet** (`IS_TABLET` oculta Abrir carpeta, Añadir capturas, Ejecutable y Carpeta de trabajo).
- El mismo `ActionMenu.svelte` sirve a los menús de capturas y de lista de juegos: cualquier rediseño debe dejar claro si los toca o no.
- Estilo visual ya decidido: tema v5 (atmósfera + degradado con los colores del juego, `--acc`), ver `docs/informe-interfaz.md`, `docs/plan-implementacion.md` y `mockups/sistema-v5.html`.
