# Switch: SW-16 y fase 4 (importación y sincronización, backend)

Orden y commits: **SW-16 → SW-17 → SW-24 → SW-25 → SW-18**, un commit por tarea
(`feat(nintendo): … (SW-xx)`), con push tras cada uno (CLAUDE.md). Estado de cada tarea en Kanvas con el CLI
(nunca a mano). SW-07/08/14 se dan por buenas. Antes de cada commit, mirar los CR de los ficheros tocados (hay
mezcla LF/CRLF).

**Regla de categorías:** todo lo nuevo actúa solo sobre la plataforma `switch` (`scan.rs:395` ya la usa por id).
Los candidatos a enlazar salen solo de juegos con `platform == "switch"` y de carpetas de `Raiz/<carpeta Switch>`.
Ningún paso renombra, crea ni detecta categorías.

Rutas: `core/` = `src-tauri/visor-core/src/`, `app/` = `src-tauri/src/`.
Tests: `cd src-tauri && cargo test -p visor-core nintendo` y `cargo test -p visor-capturas nintendo`.

---

## SW-16 — Cliente del historial y guardado

**Decisión.** El HTTP va en `app/nintendo/history.rs`; el guardado es puro en `core/nintendo/store.rs` (probado con la
fixture). Un único punto de entrada `nintendo::refresh` serializa las descargas y limita la frecuencia (mín. 2 min
entre peticiones reales), para que la importación y el hilo de SW-18 no pidan dos veces.

1. `core/nintendo/store.rs` (tras `daily_upsert`, :140): 
   ```rust
   #[derive(Debug, Default, Clone, PartialEq, Eq)]
   pub struct HistorySaved { pub titles: usize, pub days: usize, pub new: Vec<String>, pub changed: Vec<String> }
   pub fn save_history(conn: &Connection, titles: &[TitleUpsert], days: &[DailyRow], now_ms: i64) -> Result<HistorySaved>
   ```
   En una transacción (`conn.unchecked_transaction()`): por título, `title_get` antes de `upsert_title`; si no existía
   → `new`; si cambió `total_minutes` o `last_played_ms` → `changed`. Por día, comparar con lo guardado (consulta por
   clave) y, si difiere, `daily_upsert` y añadir el `nx_id` a `changed` (sin repetir). No toca `games`.
2. `app/nintendo/history.rs` (nuevo):
   ```rust
   pub enum SyncError { NotConnected, Expired(String), Transient(String), ApiChanged(String) }
   impl From<AuthError> for SyncError   // NotConnected/Revoked->Expired/Transient
   impl Display for SyncError           // ApiChanged: «La API de Nintendo ha cambiado: …»
   pub fn fetch(session: &mut Session) -> Result<PlayHistories, SyncError>
   ```
   `GET PLAY_HISTORIES_URL` con `User-Agent: HISTORY_USER_AGENT`, `Accept: application/json`,
   `Gentry-Locale: HISTORY_LOCALE`, `Authorization: Bearer <access_token>`, timeout 20 s (como `auth.rs:68`).
   401/403 → reintentar con `id_token` si lo hay → `session.invalidate()` + `session.access()` y un último intento →
   si sigue, `Transient("historial HTTP 401")` (no se borra el session token por esto: lo decide `auth.rs`).
   400 → `ApiChanged`. Error de `parse_play_histories` → `ApiChanged`. Nunca imprimir tokens ni cuerpos con tokens.
3. `app/nintendo/mod.rs`: `pub mod history;` y estado compartido:
   ```rust
   fn session() -> &'static Mutex<Session>              // OnceLock
   pub const MIN_FETCH_GAP: Duration = Duration::from_secs(120);
   pub struct Refresh { pub fetched: bool, pub saved: HistorySaved }
   pub fn refresh(state: &AppState) -> Result<Refresh, SyncError>
   ```
   Un `Mutex<Option<Instant>>` estático guarda la última petición: si es más reciente que `MIN_FETCH_GAP`, devuelve
   `fetched: false` sin red. Si no: `history::fetch`, `store::save_history(c, &ph.title_upserts(), &ph.daily_rows(),
   now_ms)`, y en `meta` (`dbd::meta_set`) `nintendo.last_sync_ms` y `nintendo.last_error` (vacío si fue bien, el
   `Display` del error si no). Quitar los `#[allow(dead_code)]` que ya no hagan falta.
4. Comando `nintendo_refresh() -> Result<RefreshReport{fetched, titles, days, new, changed}, String>` (async +
   `spawn_blocking`), registrado en `generate_handler!` (`app/lib.rs:1269`). Sirve para probar a mano hasta que exista
   la interfaz.

**Tests sin red:** `save_history` con la fixture: 10 títulos y sus días; repetir con los mismos datos → `new` y
`changed` vacíos y mismas filas; subir un total → solo ese en `changed`; la tabla `games` sigue vacía; conserva
`game_id`/`ignored` de un título enlazado. `SyncError::from` de cada `AuthError`.
**Con red (opcional, `#[ignore]`):** `live_history` en `history.rs` lee `NINTENDO_SESSION_TOKEN` del entorno (el del
script de `.claude-scratch/nintendo/`), pide access token, descarga y guarda en una base temporal; imprime solo
recuentos. `cargo test -p visor-capturas live_history -- --ignored`. La prueba con login real queda para SW-15.

---

## SW-17 — Vista previa e importación

**Decisión.** La lógica (agrupar, filtrar, aplicar) es pura en `core/nintendo/import.rs` y se prueba con base temporal;
los comandos en `app/nintendo/import.rs` solo reúnen datos. Demos fuera siempre (ni en la vista previa ni en
`apply`). El mínimo (60 min por defecto) solo limita **crear fichas**: enlazar con una ficha que ya existe no crea
nada y se propone siempre. La importación nunca crea carpetas en disco.

1. `core/config.rs`: campo `pub nintendo_min_minutes: u32` (default 60 en `Default`, ~:223). JSON: `nintendoMinMinutes`.
2. `core/nintendo/mod.rs`: `pub mod import;`, `pub const SWITCH_PLATFORM: &str = "switch";`.
3. `core/scan.rs:270`: hacer `pub fn find_dir_ci`.
4. `core/nintendo/link.rs`: `pub fn candidates(games: &[Game], folders: &[String]) -> Vec<Candidate>`: juegos con
   `platform == SWITCH_PLATFORM` (con su `folder` derivada) + carpetas cuyo `norm_key` no coincide con la de un juego
   (si coincide, es la misma ficha).
5. `core/nintendo/import.rs` (nuevo), todo `Serialize`/`Deserialize` camelCase:
   ```rust
   pub struct Item { pub nx_id: String, pub name: String, pub minutes: i64, pub last_played: Option<i64>, pub image_url: Option<String>, pub platform: Option<String> }
   pub struct NewGroup { pub name: String, pub items: Vec<Item> }        // una ficha nueva (SW-17: un item)
   pub struct LinkItem { pub item: Item, pub target: Candidate }
   pub struct DoubtItem { pub item: Item, pub options: Vec<Candidate> }
   pub struct Preview { pub nuevos: Vec<NewGroup>, pub enlazados: Vec<LinkItem>, pub dudosos: Vec<DoubtItem>,
                        pub excluidos: Vec<Item>, pub ignorados: Vec<Item>, pub demos: usize, pub min_minutes: i64 }
   pub fn preview(titles: &[SwitchTitle], cands: &[Candidate], min_minutes: i64) -> Preview
   #[serde(tag = "action", rename_all = "camelCase")]
   pub enum Choice { Create { nx_ids: Vec<String>, name: String }, Link { nx_id: String, game_id: Option<String>, folder: Option<String> }, Ignore { nx_id: String } }
   pub struct Created { pub game_id: String, pub name: String, pub nx_ids: Vec<String>, pub image_url: Option<String> }
   pub struct Report { pub created: Vec<Created>, pub linked: usize, pub ignored: usize, pub skipped: Vec<String> }
   pub fn apply(conn: &Connection, choices: &[Choice], cands: &[Candidate], new_id: impl FnMut() -> String) -> Result<Report>
   pub fn pending_count(titles: &[SwitchTitle], min_minutes: i64) -> usize   // «N nuevos sin importar» (SW-18)
   ```
   `preview`: quita demos (`is_demo`), usa `link::propose` sobre lo no ignorado; los `nuevos` con
   `minutes >= min` van a `nuevos`, el resto a `excluidos` (se pueden crear a mano con `Choice::Create`).
   `apply`, en una transacción: salta demos y `nx_id` inexistentes o ya enlazados (motivo en `skipped`); `Create` →
   `ManualGame { id, name, kind: "captures", platform: "switch", folder: None, .. }` con `dbd::manual_add` y
   `title_set_link(nx, Some(id), None)` para cada `nx_id`. Ids únicos: `new_id()` puede repetir en un bucle rápido
   (`games.rs:285`, nanos) e `INSERT OR IGNORE` lo callaría: repetir hasta que no esté en `games` ni en los ya
   generados. `Link` → solo si el destino está en `cands` (nunca otra categoría). `Ignore` → `title_set_ignored`.
6. `app/nintendo/import.rs` (nuevo), comandos async con `spawn_blocking`:
   - `nintendo_import_preview(min_minutes: Option<i64>, refresh: bool) -> Result<PreviewReport{preview, error: Option<String>}, String>`:
     si `refresh`, `nintendo::refresh` (un error no impide la vista previa: va en `error`); candidatos con el helper
     `switch_candidates(state)` (`dbd::manual_load` → `games::all_games` → `link::candidates`, carpetas por
     `read_dir` de `find_dir_ci(root, carpeta de la plataforma switch)`, solo nombres: un nivel, barato en el HDD).
   - `nintendo_import_apply(choices: Vec<Choice>) -> Result<Report, String>`. El frontend reescanea una vez después.
   - `nintendo_link(nx_id, game_id: Option<String>, folder: Option<String>) -> Result<(), String>`: valida contra
     `switch_candidates`; `(None, None)` quita el enlace.
   - `nintendo_set_ignored(nx_id, ignored: bool) -> Result<(), String>`.
   Registrar los cuatro en `generate_handler!`.

**Tests sin red** (fixture + `Db` temporal, patrón `games.rs:550-610`): con mínimo 60, `nuevos` = Smash, Ocarina,
Yoshi, Mina, Xenoblade; `excluidos` = SNES/GBA Classics e It Takes Two; las dos demos no aparecen y `demos == 2`;
`apply` con `Create` crea fila `captures`/`switch` sin `folder` y sin carpeta en disco (tempdir vacío), y el título
queda enlazado; `Create` de una demo → `skipped`; `Link` a un destino que no está en `cands` → `skipped`; generador de
ids que repite → dos fichas distintas; `Ignore` → `ignored = 1` y sale de `nuevos`; `pending_count`; `candidates`
ignora juegos de otras plataformas y funde juego + su carpeta.

---

## SW-24 — Fusión con fichas repetidas

**Decisión.** «Fusionar» = enlazar el título de Nintendo con la ficha que ya existe en vez de crear otra; varias
versiones de Nintendo (Switch 1 y «Nintendo Switch 2 Edition») van a una sola ficha (merge.rs ya suma títulos de la
misma ficha). No se fusionan dos fichas existentes entre sí (no hay forma de mover estado/colecciones/arte entre ids).
Coincidencia exacta → `enlazados`; coincidencia solo por nombre base → `dudosos` (lo confirma el usuario).

1. `core/nintendo/mod.rs`: `pub fn base_name(name: &str) -> String` (quita ™ ® © y un sufijo final «– Nintendo Switch 2
   Edition», «Nintendo Switch 2 Edition», «Switch 2 Edition» con separador `–`/`-`/`:` opcional; recorta espacios) y
   `pub fn base_key(name) -> String { norm_key(&base_name(name)) }`.
2. `core/nintendo/link.rs:50` `propose`: si no hay coincidencia exacta por `norm_key`, buscar por `base_key` (contra
   nombre y carpeta de la ficha); los aciertos van a `dudosos` aunque sea uno. Ordenar `options`: primero las que
   tienen `game_id`, luego por nombre (la interfaz preselecciona la primera).
3. `core/nintendo/import.rs` `preview`: agrupar los `nuevos` por `base_key` en un `NewGroup` con `name = base_name` del
   de más minutos. `apply` `Create`: si ya hay un candidato con el mismo `base_key`, enlazar con él (nota en
   `skipped`: «ya existía, se ha enlazado con X»); si dos `Create` de la misma tanda tienen el mismo `base_key`, se
   crea una sola ficha.

**Tests sin red:** `base_name("Mina the Hollower – Nintendo Switch 2 Edition") == "Mina the Hollower"`;
`"Xenoblade Chronicles X: Definitive Edition – Nintendo Switch 2 Edition"` conserva «Definitive Edition»;
`"Yoshi and the Mysterious Book Demo"` sigue siendo demo y no se agrupa con el juego; títulos `0100…` y `0400…` con el
mismo nombre base → un `NewGroup` y una sola ficha al aplicar; ficha «Mina the Hollower» existente → `dudosos` con una
opción; dos fichas repetidas (juego y carpeta con nombres distintos que coinciden) → `dudosos` con el juego primero;
`Create` cuando ya existe la ficha → enlaza y no crea fila.

---

## SW-25 — Grafismos de los juegos importados

**Decisión.** Reutilizar el lote de `sgdb_auto` en vez de duplicarlo: se extrae su hilo a una función común y cada
pendiente lleva un término de búsqueda y una portada de respaldo opcionales. Para los juegos importados: término =
`base_name` (sin ™ ni «Switch 2 Edition»), y si SteamGridDB no tiene portada, el icono de Nintendo (`imageUrl`) como
portada en la caché de arte. Los demás títulos de la biblioteca se comportan igual que hoy (término = nombre, sin
respaldo). Búsqueda por titleId: SteamGridDB no la ofrece para Nintendo (ver dudas); se queda por nombre.

1. `app/sgdb.rs:479` `Pending`: añadir `term: String` y `fallback_cover: Option<String>`; en `sgdb_auto` (:660)
   `term: t.name.clone(), fallback_cover: None`.
2. `app/sgdb.rs:670-733`: mover el cuerpo del hilo a
   `pub(crate) fn run_batch(app: AppHandle, todo: Vec<Pending>) -> Result<usize, String>` (incluye el
   `sgdb_busy.swap`, el `BusyGuard` y los eventos `sgdb-progress`/`sgdb-art`/`sgdb-done`); `sgdb_auto` la llama. Hacer
   `Pending` `pub(crate)`.
3. `app/sgdb.rs:564` `run_title`: buscar con `t.term`; si no hay resultado y `term != name`, otra vez con `name`. Al
   final, si falta la portada en `sgdb::find_cached` y hay `fallback_cover`, bajarla con una comprobación de host
   propia (`https://` y host acabado en `.cdn.nintendo.net`, aparte de `allowed`, :388) y guardarla con
   `sgdb::store_image(base, id, ArtKind::Cover, 0, ext_for(..), &bytes)`.
4. `app/nintendo/art.rs` (nuevo): `pub fn spawn(app: AppHandle, created: Vec<Created>)`: si no hay clave o
   `config.sgdb_auto` está apagado, nada. Hilo: espera (sondeo de 5 s, máx. 10 min) a que `sgdb_busy` esté libre y
   llama a `run_batch` con `Pending { id: game_id, name, term: base_name(name), kinds: [Cover, Hero, Logo] menos lo
   cacheado, fallback_cover: image_url }`. Lo llama `nintendo_import_apply` tras aplicar. Si el `sgdbAuto` del
   frontend tras el reescaneo choca con el lote, recibe «ya se están buscando» y sin `force` lo calla
   (`src/lib/sgdb.svelte.ts:202`); los títulos quedan con `auto_done` y no se repiten.

**Tests sin red:** comprobación de host del respaldo (acepta `atum-img-lp1.cdn.nintendo.net`, rechaza `http://`, otros
hosts y `cdn.nintendo.net.evil.com`), función pura; `base_name` como término ya probado en SW-24. El resto (descarga)
se prueba a mano: importar un juego sin arte y ver portada/hero/logo en la ficha sin reescanear.

---

## SW-18 — Sincronización periódica

**Decisión.** Hilo `nintendo-sync` con `mpsc::Receiver::recv_timeout` (la orden «sincronizar ya» lo despierta al
momento; sin bandera ni sondeo). Solo llama a `nintendo::refresh` → `save_history`: escribe únicamente `switch_*` y
`meta`, nunca `games`. El frontend parchea los tiempos con `nintendo_title_times` sin reescanear el HDD.

1. `core/nintendo/mod.rs`: `pub fn next_delay(fail_streak: u32) -> Duration` (0 fallos → 30 min; fallos → 1, 2, 4,
   8, 16, 30 min tope) y `pub const FIRST_SYNC_DELAY: Duration = 60 s`.
2. `core/scan.rs`: tipo y función pura junto a `apply_console` (:232):
   ```rust
   #[derive(Serialize)] #[serde(rename_all = "camelCase")]
   pub struct TitleTimes { pub id: String, pub play_seconds: u64, pub week: Vec<u32>, pub last_played: Option<i64>, pub first_played: Option<i64>, pub console: Option<ConsoleTime> }
   pub fn title_times(games: &[Game], stats: &Stats, map: &ConsoleMap, folders: &[String]) -> Vec<TitleTimes>
   ```
   Por cada juego `switch`: `Title::blank()` con su carpeta, `link_game`, `apply_console`; si recibe `console`, se
   devuelve con `id = g.id`. Por cada carpeta enlazada (`switch_titles.folder`) cuyo `norm_key` no sea la de un juego:
   solo `apply_console`, `id = userdata::title_id(None, "switch", folder)`. Mismos números que el escaneo.
3. `app/nintendo/sync.rs` (nuevo):
   ```rust
   pub fn spawn(app: AppHandle)                         // guarda el Sender en un OnceLock
   #[tauri::command] pub fn nintendo_sync_now()        // envia () por el canal
   #[tauri::command] pub fn nintendo_title_times(state: State<'_, AppState>) -> Result<Vec<TitleTimes>, String>
   #[derive(Serialize)] #[serde(rename_all = "camelCase")]
   struct Synced { changed: usize, new_titles: usize, expired: bool, error: Option<String> }
   ```
   Bucle: espera `FIRST_SYNC_DELAY` y luego `next_delay(streak)` (o hasta un mensaje). Sin cuenta
   (`auth::is_connected()` falso) no hace nada. `refresh` → `Ok`: `streak = 0`; `Expired`: no reintenta hasta
   «sincronizar ya» o reconectar (espera 30 min); `Transient`/`ApiChanged`: `streak += 1`. Siempre emite
   `nintendo-synced` con `changed = saved.changed.len()`, `new_titles = import::pending_count(titles_load, min)`, y el
   error si lo hay. Todo dentro de `catch_unwind` para que un pánico no mate el hilo en silencio.
   `nintendo_title_times`: `console_times(c, Local::now())`, `dbd::manual_load` → `games::all_games`,
   `state.stats`, carpetas de `titles_load`; llama a `scan::title_times`.
4. `app/lib.rs:1206` (tras `overlay::init`): `if !is_test_env { nintendo::sync::spawn(app.handle().clone()); }`.
   Registrar `nintendo_sync_now` y `nintendo_title_times` en `generate_handler!`.

**Tests sin red:** `next_delay` (0, 1, 3, 10 fallos); `title_times` da lo mismo que `scan_library_ex` para un juego
de Switch con tiempo de PC + consola y para una carpeta suelta enlazada, y nada para un juego de otra plataforma con
el mismo nombre; tras `save_history` de la fixture dos veces, el número de filas de `games` no cambia («nunca crea
fichas»). A mano: `nintendo_sync_now` y ver `nintendo-synced` en la consola del webview.

---

## Riesgos

- **`generate_handler!` y `#[cfg(windows)]`** (`app/lib.rs:18`): los comandos nuevos viven en un módulo solo de
  Windows. Si se compila para otra plataforma, fallaría; la app solo se compila para Windows. Si no fuera así, poner
  un módulo `nintendo` vacío con los mismos comandos para el resto de plataformas.
- **El día de `playedDate`** se guarda tal cual (`YYYY-MM-DD`, decisión de SW-07); si Nintendo usa UTC y no la hora
  local, el día de la semana puede salir corrido alrededor de medianoche. No bloquea; se verá con datos reales.
- **Choque de lotes de SteamGridDB** (SW-25): resuelto con la espera de `art::spawn`; si el usuario lanza la búsqueda
  forzada en ese momento ve «ya se están buscando».
