# Arreglos de interfaz

Fallos visuales vistos el 14-nov-2026 en el navegador (datos de prueba, lienzo 1280×720). Rama de Unificador `ADI`. Las líneas son de ese día; comprobarlas antes de tocar.

## Inicio y Rachas

| Paso | Fallo | Causa | Arreglo |
|---|---|---|---|
| ADI-1 | Textos grises bajo «Seguir jugando» y «Juego de hoy» cortados con «…» y fuera de la tarjeta | `src/app.css:466` `.rmeta` en `top:574px`, `width:346px`, `nowrap` + ellipsis; la tarjeta acaba en 560. Texto en `Home.svelte:37-38,98` y `ReadCard.svelte:57` | Acercarlo a la tarjeta, permitir dos líneas o acortar el texto (quitar «en total»); fecha en `--ink` |
| ADI-2 | «Seguir jugando· Últimos 7 días» | `Home.svelte:91`: el espacio queda al principio del `<small>` y se pierde | `&nbsp;·` como en `ReadCard.svelte:57` |
| ADI-3 | Barra «LB Novelas PC Switch RB» dentro de «Últimos 6 meses», repite el atajo del pie | `Streak.svelte:217-223` | Quitar el bloque (LB/RB sigue cambiando de categoría) |
| ADI-4 | Raya suelta en «A qué hora juegas» sin datos | `Streak.svelte:269` da 2 % de alto a barras vacías; `border-bottom` de `.shb` (`:409`) | Alto 0 si no hay datos; sin barras ni borde cuando todo es 0 |
| ADI-5 | «Faltan 1 h 00» | `src/lib/readfmt.ts:28`, rama larga sin «min» (lo usan Streak y `quickmenu/TileTime.svelte:61`) | «1 h» o «1 h 30 min»; añadir test |

La fecha de «Hoy» en Rachas (viernes 13 a las 00:49 del sábado 14) **no es un fallo**: el día de las metas empieza a las 7:00 (`readgoals.ts:121`). Pendiente de que el usuario diga si lo quiere así.

## Mi lista y Colecciones

| Paso | Fallo | Causa | Arreglo |
|---|---|---|---|
| ADI-6 | Raya naranja bajo las pestañas LT/RT, hasta el buscador | `src/app.css:555-556`, `.ttabs::after` compartido con `.stabs` | Dejar el `::after` solo en `.stabs` |
| ADI-7 | Franja vacía «En marcha · 0» a todo el ancho | `MiListaDual.svelte:13-14` crea todas las bandas; `:71-74` y `.band.empty` (`:242`) | Filtrar bandas vacías y quitar su reserva de alto (`:20-27`) |
| ADI-8 | «Red Dead Redemption 2» desborda en Terminado | `MiListaDual.svelte:327` `.tx` 72 px en compacto, `.tx b` sin `overflow-wrap` | `overflow-wrap:anywhere` y algo más de ancho |
| ADI-9 | Las pestañas saltan de altura al pasar de Mi lista a Colecciones/Catálogo | `Titles.svelte:366` `.list-tabs{top:72px}` frente a 144 px (`app.css:485`) | Misma altura en las tres; ajustar `.board` (`MiListaDual.svelte:162`) |
| ADI-10 | La línea de tiempo tapa los títulos de las portadas y deja una franja blanca | `Shelf.svelte:227` `.bsb` 190 px sin margen; `.slip` (`:244-245`, `:271`) con fondo a todo el ancho | Más alto o margen antes de `.slip`; sin fondo en `.slip.railed` |
| ADI-11 | La segunda colección queda cortada | `.lclip` (`app.css:827`) mide 478 px; `CollTab.svelte:23` `COLL_DY=16` + paso 241 llega a 493 | `COLL_DY=0` o ajustar alturas |

## Catálogo y Estadísticas

| Paso | Fallo | Causa | Arreglo |
|---|---|---|---|
| ADI-12 | Indicador de página doble («Página 2 de 2» y «Página 2/2 · Azules-Grises») | `TopBar.svelte:88` y `Titles.svelte:297` (`pageFam`, `:167-173`), solo con orden por color | Uno solo: sin puntos arriba cuando hay `pageFam`, o quitar «Página» del segundo |
| ADI-13 | Tarjeta gris vacía en la última página | Relleno de `realAt` (`Titles.svelte:68`, `:83`, `:178-179`) | No pintar huecos de relleno o mostrar la baldosa «Añadir juego» bien |
| ADI-14 | Pestañas de Estadísticas se salen por la derecha («Resumen anu…») | Sin localizar | Que quepan o se desplacen hasta la activa |
| ADI-15 | Clic con ratón en una pestaña de Estadísticas no cambia | Sin localizar; puede ser intencionado (mando primero) | Investigar |
| ADI-16 | Repaso final de todas las pantallas a 1280×720 y `npm run check` | | |
