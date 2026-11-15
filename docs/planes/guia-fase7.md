# Plan: guía de diseño, fase 7 y cierre (lector, hook y restos)

Objetivo (guía §14 y §15, fase 7): `reader.css` y `hook.css` no redefinen tokens de la app; su marco usa los tokens comunes y la zona de lectura tiene paleta propia `--rd-`. Después, las comprobaciones de §15 se acercan a cero en todo `src/`.

## Decisiones

- `reader.css` y `hook.css` importan `../tokens.css` (como `quickmenu.css`). Los tokens viven en `#app`, que ya existe en `reader.html` y `hook.html`.
- Sin modo oscuro en lector y hook (hoy no lo tienen; no se añade en esta fase).
- Acento: hoy es azul fijo `#00a0d8` con letra blanca encima (3:1, no cumple). Pasa a la pareja de la app para el azul: `--f1/--f2/--gf/--on` para lo que lleva texto y `--v1/--v2/--gv` para gráficos, fijados en `#app` desde `variablesCSS` (o sus valores copiados como tokens en el propio CSS). `--acc` = `--f1`, `--accv` = `--v1`.
- Grises: `#3a414e`→`--ink`; `#4a5260 #5f6776 #5c6472 #6a7282 #6a7383`→`--ink-2`; `#7a8291 #8a919c #8a92a0 #a3abb8`→`--ink-3` (solo adorno; si es texto que se lee, `--ink-2`); `#c3c9d3 #c9cfd8`→`--line-2`.
- Verde y rojo: `#3bb54a`→`--ok`, `#e8384f`→`--bad`; los oscuros con texto blanco (`#24782f #2b8a37`, `#b8323f #c42a40`) → `--ok-f` / `--bad-f`.
- Cristales: los degradados blancos de `.glass`, `.pill` y paneles → `--glass-*`, `--pill-*`, `--hl*`; sombras → `--sh-1/--sh-3`; `#dfe3e9`→`--face`.
- Letra y radios a la escala (`--t-*`, `--r-*`), como en la fase 4: cifras ≥30 px y radios ≤4 px se quedan.
- Paleta de lectura en `reader.css`: `--rd-paper`, `--rd-text`, `--rd-anki` (`#2f8ff0`), `--rd-gram` (`#8a5cd6`), `--jp`. Los morados, ámbar y azules de Anki y gramática (`#6a3fb5 #a07ae2 #8a6a00 #fff3c4 #1f6fc4 #5aa8f5 #6bb2f7 #2d47a8`…) pasan a variantes `--rd-*`.
- `yomitan-popup.css` es el estilo del popup de Yomitan: exento (paleta de lectura).

## Pasos

1. `hook.css` (+ `hook/App.svelte`): import, acento, grises, cristales, letra y radios. Comprobar con `hook.html`.
2. `reader.css`: lo mismo más la paleta `--rd-`. Comprobar con `reader.html` y el panel del hook dentro del lector.
3. Restos fuera de lector y hook: `UseOtherPc.svelte` (16 hex), `Shelf`, `Titles`, `TopBar`, `AchievementCard`, `ResumePill`, y los px de `app.css` que quedan.
4. Contraste del arcoíris con texto: decidir el relleno para texto blanco sobre `--arc-*` claros (chips de Rachas, bandas de Mi lista) y aplicarlo.
5. Comprobaciones de §15 con cifras nuevas en la guía; fase 7 marcada como hecha.
