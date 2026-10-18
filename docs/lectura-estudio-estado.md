# Lectura de estudio: estado de la implementación

Fases 1–6 de `lectura-estudio-plan.md` implementadas. Compila con `npm run tauri build -- --no-bundle` y `cargo test` en `visor-core` (137 tests).

## Dónde está cada cosa
- **Datos** (`visor-core/src/jp/`): `phrases.rs` (glosario + SRS), `known.rs`, `prompt.rs` (DeepSeek), `anki.rs` (notas/mapeo), `dict.rs` (frecuencia, kanji, gramática), `audio.rs` (recorte de silencios, WAV).
- **App** (`src-tauri/src/`): `glossary.rs` (almacén + comandos `phrases_*`), `ai.rs`, `anki.rs`, `shots.rs` (capturas en memoria + rutas `/shot`, `/audio`, `/media`), `audio.rs` (WASAPI), `voice.rs` (voz sintética), `overlay.rs` + `rec.html` (indicador REC).
- **Página** (`src/reader/`): `Reader.svelte`, `WordList.svelte`, `AnkiModal.svelte`, `Glossary.svelte`, `ExplainView.svelte`, `demo.ts` (backend simulado para `reader.html?demo`).
- **App principal**: `screens/Phrases.svelte` + `lib/phrases.svelte.ts` (overlay con mando: ficha del juego › Y › «Frases y repaso…», o Ajustes › Lectura), sección «Lectura» de Ajustes en `lib/reading.svelte.ts`.

## Datos en `<config>`
`known-words.json`, `phrases/<juego>.json`, `phrases/media/` (capturas `shot-*.jpg`, audio `audio-*.wav` y `synth-*.wav`). Diccionarios en `dicts/`; un diccionario es de gramática si su título lleva «grammar», «文法» o «gramática», o si su título/nombre de archivo contiene una línea de `dicts/grammar.txt`.

## Limitaciones conocidas
- La gramática subraya el token entero (el tokenizador junta verbo + auxiliares).
- «Ejemplo» de una palabra desplegada = la frase actual con la palabra en negrita (no se extraen ejemplos del diccionario).
- Sin probar en real: DeepSeek, AnkiConnect, captura del juego, grabación WASAPI del proceso, voz sintética, indicador REC y atajo F9.
