# API de Nintendo (SW-01, 2026-11-07)

## Login (verificado con cuenta real)
- Fuente: extensión de Raycast "Switch Game Play History" (`helpers/nintendo.ts`).
- `client_id` del historial: `5c38e31cd085304b`; `redirect_uri`: `npf5c38e31cd085304b://auth`.
- Scope: `openid user user.mii user.email user.links[].id`; PKCE S256; `response_type=session_token_code`.
- `POST https://accounts.nintendo.com/connect/1.0.0/api/session_token` (form: client_id, session_token_code, session_token_code_verifier;
  cabeceras `User-Agent: com.nintendo.znej/1.13.0 (Android/7.1.2)`, `X-Platform: Android`, `X-ProductVersion: 2.5.0`) -> **200 OK**.
- `POST https://accounts.nintendo.com/connect/1.0.0/api/token` (JSON: client_id, session_token,
  grant_type `urn:ietf:params:oauth:grant-type:jwt-bearer-session-token`) -> **200 OK**.
- El `session_token_code` caduca a los 10 min y es de un solo uso.

## Historial de juego: FUNCIONA (host nuevo, verificado 2026-11-07)
- El host antiguo de Raycast (`mypage-api.entry.nintendo.co.jp/api/v1/...`) ya no existe (NXDOMAIN).
- El actual es el de la app Nintendo Store / Nintendo Switch App (fuente: Go `wolveix/nintendo-go`):
  `GET https://app-api.znej.nintendo.com/api/v2.0/users/me/play_histories` -> **200 OK**.
- **Mismo login** (mismo `client_id` `5c38e31cd085304b`, mismo session token); Bearer = `access_token`
  (nintendo-go reintenta con `id_token` si falla; aquí no hizo falta).
- Cabeceras: `User-Agent: com.nintendo.znej/3.0.3 (iOS/26.0.1)`, `Accept: application/json`,
  **`Gentry-Locale: es-ES` obligatoria** (sin ella, 400). El idioma de los nombres sale de ahí.
- Sin f-token ni servicios de terceros.

### Respuesta (cuenta de Pablo, ~32 KB)
- Raíz: `playHistories` (87 juegos, **toda la vida de la cuenta**, desde 2017), `recentPlayHistories` (6 días: hoy y
  los 5 anteriores), `hiddenTitleList` (vacía).
- Juego: `titleId` (16 hex en **minúsculas**), `titleName`, `imageUrl`, `platform`, `firstPlayedAt`, `lastPlayedAt`,
  `lastUpdatedAt` (ISO UTC), `totalPlayedDays`, `totalPlayedMinutes`. **No** trae `deviceType`.
- Día: `playedDate` (`YYYY-MM-DDT00:00:00Z`) + `dailyPlayHistories[]` con `titleId`, `titleName`, `imageUrl`,
  `platform`, `totalPlayedMinutes`.
- **Switch 2 incluida.** `platform`: `HAC` (Switch 1) / `BEE` (Switch 2). Ids: `0100…` (Switch 1) y `0400…` (exclusivos
  de Switch 2). Un juego de Switch 1 jugado en Switch 2 sale como `BEE` con id `0100…` (p. ej. Zelda BotW): `platform`
  parece ser la última consola, no la generación del juego. Para la generación, el prefijo del id.
- Las demos vienen en la lista (nombre con «Demo»); hay que filtrarlas (decisión 3 de Pablo).
- Desglose diario corto (6 días): hay que sincronizar al menos cada pocos días para no perder días; el total no se pierde.

## Control Parental (Moon): investigado en documentación (SW-02, 2026-11-07), NO probado en vivo
- Fuente: `pynintendoparental` y `pynintendoauth` (pantherale0). Aplazado: no se implementa por ahora (SW-19 fuera del plan).
- **Login distinto** al del historial: `client_id` `54789befb391a838`, `redirect_uri` `npf54789befb391a838://auth`, mismo
  flujo PKCE y mismos endpoints de `accounts.nintendo.com`. Scopes: `openid user moonUser:administration
  moonDevice:create moonOwnedDevice:administration moonParentalControlSetting moonParentalControlSetting:update
  moonParentalControlSettingState moonPairingState moonSmartDevice:administration moonDailySummary moonMonthlySummary`.
  El session token del historial NO sirve: haría falta un segundo login.
- Base `https://app.lp1.znma.srv.nintendo.net`. Lectura:
  `GET /v2/actions/user/fetchOwnedDevices`, `/v2/actions/playSummary/fetchDailySummaries?deviceId=…`,
  `/v2/actions/playSummary/fetchLatestMonthlySummary?deviceId=…`,
  `/v2/actions/playSummary/fetchMonthlySummary?deviceId=…&year=YYYY&month=MM&containLatest=false`.
- Cabeceras: `User-Agent: moon_ANDROID/2.6.0 (com.nintendo.znma; build:700; ANDROID 34)`, `Authorization` (access token),
  `X-Moon-App-Id`, `X-Moon-Os`, `X-Moon-Os-Version`, `X-Moon-Model`, `X-Moon-App-Display-Version`,
  `X-Moon-App-Internal-Version`, `X-Moon-TimeZone`, `X-Moon-Os-Language`, `X-Moon-App-Language`.
- Desconocido sin prueba real: soporte de Switch 2, ventana de días del resumen diario (la integración de Home Assistant
  muestra ~5 días) y forma de los usuarios. Requiere cuenta con Control Parental vinculado.
- Conclusión: no aporta nada que el historial (play_histories) no tenga, salvo meses y usuarios. Se deja para más adelante.
