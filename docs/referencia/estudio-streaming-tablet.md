# Streaming a la tablet: estado y estudio de viabilidad

Informe para retomar el trabajo en otra sesión. Fecha: 2026-11-01.

## Idea
App de tablet (la app móvil de `mobile/`, Capacitor + Svelte) para novelas visuales:
1. Pulsas «Jugar en el PC» y el juego arranca en el PC con el lanzador del visor (cuenta el tiempo, Magpie, etc.).
2. La tablet salta directa a la pantalla de streaming de Moonlight, sin menús.
3. Se lee tocando la pantalla. Volumen abajo = captura en el PC. Sin botones fijos en pantalla.
4. Al cerrar el juego se vuelve a la app, con el resumen de lo leído.

En el móvil (pantalla < 600 dp) nada de esto aparece: solo ver capturas.

## Lo que ya está hecho (probado por el usuario: «funciona guay»)
- Cliente de PC **1.7.7** publicado (tag `v1.7.7`). Servidor remoto (`visor-core/src/remote.rs`, puerto 47811, Bearer con código) con `POST /api/v1/launch` y `POST /api/v1/capture`; `/hello` lista `features`.
- APK de prueba (commits `e5e1be1` y `b4bee82`):
  - Plugin `StreamingPlugin.java`: abre Moonlight con `ShortcutTrampoline`.
  - Servicio de accesibilidad `CaptureKeyService.java`: volumen abajo = captura, solo con Moonlight en primer plano; una vibración = bien, dos = error.
  - Botón «Jugar en el PC» (`Detail.svelte`) y tarjeta «Streaming» en Ajustes (`Settings.svelte`), solo en tablet (`smallestScreenWidthDp >= 600`).
- Donde se activa en el escritorio: Ajustes → sección «Mando» → «Acceso desde el móvil» (interruptor, dirección y código).

### Cómo se llega a Moonlight (verificado en el código de `moonlight-android`)
- `com.limelight.ShortcutTrampoline` está exportado. Extras: `UUID` o `Name` (PC) y `AppName` (aplicación de Sunshine). Salta directo al streaming.
- `getComputerByName` compara el nombre **con `=` de SQL, distinguiendo mayúsculas**. Fue el fallo «PC no encontrado»: el visor daba `PC-COMPUTER` (COMPUTERNAME) y Sunshine anuncia `PC-Computer`.
- Arreglo aplicado: la APK pide a Sunshine `http://<ip>:47989/serverinfo`, saca `hostname` y `uniqueid` y pasa el **UUID** (cae al nombre escrito en Ajustes si Sunshine no responde). Vale solo con el puerto por defecto.
- Moonlight: el táctil nativo está desactivado en el código; con «pantalla táctil como trackpad» apagado, tocar = clic en ese punto, pulsación larga (650 ms) = clic derecho, arrastrar = rueda.
- Licencia de Moonlight y Artemis: GPLv3 (solo importa si se distribuye una app con su código dentro).

## Problemas detectados en la prueba
1. **Resolución/calidad** no óptima (Moonlight sin tocar; tablet 2K; juegos en 1080p).
2. **Magpie**: los toques no coinciden con lo que se ve. Hipótesis: (a) desajuste de proporción entre el vídeo pedido (tablet) y el monitor del PC; (b) Magpie traduce mal el ratón inyectado por Sunshine. Sin confirmar.
3. **El cursor del PC se ve** mientras se toca.
4. **Al cerrar el juego** se queda en el menú de Moonlight, no en la app. Además el streaming de «Desktop» no termina.
5. Faltan: **cerrar el juego desde la tablet** y el **resumen/animación de estadísticas** al terminar.

## Hallazgos de la investigación

### Resolución
- Sunshine captura el monitor a su resolución y la reescala a la pedida. Si el monitor es 1080p, pedir 2K solo estira.
- Prueba rápida sin código: en Moonlight, resolución nativa de la tablet, bitrate 50–100 Mbps, HEVC/AV1 si hay, 60 fps (la APK no puede cambiar esos ajustes).
- Sunshine pasa `SUNSHINE_CLIENT_WIDTH`, `SUNSHINE_CLIENT_HEIGHT`, `SUNSHINE_CLIENT_FPS` a sus comandos (documentado, con ejemplos de Do/Undo para cambiar y restaurar resolución). El visor ya gestiona monitores (`display.rs`).

### Apollo (servidor) y Artemis (cliente Android)
- **Apollo** (`ClassicOldSong/Apollo`, GPL-3, ~11 mil estrellas): fork de Sunshine con **pantalla virtual (SudoVDA)** que copia resolución y hercios del cliente, creada al empezar y quitada al terminar. Modo sin pantalla para GPU dual, permisos por cliente, comandos al conectar/desconectar, sincronía de portapapeles. Solo Windows para la pantalla virtual. Última versión estable v0.4.6 (jul 2025); alfa v0.4.7 (ago 2025); el repositorio recibió cambios hasta may 2026. Avisan de que «eventualmente» dejará de ser compatible con Sunshine/Moonlight.
- **Artemis** (`ClassicOldSong/moonlight-android`, antes «Moonlight Noir»): cliente con resoluciones y bitrates a medida, comandos de servidor, menú al pulsar atrás, salir de la app. Su `ShortcutTrampoline` también está exportado. Última versión estable v20.2.6 (ago 2025); experimental v20.3.0 (ago 2025); cambios hasta sep 2026. **El nombre de paquete no está claro** (`com.limelight` en un flavor, `com.limelight.noir` en el enlace de Obtainium): habría que añadirlo a `StreamingPlugin.MOONLIGHT` y a `CaptureKeyService`.
- **No verificado:** si Apollo/Artemis ocultan el cursor, si Magpie funciona con su pantalla virtual, y cómo cierran la sesión.

### Orden de arranque (importa)
Hoy el visor lanza el juego **antes** de que la tablet conecte. Con pantalla virtual o cambio de resolución al conectar, el monitor correcto aún no existe. Orden propuesto:
1. La tablet pide jugar; el visor apunta el juego como «pendiente».
2. La tablet abre Moonlight/Artemis en una aplicación de Sunshine/Apollo («Juego actual»).
3. Al conectar, Sunshine/Apollo ejecuta un **ayudante** (el propio visor con un argumento, p. ej. `--stream-session`). Avisa al visor de la resolución, el visor ajusta la pantalla y lanza el juego pendiente.
4. El ayudante espera mientras el juego vive y termina cuando se cierra; así Sunshine/Apollo cierra la sesión sola.
Funciona igual con Sunshine y con Apollo: diseñarlo agnóstico.

## Viabilidad por punto

| Cosa | Viable | Cómo | Riesgo |
|---|---|---|---|
| Resolución y toques alineados | Sí | Pantalla virtual (Apollo) o monitor a la resolución de la tablet (Sunshine) | Bajo. Cambiar la resolución real mueve ventanas |
| Ocultar el cursor | Sí | Cursores transparentes de Windows durante la sesión; restaurar al terminar y al arrancar el visor | Medio. Si el visor cae, el cursor queda invisible hasta restaurar |
| Cerrar el juego desde la tablet | Sí | Ruta nueva `POST /api/v1/close` que cierra las ventanas del juego y, si no responde, lo fuerza (el visor ya conoce los PIDs) | Bajo |
| Menú con volumen arriba | Sí | Servicio de accesibilidad con ventana superpuesta (`TYPE_ACCESSIBILITY_OVERLAY`, sin permiso extra) que desaparece sola; volumen abajo sigue siendo la captura | Medio. No debe robar toques del juego |
| Volver a la app al acabar | Sí | El servicio consulta una ruta ligera nueva del PC; al ver que no hay juego, abre la app | Medio. Android restringe abrir actividades desde segundo plano |
| Resumen/animación de estadísticas | Sí | El PC añade la última sesión (`game-exited`: id, segundos) a lo que envía; la app móvil recibe un componente nuevo | Bajo, pero hay que reescribir la animación del escritorio |

## Sin saber todavía
- Si Android deja abrir la app desde el servicio de accesibilidad (alternativa: notificación).
- Cuál de las dos causas de los toques desajustados es la real (la prueba de más abajo lo separa).
- Si Apollo oculta el cursor de serie; si el cursor transparente oculta también el que dibuja Magpie.
- Qué animación de escritorio quiere el usuario copiar en la tablet (necesito verla).

## Próximos pasos
1. **El usuario prueba Apollo** en el PC (desinstalar Sunshine antes) con el Moonlight que ya tiene: resolución y toques, con y sin Magpie. Con la prueba de resolución por separado si hace falta.
2. Decidir Apollo o Sunshine con esos resultados.
3. Construir, por este orden:
   - **PC (nueva versión del cliente):** estado «juego pendiente», ayudante `--stream-session`, `POST /api/v1/close`, ruta ligera de estado, última sesión en la instantánea, cursor transparente.
   - **APK:** menú de volumen arriba, volver a la app, resumen de la sesión, soporte del paquete de Artemis.
4. Mantener en la nueva APK el cambio para que el móvil siga igual.

## Fuera de alcance por ahora
- Diseño de la interfaz de tablet (más columnas, horizontal). Se hace después de probar lo anterior.
- Las categorías de la biblioteca no se tocan (ver `CLAUDE.md`).

## Notas
- Preferencia del usuario: se puede cambiar el cliente de PC sin problema; la APK primero era solo comodidad.
- Sin botón flotante permanente: lo único aceptable es un menú que aparece con una tecla de volumen y se esconde solo.
- Finales de línea: el repo mezcla LF y CRLF; comprobar los CR antes de commitear (ver memoria `line-endings-mixed`).
