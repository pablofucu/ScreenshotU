# Visor de capturas (Tauri + Svelte)

## Compilación

Tras cada cambio en el código (frontend `src/` o backend `src-tauri/`), compila el proyecto antes de dar la tarea por terminada:

```
npm run tauri build -- --no-bundle
```

Si la compilación falla, corrige los errores y vuelve a compilar. Informa del resultado (éxito o errores) al usuario.
