# Overlay: arreglos para la 1.8.2

Tres fallos informados por el usuario tras probar la 1.8.1 con juegos reales. Nada de esto se puede reproducir sin un juego: por eso cada arreglo funciona aunque el diagnóstico exacto no sea el único posible, y el 2 deja un registro.

## 1. El botón Mute de DS4Windows no se puede asignar

DS4Windows solo permite F1–F12 en ese botón. El usuario eligió F6 y al aprender no se registra.

**Causa probable:** `learnKeydown` (`src/lib/learn.svelte.ts`) usa solo `e.code`, y una tecla inyectada por software puede llegar sin código de escaneo (`e.code === ''`). `keyName('')` da `null` y se rechaza. También puede que la webview no llegue a recibirla.

**Arreglo, las tres cosas:**
- `learnKeydown`: si `e.code` está vacío o no se reconoce, usar `e.key` cuando sea `F1`–`F24`. Si `e.code` es `''` no vale ninguna otra tecla.
- Aprender también desde Rust, para no depender de la webview. Comando `learn_key(on: bool)` con un `WH_KEYBOARD_LL` (ver `keyboard` en `src-tauri/src/quickmenu.rs` como modelo, y `pad.rs` para `learn-pad`). Solo mira `WM_KEYDOWN`/`WM_SYSKEYDOWN` de VK `0x70..=0x87` (F1–F24), incluidas las inyectadas, añade Ctrl/Alt/Shift/Super con `GetAsyncKeyState`, emite `learn-key` con el texto (`"F6"`, `"Ctrl+F6"`), y NUNCA se traga la tecla. `startLearn('key', …)` lo enciende y al cerrar el aprendizaje se apaga (mismos sitios que `backend.hotkeySuspend`). El evento entra por `backend.ts`/`types.ts`/`mock.ts` como `learn-pad`, y `controller.svelte.ts` llama a una función nueva de `learn.svelte.ts` que comparte con `learnKeydown` el reparto por `target`. Lo que llegue después de terminar se ignora porque `app.learn` ya es `null`.
- `OVERLAY_HOTKEY_PRESETS`: `['F24', 'F13', 'F6', 'F7', 'F8', 'F5', 'F10', 'Ctrl+Alt+O', '']`, para poder elegirla con ‹ › sin aprender. Actualizar el texto de `qmkey` en `src/screens/Settings.svelte`: con DS4Windows, si el botón solo admite F1–F12, usar una que el juego no use, por ejemplo F6.

## 2. El juego se cierra pero la sesión no

El vigilante (`watch`, `src-tauri/src/launcher.rs:284`) solo termina con 3 lecturas seguidas sin ningún proceso del juego. `force_close_game` (`quickmenu.rs`) mata una lista de pids y confía en eso. Si queda algo vivo en la carpeta del juego (un gestor de errores como `UnityCrashHandler`, un proceso nacido después de la lista, uno que no se deja matar), la partida no termina nunca. Además `destroy` y `perf::stop` van antes de guardar la sesión, con `unwrap` sobre mutex: cualquier bloqueo o mutex envenenado ahí deja la sesión sin guardar.

**Arreglo:**
- En `quickmenu.rs`, `static FORCED: AtomicBool`. `force_close_game` lo pone a `true` antes de matar. Se limpia al empezar partida (`prewarm`) y al salir de `watch`. `pub fn force_requested() -> bool`.
- En el bucle de `watch`, si `force_requested()`: en la primera vuelta apuntar `forced_at`. En cada vuelta matar otra vez todo `found` más sus descendientes (recorriendo `p.parent()` de `sys.processes()`, sin tocar `own`) y `child.kill()` si sigue vivo. Salir con `found` vacío sin esperar a las 3 lecturas, o a los 2 s pase lo que pase. **Un cierre forzado siempre termina la sesión.**
- La duración de la sesión debe restar la espera: usar el momento de `forced_at` como final en lugar de `first_seen.elapsed()` si hubo cierre forzado.
- `quickmenu::destroy` y `perf::stop` pasan a un hilo propio (`std::thread::Builder`), para que nada del menú pueda impedir guardar la sesión. Comprobar que la pausa se deshace igual: `destroy` llama a `close_now`, que hace `resume_all`.
- Los `lock().unwrap()` de `perf.rs` y de los estáticos de `quickmenu.rs` pasan a tolerar el envenenamiento (`unwrap_or_else(|e| e.into_inner())`, con un helper pequeño).
- `proc::terminate` devuelve si funcionó.
- **Registro:** `quickmenu.log` en `config_dir`, se vacía al empezar partida, una línea por suceso con hora: cierre pedido y pids; resultado de cada `terminate`; lo que siga vivo al llegar a los 2 s (pid y exe); salida del bucle y motivo. Si vuelve a pasar, este fichero dirá qué proceso era. Que un fallo al escribirlo no afecte a nada.

## 3. Con Magpie no se abre el menú

`exclusive_fullscreen()` (`quickmenu.rs:194`) usa `SHQueryUserNotificationState == QUNS_RUNNING_D3D_FULL_SCREEN`. La ventana de escalado de Magpie lo activa aunque no sea exclusiva de verdad, y Windows también puede decirlo en juegos sin bordes. Por eso `can_show` es falso y solo sale el aviso.

**Arreglo:** no es exclusiva si Magpie está escalando. `find_magpie()` ya existe en `src-tauri/src/platform.rs` (lo usa `foreground_info`): exponerlo con un `pub fn magpie_scaling() -> bool` y `not(windows)` que devuelva `false`, comprobando que busca la ventana de escalado y no la de ajustes de Magpie. Si hace falta, ajustarlo para que solo cuente la de escalado. `exclusive_fullscreen()` queda como `QUNS… && !magpie_scaling()`. Escribir en `quickmenu.log` el valor de las dos cosas al pulsar el atajo.

Comprobar `keep_on_top`: el menú tiene que quedar por encima de la ventana de Magpie, que también es «siempre encima». `to_top` se llama cada 250 ms y debería bastar. La foto de fondo ya pasa por `platform::grab(&pids, scaled)`, no tocarla.

## Límites

- Las categorías no se tocan.
- Fuera de estos tres arreglos no se cambia nada: ni rediseños ni limpiezas.
- No poner un ajuste nuevo de «forzar menú». Si otros juegos sin bordes dan el mismo falso positivo, se decide con el usuario.

## Comprobaciones

`npm run` no funciona en este equipo (`""node"" no se reconoce`), usar `node` directamente.

```bash
cd src-tauri && cargo check && cargo test -p visor-core
node node_modules/svelte-check/bin/svelte-check --tsconfig ./tsconfig.json
node node_modules/vite/bin/vite.js build
```

Añadir tests donde se pueda sin ventanas: extraer a `visor-core` la parte pura si sale natural, por ejemplo el texto del atajo a partir de VK y modificadores en `capture.rs`, con su test.

## Publicar

1. Finales de línea: el repo los tiene mezclados y hay que conservar los de cada fichero. Comparar con bytes contra HEAD: número de `\r\n` y de `\n` antes y después. `grep -c $'\r'` no es fiable aquí, usar Python. No añadir `.claude-scratch/`.
2. Versión 1.8.2 en `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` y la entrada `visor-capturas` de `src-tauri/Cargo.lock` (la actualiza `cargo check`). `package-lock.json` no se toca.
3. Commit en `main`, en español, terminando con la línea `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; `git push`; `git tag v1.8.2`; `git push origin refs/tags/v1.8.2`.
4. El tag lanza `build-windows.yml`. Esperar con `gh run watch <id> --exit-status` y comprobar con `gh release view v1.8.2` que están el `.exe`, el `.msi`, sus `.sig` y `latest.json`.
