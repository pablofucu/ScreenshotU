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

## Leer código y subagentes

- Buscar con grep y leer rangos de líneas; no volcar ficheros enteros (los `.svelte` y `src-tauri/src` son grandes).
- Los subagentes reciben un brief corto con la lista de ficheros/rangos que deben tocar o leer, no «explora el proyecto».
