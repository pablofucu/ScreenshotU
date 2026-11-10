# Plan: acento vivo en toda la app

## Estado

- Hecho: `vivo()` y `--v1 --v2 --gv` en `src/lib/atmos.ts` (`variablesCSS`), valores por defecto en `src/app.css`, reglas en `docs/guia-diseno.md` §3.2 y §11.
- Pendiente: pasar cada uso de `--f1 --f2 --gf --acc` al acento que le toca.

## La regla

| Uso | Acento |
|---|---|
| Lleva texto encima (botón, pastilla, selector activo, insignia) | `--f1` / `--gf` (se queda) |
| Es texto en color (cifra, enlace, etiqueta) | `--f1` (se queda) |
| Anillo, barra, línea, punto, relleno de progreso, mapa de calor | `--v1` / `--gv` |
| Icono grande sin texto, adorno, brillo, halo | `--v1` / `--gv` |
| Borde o anillo de foco | `--f1` (se queda: necesita contraste 3:1 con el fondo) |

En la duda, se queda en `--f1`. El cambio a `--v1` solo se hace cuando no hay texto implicado.

## Inventario (usos de `var(--f1|f2|gf|acc)`)

| Fichero | Usos |
|---|---|
| `src/app.css` | 96 |
| `src/components/Shelf.svelte` | 24 |
| `src/reader/reader.css` | 19 |
| `src/hook/hook.css` | 12 |
| `src/screens/Saves.svelte` | 8 |
| `src/tokens.css` | 5 |
| Settings, GamePicker, TopBar, AudioMenu | 3 cada uno |
| Titles, Home, GameSettings, Learn | 2 cada uno |
| Search, Library, ActionMenu, ReadCard, App.svelte | 1 cada uno |

`store.svelte.ts:606` (PAIR_FILL) usa `variablesCSS` para pintar parejas: decidir si sus gráficos pasan a `--v1`.

## Pasos

1. `app.css` y `Shelf.svelte`: clasificar cada uso con la tabla y cambiar solo los gráficos. Un commit.
2. `reader.css`, `hook.css`, `Saves.svelte`. Un commit.
3. El resto de componentes. Un commit.
4. Repaso visual con los siete tonos (rojo, naranja, amarillo, verde, azul, añil, violeta), en claro y oscuro: ningún texto sobre `--v1`, el amarillo se ve amarillo.

## Panorama

Se rehace con los 18 chips del catálogo (`docs/mockups-estadisticas/chips/index.html`, guía §11.1), incluido el resumen semanal y los juegos terminados. Va después de la migración, para estrenar ya el acento vivo.
