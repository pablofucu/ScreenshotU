# Lectura de estudio: acordeón, IA bajo demanda, glosario de frases y Anki

Plan para convertir la página de lectura (`src/reader/**`, servida en `127.0.0.1:6677`) en una herramienta de estudio.
Diseño de referencia: **`design/lectura-acordeon-mockup.html`** (interactivo; copiar su aspecto y comportamiento).
Contexto general de la función: `docs/novelas-japonesas.md`. Estado actual: la página ya muestra la última frase arriba
y las palabras con acepciones (diccionarios Yomitan `.zip` en `<config>/dicts`, `visor-core/src/jp/dict.rs`).

Reglas del proyecto: tras cada fase, `npm run tauri build -- --no-bundle` debe compilar (ver `CLAUDE.md`);
`cargo test` en `src-tauri/visor-core` para la lógica pura. Textos de la interfaz en español. Seguir el estilo del
código que rodea (comentarios en español sin tildes en Rust, Svelte 5 con runes, estilo Wii U de cristal).

## Decisiones ya tomadas con el usuario

- Disposición **acordeón** (propuesta 2): una fila por palabra con su primera acepción; clic → se despliega.
- **Palabras conocidas**: se marcan con ✓, salen atenuadas en su grupo «Conocidas», y en el texto pierden furigana
  y subrayado. Filtro «Todas / Sólo nuevas». Globales (no por juego).
- **Gramática** en filas moradas y subrayado punteado en la frase.
- **IA con DeepSeek, solo bajo demanda**: nada se traduce solo. Traducción (barata) al destapar la traducción
  tapada; explicación completa al pulsar «Estoy perdido». Todo se cachea → **glosario de frases**.
- **Anki** vía AnkiConnect, con **captura** del juego y **audio grabado a mano**: botón/atajo para empezar, indicador
  en la pantalla del juego, otra pulsación para parar. Sin búfer continuo.
- Frases sin voz: **voz sintética** de respaldo (Windows). VOICEVOX queda para más adelante.

---

## Fase 1 · Datos: glosario de frases y palabras conocidas (visor-core)

Nuevo `visor-core/src/jp/phrases.rs` (lógica pura, con tests) y `visor-core/src/jp/known.rs`.

**Frase guardada** (`Phrase`), una por (juego, texto):
- `id` (hash estable de `game_id` + `text`), `game_id`, `game_name`, `who`, `text`, `toks`, `words` (los del `ReaderLine`),
  `context` (las 2-3 frases anteriores, texto + quién habla: se manda a la IA y se muestra en el glosario).
- `created_at`, `last_seen_at`, `seen` (veces que ha salido).
- `translation: Option<String>`, `explanation: Option<Explanation>` con
  `Explanation { translation, literal, chain: Vec<{ kind: "v"|"g"|"n", piece, meaning }>, notes: Vec<String> }`.
- `starred: bool`, `note: String` (nota libre del usuario), `tags: Vec<String>`.
- `shot: Option<String>`, `audio: Option<String>` (rutas relativas a `<config>/phrases/media/`).
- `anki_note_id: Option<i64>`.
- `review: Review { due: i64, interval_days: f32, ease: f32, reps: u32, lapses: u32 }` (SRS tipo SM-2 simplificado:
  botones Otra vez / Difícil / Bien / Fácil).

**Cuándo entra una frase al glosario** (no se guardan todas las líneas leídas): al pedir traducción o explicación,
al marcarla con ★, al enviarla a Anki o al grabarle audio. Si ya existe, se actualiza (no se duplica).

**Almacenamiento**: `<config>/phrases/<game_id>.json` (un archivo por juego, como `store.rs`), media en
`<config>/phrases/media/`. Funciones: `load(game)`, `upsert`, `get(id)`, `list(filtro)`, `due(now)`, `grade(id, nota, now)`.

**Palabras conocidas**: `<config>/known-words.json`, conjunto de formas de diccionario (`WordInfo.base`).
`toggle(base)`, `contains`. El `ReaderLine` no cambia: la página recibe la lista de conocidas en `hello` y por un
mensaje `known` al cambiar, y marca ella misma.

## Fase 2 · Página de lectura en acordeón

Rehacer `src/reader/Reader.svelte` + `reader.css` siguiendo el mockup:
- Historial arriba (deslizar para ver frases anteriores, clic para elegir una; se conserva lo que ya hay).
- Bajo la frase, barra de ayuda: **traducción tapada** (borrosa; clic o mantener `T`), **Estoy perdido** (`E`),
  **★ Guardar** y **Frase a Anki** (`A`).
- Panel de palabras en acordeón: término + furigana, categoría, acepciones resumidas, **frecuencia**, botones ✓ y
  «+ Anki». Desplegada: acepciones completas, **kanji** y ejemplo. Grupos «Gramática» y «Conocidas · N».
  Resaltado palabra ↔ fila (ya existe con `data-i`). Pie con estadísticas de la sesión (caracteres, tiempo,
  palabras nuevas, enviadas a Anki).
- Mensajes nuevos página → app por WebSocket (`server::on_page_cmd` ya reparte los que no son `prefs`):
  `known {base}`, `star {lineId}`, `translate {lineId}`, `explain {lineId}`, `anki {lineId, word?}`,
  `record` (alternar grabación). App → página: `known {list}`, `ai {lineId, kind, text|explanation, done}`,
  `phrase {lineId, starred, inAnki, hasAudio}`, `rec {state: "idle"|"recording"|"done", secs}`.
  Actualizar `src/lib/novel-types.ts` (`ReaderMsg`) y `src/reader/demo.ts` para que `reader.html?demo` siga funcionando
  con datos de ejemplo (traducción/explicación/Anki simulados).

**Diccionarios extra** (`dict.rs`): cargar también
- `term_meta_bank_*.json` con modo `freq` (JPDB, Innocent Corpus) → `WordInfo.freq: Option<u32>` (puesto/valor).
- `kanji_bank_*.json` (KANJIDIC) → `WordInfo.kanji: Vec<{ ch, meanings, onyomi, kunyomi }>`.
- **Gramática**: el diccionario de gramática del usuario es un diccionario de términos normal. Buscar secuencias de
  tokens consecutivos (hasta 4) que empiecen por partícula/auxiliar/forma て… y que coincidan con un término de un
  diccionario marcado como gramatical (heurística: título que contenga «Grammar»/«文法», o configurable) →
  `ReaderLine.grammar: Vec<{ toks: [desde, hasta], term, defs }>`. Añadir campos con `#[serde(default)]`.

## Fase 3 · Traducción y explicación con DeepSeek (bajo demanda)

- Ajustes (`visor-core/src/config.rs` + `src/screens/Settings.svelte`): `deepseek_key` (campo de contraseña) y
  `deepseek_model` (por defecto `deepseek-chat`). La clave **solo vive en la app**; la página nunca la recibe.
- Nuevo `src-tauri/src/ai.rs`: cliente de la API compatible con OpenAI (`https://api.deepseek.com/chat/completions`,
  `ureq` ya está en `Cargo.toml`), en un hilo, con **streaming** (SSE) para la traducción.
  - `translate`: prompt corto; se envían el contexto (frases anteriores con quién habla) y la frase. Respuesta: solo
    la traducción natural al español.
  - `explain`: `response_format: json_object` con el esquema de `Explanation` (traducción, literal, cadena de piezas
    con su función, notas de matiz/gramática). Si ya hay traducción, se reutiliza en el prompt.
- **Caché**: antes de llamar, mirar el glosario (fase 1); al terminar, guardar la frase con el resultado. Volver a una
  frase o releer la novela no vuelve a llamar.
- Errores (sin clave, sin red, 401, saldo) → mensaje claro en la página, sin romper nada.

## Fase 4 · Anki (AnkiConnect) con captura

- Ajustes: URL de AnkiConnect (por defecto `http://127.0.0.1:8765`), mazo, tipo de nota y **mapeo de campos**
  (palabra, lectura, acepciones, frase con la palabra en negrita, traducción, captura, audio). Botón «Probar conexión».
  Indicador de estado de Anki en la cabecera de la página.
- Nuevo `src-tauri/src/anki.rs`: `version`, `deckNames`, `modelNames`, `modelFieldNames`, `addNote` (con `picture` y
  `audio` como archivos en `data` base64 o `path`), `guiBrowse` opcional.
- **Captura**: reutilizar la captura de ventana de `snap.rs` / `windows-capture` sobre la ventana del juego (PID del
  texthook). Se toma al llegar cada frase **solo en memoria** (la última) y se guarda en disco al pedir Anki/★; así la
  imagen corresponde a la frase aunque el juego ya haya avanzado. Miniaturas de las 3 últimas para elegir.
- Ventana de confirmación en la página (como el mockup): vista previa anverso/reverso, captura, audio, mazo y tipo
  de nota → «Enviar». Guardar `anki_note_id` en el glosario.

## Fase 5 · Grabación manual de audio + voz sintética

- Nuevo `src-tauri/src/audio.rs`: captura **solo del proceso del juego** con WASAPI *process loopback*
  (`ActivateAudioInterfaceAsync` + `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK`, Windows 10 2004+; si falla,
  loopback del dispositivo por defecto). `start(pid)`, `stop() -> wav`. Recorte automático de silencios al principio
  y al final. Guardar WAV mono 16 bits (Anki y AnkiDroid lo reproducen) en `<config>/phrases/media/`.
- **Atajo global** configurable para alternar la grabación (por defecto `F9`, con `tauri-plugin-global-shortcut`
  como `capture_hotkey`), además del botón de la página y de la ventana de Anki.
- **Indicador** sobre la pantalla del juego: ventana Tauri pequeña, transparente, siempre encima, sin foco y que no
  captura clics, en una esquina del monitor del juego: «● REC 0:03» mientras graba y «✓ 2,4 s» un momento al parar.
- El audio se asocia a la frase actual (o la elegida en el historial) y aparece en la ventana de Anki con
  reproducir / volver a grabar.
- **Voz sintética** de respaldo (frase sin audio grabado): `Windows.Media.SpeechSynthesis` (WinRT, voz japonesa
  instalada) → WAV. En la ventana de Anki, selector de audio: Grabado / Sintético / Ninguno. Marcar «voz sintética».

## Fase 6 · Glosario de frases (repaso)

- En la **página de lectura**: pestaña/panel «Glosario» (botón en la cabecera) con las frases del juego actual:
  buscar, filtrar (★, con audio, en Anki, pendientes de repaso), abrir una frase → ver traducción, explicación,
  captura, audio, nota editable, enviar a Anki.
- En la **app principal** (con mando, estilo de las demás pantallas): pantalla nueva `src/screens/Phrases.svelte`
  accesible desde la ficha de un juego de novelas (y desde Inicio si hay repasos pendientes):
  - Lista de frases por juego con captura de fondo, búsqueda y filtros.
  - **Modo repaso** (SRS): muestra la frase (con furigana opcional) y su captura → revelar traducción/explicación y
    escuchar el audio → Otra vez / Difícil / Bien / Fácil. Contador de pendientes.
- Comandos Tauri: `phrases_list`, `phrases_get`, `phrases_update` (★, nota, etiquetas), `phrases_grade`,
  `phrases_delete`, `phrases_due_count`.

## Orden y verificación

Fases en orden (1 → 6); cada una compila y se prueba antes de la siguiente. La página se verifica con
`reader.html?demo` en el servidor de Vite (configuración `vite` de `.claude/launch.json`). Lo que dependa del juego,
Anki o DeepSeek reales se deja con datos simulados en la demo y se anota en el informe como «sin probar en real».
