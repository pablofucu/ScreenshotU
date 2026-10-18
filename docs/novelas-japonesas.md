# Novelas japonesas: texthook + página de lectura

Función exclusiva de una **nueva categoría «Novelas japonesas»** (plataforma `jvn`). El resto de la app
(capturas, Magpie, mando, estadísticas) no cambia. Al lanzar un juego de esa categoría:

1. Se lanza el juego como siempre (`launcher.rs`) y, en cuanto se conoce su PID, se inyecta **LunaHook**.
2. Se abren dos ventanas en el **monitor a la izquierda** del monitor donde está la ventana del juego
   (si no hay monitor a la izquierda: cualquier otro monitor; si solo hay uno: el mismo):
   - **Ventana de hook** (Tauri, label `texthook`, entrada `hook.html`): diseño «Lista + vista previa»
     (`design/texthook-propuestas.html`, propuesta 1). Ratón, sin mando.
   - **Página de lectura** (Edge/Chrome en modo `--app` con perfil propio donde el usuario instala Yomitan;
     la sirve la app en `http://127.0.0.1:6677/`, entrada `reader.html`): diseño «Furigana y colores»
     combinado con «Burbujas» (`design/lectura-propuestas.html`, propuestas 2 y 5). Cada personaje tiene una
     burbuja de color configurable; gris por defecto. La narración va en tarjetas sin icono.
   - Colocación: la página de lectura ocupa el monitor izquierdo entero; la ventana de hook se abre encima,
     centrada (1280×720 o lo que quepa). Si el juego ya tiene un hook guardado, la ventana de hook se abre
     minimizada.
3. Al cerrar el juego: se desconecta LunaHook, se cierra la ventana de hook y se avisa a la página de lectura.

## Reparto (cada agente toca SOLO sus archivos)

| Quién | Archivos | Qué |
|---|---|---|
| A · LunaHook | `src-tauri/src/texthook/**` | FFI con LunaHost (libloading), inyección, hilos, códigos H, búsqueda, comandos `th_*` |
| B · Texto | `src-tauri/visor-core/src/jp/**`, `src-tauri/visor-core/Cargo.toml` | Filtros, nombres, tokenizador + furigana (lindera), puntuación de hilos, prefs por juego (`novels.json`) |
| C · Lectura | `src-tauri/src/reader/**`, `reader.html`, `src/reader/**` | Servidor HTTP+WebSocket, lanzar navegador, monitores, página de lectura, tema CSS de Yomitan |
| D · Ventana hook | `hook.html`, `src/hook/**` | Interfaz «Lista + vista previa» |
| Integración | todo lo demás (`lib.rs`, `state.rs`, `launcher.rs`, `novel.rs`, `Cargo.toml`, `config.rs`, `tauri.conf.json`, `capabilities/`, `vite.config.ts`, `src/lib/novel-types.ts`, categoría nueva) | Se hace al final |

Si necesitas algo de un archivo que no es tuyo, **no lo edites**: déjalo anotado en tu informe final.
Los tipos compartidos están en `src-tauri/src/novel.rs` (Rust) y `src/lib/novel-types.ts` (TS); no se cambian
sin avisar (si hace falta un campo nuevo, añádelo en tu informe).

## Flujo de datos

```
LunaHook (A) ──texto bruto por hilo──► novel::on_text(app, thread_key, text)   [integración]
                                              │  usa visor_core::jp (B): filtros, nombres, tokens
                                              ├─► evento Tauri "th-text"  {key, text}  (todos los hilos, filtrado)
                                              ├─► evento Tauri "th-line"  ReaderLine  (solo hilo elegido)
                                              ├─► reader::broadcast(ReaderLine) (C) → WebSocket → página
                                              └─► portapapeles (si está activado)
```

## Contrato

### Eventos Tauri (backend → ventana `texthook`)
- `th-proc`  → `ProcInfo`
- `th-thread` → `ThreadInfo` (alta o actualización: contador, último texto)
- `th-thread-removed` → `{ key: string }`
- `th-text` → `{ key: string, text: string }` (texto ya filtrado, de cualquier hilo)
- `th-line` → `ReaderLine` (línea final del hilo elegido)
- `th-search` → `SearchHit` (cada resultado de una búsqueda de hooks)
- `th-search-done` → `{ count: number }`
- `th-log` → `string` (mensajes de la consola de LunaHook)

### Comandos (ventana `texthook` → backend)
Agente A (`texthook::commands`):
- `th_status() -> ThStatus`
- `th_select_thread(key: String)`; `th_select_name_thread(key: Option<String>)`
- `th_insert_code(code: String) -> Result<(), String>`; `th_remove_hook(key: String)`
- `th_search_text(text: String, codepage: u32, deep: bool)`; `th_search_cancel()`
- `th_reinject()`; `th_detach()`
Integración (`novel.rs`):
- `novel_get_prefs(game_id: String) -> NovelPrefs`; `novel_save_prefs(game_id: String, prefs: NovelPrefs)`
- `novel_open_reader()`; `novel_reader_url() -> String`; `novel_yomitan_css() -> String`

### Tipos (camelCase en JSON)
```ts
ProcInfo   { pid: number; exe: string; arch: 32 | 64; connected: boolean }
ThreadInfo { key: string; name: string; code: string; lines: number; last: string; live: boolean }
ThStatus   { proc: ProcInfo | null; threads: ThreadInfo[]; selected: string | null; nameThread: string | null; gameId: string | null; gameName: string | null }
SearchHit  { code: string; text: string }
Tok        { parts: [string, string | null][]; pos: Pos }   // parts = trozos con su furigana (null = sin ruby)
Pos        = 'n' | 'v' | 'a' | 'adv' | 'p' | 'aux' | 'i' | 'x'   // sustantivo, verbo, adjetivo, adverbio, partícula, auxiliar, interjección, signo/otro
ReaderLine { id: number; at: number; who: string | null; color: string | null; text: string; toks: Tok[] }
NovelPrefs {
  hookCodes: string[]; threadKey: string | null; nameThreadKey: string | null;
  filters: { dedupeChars: boolean; joinLines: boolean; dedupeLines: boolean; splitName: boolean;
             dedupePhrase: boolean; dedupeSegments: boolean; dedupeIncremental: boolean; removeControl: boolean; removeTags: boolean; removeBraces: boolean;
             regex: { pattern: string; replace: string; on: boolean }[] };
  characters: { name: string; color: string | null }[];         // null = gris por defecto
  reader: { furigana: 'all' | 'off'; posColors: boolean; fontSize: number };
  out: { reader: boolean; clipboard: boolean };
}
```

### Mensajes WebSocket (`ws://127.0.0.1:6677/ws`, servidor → página)
- `{ "type": "hello", "game": string | null, "prefs": NovelPrefs | null, "history": ReaderLine[] }`
- `{ "type": "line", "line": ReaderLine }`
- `{ "type": "prefs", "prefs": NovelPrefs }` (cambian colores, furigana…)
- `{ "type": "bye" }` (el juego se cerró)
Página → servidor: `{ "type": "prefs", "reader": {...} }` cuando el usuario cambia furigana/colores/tamaño desde la página.

## Reconocimiento de nombres (agente B)
Por orden:
1. **Hilo de nombre**: si el usuario marca un hilo como «hilo de nombre», su último texto se asigna a la
   siguiente línea del hilo principal (muchos motores, p. ej. SiglusEngine, sacan el nombre en otro hilo).
2. **Patrones en la línea**: `名前「…」`, `名前『…』`, `【名前】…`, `名前：…` / `名前:…` (nombre corto, ≤ 10 caracteres, sin signos).
3. Sin nombre y sin comillas → narración (`who = null`).
Cada nombre nuevo se añade a `characters` con `color = null` (gris); el usuario le pone color en la ventana de hook.

## Compilación
`npm run tauri build -- --no-bundle` (ver `CLAUDE.md`). Mientras trabajáis en paralelo, comprobad vuestro
código con `cargo check` (en `src-tauri`) o `npx vite build`; si falla por archivos de otro agente,
esperad un poco y reintentad: no los toquéis.
