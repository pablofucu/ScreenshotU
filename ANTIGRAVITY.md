# Guía de Antigravity (Colaborador con Claude)

Este archivo define el rol y los principios de trabajo de **Antigravity** en este proyecto.

## Rol en el proyecto

- **Arquitecto principal**: Claude. Las decisiones arquitectónicas, planes base (`docs/plan-1.4.0.md`, `CLAUDE.md`, etc.) y contratos entre módulos son definidos por Claude.
- **Antigravity**: Ingeniero colaborador para desarrollo de código, implementación de fases específicas, refactorizaciones, optimizaciones y soporte técnico.
- **Flujo de trabajo en Git**: El trabajo de Antigravity se realiza en **ramas separadas** (`feature/<nombre>` o `antigravity/<nombre>`) para no interferir con las ramas activas de Claude, asegurando commits limpios y verificados.

---

## Reglas invariables (alineadas con CLAUDE.md)

1. **Las categorías no se tocan**:
   - Las plataformas/categorías derivadas de carpetas (`Juegos`, `Switch`, `Novelas visuales`, `Novelas japonesas`...) no se renombran, fusionan ni reorganizan.
   - Cualquier novedad funcional (colecciones, estados, «Mi lista», filtros) vive estrictamente **dentro** de cada categoría.
2. **Ergonomía y diseño para TV/Mando**:
   - Canvas base a **1280×720** con escalado uniforme.
   - Navegación pensada primero para **mando de Xbox** (stick, cruceta, A/B/X/Y, LB/RB, LT/RT, View/Menu), con soporte complementario para teclado y ratón.
   - Estética visual coherente: estilo Wii U (fondos vivos, cristales translúcidos, burbujas/iconos fantasma, acento armónico o arcoíris).
3. **Integridad de base de datos (`visor-core/src/db.rs`)**:
   - Las migraciones publicadas son inmutables.
   - Nuevos esquemas requieren una migración nueva numerada correlativa al final con sus pruebas correspondientes.
4. **Finales de línea (LF / CRLF)**:
   - Conservar los finales de línea preexistentes de cada archivo.
5. **Calidad y verificación**:
   - Frontend: `npm run check` limpio.
   - Backend: `cargo test -p visor-core` en verde cuando aplique.
