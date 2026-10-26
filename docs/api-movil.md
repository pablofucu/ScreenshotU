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

## Endpoints

### `GET /api/v1/hello` (sin código)

```json
{ "app": "visor-capturas", "api": 1, "name": "PC-SALON", "version": "0.1.0" }
```

Sirve para comprobar que en esa dirección hay un visor antes de pedir el código.

### `GET /api/v1/snapshot`

Todo lo que necesita la app móvil, de una vez:

```json
{
  "api": 1,
  "generatedAt": 1760980000000,
  "name": "PC-SALON",
  "config": { "goalVn": 60, "goalJvn": 30, "accentMode": "art", "accentFixed": 4 },
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

## Errores

`{"error": "mensaje en español"}` con `400` (parámetros), `401` (código), `404` (ruta desconocida o
desaparecida), `429` (demasiados intentos) o `500`.
