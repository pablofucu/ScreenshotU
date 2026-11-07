# Plan: Magpie para la tablet y arreglo del táctil

Fecha: 2026-11-05. Sustituye al punto 7 del bloque 2 de `docs/plan-tablet.md`.

## Estado

| Paso | Estado |
|---|---|
| A1 y A2, perfil de tablet | Hecho (`0d214c1`). Sin probar en la tablet |
| Elegir la copia de Magpie con la versión más alta | Hecho (`5357e0b`) |
| B0, soporte táctil de Magpie | **Probado en multi-touch: cada toque cuenta doble**, uno bien y otro cerca (ver sección 3) |
| B1, prototipo del gancho | **Código hecho** (`stream_touch.rs`), falta la prueba con la tablet (sección 3) |
| A3, A4, A5, B2, B3 | Pendientes |

Tablet: POCO Pad M1, 2560×1600 (16:10), 120 Hz. Novelas a 720p y 1080p. La GPU va sobrada.

## 1. Cómo funciona hoy

El visor ya recalcula el perfil según el monitor donde se abre el juego: `launcher.rs:149-151` pasa su tamaño a
`magpie::on_launch`, que lo guarda y vuelve a generar el perfil. La suposición es correcta en lo esencial, con estos
límites:

| Límite | Dónde | Efecto |
|---|---|---|
| Solo hay un modo por juego (`off`, `auto`, `2x`, `4x`) | `visor-core/src/magpie.rs:19` | No se puede tener un ajuste en el PC y otro en la tablet |
| El modelo depende solo de la altura del juego | `cunny_for`, `visor-core/src/magpie.rs:47` | 1080p usa CuNNy 4x16 «para que vaya fluido», aunque sobre GPU |
| La nitidez es fija (RCAS 0,45) | `rcas`, `visor-core/src/magpie.rs:64` | En streaming, enfocar puede resaltar artefactos del vídeo. Sin probar |
| Al reconectar con el juego abierto no se recalcula | `session_start`, `stream.rs:330-340` | Si la partida empezó en el PC, puede seguir con el perfil del PC. Por comprobar |

Resultado actual en la tablet:

| Juego | Escalado | Salida | Valoración |
|---|---|---|---|
| 720p | 2x CuNNy 8x32 + RCAS | 2560×1440, escala exacta | Ya es lo mejor; no hay nada que cambiar en el modelo |
| 1080p | 2x CuNNy 4x16, Lanczos hasta 2560×1440, RCAS | 2560×1440 | Se puede subir a 8x32 |

Las bandas de 80 px arriba y abajo son normales: la pantalla es 16:10 y el juego 16:9.

## 2. Parte A: perfil propio para la tablet

**A1. Saber el destino.** `start_at` ya sabe si es streaming (`launcher.rs:104`). Pasar a `on_launch` un destino
`Pc` o `Tablet` en vez de solo el tamaño.

**A2. Ajustes separados en `visor-core/src/magpie.rs`.** Añadir a `choose` un parámetro con el destino:

| | PC | Tablet |
|---|---|---|
| Modelo en 1080p | 4x16 (como ahora) | 8x32 |
| Modelo en 720p | 8x32 | 8x32 |
| Nitidez | 0,45 (como ahora) | La que salga de A5 |
| Nombre del modo | `Visor · 2x · …` (como ahora) | `Visor · Tablet · 2x · …` |

El nombre distinto evita que un destino sobrescriba los efectos del otro, porque `ensure_mode` los corrige por nombre.
El comportamiento en el PC no cambia. Hay que ampliar las pruebas de `choose` con los dos destinos.

**A3. Modo por juego y destino.** Añadir un modo de tablet junto al actual, por defecto `auto`, y su selector en
Ajustes del juego. Si en la práctica nadie necesita diferenciarlo por juego, se puede dejar en un ajuste global.

**A4. Reconexión.** Al entrar en `session_start` con la partida ya abierta, comprobar si el juego está en la pantalla
virtual y, si lo está, regenerar el perfil para la tablet. Primero hay que comprobar si Apollo mueve el juego a esa
pantalla al conectar.

**A5. Ajustes de la cadena de streaming (sin código, los haces tú).**
- Artemis: resolución personalizada 2560×1600, 60 fps, vídeo en «Fit», 60–80 Mbps de partida.
- Códec: HEVC. Probar AV1 y quedarse con el que se vea mejor; no he podido confirmar el soporte de AV1 de la tablet.
- Color 4:4:4: usarlo solo si Artemis para Android lo ofrece; no está confirmado.
- Comparar RCAS a 0,45, 0,25 y sin RCAS en una escena con texto de color y degradados.

**Hecho cuando:** una novela a 1080p abierta desde la tablet usa `Visor · Tablet · 2x · CuNNy 8x32`, la misma novela en
el PC sigue con su perfil y ninguno pisa al otro.

**Hallazgos de la prueba real (2026-11-05):**
- La entrada de Tsukihime que se lanza (`manual:18df79a09f37fe90`) no tiene fila en `title_magpie`, así que el visor no
  le aplica el perfil de tablet. Hay que activar Magpie en los ajustes de ese juego.
- La fila con esa ruta es de otro identificador (`manual:18e22d0a0eff7978`) y está huérfana.
- Tsukihime va a 2275×1280, y la regla de A2 solo sube a 8x32 hasta 1100 px de alto: en la tablet seguiría con
  `CuNNy fast`. Queda decidir si subir el límite (punto 4 de la sección 5).

## 3. Parte B: desvío del táctil

### La causa

Confirmada en `Magpie.Core/CursorManager.cpp`. Mientras escala, Magpie espera el cursor real dentro de la ventana del
juego (`srcRect`) y lo convierte a la imagen grande (`destRect`) con `SrcToScaling`. Pero Apollo manda coordenadas de la
pantalla completa, sin esa conversión.

Ejemplo con un juego 1280×720 centrado en 2560×1600:
- `srcRect` va de (640, 440) a (1920, 1160) y `destRect` de (0, 80) a (2560, 1520).
- Un toque en (1000, 800) cae dentro de `srcRect`, en su mismo punto relativo: Magpie lo pinta en (720, 800), 280 px a la izquierda.
- Solo el centro de la pantalla coincide, y el desvío crece al alejarse. Fuera de `srcRect` el cursor se sale de la
  zona del juego. Por eso «acierta en unas zonas y se desvía en otras».

### Novedades respecto al plan anterior

1. **Magpie sí expone los rectángulos.** El plan decía lo contrario. Su ventana de escalado tiene las propiedades
   `Magpie.SrcLeft/Top/Right/Bottom` y `Magpie.DestLeft/Top/Right/Bottom`, y avisa con el mensaje `MagpieScalingChanged`.
   Requiere proceso con DPI Per-Monitor V2.
2. **Magpie tiene soporte táctil propio.** `TouchHelper` usa `MagSetInputTransform`, la conversión de Windows para
   toques, que puede evitar escribir el gancho. Esa función no cubre el ratón, así que solo sirve si Artemis manda toques reales.

### B0. Prueba sin código (empezar aquí)

Es la vía más barata y se prueba en unos 15 minutos.

1. En Magpie, activar el soporte táctil. Pide permisos de administrador e instala un **certificado autofirmado en las
   raíces de confianza del equipo** y `TouchHelper.exe` en `System32\Magpie`. Desactivar la opción lo revierte. La propia
   documentación de Magpie aconseja desactivarla si no se usa.
2. En Artemis, elegir el modo de ratón «multi-touch».
3. Abrir una novela con Magpie y tocar las cuatro esquinas de la imagen, el centro y un botón pequeño.
4. Comprobar también: que un toque cuenta como clic, que arrastrar funciona y que deslizar desde el borde no abre paneles de Windows.

| Resultado | Siguiente paso |
|---|---|
| Aciertan todos los toques | Arreglado. Solo queda explicarlo en Ajustes (B3) |
| Sigue igual, o Artemis no manda toques | B1 |
| Funciona, pero no quieres el certificado | B1 |

No sé si la conversión de Windows afecta a los toques que inyecta Apollo; por eso va primero una prueba y no el código.

### Resultado de B0 (2026-11-05)

**La primera prueba no vale:** Artemis no estaba en multi-touch. Se repitió con multi-touch y soporte táctil activados.

**Resultado con multi-touch:** cada toque produce dos pulsaciones, una en el sitio correcto y otra cerca, en un punto
equivocado. No sirve para jugar, pero es mejor que antes: una de las dos acierta, así que lo que falta podría ser
eliminar la sobrante en vez de convertir coordenadas.

**Hipótesis, sin confirmar:** la correcta es el toque corregido por `TouchHelper`. La otra es un evento de ratón con las
coordenadas sin corregir, que Magpie desplaza como se describe arriba. Encaja con que caiga cerca: en Tsukihime el desvío
calculado es de unas decenas de píxeles. No se sabe si ese ratón lo genera Windows a partir del toque o lo manda Apollo.

Datos reales de la primera sesión, para las pruebas (Tsukihime en la pantalla virtual de 2560×1600):
- `srcRect` va de (142, 171) a (2417, 1451), 2275×1280. `destRect` va de (0, 80) a (2560, 1520), 2560×1440.
- Un toque en (1000, 800) debería llevar el cursor real a unos (1031, 811). Hoy Magpie lo toma como posición dentro de
  `srcRect` y lo pinta en unos (966, 788): 34 px a la izquierda y 12 px arriba.
- Un toque en (2500, 1500) cae fuera de `srcRect`.

### B1. Prototipo del gancho (siguiente fase)

**Objetivo:** saber qué recibe Windows en cada modo de Artemis, para decidir si B2 es viable. No corrige nada: solo
observa y escribe un registro, sin bloquear ningún evento.

**Duda principal.** De memoria, sin confirmar en el código de Sunshine: los toques se inyectan con
`InjectSyntheticPointerInput` y el ratón con `SendInput`. Si es así, `WH_MOUSE_LL` no vería los toques como tales, solo el
ratón que Windows genera a partir de ellos, con la firma `dwExtraInfo & 0xFFFFFF00 == 0xFF515700`. B2 depende de esto.

**Foco tras la prueba con multi-touch:** identificar la segunda pulsación. Con el soporte táctil activado, B2 podría
quedar en bloquear ese ratón sobrante, sin convertir coordenadas, y sería bastante más pequeño. Hay que hacer la pasada de
multi-touch también con el soporte táctil desactivado, para comparar.

**Código**

1. Módulo nuevo `src-tauri/src/stream_touch.rs`, con su propio `WH_MOUSE_LL`. El gancho de `mouse.rs` no se toca.
2. Solo se activa si existe el archivo `stream_touch_diag` en `%APPDATA%\app.visorcapturas.desktop`. Sin ese archivo, el
   módulo no hace nada. No hay opción en Ajustes.
3. Se instala al empezar la partida por streaming y se quita al acabar. El inicio va junto a `launch_now`
   (`stream.rs:289`) y a la rama de reconexión de `session_start` (`stream.rs:330-340`). El final, donde la sesión vuelve
   a `Phase::Idle`: `game-exited` (`stream.rs:99`) y `reset_idle` (`stream.rs:279`).
4. El gancho solo copia los datos a un canal y vuelve; otro hilo escribe en `stream_touch.log`. Si tardara, Windows lo
   eliminaría.
5. Por cada evento se registra: hora, mensaje, posición, `flags` (`LLMHF_INJECTED`), `dwExtraInfo`, la posición real con
   `GetCursorPos` y si Magpie está escalando.
6. Al arrancar y al recibir `MagpieScalingChanged` se registran `Magpie.SrcLeft/Top/Right/Bottom` y
   `Magpie.DestLeft/Top/Right/Bottom`. El nombre de la clase de la ventana (de memoria,
   `Window_Magpie_967EB565-6F73-4E94-AE53-00CC42592A22`) hay que confirmarlo en la documentación de Magpie.
7. Una llamada de prueba a `SetCursorPos` desde el hilo auxiliar, con marca en el registro, para ver cómo aparece en el
   gancho un movimiento propio.

Tamaño: pequeño, unas 150 líneas. Hay que instalar una versión nueva del visor para probarlo.

**Prueba (la hace el usuario, unos 10 minutos)**

Con Tsukihime escalado y en cada modo de ratón de Artemis (multi-touch, trackpad y ratón directo, si lo ofrece):
1. Tocar por este orden: centro, esquina superior izquierda de la imagen, esquina inferior derecha y un botón pequeño.
2. Mantener pulsado y arrastrar una vez.
3. Mover el ratón físico del PC, para tener eventos de referencia.
4. Anotar en cada modo si el toque responde y si cae donde debe.

**Preguntas que debe responder el registro**

| Pregunta | Qué se mira | Si la respuesta es mala |
|---|---|---|
| ¿Qué es la segunda pulsación? | Eventos de botón por cada toque, su posición y su firma | Si el gancho no la ve, no se puede bloquear desde él |
| ¿Bloquearla dejaría intacta la correcta? | Si solo aparece un clic de ratón por toque y coincide con el punto equivocado | Si coincide con el correcto, la hipótesis falla: revisar |
| ¿Los toques de multi-touch pasan por el gancho? | Si hay eventos al tocar, y con qué firma | B2 exigiría otro modo de Artemis |
| ¿Se distinguen del ratón físico? | `LLMHF_INJECTED` y `dwExtraInfo` | B2 podría desviar el ratón del PC: no es viable tal cual |
| ¿Se distinguen de los movimientos de Magpie? | Eventos tras cada toque, comparados con el `SetCursorPos` de prueba | Riesgo de bucle: hay que replantear B2 |
| ¿`SetCursorPos` vuelve a entrar en el gancho? | El evento marcado de la prueba | Hay que filtrar los movimientos propios en B2 |
| ¿El modo manda posiciones absolutas? | Si la posición coincide con el punto tocado | Sin absolutas no hay nada que convertir: se queda en trackpad |
| ¿Cuadra la fórmula con los rectángulos reales? | Comparar con los valores calculados arriba | Revisar la conversión antes de B2 |

**Decisión al acabar**

| Resultado | Siguiente paso |
|---|---|
| La pulsación sobrante es un ratón distinguible | B2 reducido: bloquearla con el soporte táctil activado |
| Multi-touch llega al gancho y se distingue, pero no basta con bloquear | B2 completo sobre multi-touch |
| Multi-touch no llega, pero otro modo manda posiciones absolutas distinguibles | B2 sobre ese modo |
| Ningún modo sirve | Parar B2. Quedan el trackpad, o jugar sin Magpie cuando se quiera táctil directo |

**Hecho cuando:** hay un registro de cada modo y la tabla de preguntas está contestada en este documento. Después se
borra el archivo `stream_touch_diag`, y el módulo se reaprovecha en B2 o se elimina.

### B2. Corrección con gancho

Módulo nuevo `src-tauri/src/stream_touch.rs`, separado del gancho de botones de `mouse.rs`:

1. Instalar el gancho solo durante una sesión de streaming y quitarlo al acabar.
2. Un hilo aparte lee los rectángulos de Magpie al recibir `MagpieScalingChanged`; el gancho solo lee una copia atómica,
   porque Windows lo elimina si tarda.
3. Para cada movimiento inyectado de Apollo, aplicar la inversa de Magpie y bloquear el original:
   `src = srcLeft + (x - destLeft) * (srcAncho - 1) / (destAncho - 1)`, igual en vertical y con el mismo redondeo que `ScalingToSrc`.
4. Llamar a `SetCursorPos` dentro del propio gancho, para que el movimiento quede antes que el clic siguiente. Si Magpie
   lo deshace (sus comentarios avisan de que puede pasar), usar su técnica de `_ReliableSetCursorPos`, adaptada para no bloquear el gancho.
5. Toques en las bandas negras: llevarlos al borde más cercano de la imagen.

**Dejar pasar sin tocar** cuando:
- Magpie no esté escalando.
- El menú sobre el juego esté abierto o el juego no esté delante: Magpie suelta el cursor y las coordenadas de pantalla ya son correctas.
- El evento venga del ratón físico.
- Artemis esté en modo trackpad.

Este último caso exige un ajuste «Corregir toques con Magpie» en Ajustes, porque el gancho no lo detecta por sí solo.

### B3. Pruebas y documentación

- Pruebas unitarias de la conversión en `visor-core`: esquinas, centro, bandas y DPI, con los números del ejemplo de arriba.
- Prueba real con novelas a 720p y 1080p, con el menú de volumen abierto y tras reconectar.
- Explicar en Ajustes qué modo de Artemis corresponde a cada opción.

**Hecho cuando:** los toques en las esquinas y en un botón pequeño aciertan en 720p y 1080p, sin afectar al ratón del PC
ni al menú sobre el juego. Hasta entonces, el modo trackpad sigue siendo la alternativa.

## 4. Orden

| Paso | Quién | Depende de | Tamaño |
|---|---|---|---|
| A5, ajustes de Artemis y prueba de nitidez | Usuario | Nada | 20 min |
| B0, prueba del táctil de Magpie (hecha: toque doble) | Usuario | Nada | 15 min |
| A1 y A2, perfil de tablet (hecho, RCAS sigue en 0,45 hasta A5) | Código | Nitidez de A5 | Pequeño |
| A3, selector por destino | Código | A2 | Pequeño |
| A4, reconexión | Código | Comprobar si Apollo mueve el juego | Pequeño |
| **B1, prototipo (siguiente)** | Código y usuario | Nada | Pequeño |
| B2 y B3, corrección | Código | B1 viable | Medio |

Las categorías de la biblioteca no se tocan: todo esto afecta solo al lanzamiento y a Ajustes.

## 5. Decisiones pendientes

1. **Certificado de B0:** ya instalado para la prueba. Si no sirve, desactivar el soporte táctil en Magpie lo revierte;
   la documentación de Magpie aconseja desactivarlo si no se usa.
2. **Modelo de tablet:** 8x32 en todo, salvo que la prueba muestre tirones al codificar a la vez.
3. **A3:** selector por juego o ajuste global.
4. **Modelo por encima de 1100 px de alto en la tablet:** si Tsukihime (1280 px) sube de `CuNNy fast` a 8x32. Antes hay
   que medir si la GPU aguanta el modelo y la codificación a la vez.
5. **Copias antiguas de Magpie:** si se borran la 0.10.3 de `Escritorio\Magpie v.2` y la 0.9.1 de
   `OneDrive\Escritorio\Magpie`. El visor ya elige la más nueva, pero abrir una a mano vuelve a dejar el perfil sin aplicar.

## 6. Fuentes

- [Magpie, CursorManager.cpp](https://github.com/Blinue/Magpie/blob/main/src/Magpie.Core/CursorManager.cpp)
- [Magpie, interacción por programa](https://github.com/Blinue/Magpie/blob/main/docs/Interact%20with%20Magpie%20programally.md)
- [Magpie, soporte táctil](https://github.com/Blinue/Magpie/blob/main/docs/About%20touch%20support.md)
- [Magpie, TouchHelper](https://github.com/Blinue/Magpie/blob/main/src/TouchHelper/App.cpp)
- [Artemis para Android](https://github.com/ClassicOldSong/moonlight-android)
- [Ficha oficial de la POCO Pad M1](https://www.po.co/global/product/poco-pad-m1/specs)
