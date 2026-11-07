# Switch: tiempo de juego — plan por fases

Ordena las tareas SW del tablero Kanvas (detalle de diseño en `plan-switch-playtime.md`). Las revisiones se acuerdan en el chat, no en el tablero.

**Fuera de alcance por ahora:** SW-19 (Control Parental / Moon) y SW-23 (importar capturas al conectar la Switch). SW-02 (investigar Moon) también se omite; SW-03 solo necesita SW-01.

**Regla de categorías:** todo vive dentro de la categoría Switch; no se tocan ni se detectan distinto las demás.

## Fase 0 — Cerrar lo hecho
- Revisar y dar el OK a **SW-08** (store `switch_*`) y **SW-14** (autenticación).

## Fase 1 — Login
- **SW-04** Probar la ventana de login en WebView2 (rama aparte, no se fusiona): `on_navigation` ve `npf…://auth`.
- **SW-15** `login.rs`: captura de la redirección, pegar enlace como respaldo, estado conectado/caducado.
- Hecho cuando: se puede iniciar sesión con la cuenta real y el estado se lee por comando.

## Fase 2 — Datos de Nintendo
- **SW-03** Fixtures anonimizadas en `visor-core/src/nintendo/fixtures/`.
- **SW-07** Modelos serde (`models.rs`) con tests de fixtures.
- **SW-16** `history.rs`: descarga y guarda totales, primera y última vez.
- Hecho cuando: el historial real se descarga y queda en las tablas `switch_*`.

## Fase 3 — Lógica de tiempos (sin red, con tests)
- **SW-09** `merge.rs`: emulador + consola se suman; entre APIs de Nintendo el máximo por día; DST.
- **SW-10** `link.rs`: emparejar título de Nintendo con ficha o carpeta de la categoría Switch, sin cambiar ids.
- **SW-11** Aplicar `Title.console` en el escaneo (`scan.rs`, `src-tauri/src/lib.rs`, `examples/serve.rs`).
- SW-09 y SW-10 son independientes de las fases 1-2 y pueden hacerse antes o en paralelo.

## Fase 4 — Importación y sincronización (backend)
- **SW-17** Vista previa e importación: nunca demos; mínimo por defecto 60 min; importación explícita; enlace manual.
- **SW-18** `sync.rs`: cada 30 min, evento `nintendo-synced`; solo actualiza tiempos, nunca crea fichas.

## Fase 5 — Interfaz
- **SW-12** Tipos y mock (`src/lib/types.ts`, `mock.ts`).
- **SW-20** Sección «Nintendo» de Ajustes: conectar, estado, mínimo de tiempo, importar ahora.
- **SW-21** Pantalla de importación: vista previa de importados y excluidos; añadir a mano.
- **SW-22** Ficha del juego: desglose consola / PC, panel de semana; se actualiza tras `nintendo-synced`.

## Fase 6 — Publicar
- Subir versión (propuesta **1.9.0**) en `package.json`, `src-tauri/Cargo.toml` y `src-tauri/tauri.conf.json`.
- Commit, `git tag v1.9.0`, `git push --tags`; el workflow compila, firma y crea la release (ver `publicar.md`).
- Antes: probar la migración 15 sobre una copia de la base real (hay copia `visor.db.antes-vN.bak`).

## Cortes posibles
- Tras la fase 4 el backend está completo pero sin interfaz: no publicar aquí.
- Tras la fase 5 ya es publicable.
