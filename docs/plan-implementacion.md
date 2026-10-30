# Plan de implementación (borrador, fase de planificación)

Estado: **nada se programa todavía**. Este documento ordena el trabajo para cuando se dé luz verde. Las decisiones marcadas con ❓ siguen abiertas.

## 0. Decisiones tomadas

| Tema | Decisión |
|---|---|
| Mi lista | **Tablero de franjas** (`mockups/milista-v3.html`, propuesta 3). Todas las portadas del mismo tamaño; el estado se distingue por franja, etiqueta lateral de color y horas / días sin jugar en cada portada. |
| Modelo de estados | En marcha (≤14 d) · A medias (15–90 d) · Pendiente (manual) · Aparcado (>90 d o abandonado) · Terminado (por año). X fija un juego. Aviso discreto «Se aparca en N d» cuando faltan ≤10. |
| Logros | Se mantienen las propuestas `logros-1/2/3`; ❓ falta elegir. |
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
1. ❓ Logros: elegir entre lista, mosaico o rareza.
2. ❓ Nombre del último cubo: Aparcados / En el cajón / Archivo.
3. ❓ Aspecto final del degradado y del foco con los colores del juego (`sistema-v4`).
4. ❓ ¿Dónde se guarda el «fijar»? (confirmar al empezar la fase 3).
5. ❓ ¿Los umbrales 14/90 días se ajustan en Ajustes o quedan fijos?
