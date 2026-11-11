# Guía de diseño · Visor de capturas

Fecha: 2026-11-10. Acordada con Pablo en seis rondas de preguntas.

- **Muestra viva:** [guia-diseno/index.html](guia-diseno/index.html). Carga el `src/app.css` real y se ve en claro y oscuro. Se abre con el servidor de desarrollo en `/docs/guia-diseno/index.html`.
- **Tokens y componentes propuestos:** [guia-diseno/propuesta.css](guia-diseno/propuesta.css). Aún no están en `src/`; entran en la fase 0 de la migración (§15).
- **Antecedentes:** `docs/archivo/informe-interfaz.md` (diagnóstico) y `docs/archivo/informe-revision-2026-11-09.md` (contrato de botones).

Cómo leerla: cada apartado da la regla y, si hace falta, la razón. Lo que la guía no menciona se resuelve copiando cómo lo hace **Inicio**, que es la pantalla de referencia.

---

## 1. Principios

1. **Cristal Wii U.** Las superficies son cristal con borde, arco de brillo y cara. No se toca su esencia.
2. **Arcoíris ordenado.** El color viene del arcoíris de la app, siempre en su orden. No hay colores sueltos.
3. **Poca carga.** Un elemento protagonista y, como mucho, una acción principal por pantalla. Los controles aparecen con el foco, no están siempre a la vista.
4. **Pensada para el sofá.** Se usa con mando desde la tele, con mando o ratón en el monitor y con el dedo en la tablet. Todo se lee a tres metros y todo se alcanza con el mando.
5. **Dos temas de primera.** Claro y «Noche ahumada» valen lo mismo. Nada se da por hecho hasta verse bien en los dos.
6. **Las categorías no se tocan.** Lo nuevo vive dentro de cada categoría (ver `CLAUDE.md`).

## 2. Alcance

| Superficie | Qué comparte | Qué tiene propio |
|---|---|---|
| App principal | Todo | Nada |
| Menú del juego (Plaza) | Todos los tokens y componentes | Maquetación de overlay (§14) |
| Lector y hook | Marco: tokens, botones, pistas | Paleta de la zona de lectura (§14) |
| App de tablet | Todos los tokens | Maquetación táctil (§14) |

## 3. Color

### 3.1 Tinta y superficies

Solo tokens. Un hex escrito en una pantalla es un error, salvo en `src/lib/icons.ts` (`RAINBOW`) y en la paleta de lectura (§14).

| Token | Claro | Oscuro | Uso |
|---|---|---|---|
| `--ink` | `#3a414e` | `#e6e9f0` | Texto |
| `--ink-2` | `#5f6776` | `#b4bccb` | Texto secundario |
| `--ink-3` | `#757d8c` (antes `#8a92a1`) | `#8f98a9` (antes `#7f8899`) | Solo adorno: iconos apagados, separadores, texto desactivado |
| `--line`, `--line-2` | | | Bordes finos y divisores |
| `--face`, `--face-hi` | | | Cara interior del cristal |
| `--panel` | | | Fondo de panel con texto |
| `--glass-in`, `--glass-chip` | | | Panel interior y pastilla |
| `--scrim` | | | Velo tras un modal |

Variantes de un color: con `color-mix(in srgb, var(--f1) X%, var(--tint-base))`, nunca con un hex nuevo. `--tint-base` es blanco en claro y gris oscuro en oscuro, así la mezcla funciona en los dos temas.

### 3.2 Acento

El acento no se elige en cada pantalla: lo inyecta `App.svelte` desde la atmósfera (`src/lib/atmos.ts`).

| Variable | Qué es | Para qué |
|---|---|---|
| `--g1 --g2 --grad` | Pareja suave | Fondos, halos, tintes |
| `--f1 --f2 --gf` | Pareja de relleno, oscurecida hasta dar contraste AA frente a `--on` | Todo lo que lleva texto encima o es texto: botón principal, selector activo, pastillas, cifras en color |
| `--v1 --v2 --gv` | Pareja viva: el color puro del arcoíris para ese tono (`vivo()` en `atmos.ts`) | Gráficos y adornos sin texto encima: anillos, barras, líneas, rachas, mapas de calor, iconos grandes, brillos |
| `--on` | Texto sobre relleno | Siempre encima de `--gf` |
| `--acc` | Igual a `--f1` | Código antiguo; en lo nuevo se usa `--f1` |

**Dos acentos.** El relleno `--f1` tiene que oscurecerse para que el blanco se lea encima, y en amarillo, naranja o verde acaba en mostaza o marrón. Por eso hay un segundo acento, `--v1`, que es el color tal cual (el amarillo es amarillo). La regla es sencilla:

- ¿Hay texto encima, o es texto? → `--f1` / `--gf`.
- ¿Es un trazo, una barra, un relleno de gráfico o un icono grande sin texto? → `--v1` / `--gv`.
- Nunca texto en `--v1` sobre fondo claro, ni texto blanco sobre `--v1`.
- Los trazos vivos pueden llevar brillo: `drop-shadow(0 2px 6px color-mix(in srgb,var(--v1) 55%,transparent))`.
| `--rby`, `--rbsec` | Arcoíris completo y tramo de la sección | Barras y subrayados decorativos |

De dónde sale el acento:

| Dónde estás | Acento |
|---|---|
| Inicio y menús generales | Arcoíris completo, en orden |
| Dentro de una categoría | El tramo de arcoíris de esa categoría, por su posición en Inicio |
| Dentro de un juego | La pareja del juego: color manual, si no el de la portada, si no arcoíris por nombre |

Esto ya funciona así y se queda como está.

### 3.3 Colores con significado reservado

Estos tres no se usan como decoración ni como color de serie.

| Token | Significa | Dónde |
|---|---|---|
| `--ok` verde | Hecho, correcto, confirmar | Marca de completado, aviso de éxito, botón A |
| `--bad` rojo | Peligro, borrar, error | Borrar, aviso de error, botón B |
| `--warn` ámbar | Aviso, atención, oro | Avisos, favorito, platino, botón Y |

Con texto blanco encima se usan los rellenos `--ok-f` (`#27833a`) y `--bad-f` (`#d92c44`), que cumplen 4,5:1. Sobre `--warn` el texto va en `--ink`.

### 3.4 Posiciones fijas en el arcoíris

Son colores de identidad: siempre llevan su nombre al lado, nunca informan solos.

| Mi lista (`BAND_DEF`) | Color | Rareza de logros | Color |
|---|---|---|---|
| En marcha | Rojo | Común | Azul |
| A medias | Naranja | Poco común | Verde |
| Pendiente | Verde | Rara | Naranja |
| Aparcado | Azul | Ultra rara | Rojo |
| Terminado | Violeta | Legendaria | Violeta |
| | | Platino | Ámbar (`--warn`) |

`LIST_COLOR` (`store.svelte.ts`, color de cada estado de un juego) usa los colores de `BAND_DEF` (`milista.svelte.ts`) desde la fase 3: Jugando = En marcha, Pendiente = Pendiente, Terminado = Terminado, Abandonado = Aparcado. El menú del juego tiene además sus propias rarezas (`#E9A100`, `#6E56CF`…), que pasan a esta tabla.

### 3.5 Contraste

Texto legible: 4,5:1 en los dos temas. Medido sobre el fondo de la app:

| Pareja | Claro | Oscuro |
|---|---|---|
| `--ink` | 8,8 | 14,6 |
| `--ink-2` | 4,9 | 9,3 |
| `--ink-3` actual | 2,7 (no cumple) | 5,0 |
| `--ink-3` propuesto | 3,6 | 5,0 sobre `--face` |
| Blanco sobre `--ok` | 2,7 (no cumple) | igual |
| Blanco sobre `--bad` | 4,1 (no cumple) | igual |

Consecuencias: `--ink-3` no se usa para texto que haya que leer; el texto secundario es `--ink-2`. Las letras blancas sobre verde o rojo usan los rellenos `--ok-f` y `--bad-f`.

## 4. Tipografía

Una fuente (`--round`), siete tamaños, tres pesos.

| Token | px | Peso | Uso |
|---|---|---|---|
| `--t-3xl` | 44 | 800 | Cifra protagonista |
| `--t-2xl` | 28 | 800 | Título de pantalla, uno por pantalla |
| `--t-xl` | 20 | 800 | Título de panel, migas |
| `--t-l` | 17 | 700 | Texto destacado, botones, pistas, filas |
| `--t-m` | 15 | 500 | Texto normal |
| `--t-s` | 13 | 500 | Secundario: fechas, recuentos, ayudas. **Mínimo legible** |
| `--t-xs` | 11 | 800 | Solo adorno: etiquetas en mayúsculas, números en distintivos |

- Pesos: 500 texto, 700 énfasis, 800 títulos. No se usan 400, 600 ni 900.
- Interlineado: 1,2 en títulos y 1,35 en texto.
- Si quitar un texto de 11 deja la pantalla sin entenderse, ese texto no era adorno: sube a 13.
- El escenario mide 1280 × 720 y se escala; 13 px de lienzo son unos 19 px reales en una tele 1080p.

Tamaños a mano que hay hoy y a qué paso van:

| Hoy | Pasa a |
|---|---|
| 9 a 11,5 | 11 si es adorno, 13 si se lee |
| 12 a 13,5 | 13 |
| 14 a 15,5 | 15 |
| 16 a 18 | 17 |
| 19 a 22 | 20 |
| 24 a 36 | 28 |
| 40 o más | 44 |

El 14 es el tamaño a mano más usado (unas 80 veces) y no está en la escala: pasa a 15.

## 5. Radios y espaciado

| Radio | px | Uso |
|---|---|---|
| `--r-xs` | 6 | Etiquetas, barras, distintivos |
| `--r-s` | 12 | Campos, miniaturas |
| `--r-m` | 18 | Tarjetas, panel interior |
| `--r-l` | 28 | Panel de cristal |
| `--r-pill` | 999 | Pastillas, selectores, botones |

Un elemento dentro de otro usa el radio de un paso menos.

| Espaciado | px | Uso típico |
|---|---|---|
| `--s-1` | 4 | Entre icono y cifra pegados |
| `--s-2` | 8 | Entre icono y texto, entre selectores |
| `--s-3` | 12 | Entre filas, relleno de tarjeta pequeña |
| `--s-4` | 16 | Entre tarjetas |
| `--s-5` | 24 | Relleno de panel, margen de pantalla |
| `--s-6` | 32 | Entre bloques de una pantalla |

Todo hueco es uno de estos seis. Los 6, 10, 14 y 18 de hoy van al paso más cercano.

## 6. Cristal y alturas

Tres cristales. Una superficie nueva es uno de ellos; no se escribe `backdrop-filter` ni un degradado de fondo en una pantalla.

| Cristal | Clase | Desenfoque | Uso |
|---|---|---|---|
| Panel | `.glass` | 14 px | Teselas, portadas, paneles, modales |
| Panel interior | `.glass-in` | Ninguno (ya está sobre un panel) | Grupos dentro de un panel, filas, tarjetas de logro |
| Pastilla | `.pill` | 16 px | Migas, pistas, botón secundario, avisos |

Tres alturas, que son las únicas sombras exteriores:

| Altura | Token | Uso |
|---|---|---|
| Apoyado | `--sh-1` | Estado normal de cualquier cristal |
| Enfocado | `--sh-2` | Elemento con el foco: brillo del acento |
| Flotante | `--sh-3` | Modales, menús, avisos sobre el velo |

Las sombras interiores del cristal (borde, brillo) son parte de `.glass` y no se copian.

## 7. Botones y mando

Tres familias. Todo botón propio de una pantalla se convierte en una de ellas.

| Familia | Clase | Regla |
|---|---|---|
| Principal | `.b-main` | Relleno de acento. **Una por pantalla** como máximo |
| Secundario | `.b-soft.pill` | Cristal, sin color propio |
| Selector | `.chip` | Pestañas y filtros; el activo lleva el acento |

`.btn` sigue siendo el glifo del mando (A, B, X, Y, LB…), no un botón.

Las pistas del mando van **solo en la banda inferior**. No se repiten dentro de la pantalla. La excepción es un estado vacío, que puede mostrar la pista de su única acción.

Contrato del mando, igual en todas las pantallas:

| Botón | Hace |
|---|---|
| A | Abrir, confirmar |
| B | Volver |
| X | Jugar |
| Y | Acción secundaria de la pantalla |
| View | Vista, orden, filtro |
| Start | Menú |
| LB / RB | Página, fecha |
| LT / RT | Pestaña, categoría |

### Foco

Un solo lenguaje: el elemento crece a 1,06 y recibe el acento (`.glass.f`) o el anillo `--ring` si no es un cristal. No hay otros estilos de foco. Con ratón, pasar por encima equivale al foco.

Estados que ya son norma: pulsado baja a 0,97 en 120 ms; desactivado va a opacidad 0,45 y pierde la sombra.

## 8. Movimiento

Vivo y elástico, como la Wii U. Tres duraciones y dos curvas; ninguna transición usa otros valores.

| Token | Valor | Uso |
|---|---|---|
| `--d-1` | 120 ms | Pulsar, cambios de color |
| `--d-2` | 200 ms | Foco, selectores |
| `--d-3` | 300 ms | Cambio de pantalla, paneles, modales |
| `--e-soft` | `cubic-bezier(.2,.8,.3,1)` | Entradas, salidas, desplazamientos |
| `--e-bounce` | `cubic-bezier(.34,1.32,.64,1)` | Foco y aparición de elementos: el rebote |

Los `.24s` y `.28s` de hoy pasan a 200 y 300. Las animaciones de ambiente (manchas del fondo, brillo de la carga) quedan fuera de esta tabla. No hay modo de movimiento reducido.

## 9. Iconos

- Juego propio en `src/lib/icons.ts`, sin emojis y sin fuentes de iconos.
- Rejilla de 24, trazo de 2 px, extremos y uniones redondos, sin relleno.
- Tres tamaños: `--i-s` 16, `--i-m` 20, `--i-l` 28.
- El color es `currentColor`: el icono hereda el del texto.
- Los glifos grandes de categoría en Inicio (rejilla 120, blancos sobre acento) son ilustración y se quedan como están.

Los iconos actuales con `stroke="#fff"` y rejillas de 40 o 48 se redibujan al formato nuevo cuando se migra la pantalla que los usa.

## 10. Plantilla de pantalla

Escenario de 1280 × 720. Todas las pantallas usan el mismo esqueleto; solo el visor y el lector a pantalla completa se salen.

| Zona | Medidas | Contenido |
|---|---|---|
| Barra superior | y 0 a 76 | Avatar de 60 (x 22), migas de 40 de alto en 20/800 (x 98), reloj a la derecha (24) |
| Contenido | x 24 a 1256, y 88 a 644 | La pantalla |
| Banda inferior | y 656 a 720 (64 px) | Solo pistas del mando, a la derecha |

- El contenido no entra en la banda inferior. Una lista larga se desvanece antes de llegar.
- Las migas dicen dónde estás; no hace falta repetir el título dentro.
- Entrar y salir de una pantalla: `enterFwd` y `enterBack`, 300 ms con `--e-soft`.
- Un modal es un `.glass` flotante centrado sobre `--scrim`, con sus pistas en la misma banda inferior.

## 11. Gráficos

El color de un gráfico dice **de quién es el dato**, nunca si es bueno o malo.

| Caso | Color |
|---|---|
| Una sola serie | Acento vivo de la sección (`--v1`, `--gv`) |
| Varios juegos | El color de cada juego |
| Varias categorías | El tramo fijo de cada categoría |
| Destacar un dato | Ese dato en acento vivo y el resto en gris (`--line-2`) |
| Series sin dueño, decoración, menús | Arcoíris en su orden |

- Verde, rojo y ámbar no son colores de serie.
- Ejes y rejilla en `--line`; etiquetas en `--ink-2` a 13 (11 solo en marcas del eje).
- Barras con las esquinas en `--r-xs`. Las cifras importantes se escriben, no se dejan a la lectura del eje.
- Un gráfico tiene que ser bonito además de correcto: mismo cristal y mismos huecos que el resto.
- Cifras y etiquetas en color dentro de un gráfico van en `--f1` (o en `--ink`), nunca en `--v1`.

### 11.1 Chips de estadísticas

Catálogo aprobado, todos animados: `docs/maquetas/estadisticas/chips/index.html`. Se usan los 18 en Panorama.

| Grupo | Chips |
|---|---|
| Cifras de la categoría | A odómetro, B anillo de meta, C medidor, D chispa con tendencia, L récord, H hito con bandera |
| Resumen semanal | E barras con día destacado, R siete anillos, G comparación, F racha encadenada, P semana por fuente, Q destacados que rotan, J reloj de 24 h, K abanico de capturas, I mapa de calor en ola |
| Juegos terminados | M estantería que se llena, N medalla, O bolas por mes (violeta de «Terminado», siempre con su nombre) |

- Cada chip es un `.glass` con cabecera (icono + rótulo en `--ink-2`) y su cifra protagonista.
- La animación arranca al entrar en pantalla y se repite al enfocarlo.
- P es el único con varios colores: cada categoría con el suyo.

## 12. Estados

Se normalizan dos.

**Vacío.** Un icono de trazo de 56 en `--ink-3`, un título en 20/800, una frase en 15 que diga qué hacer y, si procede, la pista de la única acción. Clase `.empty`. Sin ilustraciones ni varios botones.

**Carga.** Siluetas de cristal (`.skel`) con la forma y el tamaño de lo que va a llegar, con un brillo que las recorre. Sin ruedas ni barras. Si la carga dura menos de 200 ms no se muestra nada.

Error, confirmación, desactivado y foco no cambian: usan los colores reservados (§3.3) y las reglas de foco (§7).

## 13. Textos

- Cercano y breve, de tú.
- Frases cortas. Sin tecnicismos y sin exclamaciones.
- Los botones empiezan por un verbo: «Cambiar portada», «Borrar captura».
- Mayúscula solo en la inicial. Las mayúsculas sostenidas quedan para las etiquetas de 11.
- Los números llevan su unidad: «128 h», «14 capturas».

## 14. Superficies con reglas propias

### Lector y hook

El marco (barra, botones, menús, pistas) usa los tokens y componentes de la app. La zona de lectura tiene paleta propia, con el prefijo `--rd-` y definida solo en `reader.css`:

| Token | Uso |
|---|---|
| `--rd-paper`, `--rd-text` | Papel y texto de lectura, en claro y oscuro |
| `--rd-anki` | Palabra enviada a Anki |
| `--rd-gram` | Gramática |
| `--jp` | Fuente japonesa |

Hoy `reader.css` y `hook.css` redefinen `--acc`, `--ink`, `--ok` y `--bad` con valores propios. Eso se elimina: los heredan de los tokens comunes.

### Menú del juego (Plaza)

Mismos tokens, mismos cristales y mismos botones. Su arcoíris pastel (`--r1` a `--r6`) se sustituye por el de la app, y sus rarezas por la tabla de §3.4. Lo propio es la maquetación de overlay.

### Tablet

Mismos tokens y mismos componentes, con maquetación táctil:

- Zonas de toque de 44 px como mínimo.
- Sin pistas de mando ni banda inferior.
- Sin foco: el estado activo es el pulsado (0,97).
- Navegación por gestos y botones visibles.

## 15. Plan de migración

Cada fase se sube por separado (commit y push) y deja la app funcionando. No cambia el comportamiento ni las categorías: solo estilos.

Dónde están hoy los valores a mano (hex, tamaños y radios en px):

| Archivo | Hex | Tamaños | Radios |
|---|---|---|---|
| `src/app.css` | 102 | 165 | 104 |
| `src/reader/reader.css` | 157 | 83 | 43 |
| `src/quickmenu/quickmenu.css` | 71 | 68 | 35 |
| `src/hook/hook.css` | 75 | 33 | 17 |
| `Streak.svelte` | 1 | 36 | 13 |
| `Saves.svelte` | 0 | 33 | 15 |
| `AchievementCard.svelte` | 5 | 25 | 12 |
| `HuchaRescate.svelte` | 0 | 27 | 14 |
| `Shelf.svelte` | 12 | 13 | 10 |

Casi todo el estilo de las pantallas vive en `app.css`, así que la migración es sobre todo de ese archivo, bloque a bloque.

| Fase | Qué | Hecho cuando |
|---|---|---|
| 0. Cimientos (**hecha el 2026-11-10**) | Tokens en `src/tokens.css`, importado por `app.css`, con los nuevos (espaciado, movimiento, alturas, iconos, `--ok-f`, `--bad-f`, `--ink-3` corregido) y las clases `.glass-in`, `.b-main`, `.b-soft`, `.skel`. Aplazado: `.chip`, `.empty` e `.ic` chocan con clases locales del mismo nombre y se hacen globales al migrar esas pantallas (fases 1, 2 y 5); el menú del juego, el lector y el hook importan `tokens.css` en sus fases (6 y 7). | La app se ve igual, salvo `--ink-3` algo más oscuro. |
| 1. Logros (**hecha el 2026-11-10**) | `AchievementsModal`, `AchievementCard`: cristales comunes, rarezas desde `RAINBOW`, botones a las tres familias, pistas a la banda. Hecho: rarezas como tokens `--rar-0`…`--rar-5`; el modal usa `TopBar`, `.glass-in`, `.b-main` y tokens (sin hex ni px a mano); en la tarjeta se rehízo la variante «Colección» (la preferida: orden por rareza, aro de color, «+N»). Platino sigue azul (el ámbar se confunde con Oro). Aplazado a la fase 4: los px del resto de variantes de la tarjeta. | Sin hex ni px a mano en esos dos archivos. |
| 2. Modales y selectores (**hecha el 2026-11-10**) | `Picker`, `ActionMenu`, `AddChooser`, `MoveTo`, `AccentPicker`, `ReleasePicker`, `SgdbPicker`, `GamePicker`, `CollForm`, `GameForm`, `StatusBar`, `HiddenList`. Un solo patrón de modal y un solo `.chip`. Hecho: clase `.modal` en `app.css` (cristal con `--r-l`, `--sh-3`, cara `--panel`, `h2` a `--t-2xl`, entrada `modalIn` de 300 ms con rebote y fundido `scrimIn` del velo), que llevan también `Learn`, `Osk`, `Phrases`, `Library` y `GameDirWizard`; `.chip` global en `tokens.css` (pestañas de `SgdbPicker`, filtros de `GamePicker`, grupo de `AchievementsModal`); los distintivos que se llamaban `.chip` (contadores de `Titles`, `Shots`, `Shelf`, `AccentPicker`, `SgdbPicker`) pasan a `.tag` con sus reglas de siempre; el selector ▲▼ (`.fbox`, `.fbtns`) es común en `app.css`; guardar es `.b-main` y lo secundario `.b-soft.pill`, sin glifos dentro (las pistas ya están en la banda). Aplazado: `.tflt` de `Titles` (fase 3) y algunos px de tamaño fijo de los paneles. | Todos los modales comparten cristal, altura flotante y animación. |
| 3. Plantilla (**hecha el 2026-11-10**) | Banda inferior de 64 y márgenes de 24 en todas las pantallas; pistas fuera del contenido. Unificar `LIST_COLOR` con `BAND_DEF`. Hecho: tokens `--band`, `--gut` y `--content-end` (644); `.hints`, `.scrim .hints` y `.hdock` centradas en la banda a 24 del borde; Mi lista acaba en 644 (fichas de 92 y filas de 100 para que quepan las cinco franjas); `.lclip` de Colecciones y la tarjeta de Año (puntos en 610–644) acaban en 644; Ajustes, Ajustes del juego y Frases acortados para no bajar de 644. `LIST_COLOR` toma los colores de `BAND_DEF` (Jugando rojo, Pendiente verde, Terminado violeta, Abandonado azul como Aparcado). Medido con la app de prueba: ninguna pantalla ni panel baja de 656, salvo el visor (exento). | Ninguna pantalla pisa la banda. |
| 4. Barrido (**hecha el 2026-11-10**) | Tamaños, radios, huecos, duraciones y sombras de `app.css` y pantallas restantes (`Streak`, `Saves`, `HuchaRescate`, `Shelf`…) a tokens. Trabajo mecánico: se puede delegar por bloques revisando el diff. Hecho: `font-size` y `border-radius` de `src/` (salvo `reader/`, `hook/` y `quickmenu/`, fases 6 y 7) ajustados al escalón de tokens más cercano; quedan en px las cifras grandes (30 px o más) y los radios de 4 px o menos. Huecos, duraciones y sombras no se tocaron. Pendiente: revisar a mano en claro y oscuro. | Las comprobaciones de abajo dan cero. |
| 5. Estados e iconos (**hecha el 2026-11-10**) | `.empty` y `.skel` en todas las pantallas; iconos al trazo de 24. Hecho: `.empty` global en `tokens.css` (icono de 56, `h3` y `p`) en Logros y Copias; `.skel` aparece a los 200 ms y tiene variantes `.line` y `.round`; los textos de carga de Copias, SteamGridDB, Carpeta con juegos y la ficha de logros del menú del juego pasan a esqueletos (no había ruedas de carga: los aros que giran son decorativos); los iconos pequeños de `icons.ts` se dibujan con `ic()` en `currentColor` (`audioSvg` ya no recibe color). Exentos: los glifos de plataforma (§9). | No quedan ruedas de carga ni iconos con `#fff` fijo. |
| 6. Menú del juego (**hecha el 2026-11-10**) | `quickmenu.css` a tokens comunes. Hecho: letra y radios a la escala; el arcoíris pasa a tokens `--arc-*`, `--arc` y `--arco` en `tokens.css` (lo usan también los degradados de `TilePerf`); botones A/B/X/Y con `--ok-f`, `--bad-f`, `--arc-azul` y `--warn`; los chips propios se llaman `.qchip` para no chocar con el `.chip` común. | Sin `--r1`…`--r6` ni rarezas propias. |
| 7. Lector y hook | Marco a tokens comunes; paleta `--rd-`. | `reader.css` y `hook.css` no redefinen tokens de la app. |
| 8. Tablet | Maquetación táctil sobre los mismos tokens. | Ver `docs/archivo/plan-tablet.md`. |

Cada pantalla migrada se revisa en claro y en oscuro, comparándola con Inicio.

Comprobaciones a mano (deben acercarse a cero en `src/`, fuera de `tokens.css`, `icons.ts` y la paleta de lectura):

```bash
grep -rnoE "#[0-9a-fA-F]{3,8}\b" src --include=*.svelte --include=*.css | wc -l
```

```bash
grep -rnoE "font-size:\s*[0-9.]+px" src --include=*.svelte --include=*.css | wc -l
```

```bash
grep -rnoE "border-radius:\s*[0-9.]+px" src --include=*.svelte --include=*.css | wc -l
```

## 16. Lista de comprobación para una pantalla nueva

1. Usa la plantilla: barra superior, márgenes de 24, banda inferior libre.
2. Solo tokens: ni hex, ni px de letra, radio o hueco fuera de las escalas.
3. Las superficies son `.glass`, `.glass-in` o `.pill`.
4. Como mucho una acción principal; pistas solo abajo.
5. Texto legible de 13 o más, en `--ink` o `--ink-2`.
6. Verde, rojo y ámbar solo con su significado.
7. Tiene estado vacío y de carga.
8. Se ha visto en claro y en oscuro.
9. Se recorre entera con el mando y el foco siempre se ve.
10. No mezcla títulos de categorías distintas.
