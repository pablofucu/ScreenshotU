# API del visor para la app móvil (v1)

La app de Android (carpeta `mobile/`) no lee archivos: todo lo pide por HTTP a un **servidor del visor**.
Hoy ese servidor es la propia app de escritorio (Ajustes → *Acceso desde el móvil*); más adelante será un
servidor en la nube con **la misma API**, así que el móvil y el escritorio podrán sacar los datos de él sin
cambiar nada en la app móvil salvo la dirección.

- Base: `http://<ip-del-pc>:<puerto>` (puerto por defecto **47811**).
- Todas las respuestas JSON van en UTF-8 y con los nombres en `camelCase` (los mismos tipos que `src/lib/types.ts`).
- CORS abierto: `Access-Control-Allow-Origin: *`, se permite la cabecera `Authorization` y las peticiones
  `OPTIONS` (preflight) se contestan con `204` sin pedir código.

## Autenticación

El servidor muestra un **código de emparejamiento** de 8 caracteres (`ABCD-EFGH`, alfabeto sin caracteres
ambiguos: `ABCDEFGHJKMNPQRSTUVWXYZ23456789`). Se compara sin guiones ni espacios y sin distinguir mayúsculas.

- JSON: cabecera `Authorization: Bearer <código>`.
- Imágenes (`<img src>`): parámetro `?k=<código>`.

Sin código o con uno incorrecto: `401` con `{"error":"..."}`. Tras 10 intentos fallidos en un minuto desde la
misma IP se responde `429` durante ese minuto.

**Token de dispositivo (apiRev 2).** Cada dispositivo (otro PC o la tablet) se empareja una vez con
`POST /api/v1/pair` y recibe un token propio (64 caracteres hex), que manda igual que el código
(`Authorization: Bearer <token>` o `?k=<token>`). Así el host sabe quién llama y guarda sus partidas a su nombre.
El código de siempre sigue entrando, como «Tablet» (`legacy-tablet`). Opcionales en cada petición:
`X-Visor-Version` (versión de la app) y `X-Visor-Api` (revisión que usa el cliente); si `X-Visor-Api` es menor que
`apiMin`, `426` «Actualiza el visor de este equipo».

## Endpoints

### `GET /api/v1/hello` (sin código)

```json
{ "app": "visor-capturas", "api": 1, "apiRev": 2, "apiMin": 1, "name": "PC-SALON", "version": "0.1.0", "features": ["launch", "capture", "call", "events", "asset", "pair", "devices"] }
```

`api` se queda en 1 (las rutas `/api/v1` no cambian y la APK antigua solo acepta 1); lo nuevo se mira en `apiRev`.

### `POST /api/v1/pair` (sin cabecera)

Cuerpo `{ "code": "ABCD-EFGH", "name": "Portátil", "kind": "pc"|"tablet", "appVersion": "0.9.0", "api": 2 }` →
`{ "deviceId", "token", "hostId", "hostName", "api" }`. Código malo: `401` (cuenta para el `429`); sin nombre: `400`.

### `POST /api/v1/unpair` (con token)

El dispositivo se quita a sí mismo: su token deja de valer. Con el código antiguo: `400`.

Sirve para comprobar que en esa dirección hay un visor antes de pedir el código. `features` lista las acciones
(`POST`) que ofrece este PC.

### `GET /api/v1/snapshot`

Todo lo que necesita la app móvil, de una vez:

```json
{
  "api": 1,
  "generatedAt": 1760980000000,
  "name": "PC-SALON",
  "config": { "goalVn": 60, "goalJvn": 30, "goalPc": 60, "goalSwitch": 60, "accentMode": "art", "accentFixed": 4 },
  "library": { "platforms": [ /* Platform[] con sus Title[] y Shot[] */ ], "hidden": [] },
  "favorites": [ "D:\\Capturas\\PC\\Elden Ring\\2024-03-09 21-05-44.png" ],
  "sagas": [ { "id": "s1", "name": "Zelda", "members": ["switch/Zelda TOTK"], "platform": "switch", "color": 2 } ],
  "playing": { "id": "manual:18a", "name": "Elden Ring", "since": 1760979000000 }
}
```

- `library` es exactamente lo que devuelve `scan_library` en el escritorio, **sin los juegos ocultos**
  (`Title.plays` trae las partidas `[inicio, fin]` en ms, que es de donde salen todas las estadísticas y las
  metas diarias).
- `sagas` son las agrupaciones del usuario (colecciones dentro de su categoría, nunca mezclan títulos entre plataformas). Cada objeto `Saga` incluye:
  - `id`: identificador de la saga.
  - `name`: nombre visible.
  - `members`: lista de identificadores de títulos pertenecientes a la saga.
  - `platform`: id de la plataforma/categoría a la que pertenece la saga (`null` si es una saga antigua sin migrar).
  - `color`: índice manual del color (número `0..6`, correspondiente a la paleta de acentos) o `null` si es automático (sacado de las portadas).
- `playing` es la partida en curso lanzada desde el visor, o `null`.
- Las rutas (`Shot.path`, `Title.cover`, `hero`, `logo`) son **identificadores opacos**: el móvil nunca las
  interpreta salvo para mostrar el nombre del archivo; solo las devuelve en `p=` para pedir la imagen.
- El servidor reescanea si su copia tiene más de 30 s o si se pide `?fresh=1`.
- Responde con `ETag`; si el cliente manda `If-None-Match` con el mismo valor, `304` sin cuerpo.
- Si el cliente acepta `gzip` (`Accept-Encoding`), la respuesta va comprimida.

### `GET /api/v1/thumb?p=<ruta>&k=<código>`

Miniatura JPEG de 480 px de ancho (la misma caché que usa el escritorio). `Cache-Control: private, max-age=604800`.

### `GET /api/v1/preview?p=<ruta>&k=<código>`

JPEG de como mucho 1920 px de ancho para el visor del móvil (se guarda en caché en el servidor).

### `GET /api/v1/file?p=<ruta>&k=<código>`

El archivo original tal cual (`image/png`, `image/jpeg` o `image/webp`). Se usa para los grafismos con
transparencia (logos), los hero y para «Ver original».

**Seguridad de rutas**: `thumb`, `preview` y `file` solo sirven rutas que aparezcan en la última instantánea
(capturas, portadas, hero y logos). Cualquier otra ruta da `404`, así que el servidor nunca expone otros archivos
del PC.

### `POST /api/v1/launch`

Cuerpo `{"id": "<id del juego>"}` (el `id` de la instantánea). Lanza el juego con el lanzador del visor, así que
cuenta el tiempo de juego. Si el juego no está en el último escaneo se escanea de nuevo. Respuesta `{"ok": true}`;
`400` sin `id`, `409` con el motivo (juego no encontrado, ya hay otro en marcha, sin ejecutable…).

### `POST /api/v1/capture`

Sin cuerpo. Hace una captura ahora, como el atajo del escritorio, y responde `{"ok": true}` en cuanto la lanza (la
captura se hace en segundo plano; si no hay juego abierto el visor lo avisa en el PC). Mismo código y mismo límite
de intentos que el resto; con `GET` da `405`.

### `POST /api/v1/call`

Cuerpo `{"cmd": "<comando de Tauri>", "args": {...}}`: ejecuta en el PC el mismo comando que la interfaz del
escritorio llama con `invoke`. `args` usa las claves que manda el JS (camelCase: `gameId`, `targetDir`…). Responde
`200` con el resultado JSON del comando (`null` si no devuelve nada), `400` (cuerpo o argumentos no válidos), `403`
`{"error":"No disponible desde la tablet"}` para los comandos que no se exponen (ventanas y atajos del PC, copias de
seguridad y biblioteca, abrir carpetas, texthook, Anki, lector de novelas, datos del servidor remoto) y `500` con el
mensaje cuando el comando devuelve error. Puede tardar (p. ej. `scan_library`); cada petición va en su propio hilo.

### `GET /api/v1/events?since=<seq>&wait=<s>`

Eventos del escritorio (los que la interfaz escucha con `listen`: `game-started`, `game-exited`, `capture-saved`,
`thumb-ready`, `sgdb-*`, `library-progress`…). Respuesta `{"seq": N, "events": [{"seq", "name", "payload"}]}`. Es un
long-poll: si hay eventos con `seq > since` los devuelve ya; si no, espera hasta `wait` segundos (máx. 25) y devuelve
la lista vacía. Con `since` ausente o `0` devuelve solo la `seq` actual (punto de partida). El servidor guarda los
últimos 200; si `since` es mayor que `seq` (el PC se reinició) también devuelve solo la `seq` actual.

### `GET /api/v1/asset?p=<ruta absoluta>&k=<código>`

Sirve un archivo cualquiera de las carpetas de datos que la interfaz muestra con `convertFileSrc`: raíz de la
biblioteca, miniaturas, vistas previas, grafismos (`art`), partidas/capturas de partidas y medios de las frases. La
ruta se canoniza y debe quedar dentro de una de esas carpetas (`..` o enlaces que salgan → `404`). Cabeceras:
`Cache-Control: public, max-age=31536000, immutable`, `ETag` (tamaño + fecha; `If-None-Match` → `304`) y el
`Content-Type` por extensión.

## Errores

`{"error": "mensaje en español"}` con `400` (parámetros), `401` (código), `403` (comando no disponible), `404` (ruta desconocida o
desaparecida), `405` (método), `409` (acción rechazada), `426` (cliente demasiado antiguo), `429` (demasiados intentos) o `500`.
