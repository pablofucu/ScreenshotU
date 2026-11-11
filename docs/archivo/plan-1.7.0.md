# Plan 1.7.0 — pulido v5 tras la revisión en la web

Notas tomadas navegando la versión web (2026-10-31). Las categorías no se tocan: todo vive dentro de cada una.

## Notas del usuario (tal cual, ordenadas)

| # | Dónde | Qué pasa | Qué quiere |
|---|-------|----------|------------|
| 1 | Inicio, iconos de plataforma (`Home.svelte:79`, `.glass.plat`) | La animación al pasar el ratón se ve rara | Suave y coherente con el resto |
| 2 | Mi lista (`MiListaDual.svelte`, `.board`) | La estructura está bien | Más *glass*, como el resto de la app |
| 3 | Colecciones (`Shelf.svelte:88`, `.bsh`) | Se ve mal, no está al día con v5 y choca con la barra de arriba | Rehacerla con el diseño v5 |
| 4 | Catálogo (`Titles.svelte:127`) | Insignia de estado (✓ terminado / – abandonado) | Quitarla |
| 5 | Portadas | Punto de color encima de cada juego (`.pip`) | Quitarlo |
| 6 | Logros (`AchievementsModal.svelte:245`) | La ficha (`.pop`) sale abierta al entrar y siempre hay una abierta | Que solo salga cuando se pide |
| 7 | Logros, iconos (`:228`) | Iconos redondos | Cuadrados |
| 8 | Logros | Hay que agrupar por rareza de verdad | Buscar de dónde sacar la rareza |
| 9 | Catálogo, halo de la portada enfocada (`Titles.svelte:219`) | Estático | Que se mueva un poco, como en el mockup |
| 10 | Catálogo | Multicolor | Opciones para un arcoíris que quede mejor |
| 11 | Catálogo, barra lateral (`Titles.svelte:229`, `.sbar.rb`) | Gusta, pero no sirve para nada | Darle una función |

## Diagnóstico

**1 · Hover de plataformas.** Al enfocar, `.glass.f` (`app.css:73`) hace tres cosas a la vez:
- escala con rebote (`cubic-bezier(.34,1.32,…)`);
- cambia el fondo de cristal a un degradado de color. Los degradados no se interpolan, así que el color salta de golpe mientras la escala aún está animándose;
- repinta el `backdrop-filter`.

Además, el `.plabel` de debajo cambia de golpe (`class:on`). El resultado es un parpadeo y un rebote que no van a la vez. Hay que confirmarlo viéndolo en el navegador antes de tocarlo.

**4/5 · Insignia y punto.**
- Insignia: solo existe en `Titles.svelte:127`; los estilos están en `app.css:512-514` y `830-833`.
- Punto `<i class="pip">`: aparece en Titles, Home, Shelf y MiListaDual; el estilo está en `app.css:366`.
- Ojo: `app.pipStyle()` también define `--p1/--p2`, que usan el halo y el degradado. Se quita solo el `<i>`; la función se queda.

**6 · Ficha de logros abierta.** `selected` cae a `filtered[0]` cuando no hay nada elegido (`AchievementsModal.svelte:50`), y `selectedIdx` empieza en 0. Por eso siempre hay una ficha abierta.

**8 · Rareza.**
- La UI ya agrupa por 5 niveles: Legendario <5 %, Ultra raro <12 %, Raro <25 %, Poco común <50 % y Común.
- El backend nunca rellena la rareza: `achievements.rs:373` pone `rarity_tier: 0, rarity_pct: None`. Con datos reales, todo cae en «Común». Los porcentajes que se ven en la web son datos de prueba.

**9 · Halo.** En `mockups/sistema-v5.html:40` el foco lleva un `.ring`: un borde de 3 px con `linear-gradient(115deg,g1,g2,g1)` que recorre la portada en bucle (`@keyframes sheen`, 3 s). Encima va un `.halo` de dos sombras de color, una a cada lado. En la app el foco es un degradado fijo.

## Opciones para lo que hay que decidir

### 8 · De dónde sacar la rareza
- **A (recomendada): API pública de Steam.** `ISteamUserStats/GetGlobalAchievementPercentagesForGame/v2/?gameid=<appid>` no necesita clave. Devuelve `name` (= `apiName`) y `percent`. Ya tenemos el appid, porque los logros salen de Goldberg/GSE con appid de Steam. Plan:
  - se guarda en `visor.db` (tabla `ach_rarity(app_id, api_name, pct, fetched_at)`);
  - se refresca como mucho una vez por semana, en segundo plano con `ureq`, igual que `fetch_missing_icons`.
- **B: añadir una heurística sin red.** Si no hay datos, los secretos sin conseguir van a «Raro» y el resto a un grupo **«Sin datos de rareza»**, en vez de mentir con «Común».
- Descartado: Exophase y TrueAchievements, porque no tienen API y habría que hacer *scraping*.
- Propuesta: **A + B**. B solo cubre los juegos sin appid o cuando no hay conexión.

### 10 · Arcoíris en el catálogo
1. **Arcoíris por posición (recomendada).** Cada portada recibe el tono que le toca por su sitio en la lista: A→Z recorre rojo→violeta. Ese tono tiñe el halo y el anillo del foco. La barra lateral muestra el mismo arcoíris, así que la barra y la portada enfocada coinciden.
2. **Una franja por página.** Cada página de la rejilla tiene un tono del arcoíris; al pasar de página cambia la atmósfera. Es más tranquilo, pero dentro de la página no hay variación.
3. **Anillo arcoíris animado.** Solo el borde del foco es arcoíris (conic/linear que gira), al estilo del mockup. Las portadas quedan neutras. Combina con 1 o 2.
4. **Fondo que cambia con el scroll.** La atmósfera cambia de tono según dónde estás en la lista. Es sutil, pero ya tenemos el tween de `atmos.ts`.

Las opciones 1, 3 y 4 se pueden combinar. Mi propuesta: **1 + 3** (posición + anillo animado), con 4 como extra si queda bien.

**Conflicto:** hoy el foco de cada juego usa *sus* dos colores (decisión v5). Si se elige 1, en el catálogo manda el arcoíris y en la ficha del juego siguen sus colores.

### 11 · Que la barra lateral sirva
- **Minimapa clicable (recomendada):**
  - clic o arrastre en la barra = saltar a esa altura;
  - marcas de letra (orden A-Z) o de año (orden por fecha) en los cortes;
  - al pasar el ratón, una burbuja con el juego que hay ahí.
- **Barra de estados:** muescas de color donde hay juegos «jugando» o «terminado». Ya hay colores de estado en `LIST_COLOR`.
- Con el mando, LB/RB ya pasan página. Se puede añadir mantener pulsado = salto por letra, enseñando la letra grande en la barra.

### 7 · Iconos cuadrados
- Pasar `.ico` a un cuadrado con esquinas de unos 12 px, como los iconos de Steam.
- El arco de progreso (`.arc`, una forma *conic* redonda) tiene que pasar a un rectángulo redondeado. Se puede hacer con un SVG `rect` y `stroke-dasharray`.

### 6 · Cuándo se abre la ficha
- Propuesta:
  - nada elegido al entrar (`selectedIdx = -1`);
  - con el mando, la ficha se abre al mover el foco y se cierra con B (el primer B cierra la ficha, el segundo cierra Logros);
  - con el ratón, la ficha sale al pasar por encima o al hacer clic, y se cierra al salir o al hacer clic fuera.

## Fases (orden de trabajo)

**Fase 1 — rápidas (una tanda, Sonnet o yo)**
- 4 y 5: quitar la insignia y el punto (en los 4 componentes y el CSS que deja de usarse).
- 6: la ficha de logros cerrada por defecto.
- 7: iconos cuadrados y el arco adaptado.
- 1: arreglar el hover de las plataformas (sin rebote en el cambio de color; el fondo de color entra por opacidad con un `::before` en vez de cambiar el `background`; la etiqueta, a la vez).

**Fase 2 — rediseño de pantallas**
- 3: Colecciones a v5. Primero ver en el navegador el choque con `TopBar` (`top` del `.shelf`). Después, la cabecera del estante como píldora de cristal, la aurora dentro del cristal y la línea de tiempo con tokens.
- 2: Mi lista más *glass*. Las franjas (`.band`) pasan a cristal real (`backdrop-filter`, el borde blanco y el brillo de `.glass`), y las tarjetas `.it` también.
- 9: halo que se mueve (`.ring` + `sheen` del mockup en `.glass.f` de las portadas; respeta `prefers-reduced-motion`).

**Fase 3 — catálogo arcoíris + barra útil** (según lo que se decida en 10 y 11)

**Fase 4 — rareza real** (backend Rust + caché en `visor.db` + fallback sin datos). Es el único cambio en `src-tauri`, así que hay que comprobarlo con la app de escritorio, no solo en la web.

Al acabar cada fase: verificar en el navegador (http://localhost:5174), hacer commit y push.

## Decisiones (cerradas tras ver `mockups/colecciones-arcoiris-v5.html`)
1. **Colecciones:** diseño A, «Estante de cristal».
   - Las colecciones van en el orden del usuario, no por color. Los juegos de dentro van por año.
   - El color sale de los juegos: aurora de hasta 5 colores de portada armonizados. La píldora y el foco usan los 2 tonos más repetidos. Un acento manual tiene prioridad.
   - **Línea de tiempo T1:** un riel bajo las portadas.
     - Cada portada se apoya en su año, con marcas de año en el riel y los huecos entre años comprimidos.
     - El riel se pinta con los colores de los juegos en orden.
     - El año de la portada enfocada crece.
2. **Arcoíris del catálogo:**
   - Orden «Por color» (ya existe en `TITLE_ORDERS`): de rojo a violeta, con los grises al final, junto a A–Z.
   - El fondo sigue la posición en la lista (opción 4).
   - El halo del foco usa **los dos colores del propio juego**, en un anillo `g1→g2→g1` que gira, más sus dos sombras de color. Así desaparece el conflicto: cada juego conserva sus colores.
3. **Barra lateral:** minimapa.
   - Se pinta con los colores reales de los juegos en orden.
   - Clic o arrastre salta a esa altura.
   - Al pasar el ratón sale una burbuja con el nombre del juego y la familia de color.
   - La píldora de página dice, por ejemplo, «Página 2/3 · Verdes–Azules».
4. **Rareza:** A + B (API de Steam con caché semanal en `visor.db`, más el grupo «Sin datos de rareza» cuando no hay datos).
5. **Punto de color:** fuera de todas las portadas. `pipStyle()` se queda porque da `--p1/--p2` al halo.

## Reparto (Opus planifica y revisa; los agentes trabajan en ficheros que no se pisan)

| Bloque | Agente | Notas | Ficheros (solo estos) |
|---|---|---|---|
| A · Logros | Sonnet | 6, 7, 8 (UI) | `src/screens/AchievementsModal.svelte`; en `controller.svelte.ts`, solo lo de logros |
| B · Rareza | Sonnet | 8 (backend) | `src-tauri/src/achievements.rs`, la tabla nueva en la base de datos y `cargo check` |
| C · Colecciones | Sonnet | 3, 5 | `src/screens/CollTab.svelte`, `src/components/Shelf.svelte`, `src/lib/coleccion.svelte.ts`, `src/lib/collections.ts` |
| D · Catálogo | Sonnet | 4, 5, 9, 10, 11 | `src/screens/Titles.svelte`; en `app.css`, solo `.sbar`, `.sbadge` y el bloque nuevo `.ring` al final |
| E · Inicio y Mi lista | agy flash | 1, 2, 5 | `src/screens/Home.svelte`, `src/screens/MiListaDual.svelte` (estilos propios del componente) |

Al final (Opus):
- quitar `.pip` de `app.css`;
- verificar en http://localhost:5174;
- hacer commit y push.
