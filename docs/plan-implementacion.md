# Plan de implementación (borrador, fase de planificación)

Estado: **nada se programa todavía**. Este documento ordena el trabajo para cuando se dé luz verde. Las decisiones marcadas con ❓ siguen abiertas.

## 0. Decisiones tomadas

| Tema | Decisión |
|---|---|
| Mi lista | **Tablero de franjas** (`mockups/milista-v3.html`, propuesta 3). Todas las portadas del mismo tamaño; el estado se distingue por franja, etiqueta lateral de color y horas / días sin jugar en cada portada. |
| Modelo de estados | En marcha (≤14 d) · A medias (15–90 d) · Pendiente (manual) · Aparcado (>90 d o abandonado) · Terminado (por año). X fija un juego. Aviso discreto «Se aparca en N d» cuando faltan ≤10. |
| Logros | **Rareza** (`mockups/logros-3-rareza.html`), elegida el 2026-10-31. |
| Color por juego | Se conserva el algoritmo de v3: la portada decide el **tono**, el sistema fija luminosidad y saturación. Falta cerrar el aspecto visual (ver `sistema-v4.html`, en curso). |
| Modo oscuro | **«Noche ahumada»** (con neón, de `mockups/propuestas-v2.html`) se implementa como el modo oscuro oficial. Los tokens de superficie y texto deben tener variante oscura desde la fase 1. |
| Categorías | No se tocan (ver `CLAUDE.md`). |

## 1. Lo que ya existe y se reutiliza

- `src/lib/oklch.ts` (réplica de `visor_core::palette`): `accentForHue`, `hueOf`, `inkDark`. Es la base de la normalización de tonos; no hay que escribirla de cero.
- Paleta por título (`t.accent.palette`) y selector `AccentPicker.svelte`.
- Modelo de estados y estantes: `src/lib/store.svelte.ts` (STATUS_ORDER, STATUS_LABEL, LIST_COLOR), `src/lib/milista.svelte.ts`, `StatusBar`, `StalledCard`, `MiListaDual`.

## 2. Fases (orden recomendado; cada fase se sube con commit + push)

### Fase 1 · Tokens y base visual (sin cambios de pantalla)
- Añadir a `src/app.css` los tokens: radios (6/12/18/28/999), tipografía (11/13/15/17/20/28/44), espaciado (múltiplos de 4), superficies y texto (`--ink`, `--ink-2`, `--ink-3`, `--line`, `--face`, `--face-hi`), tres niveles de cristal (`glass`, `glass-in`, `glass-chip`).
- Variables de color de juego (`--g1`, `--g2`, `--grad`) calculadas en un único sitio del store a partir del tono de la portada; fallback al arcoíris global.
- Incluir la variante «Noche ahumada» de los tokens (`[data-theme=dark]` o equivalente) y un ajuste para elegir tema.
- Riesgo bajo: solo se añaden variables, no se cambia ninguna regla existente.

### Fase 2 · Foco y degradado del sistema
- Cuando `sistema-v4` esté aprobado: nuevo estilo de foco y degradado en `app.css` (`.fx`, `.glass`, `.chip`, `.btn`), aplicado primero a una sola pantalla (Detail) y luego al resto.

### Fase 3 · Mi lista (franjas + modelo automático)
- `milista.svelte.ts`: el cálculo de cubos por fecha del último juego (umbrales desde `listStalledDays`); retirar el flujo de preguntas (`StalledCard`) a favor del aviso discreto.
- Nuevo componente de franja con etiqueta lateral; reutilizar la tarjeta de portada.
- Acción X = fijar (nuevo campo persistido por título; ❓ en SQLite o en el JSON actual de la lista, comprobar dónde vive `listPos`).
- Backend (`src-tauri/src`): solo si la fecha de último juego no está ya disponible; comprobar antes.

### Fase 4 · Logros a pantalla completa
- Tras elegir propuesta: reescribir `AchievementsModal.svelte` y `AchievementCard.svelte` con los tokens, eliminando los ~128 hex fijos y los botones propios.

### Fase 5 · Barrido de consistencia
- Sustituir hex fijos por tokens pantalla a pantalla, empezando por Saves (84), Streak (31), HuchaRescate (25), Titles (19), Shelf (18), StatusBar (17).
- Unificar botones en tres tipos (principal, secundario, selector).

## 3. Cómo verificar cada fase
- Abrir `http://localhost:5173/?mock` (sin `?mock` sale pantalla negra), revisar a 1280×720 y con mando (flechas, A/B/X/Y).
- Comprobar 4–5 juegos con paletas muy distintas (casi negro, casi blanco, gris, saturado).
- Sin compilar el binario: basta `npm run build`/typecheck cuando proceda.

## 4. Decisiones pendientes
1. ✅ Logros: rareza (2026-10-31).
2. ❓ Nombre del último cubo: Aparcados / En el cajón / Archivo.
3. ❓ Aspecto final del degradado y del foco con los colores del juego (`sistema-v4`).
4. ❓ ¿Dónde se guarda el «fijar»? (confirmar al empezar la fase 3).
5. ❓ ¿Los umbrales 14/90 días se ajustan en Ajustes o quedan fijos?

## 5. Traspaso a la siguiente sesión (2026-10-31)

Estado: **planificación terminada a falta de 4 decisiones; no se ha programado nada** (`src/` y `src-tauri/` intactos).

### Qué hay en `mockups/` (todo subido a GitHub)
- `milista-v3.html` → **elegida la propuesta 3, Tablero de franjas** (resto descartadas).
- `sistema-v4.html` → 9 vistas (teclas 1-9, `D` cambia claro/oscuro): V1 Cristal teñido, V2 Hilo nácar, V3 Atmósfera, algoritmo de dos tonos (`--g1`, `--g2`, `--grad`), dosis, 12 portadas, **Noche ahumada** (modo oscuro oficial), constitución (10 reglas) y código (`extraerDosTonos`, `tonosDelJuego`, `variablesCSS(h1,h2,modo)`).
- `sistema-v5.html` → arcoíris ordenado fuera de los juegos (modo claro, dosis V3 Atmósfera), 3 opciones a comparar (teclas 1-3): **por sección** (recomendada), por posición en pantalla, por orden de la lista. Incluye la transición al entrar/salir de un juego (cada juego conserva su pareja g1/g2).
- `sistema-v3.html` → **rechazado** (el degradado solo salía en barras de progreso y el foco usaba el arcoíris global).
- `logros-3-rareza` → **elegido**; `logros-1-lista` y `logros-2-mosaico` descartados.
- `propuesta-1..3.html` son ficheros propios del usuario (sin versionar; no tocar).
- Informe de diagnóstico: `docs/informe-interfaz.md`.

### Decisiones tomadas
1. Mi lista = Tablero de franjas, portadas del mismo tamaño, modelo automático (En marcha / A medias / Pendiente / Aparcado / Terminado por año; X fija; aviso discreto a ≤10 días).
2. El foco y los degradados usan los **dos colores del juego**, en toda la interfaz (no solo barras). El arcoíris global solo en pantallas sin juego. Sin borde arcoíris continuo.
3. El color de la portada solo decide el tono; el sistema fija luminosidad y saturación (OKLCH).
4. **Noche ahumada** (con neón) = modo oscuro oficial.
5. Las vistas (mockups de pantallas) se quedan como están.
6. Las categorías de la biblioteca no se tocan (`CLAUDE.md`).

### Decisiones abiertas (empezar por aquí)
1. ✅ Dosis del degradado en `sistema-v4`: **V3 Atmósfera** (2026-10-31). Dentro de un juego, su pareja g1/g2. ✅ Fuera de los juegos: arcoíris ordenado **por sección** (`sistema-v5`, opción 1). Las categorías se reparten en su orden de rojo a violeta; Inicio, Mi lista, Estadísticas/Racha y Ajustes tienen tono fijo. Se quitan los modos `accentMode` (fijo/arte/arcoíris); se conserva el selector de color por juego.

### Alcance de la primera entrega (2026-10-31)
Solo **atmósfera + degradado**: manchas del fondo con g1/g2 (`--b1..3`), variables `--g1 --g2 --grad --f1 --f2 --gf --on` en `#stage` con transición de tono (~700 ms, respeta movimiento reducido), botón principal y barras de progreso con el degradado.

### Apuntado para después
- **Ajustes como en el mockup** (`sistema-v5`, `scrAjustes`): menú General / Apariencia / Mando / Biblioteca / Mi lista / Acerca de, y **cada sección de Ajustes con un color del arcoíris**.
- Estilo de foco y pestañas con subrayado degradado.
- Barra lateral A-Z en arcoíris.
- Punto de color en cada portada.
- Animación de la portada «volando» al entrar en un juego.
- Modo oscuro «Noche ahumada».
2. ✅ Logros: rareza. «Fijar» se guarda en `config.pinned` (ids de título); el tema en `config.theme`.
3. ❓ Nombre del cubo «Aparcado»: Aparcados / En el cajón / Archivo.
4. ❓ Dónde se guarda «fijar» (mirar dónde vive `listPos`) y si los umbrales 14/90 días se ajustan en Ajustes.

### Cómo retomar
- Con la dosis elegida, pasar a la **Fase 1** (tokens + variante oscura + `--g1/--g2/--grad`). Reutilizar `src/lib/oklch.ts`; la réplica de Rust está en `visor_core::palette`.
- Verificar con `http://localhost:5173/?mock` (sin `?mock` sale pantalla negra).
- Cada cambio de código: commit + push (ver `CLAUDE.md`). Cuidado con los finales de línea mixtos (LF/CRLF).
