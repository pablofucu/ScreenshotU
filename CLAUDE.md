# Visor de capturas (Tauri + Svelte)

## Las categorías no se tocan

Las categorías de la biblioteca (Juegos, Novelas visuales, Switch, Novelas japonesas… las plataformas que salen de las carpetas) se quedan **exactamente como están**. No se renombran, no se fusionan, no se reorganizan ni se cambia cómo se detectan. Todo lo nuevo (colecciones, estados, «Mi lista», órdenes, vistas) vive **dentro** de cada categoría y nunca mezcla títulos de categorías distintas.

## Subir los cambios

Tras cada cambio en el código (frontend `src/` o backend `src-tauri/`), no hace falta compilar: haz commit y súbelo a GitHub antes de dar la tarea por terminada:

```
git add <archivos cambiados>
git commit -m "<mensaje>"
git push
```

Informa del resultado (éxito o errores) al usuario.

## Reparto de modelos

- **Sonnet (sesión principal):** habla con el usuario, edita, hace commit y push. No se usa Opus como sesión principal.
- **Opus (subagente `planificador`):** se le llama para diseño nuevo, cambios que tocan varios módulos, decisiones de arquitectura o un bug tras dos intentos fallidos, y también cuando el usuario lo pida. No edita código. Empieza sin el historial del chat: el brief lleva el objetivo, las decisiones ya tomadas con el usuario y los ficheros o rangos relevantes. Si los pasos del plan no caben en la respuesta, van a `docs/plan-<tema>.md`.
- **Antigravity (`agy`, subagente `antigravity:antigravity-delegate`):** solo trabajo masivo o repetitivo: migraciones, tests en cantidad, lecturas largas que acaban en un resumen. Las tareas pequeñas no se delegan (sale más caro). Su «todo OK» no vale: quien delega comprueba el diff y pasa las pruebas.

## Leer código y subagentes

- Buscar con grep y leer rangos de líneas; no volcar ficheros enteros (los `.svelte` y `src-tauri/src` son grandes).
- Los subagentes reciben un brief corto con la lista de ficheros/rangos que deben tocar o leer, no «explora el proyecto».
