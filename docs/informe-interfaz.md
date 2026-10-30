# Informe de interfaz: coherencia visual, Logros y Mi lista

Versión de la app: 1.4.16. Fecha: 2026-10-30.

## 0. Lo que pide Pablo (restricciones de diseño)

1. La base está bien; hay pequeños errores y la interfaz **no es consistente**: los estilos no terminan de casar entre pantallas.
2. **El cristal (glass) estilo Wii U es fundamental.** No se toca su esencia: `.glass` con borde (rim), arco de brillo (gloss) y cara.
3. **Los patrones de color arcoíris (`RAINBOW`) son fundamentales.** Cada elemento tiene su acento.
4. **La interfaz no debe estar cargada**, sobre todo de botones. Menos es más.
5. **Las categorías de la biblioteca no se tocan** (Juegos PC, Switch, Novelas visuales, etc.). Todo lo nuevo vive dentro de cada categoría. Ver `CLAUDE.md`.
6. Dos pantallas prioritarias:
   - **Logros a pantalla completa** (`AchievementsModal`): necesita mucho trabajo.
   - **Mi lista** (jugando / en pausa / pendiente): las categorías no convencen y la lógica debe ser más automática.

## 1. Diagnóstico de inconsistencias (datos medidos en el código)

### 1.1 Radios de borde: escala fragmentada
Valores en uso: 999px ×60, 50 ×33, 12px ×16, 14px ×15, 18px ×13, 3px ×13, 10px ×12, 16px ×10, 11px ×9, y además 4, 5, 6, 7, 8, 20, 22, 24, 26, 28, 30. No hay escala; cada componente elige a ojo.

**Propuesta:** 5 radios con nombre.
| Token | Valor | Uso |
|---|---|---|
| `--r-xs` | 6px | etiquetas, barras, badges pequeños |
| `--r-sm` | 12px | chips, campos, miniaturas |
| `--r-md` | 18px | tarjetas, paneles internos |
| `--r-lg` | 28px | superficies glass grandes (ya es `.glass --r`) |
| `--r-pill` | 999px | pills y botones redondos |

### 1.2 Escala tipográfica: ~35 tamaños
Aparecen muchos medios tamaños (9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5). Los más usados: 13, 14, 15, 17, 18, 16, 11, 12.

**Propuesta:** 7 pasos: 11 (micro/etiqueta), 13 (secundario), 15 (cuerpo), 17 (destacado), 20 (título de panel), 28 (título de pantalla), 44+ (cifras héroe). Una sola fuente (`--round`), pesos 500/700/800.

### 1.3 Colores hex fijos en vez de tokens
Concentración: AchievementsModal 73, Saves 84, AchievementCard 55, Streak 31, HuchaRescate 25, Titles 19, Shelf 18, StatusBar 17. Hace que cada pantalla tenga "su" azul, su gris y su blanco.

**Propuesta:** tokens de superficie y texto (`--ink`, `--ink-2`, `--ink-3`, `--line`, `--face`, `--face-hi`) y uso de `color-mix(in srgb, var(--acc) X%, white)` para derivar variantes del acento. Sustituir hex por tokens pantalla a pantalla.

### 1.4 Cristal aplicado de forma desigual
El cristal vive en `app.css`, pero AchievementsModal (2), Saves (1), Titles (1) y AchievementCard (1) redefinen `backdrop-filter` y degradados por su cuenta, con otros blancos y otras sombras.

**Propuesta:** tres niveles de glass reutilizables, no más: `glass` (panel), `glass-in` (panel dentro de panel, más tenue) y `glass-chip` (pastilla). Cualquier superficie nueva usa uno de ellos.

### 1.5 Botones: cuatro familias más estilos propios
Existen `.btn` (A/B/X/Y), `.chip`, `.soft`, `.pill`, y además AchievementsModal (7 botones propios), Saves (10), StatusBar (5), ReleasePicker (4).

**Propuesta, muy reducida:**
- **Acción principal**: un solo `.btn` con acento (máx. 1 por pantalla).
- **Acción secundaria**: `.pill` de cristal, sin color propio.
- **Selector** (pestañas/filtros): `.chip`, con acento al activarse.
- **Hints de mando** (A/B/X/Y/LB/RB): solo en la barra inferior, nunca repetidos dentro de la pantalla.
Todo lo demás se resuelve con foco de mando, no con botones visibles.

### 1.6 Espaciado y barra inferior
Los márgenes varían entre pantallas (huecos de 10, 12, 14, 18, 22…). **Propuesta:** escala de 4 (4/8/12/16/24/32). En Mi lista, la barra inferior de hints se solapa visualmente con la última fila de contenido; reservar una banda fija de ~64px.

## 2. Pantalla de Logros (`AchievementsModal.svelte`, `AchievementCard.svelte`)

Estado: 911 + 762 líneas, 128 colores hex fijos, 20 radios distintos, 7 botones propios. Es la pantalla más alejada del sistema.

Problemas:
- Mucha información compitiendo al mismo nivel (progreso, rareza, filtros, orden, iconos, fechas).
- Botones propios que no son los del sistema.
- Rareza y estados resueltos con colores sueltos, no con el arcoíris de la app.
- El cristal es una imitación local, no el `.glass` común.

Qué debe resolver el rediseño:
- **Cabecera** con el progreso global (anillo o barra arcoíris) y poco más: nombre del juego, X/Y logros, %.
- **Filtros mínimos** (Todos / Conseguidos / Pendientes / Raros) como un único selector `.chip`, orden como segundo selector discreto. Nada de botones sueltos.
- **Rareza** mapeada al arcoíris (común → azul/verde, rara → naranja, ultra rara → rojo/violeta), con un solo criterio visual.
- **Tarjeta de logro** simple: icono, título, una línea de descripción, fecha o progreso. Detalles al enfocar, no siempre visibles.
- Navegación con mando (foco claro con el acento).
- Pedir 3 propuestas con enfoques distintos: lista vertical, mosaico/rejilla de iconos, y por "rareza/trofeos" agrupado.

## 3. Mi lista (jugando / en pausa / pendientes)

### 3.1 Cómo está ahora
Estados manuales: `none | pending | playing | finished | dropped`.
Estantes: **Jugando ahora**, **En pausa / Abandonados** (kind `dropped`), **Pendientes** (orden manual `listPos`) y terminados por año. Hay lógica de "estancados" (`stalledGames`, `checkStalled`): un juego en `playing` sin uso durante `listStalledDays` se mueve solo a pendiente/abandonado, o se pregunta (`StalledCard`).
Config: `listAutoPlay`, `listStalled`, `listStalledDays`, `listStalledAction`.

### 3.2 Lo que no convence
- **"En pausa / Abandonados" mezcla dos cosas distintas**: lo que dejaste a medias (quieres volver) y lo que ya no vas a jugar.
- **"Pendiente" y "en pausa" se confunden**: ambos son "no lo estoy jugando ahora". No queda claro qué distingue uno de otro.
- Demasiado manual: hay que mover los juegos y responder preguntas.
- El estante de pausa casi vacío y el de pendientes gigante dejan una pantalla desequilibrada.

### 3.3 Idea de modelo automático (basado en uso reciente)
La señal principal es **cuándo se jugó por última vez** y **cuánto se juega**, que la app ya sabe (capturas, tiempo de juego, apertura). El usuario solo marca lo que no se puede deducir.

| Cubo | Regla (propuesta, ajustable) | Qué es |
|---|---|---|
| **En marcha** | jugado en los últimos ~14 días, ordenado por actividad reciente | Lo que juegas y abres a menudo; sube solo al abrirlo |
| **A medias** | empezado (hay tiempo/capturas) y sin jugar 15–90 días | Lo medio abandonado, que se sigue viendo y puede volver a "En marcha" al abrirlo |
| **Pendiente** | marcado por el usuario o añadido sin jugar; nunca empezado | Cola de por jugar, con orden manual |
| **Aparcado** (nombre a debatir) | sin jugar >90 días, o marcado "abandonado", o llevaba mucho tiempo sin tocarse en Pendiente | Lo que no se juega o se dejó del todo; visible pero discreto, más allá de pendientes |
| **Terminado** | marcado por el usuario, agrupado por año | Sin cambios |

Principios:
- Los movimientos entre **En marcha ⇄ A medias ⇄ Aparcado** son **automáticos** por fecha; sin preguntas. Un aviso discreto (no una tarjeta modal) si algo está a punto de pasar a Aparcado.
- El usuario solo decide: Pendiente, Terminado, y "Aparcar/Abandonar" explícito. Hay un "fijar" para que un juego no baje automáticamente.
- Abrir o jugar un juego lo devuelve siempre a **En marcha** (`listAutoPlay` ya lo hace en parte).
- Los umbrales (14/90 días) se pueden ajustar en Ajustes, reutilizando `listStalledDays`.
- Todo ocurre **dentro de cada categoría**, sin mezclar títulos de categorías distintas.

### 3.4 Preguntas abiertas para las propuestas
- ¿Nombre del último cubo? Candidatos: "Aparcados", "En el cajón", "Archivo".
- ¿Mostrar los cubos como estantes horizontales (como ahora), como columnas o como una sola vista con "temperatura" (de más caliente a más frío)?
- ¿Cómo evitar que Pendientes (muchos) aplaste al resto? (límite visible + "ver todos", o estante en carrusel).
- Una idea de lenguaje visual: la **temperatura** de un juego como gradiente de arcoíris (rojo/naranja = caliente/reciente → azul/violeta = frío/aparcado), sin necesidad de etiquetas ni botones.

## 4. Sistema de diseño unificado propuesto (resumen)

- **Tokens**: radios (5), tipografía (7), espaciado (escala de 4), superficies/texto, acento `--acc` derivado del `RAINBOW`.
- **Glass**: 3 niveles. Conserva el rim, el arco de brillo y la cara del Wii U.
- **Botones**: 1 principal + 1 secundario + 1 selector. Hints de mando solo abajo.
- **Arcoíris**: cada entidad conserva su color (estado, categoría, rareza), pero todo se obtiene de `RAINBOW`; nada de hex sueltos.
- **Menos carga**: ocultar controles hasta el foco; priorizar un elemento protagonista por pantalla.

## 5. Entregables pedidos al diseñador (Opus)

HTML autónomos (un fichero por propuesta), resolución del escenario 1280×720, estética Wii U cristal + arcoíris, en `mockups/`:

1. **Sistema de diseño**: una hoja con radios, tipografía, glass (3 niveles), botones (3 tipos), chips, arcoíris. (1 propuesta clara + variantes si aportan.)
2. **Logros a pantalla completa**: 3 propuestas distintas (lista, mosaico, agrupada por rareza).
3. **Mi lista**: 3 propuestas con el modelo automático de §3.3 (estantes, columnas, "temperatura").
4. Opcional: una pantalla existente (p. ej. Estadísticas o Ajustes) pasada por el sistema para demostrar la consistencia.

Ficheros de referencia: `src/app.css` (glass, `.btn`, `.chip`, `.pill`), `src/lib/store.svelte.ts` (`LIST_COLOR`, `RAINBOW`), `src/lib/milista.svelte.ts`, `src/screens/AchievementsModal.svelte`, `src/components/AchievementCard.svelte`, y trabajos anteriores en `design/`, `diseno/`, `mockups/propuesta-1..3.html`.
