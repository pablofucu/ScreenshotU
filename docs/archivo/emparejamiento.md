# Emparejamiento nuevo (tablet y PC cliente)

Rediseño desde cero de cómo un dispositivo (la tablet u otro PC) se conecta al PC principal. Sustituye al código
fijo, a las dos secciones de Ajustes que se pisan y al PIN de Apollo como paso aparte.

## Qué falla hoy

- **Ajustes partidos en dos.** «Acceso desde el móvil» (`src/lib/remote.svelte.ts`: activar, QR, «Dirección y código»,
  «Generar código nuevo», arrancar con Windows, cursor, Apollo, ayuda) y «Equipos» (`src/lib/devices.svelte.ts`:
  nombre, dispositivos, renombrar, quitar, «Conectarse a otro PC» con buscar/dirección/nombre/conectar, y en modo
  cliente Moonlight, direcciones, encender, desconectar).
- **Código fijo compartido** (`config.remote_token`, `remote::new_code`/`code_matches`). Quien lo sepa entra sin que
  el PC apruebe nada, y además sigue sirviendo como credencial directa (entrada «legacy» de APK viejas).
- **Tres caminos en la tablet** (`mobile/src/screens/Connect.svelte`): QR, «Visores en esta red», dirección y código a
  mano.
- **Direcciones de Tailscale** a mano en el PC cliente (`pcaddrs`).
- **Moonlight/Artemis con Apollo** se empareja aparte con su PIN; el visor no ayuda.

## Decisiones tomadas (12-nov-2026)

1. **Dos formas de emparejar:**
   - **Pedir y aprobar** (la normal): el dispositivo ve el PC en la red y pide entrar. El PC muestra «Tablet quiere
     conectarse · 4821» y el dispositivo muestra el mismo número; se aprueba con A en el PC.
   - **QR de un solo uso** (cuando el PC no se ve en la red: otra subred, Tailscale): caduca a los 5 minutos y vale
     una vez. Escanearlo ya cuenta como aprobación. En un PC cliente, que no escanea, el mismo billete se escribe a
     mano (`ABCD-EFGH`) junto con la dirección.
2. **PIN de Apollo en el mismo paso:** al emparejar, el visor también empareja Moonlight/Artemis con Apollo, a través de
   la API local de Apollo. El usuario y la contraseña de la web de Apollo se guardan una vez en el PC principal.
3. **Romper con lo viejo:** fuera el código fijo y la entrada por código. Solo valen los tokens de dispositivo.
   `apiMin` sube, y una APK o un PC con versión vieja recibe `426` con el mensaje «Actualiza el visor».
4. **Se conservan los dispositivos ya emparejados** (tabla `devices`, tokens con SHA-256): siguen entrando.
5. **Aviso de solicitud** en la ventana del visor (modal: A aprueba, B rechaza) y en una **notificación de Windows**
   con Aprobar/Rechazar, aunque el visor esté solo en la bandeja. Sin respuesta, la solicitud caduca a los 2 minutos.
6. **Una sola sección «Conexiones»** en Ajustes, con asistentes. Lo técnico va en «Avanzado».
7. **Fuera de casa automático:** el PC principal detecta sus IP de casa y de Tailscale y se las pasa al dispositivo
   al emparejar (y en cada `hello`). No se escribe ninguna dirección a mano.
8. **Mismos permisos para todos** los dispositivos emparejados.
9. **Un PC principal por dispositivo.** Para cambiar de PC, primero hay que desconectarse.
10. **Un papel por PC:** principal o cliente. En modo cliente desaparece todo lo de dispositivos conectados.
11. **Al quitar o desconectar:** el dispositivo muestra «Este dispositivo ya no está emparejado con <PC>» y abre el
    asistente. El PC principal también quita el cliente de Apollo.
12. **Maquetas antes del código** (fase M).

## Fase 0: estudio (sin tocar la app)

Hay que confirmarlo antes de cerrar los detalles de las fases 2 a 4. Resultado en `docs/referencia/emparejamiento-apollo.md`.

- **API de Apollo:** `POST https://localhost:47990/api/pin` `{pin, name}` con autenticación básica y certificado
  autofirmado; listar clientes y desemparejar uno (`/api/clients/list`, `/api/clients/unpair`). Comprobar con la
  versión instalada y ver qué identificador devuelve para guardarlo en el dispositivo.
- **Moonlight (PC cliente):** comprobar si `Moonlight.exe pair <host> --pin <pin>` empareja sin interfaz y qué
  devuelve si ya estaba emparejado.
- **Artemis (tablet):** ver si se puede iniciar el emparejamiento desde la app con un PIN que elijamos nosotros: un
  intent o URI de Artemis, o el emparejamiento por OTP de Apollo. **Plan B** si no se puede: un paso guiado en la
  tablet («Artemis te mostrará un PIN; no hace falta escribirlo») en el que el PC lee la solicitud pendiente de
  Apollo y la aprueba solo, o al menos un aviso claro con un botón para abrir Artemis.
- **Notificación de Windows con botones:** `tauri-plugin-notification` (ya es dependencia) sobre si admite acciones en
  escritorio. Si no las admite, un toast de WinRT propio o una notificación que al pulsarla abre el visor con el
  aviso.
- **IP de Tailscale:** detectarla por la interfaz (rango 100.64.0.0/10, adaptador «Tailscale») sin depender de que
  `tailscale.exe` esté en el PATH.
- **Credenciales de Apollo:** guardarlas cifradas con DPAPI de Windows (o en el Administrador de credenciales), nunca
  en `config.json` en claro.

## Fase M: maquetas

En `docs/maquetas/emparejamiento/`, siguiendo `docs/guia-diseno.md` (1280×720, mando primero, tokens, iconos SVG,
estilo claro Wii U glass). Una o dos variantes de cada pantalla:

1. Ajustes › Conexiones en el PC principal (este PC, dispositivos, Añadir dispositivo, Avanzado).
2. Asistente «Añadir dispositivo»: esperando solicitudes y «Mostrar QR».
3. Aviso de solicitud en el visor (con el número de 4 cifras) y el aspecto de la notificación de Windows.
4. Tablet: pantalla de conexión (PCs en la red, esperando aprobación, paso de Artemis, listo, «ya no emparejado»).
5. PC cliente: asistente «Usar la biblioteca de otro PC» y la sección Conexiones en modo cliente.

La elegida se anota aquí y las demás se borran.

**Estado (12-nov):** las maquetas 1 a 4 están aprobadas tal cual. La notificación de Windows es la normal del
sistema con Aprobar/Rechazar (se descarta la variante arcoíris). Falta la 5 (PC cliente).

**Emparejar el streaming (según la fase 0):** `POST /api/v1/stream/pair` con `{pin}` lo usa el PC cliente (Moonlight
genera el PIN y el PC principal lo mete en Apollo); sin `pin`, el PC principal pide un OTP a Apollo y responde
`{pin, passphrase, port}` para que la tablet abra Artemis con `art://`.

## Fase 1: protocolo (visor-core)

`src-tauri/visor-core/src/remote.rs`, `devices.rs`, `remote_client.rs`, `config.rs`.

- **Fuera el código:** el auth (`:685`) solo acepta tokens (`device_by_token`). Se quitan `pair_reply` (`:877`) y
  `new_code`/`normalize_code`/`code_matches` (o se reutilizan solo para el billete), y `remote_token` deja de leerse
  (el campo se queda en la configuración y se ignora, sin migración).
- **Solicitudes** (en memoria, no en la base): `POST /api/v1/pair/request {name, kind, appVersion, api}` →
  `{requestId, digits}`; `GET /api/v1/pair/wait?id=` (long-poll de unos 25 s) →
  `{status: pending|approved|rejected|expired, deviceId?, token?, hostId, hostName, addrs[]}`. Como mucho 3
  pendientes, caducan a los 2 min, con el `throttle` actual por IP. El número de 4 cifras es aleatorio y solo sirve
  para que el usuario reconozca la solicitud.
- **Aprobar/rechazar** desde el propio PC: métodos del `Provider` y comandos Tauri `pair_pending`, `pair_approve`,
  `pair_reject` + evento `pair-request` para el modal y la notificación.
- **Billete de un solo uso:** `pair_ticket_new()` → `{ticket, expiresMs}`. `POST /api/v1/pair/ticket {ticket, name,
  kind, appVersion, api}` → token directo. QR v2: `{v:2, id, name, addrs[], port, ticket}`.
- **Direcciones:** `hello` y las respuestas de emparejamiento llevan `addrs[]` (de casa primero, Tailscale después),
  y el dispositivo las guarda y las renueva en cada `hello`.
- **Versión:** `API_MIN` sube y `features` cambia `pair` por `pair2`. Lo viejo recibe `426`.
- **Apollo:** columna nueva para el id de cliente de Apollo en `devices` (migración nueva al final de `db.rs`, con su
  prueba) para poder quitarlo al revocar.
- **Pruebas:** solicitud aprobada, rechazada y caducada; más de 3 pendientes; billete válido, usado dos veces y
  caducado; token revocado → 401; el código viejo ya no entra; API vieja → 426; `addrs` en `hello`.

## Fase 2: PC principal (escritorio)

- `src-tauri/src/remote.rs` (`AppProvider`): solicitudes, billetes y el emparejamiento de Apollo (`apollo.rs`:
  `pair_pin`, `unpair_client`, credenciales con DPAPI).
- Emparejar el streaming: el dispositivo, ya con token, llama a `POST /api/v1/stream/pair {pin}`, y el PC le pasa el
  PIN a Apollo. Así el dispositivo genera el PIN y el PC lo mete en Apollo; nadie lo escribe.
- **Ajustes › Conexiones** (sustituye a `remote.svelte.ts` y a la parte de PC principal de `devices.svelte.ts`):
  - Este PC: nombre, permitir conexiones, arrancar con Windows.
  - Dispositivos: lista (tipo, último uso, versión, streaming emparejado sí/no), renombrar, quitar (con doble
    pulsación, como ahora).
  - Añadir dispositivo → asistente (esperando solicitudes / Mostrar QR con cuenta atrás).
  - Avanzado: puerto, direcciones detectadas, app «Juego actual» en Apollo, credenciales de Apollo, ocultar cursor.
- Modal de solicitud en el visor y notificación de Windows (según lo que dé la fase 0).

## Fase 3: PC cliente (escritorio)

- `src-tauri/src/client.rs`: `client_pair` pasa a solicitud + espera (o billete), guarda `addrs[]` y después
  empareja Moonlight (`pair --pin`) y llama a `/stream/pair`. Se quitan el campo de direcciones a mano y el código.
- Asistente «Usar la biblioteca de otro PC»: PCs en la red → elegir → «Esperando aprobación en <PC> · 4821» →
  emparejando streaming → listo y recargar. Un enlace secundario lleva a «Escribir dirección y billete».
- En modo cliente, Conexiones muestra: conectado a, estado, Moonlight, encender el PC principal, desconectar.
- `401` → aviso «ya no emparejado» y el asistente.

## Fase 4: tablet (`mobile/`)

- `Connect.svelte` nuevo: tarjetas grandes de los PCs en la red → pedir → número de 4 cifras y espera → paso de
  Artemis (según la fase 0) → listo. Secundario: Escanear QR. Terciario: escribir dirección y billete. Se queda la
  demo.
- `ServerCfg` guarda `token`, `id` y `addrs[]`. Hoy el campo se llama `code` y lleva el token: se renombra con su
  lectura de lo viejo. `locate.ts` prueba `addrs` y después busca en la red.
- `401` → aviso y asistente; `426` → «Actualiza el visor del PC» o «Actualiza la app».

## Fase 5: limpieza

- Quitar las filas viejas (`remotecode`, `remoteaddr`, `pcaddr`, `pcaddrs`, `pcpair`…) y textos de ayuda que hablen del
  código. Actualizar `docs/referencia/api-movil.md`.
- `docs/planes/multi-pc.md`: anotar que el emparejamiento lo sustituye este plan (puntos 1C, 1E y 7 de sus dudas).
- Al terminar, este plan pasa a `docs/archivo/`.

## Comprobación

- `cargo test -p visor-core` (fase 1), `npm run check` (fases 2, 3 y 5), `node --test` si se añade lógica en `src/lib`.
- Manual (lo hace el usuario): tablet por solicitud, tablet por QR, portátil por solicitud con Moonlight emparejado
  solo, quitar un dispositivo desde el PC (el dispositivo vuelve al asistente y desaparece de Apollo), conectar por
  Tailscale fuera de casa sin escribir direcciones.

## Riesgos

- Artemis no se deja automatizar → plan B guiado. El resto del flujo no depende de eso.
- La notificación de Windows sin botones → el toast abre el visor con el modal.
- La API de Apollo cambia entre versiones → comprobar la versión en `apollo::status` y, si no es compatible, caer a
  «empareja el streaming a mano» con explicación.
