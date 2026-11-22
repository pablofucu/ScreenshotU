# Rediseño de Estadísticas

Rama RDE de Unificador. Maqueta elegida (22-nov): [maquetas/estadisticas-rediseno/index.html](../maquetas/estadisticas-rediseno/index.html), con los ajustes de abajo.

## Qué cambia

Hoy hay siete pestañas sueltas (Semana, Terminados, Panorama, Rachas, Clasificación, Calendario, Resumen anual), cada una con su estilo y sus botones, y la información se repite. Se pasa a **tres niveles**:

1. **Mi actividad** (lo que se abre desde Inicio). Tres pestañas:
   - **Semana:** la de hoy, compacta (cuatro chips arriba, columnas por categoría).
   - **Días y rachas:** dos paneles separados arriba, **racha de jugar** (llama) y **racha de meta** (diana, anillo y **lunas** de días libres, como la pantalla de racha; no escudos). Debajo, el **mapa del año** grande (cuadraditos de todo el año) y las sesiones del día enfocado.
   - **Terminados:** tira con todos los años y, debajo, el año en una rejilla de 12 meses con un carril por categoría y la carátula de cada juego terminado en su mes.
2. **Categoría** (A sobre una categoría): chips, horas del día y la clasificación de sus juegos (sustituye a Panorama y Clasificación).
3. **Juego** (A sobre un juego): carátula, chips y horas por mes (lo que hoy es Estadísticas de juego).

«Tu año jugando» (hoy Resumen anual) se abre con **Y** desde cualquier nivel. El Calendario de 6 meses lo sustituye el mapa del año. Las estadísticas del lector se quedan en el lector.

## Mando (guía, §7)

| Botón | Acción |
|---|---|
| LT / RT | Pestaña (nivel 1) |
| LB / RB | Año o periodo |
| View | Filtrar por categoría (mapa, Terminados) u ordenar (clasificación) |
| A | Abrir (día → sesiones, categoría, juego) |
| B | Volver al nivel anterior |
| X | Jugar (en el nivel Juego) |
| Y | Tu año jugando |

## Ajustes pedidos sobre la maqueta

- **Colores de categoría más neón y ordenados, como Mi lista.** Se usan los cinco colores de los estados de Mi lista, bien separados: Switch rojo, Juegos naranja, Novelas japonesas verde, PC azul, Novelas visuales violeta (`CAT_TONE` en `chiptone.ts`; las categorías en sí no cambian). Rellenos en `--arc-*` con brillo suave en oscuro.
- **Mapa con todas las categorías:** no se mezclan colores por día (quedaba feo). Con «Todas» el mapa usa una sola escala neón (acento vivo de Estadísticas) por horas jugadas, y al enfocar un día se ve el reparto por categoría. Con una categoría filtrada, la escala va en el color de esa categoría.
- **Racha de meta con lunas** (días libres), no escudos.
- **Textos:** cada chip lleva cifra, etiqueta y una línea de contexto; nada repetido entre chips del mismo panel.
- **Más animaciones:** entrada escalonada con `--e-bounce`, cifras que cuentan, el mapa se rellena en ola, carátulas de Terminados que entran por meses, cambio de pestaña deslizado.

## Pasos

| Paso | Qué |
|---|---|
| RDE-4 | Nivel Mi actividad: armazón de tres pestañas, botones nuevos, Semana |
| RDE-9 | Días y rachas: paneles de racha y mapa del año |
| RDE-10 | Terminados con todos los años |
| RDE-5 | Nivel Categoría |
| RDE-6 | Nivel Juego |
| RDE-7 | Botones y animaciones comunes, colores de categoría |
| RDE-8 | Probar con mando y biblioteca real |

## Ficheros

- `src/screens/Stats.svelte` (armazón), `StatsWeek.svelte`, `StatsFin.svelte`, `stats/` (pestañas nuevas), `GameStats.svelte` (niveles Categoría y Juego), `Year.svelte` (Tu año).
- `src/lib/store.svelte.ts` (`statsTab`), `controller.svelte.ts` (foco y botones), `stats.ts` (datos del mapa del año y de Terminados por años, con pruebas en `tests/`).
- `src/lib/chiptone.ts` (tonos de categoría).
