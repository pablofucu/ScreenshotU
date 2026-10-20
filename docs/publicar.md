# Publicar una versión

Las actualizaciones automáticas leen `latest.json` de la última release de GitHub y comprueban la firma.

## Una sola vez
1. En `src-tauri/tauri.conf.json`, cambia `pablofucu/ScreenshotU` en `plugins.updater.endpoints` por tu repositorio.
2. La clave de firma está en `%USERPROFILE%\.tauri\visor-capturas.key` (sin contraseña). **Haz una copia en un sitio seguro**: si se pierde, las versiones ya instaladas no podrán actualizarse. La pública ya está en `tauri.conf.json`.
3. En GitHub › Settings › Secrets › Actions crea `TAURI_SIGNING_PRIVATE_KEY` (el contenido del archivo `.key`) y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (vacío).

## Cada versión
1. Sube el número en `package.json`, `src-tauri/Cargo.toml` y `src-tauri/tauri.conf.json` (el mismo en los tres).
2. Commit, `git tag vX.Y.Z`, `git push --tags`.
3. El workflow «Compilar para Windows» corre las pruebas, compila, firma y crea la release con el instalador y `latest.json`.

Compilar el instalador en local (`tauri build` sin `--no-bundle`) necesita `TAURI_SIGNING_PRIVATE_KEY_PATH` apuntando a la clave.

## Datos del usuario
Nunca se edita una migración ya publicada en `visor-core/src/db.rs`: se añade una nueva al final. Antes de migrar una base existente se guarda `visor.db.antes-vN.bak` junto a ella.
