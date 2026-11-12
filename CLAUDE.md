# Visor de capturas (Tauri 2 + Svelte)

App de escritorio (`src/` Svelte, `src-tauri/` Rust con el núcleo en `src-tauri/visor-core`) y app de tablet (`mobile/`, Capacitor). Las reglas de aquí valen también para quien trabaje por delegación (`AGENTS.md` las resume).

## Reglas fijas

- **Las categorías no se tocan.** Las categorías de la biblioteca (Juegos, Novelas visuales, Switch, Novelas japonesas… las plataformas que salen de las carpetas) se quedan exactamente como están: no se renombran, no se fusionan, no se reorganizan ni se cambia cómo se detectan. Todo lo nuevo (colecciones, estados, «Mi lista», órdenes, vistas) vive **dentro** de cada categoría y nunca mezcla títulos de categorías distintas.
- **Migraciones inmutables.** Las migraciones ya publicadas de `visor-core/src/db.rs` no se editan; un cambio de esquema es una migración nueva al final, con su prueba.
- **Diseño:** se sigue `docs/guia-diseno.md` (lienzo 1280×720, mando primero, tokens en vez de colores fijos). Sin emojis; iconos SVG.
- **Finales de línea:** el repo mezcla LF y CRLF; cada fichero conserva los suyos.

## Dónde está cada cosa

- `docs/ESTADO.md`: **única** lista de proyectos (en curso, siguiente, aparcado) con el siguiente paso de cada uno. Se lee al empezar y se actualiza en el mismo commit que cambia el estado de algo. No hay tablero ni otra lista de tareas.
- `docs/planes/`: un plan por proyecto vivo (`<tema>.md`). Al terminar el proyecto, el plan pasa a `docs/archivo/`.
- `docs/maquetas/<tema>/`: maquetas HTML. La elegida se anota en el plan; las descartadas no se guardan.
- `docs/referencia/`: documentación que no caduca (APIs, publicar, estudios).
- `docs/archivo/`: planes hechos e informes viejos. No se lee salvo que se pida.
- Los ficheros temporales van al directorio temporal de la sesión, no al repositorio.

## Flujo de trabajo

1. **Un proyecto cada vez.** Antes de abrir otro, el actual se termina o se anota en `docs/ESTADO.md` como aparcado con lo que falta.
2. **Todo en `main`**, en commits pequeños que dejan la app funcionando. Una rama solo para un experimento que pueda romper la app; se fusiona o se borra en esa misma sesión. Nada de ramas que duran días.
3. **Plan solo si hace falta:** cambios de un fichero o evidentes se hacen sin plan. Diseño nuevo o varios módulos: plan en `docs/planes/` antes de tocar código.
4. **Comprobar** lo que se toca: `npm run check` (frontend), `cargo test -p visor-core` (núcleo), `node --test tests/<fichero>.test.ts` (lógica de `src/lib`). No hace falta compilar la app.
5. **Subir:** tras cada cambio, `git add <archivos>`, `git commit`, `git push`, e informar del resultado.

## Reparto de modelos

- **Sonnet (sesión principal):** habla con el usuario, edita, hace commit y push.
- **Opus (subagente `planificador`):** diseño nuevo, cambios de varios módulos, decisiones de arquitectura, un bug tras dos intentos fallidos, o cuando el usuario lo pida. No edita código. Empieza sin el historial: el brief lleva el objetivo, las decisiones ya tomadas y los ficheros o rangos relevantes.
- **Antigravity (`agy`):** solo trabajo masivo o repetitivo (migraciones, tests en cantidad, lecturas largas que acaban en un resumen). Su «todo OK» no vale: quien delega revisa el diff y pasa las pruebas.

## Leer código

- Grep y rangos de líneas; no volcar ficheros enteros (los `.svelte` y `src-tauri/src` son grandes).
- Los subagentes reciben un brief corto con los ficheros o rangos que deben leer o tocar, no «explora el proyecto».

@AGENTS.md
