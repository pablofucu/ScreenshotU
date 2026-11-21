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

<!-- unificador:inicio (lo mantiene Unificador; los cambios aquí se pierden) -->
## El plan del proyecto (Unificador)

La persona sigue este proyecto en Unificador, que enseña el plan como **ramas → fases → pasos**.
El plan vive en `unificador.json` y se toca solo con `python .unificador/uni.py` (nunca edites el JSON a mano).
Unificador es un apoyo: la persona te habla en tu chat y puede contestar también en la web.
Cuando preguntes algo, pregúntalo SIEMPRE en el chat (y apúntalo con `ask` si quieres que se vea en la web).

- **Al empezar**, mira cómo va: `python .unificador/uni.py status`. Si un paso dice «PIDE CAMBIOS», atiéndelo lo primero.
- **Al empezar tu turno**, si no te ha llegado ya un aviso automático, ejecuta `python .unificador/uni.py inbox`:
  imprime lo que la persona dejó en la web (y nada si no hay nada). Hazle caso.
- **Trabajo nuevo** (algo más que una pregunta o un retoque): apúntalo ANTES de empezar, como una rama,
  con esta única orden (vale en PowerShell, cmd y bash; cada cosa entre comillas dobles):

  ```
  python .unificador/uni.py branch "Carrito de compra" --icon 🛒 --goal "Poder comprar desde la web" --phase "Cimientos" "codigo: Modelo del carrito" "codigo: Guardar y recuperar" "+pruebas: Probar el guardado" --phase "Pagos" "decidir: Elegir pasarela" "codigo: Pasarela de pago"
  ```

  - Una **rama** es una función o un conjunto de cosas que hacer. Detrás de cada `--phase` va el nombre
    de la fase y después sus pasos.
  - Cada **paso** empieza por su categoría: `codigo:`, `interfaz:`, `pruebas:`, `investigar:` o `decidir:`.
    Un `+` delante (`"+pruebas: …"`) = se hace a la vez que el anterior. Sin categoría se rechaza.
    Cambiar la de un paso: `python .unificador/uni.py cat ID pruebas`.
  - Si una fase no puede empezar hasta que acabe otra (de esta u otra rama):
    `python .unificador/uni.py after RAMA "Fase" OTRA "Fase de la otra"`.
  - Entre 2 y 4 **fases** por rama, en orden, con un nombre que se reconozca («Cimientos», «Pagos», «Pulido»).
    «Fase 1» o «Etapa 2» se rechazan. **Pasos** pequeños y comprobables, con título corto en lenguaje llano.
  - No pidas permiso ni confirmación para apuntar el plan: hazlo y sigue. La persona te corregirá si algo no encaja.
  - Para añadir un paso suelto a una rama que ya existe: `python .unificador/uni.py add RAMA "Fase" "codigo: Título"`.
- **En cada paso**: `python .unificador/uni.py start ID` → trabaja → `python .unificador/uni.py done ID -m "qué ha cambiado y cómo lo has comprobado"`.
  Cada orden te dice cuál es la siguiente. Termina las fases en orden; si descubres algo que la persona
  deba saber, `python .unificador/uni.py note ID "…"`.
- **Al cerrar una fase**: pasa `python .unificador/uni.py check` y arregla sus errores. Cuenta en una línea qué se ha hecho y sigue
  con la siguiente fase SIN preguntar: la revisión es opcional y la persona la hace en la web si quiere.
  Para y pregunta solo si te pidió revisar antes de seguir. Si da una fase por buena: `python .unificador/uni.py approve RAMA`;
  si pide cambios: `python .unificador/uni.py revise RAMA -m "qué quiere cambiar"` y atiéndelos lo primero.
- **Dudas con opciones**: ficha fija: contexto corto en `-d` (Markdown ligero, máx. 800), cada opción
  `"Título | qué implica"` y la que recomiendas con `--recommend N` (0 = sin preferencia):
  `python .unificador/uni.py ask "¿Qué color?" --step ID -d "Por qué importa" --option "Azul | más sobrio" --option "Verde | más contraste" --recommend 1`.
- **Varias propuestas de diseño o estilos**: haz un HTML por propuesta. Cada uno AUTOCONTENIDO
  (CSS y JS dentro del fichero, sin recursos de internet, imágenes como `data:` o SVG en línea),
  que se vea bien a cualquier ancho (móvil incluido), con contenido realista del proyecto,
  y que solo sea distinto lo que cambia entre propuestas. Después:
  `python .unificador/uni.py propose "Título" --step ID --option "Clara | fondo blanco" a.html --option "Oscura | fondo negro" b.html --recommend 1`
  (un fragmento sin `<html>` se envuelve solo; si carga algo de internet se rechaza con la línea),
  enseña las rutas en el chat y pregunta cuál prefiere.
  Si contesta en el chat: `python .unificador/uni.py answer P-1 N -m "lo que dijo"`. Para esperar un rato a la web: `python .unificador/uni.py wait P-1`.
- Firma con `--as <tu nombre>` (por ejemplo `--as claude`) salvo que `UNI_AGENT` ya esté puesto.
- **Commits**: pon en el mensaje los IDs de los pasos que cierra (`… (CDC-1, CDC-2)`), así quedan enlazados.
- **Retoques** (un cambio rápido que no merece rama): hazlo y, al comitearlo, apúntalo en una línea:
  `python .unificador/uni.py tweak "Qué ha cambiado, en llano" --commit HEAD`. Si `status` dice «Commits sin apuntar»,
  `python .unificador/uni.py commits` te dice cuáles y cómo apuntarlos.
- **Versiones** (SemVer): cada fase cerrada se apunta sola como novedad; apunta las demás con
  `python .unificador/uni.py news "Qué trae, en llano" --kind nuevo|arreglo|cambio`. `python .unificador/uni.py news` enseña la próxima versión y lo
  comiteado sin publicar. Publica solo cuando la persona lo pida, y entonces hazlo sin más preguntas: `python .unificador/uni.py release` (crea el tag `vX.Y.Z`).
- **Ideas** que salen y no tocan ahora: `python .unificador/uni.py idea "Qué se te ocurre"` (van a la caja; las de la persona
  te llegan por `inbox`). Para convertirlas en trabajo: `python .unificador/uni.py branch … --ideas I-1,I-3` o
  `python .unificador/uni.py ideas plan I-2 --branch RAMA`; si no van a hacerse, `python .unificador/uni.py ideas drop I-4 -m "por qué"`.
- **Encargos en cola** (te llegan por `inbox`, a menudo un prompt maestro): hazlos cuando acabes lo que
  tengas entre manos y ciérralos con `python .unificador/uni.py queue done C-1 -m "qué has hecho"`. Ver la cola: `python .unificador/uni.py queue`.
- Preguntas sueltas: contesta sin tocar el plan.
<!-- unificador:fin -->
