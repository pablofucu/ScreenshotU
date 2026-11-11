# Visor de capturas (Tauri 2 + Svelte)

Las reglas del proyecto están en `CLAUDE.md`: léelo antes de empezar. Si trabajas por delegación (Antigravity u otro agente):

- **Las categorías no se tocan.** Juegos, Novelas visuales, Switch, Novelas japonesas y las demás plataformas que salen de las carpetas no se renombran, no se fusionan, no se reorganizan ni se cambia cómo se detectan.
- **Cíñete al encargo.** No toques ficheros fuera de lo pedido.
- **No hagas commit ni push, ni crees ramas.** Eso lo hace quien te delegó, tras revisar el diff.
- **No alteres el entorno para que pasen las pruebas.** Nada de parchear dependencias ni simular módulos. Si algo falla, dilo.
- **Migraciones inmutables.** En `visor-core/src/db.rs` solo se añade una migración nueva al final, con su prueba.
- **Diseño:** `docs/guia-diseno.md` (lienzo 1280×720, mando de Xbox primero, tokens en vez de colores fijos, sin emojis).
- **Finales de línea:** el repo mezcla LF y CRLF. Conserva los que tenga cada fichero.
- **Comprobación:** `npm run check` limpio y `cargo test -p visor-core` en verde cuando toques el núcleo.
