# Semana y navegación de Estadísticas (hecho, v2.9.0)

Rama SYN de Unificador. Maqueta elegida: «Riel lateral» (P-23, opción 1). No se guarda en el repositorio. Las variantes con velas y lunas y con más neón se descartaron.

## Navegación de Mi actividad (Semana, Días y rachas, Terminados)

- **LT/RT**: cambia de pestaña.
- **LB/RB**: cambia de categoría. La categoría es una sola (`app.statsCat`, -1 = todas) y la comparten las tres pestañas. El orden es el del riel: `statsCats()`, con Todas y luego `catRank`.
- **Cruceta**: arriba llega a la tira del periodo (`data-k="per"`). Izquierda y derecha cambian de semana o de año.
- **Ratón**: las flechas usan `pshift:±1` y los elementos del riel `cat:<pi>`.
- View ya no filtra en estas pestañas. Las pistas son las mismas en las tres.

## Semana

- **Riel a la izquierda**: Todas y cada categoría con su color fijo y las horas de la semana.
- **Resumen (wk0)**: total, diferencia con la semana anterior, días activos y **una sola** llama de racha. Es la racha de la categoría; con Todas, la más larga. Debajo, las etiquetas (terminados, hitos, logros, capturas) o la lectura japonesa si la categoría es Novelas japonesas.
- **Día a día (wk1)**: barras sencillas apiladas con el color de cada categoría. Con una categoría que tiene meta, una línea discontinua marca la meta.
- **Lo más jugado (wk2)**: hasta tres títulos con portada.
- **Metas de la semana (wk3)**: siete cuadritos por categoría con meta. El relleno depende del tiempo del día: vacío con 0 y color pleno con 2 h o más. Los días con la meta cumplida llevan una marca. Sin metas, la tarjeta se llama «Días jugados» y no lleva marcas.
- Nunca se mezclan categorías: «Todas» solo apila cada una con su color.
