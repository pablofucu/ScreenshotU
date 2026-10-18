// Ajustes de la seccion «Lectura»: DeepSeek (traduccion y explicacion bajo demanda), Anki (AnkiConnect) y grabacion de audio.
import { app } from './store.svelte';
import { openOsk } from './osk.svelte';
import { blip } from './sound';
import { IS_TAURI } from './backend';
import { patchConfig, toast } from './controller.svelte';
import { startLearn } from './learn.svelte';
import { openPhrases } from './phrases.svelte';
import type { AnkiMap } from './types';

export type MapSlot = keyof AnkiMap;
export type ReadingId =
  | 'dskey' | 'dsmodel'
  | 'ankiurl' | 'ankitest' | 'ankiauto' | 'ankiexe' | 'ankideck' | 'ankicreate' | 'ankimodel' | 'ankistyle'
  | 'rechotkey' | 'phrasesreview'
  | 'map_word' | 'map_reading' | 'map_meaning' | 'map_sentence' | 'map_translation' | 'map_shot' | 'map_audio';

export const READING_ROWS: ReadingId[] = [
  'dskey', 'dsmodel',
  'ankiurl', 'ankitest', 'ankiauto', 'ankiexe', 'ankideck', 'ankicreate', 'ankimodel', 'ankistyle',
  'map_word', 'map_reading', 'map_meaning', 'map_sentence', 'map_translation', 'map_shot', 'map_audio',
  'rechotkey', 'phrasesreview',
];
export const isReadingId = (id: string): id is ReadingId => (READING_ROWS as string[]).includes(id);
export const READING_CYCLE = new Set<ReadingId>(['dsmodel', 'ankideck', 'ankimodel', 'ankistyle', 'map_word', 'map_reading', 'map_meaning', 'map_sentence', 'map_translation', 'map_shot', 'map_audio', 'rechotkey']);

const REC_PRESETS = ['F9', 'F8', 'F7', 'Ctrl+F9', 'Alt+F9', ''];

const DS_MODELS = ['deepseek-chat', 'deepseek-reasoner'];

/** Tipo de nota propio de la app (se crea solo en Anki) y sus diseños de tarjeta. */
export const OWN_MODEL = 'Visor · Japonés';
const STYLES: { id: string; name: string; d: string }[] = [
  { id: 'limpio', name: 'Limpio', d: 'palabra grande centrada y la frase debajo; detrás lectura, acepciones y la captura.' },
  { id: 'escena', name: 'Escena de juego', d: 'la captura de fondo con la frase en un cuadro de diálogo de novela visual.' },
  { id: 'cuaderno', name: 'Cuaderno', d: 'papel crema y letra mincho, la palabra en vertical y el juego como sello rojo.' },
  { id: 'contexto', name: 'Contexto primero', d: 'solo la frase con la palabra subrayada (la captura como pista); detrás todo en fichas.' },
];
const styleOf = (id: string) => STYLES.find((s) => s.id === id) ?? STYLES[0];

const SLOT: Record<string, { slot: MapSlot; l: string; d: string }> = {
  map_word: { slot: 'word', l: 'Campo · palabra', d: 'Campo del tipo de nota que recibe la palabra (en las notas de frase recibe la frase entera).' },
  map_reading: { slot: 'reading', l: 'Campo · lectura', d: 'Lectura en kana de la palabra.' },
  map_meaning: { slot: 'meaning', l: 'Campo · acepciones', d: 'Acepciones del diccionario (en las notas de frase, las palabras de la frase con su significado).' },
  map_sentence: { slot: 'sentence', l: 'Campo · frase', d: 'La frase donde salió, con la palabra en negrita.' },
  map_translation: { slot: 'translation', l: 'Campo · traducción', d: 'Traducción de la frase, si ya la pediste a DeepSeek.' },
  map_shot: { slot: 'shot', l: 'Campo · captura', d: 'Imagen de la captura del juego.' },
  map_audio: { slot: 'audio', l: 'Campo · audio', d: 'Audio de la frase (grabado del juego o voz sintética).' },
};

/** Lo ultimo que Anki ha contestado: mazos, tipos de nota y campos del tipo elegido. */
const rd = $state({ decks: [] as string[], models: [] as string[], fields: [] as string[], status: '' });

const secret = (k: string | undefined) => {
  const v = k?.trim() ?? '';
  return v ? `••••${v.slice(-4)}` : 'Sin clave';
};

export function readingRow(id: ReadingId): { l: string; v: string; d: string } {
  switch (id) {
    case 'dskey':
      return { l: 'Clave de DeepSeek', v: secret(app.config.deepseekKey), d: 'Clave de API de platform.deepseek.com. Sirve para traducir y explicar frases en la página de lectura, SOLO cuando tú lo pides (traducción tapada, «Estoy perdido»). La clave se queda en la aplicación: la página de lectura nunca la recibe. Pulsa A para escribirla o pegarla con Ctrl+V.' };
    case 'dsmodel':
      return { l: 'Modelo de DeepSeek', v: app.config.deepseekModel || 'deepseek-chat', d: 'Pulsa A para escribir otro nombre de modelo, o ‹ › para alternar entre deepseek-chat (rápido y barato, recomendado) y deepseek-reasoner.' };
    case 'ankiurl':
      return { l: 'Anki · dirección de AnkiConnect', v: app.config.ankiUrl, d: 'Anki debe estar abierto con el complemento AnkiConnect (código 2055492159). Por defecto http://127.0.0.1:8765. Pulsa A para cambiarla.' };
    case 'ankitest':
      return { l: 'Anki · probar conexión', v: rd.status, d: 'Comprueba AnkiConnect y trae la lista de mazos y tipos de nota para elegir con ‹ ›. El resultado también sale en la cabecera de la página de lectura.' };
    case 'ankideck':
      return { l: 'Anki · mazo', v: app.config.ankiDeck || '(el primero)', d: 'Mazo donde van las notas. ‹ › recorre los de Anki (pulsa antes «probar conexión»); A para escribirlo. También se puede elegir en cada envío.' };
    case 'ankicreate':
      return { l: 'Anki · crear mazo', v: app.config.ankiDeck || '', d: `Crea en Anki el mazo elegido arriba y el tipo de nota «${OWN_MODEL}» si todavía no existen (por defecto Japonés::Novelas). También se crean solos al enviar la primera nota.` };
    case 'ankimodel':
      return { l: 'Anki · tipo de nota', v: app.config.ankiModel || `${OWN_MODEL} (propio)`, d: `Tipo de nota. Por defecto «${OWN_MODEL}», el de la app: se crea solo en Anki con el diseño elegido abajo y no hay que asignar campos. ‹ › recorre también los tuyos (Lapis, etc.); con uno ajeno se adivina qué campo recibe cada dato (revisa las filas siguientes). A para escribirlo.` };
    case 'ankistyle': {
      const s = styleOf(app.config.ankiStyle);
      return { l: 'Anki · diseño de tarjeta', v: s.name, d: `${s.name}: ${s.d} ‹ › cambia de diseño y A lo aplica en Anki (cambia también las tarjetas ya creadas del tipo «${OWN_MODEL}»).` };
    }
    case 'ankiauto':
      return { l: 'Anki · abrir al jugar', v: app.config.ankiAutostart ? 'Sí' : 'No', d: 'Al lanzar una novela japonesa, si Anki no está abierto se abre minimizado (sin quitar el foco al juego) para poder enviar tarjetas desde la página de lectura.' };
    case 'ankiexe':
      return { l: 'Anki · programa', v: app.config.ankiExe || 'Buscar solo', d: 'Ruta de anki.exe. Vacío = se busca en las carpetas de instalación habituales (Archivos de programa\\Anki, AppData\\Local\\Programs\\Anki). Pulsa A para escribirla.' };
    case 'phrasesreview':
      return { l: 'Frases guardadas · repasar', v: '', d: 'Abre el glosario de frases de todos los juegos con su repaso (SRS): ves la frase, descubres la traducción y la calificas con Otra vez / Difícil / Bien / Fácil. Cada juego también tiene «Frases y repaso…» en su ficha (Y) si es una novela japonesa.' };
    case 'rechotkey':
      return { l: 'Grabar audio del juego', v: app.config.recordHotkey || 'Desactivado', d: 'Atajo global (funciona con el juego delante) que EMPIEZA la grabación del audio del juego y, al pulsarlo otra vez, la PARA. Se graba solo el proceso del juego, sin música de fondo de otros programas, y se recortan los silencios. Sale un indicador «REC» en la pantalla del juego. Pulsa A y luego la tecla; ‹ › recorre algunas de serie.' };
    default: {
      const s = SLOT[id];
      if (!app.config.ankiModel || app.config.ankiModel === OWN_MODEL) return { l: s.l, v: 'Automático', d: `Con el tipo de nota propio «${OWN_MODEL}» los campos se rellenan solos. Esta fila solo sirve con un tipo de nota ajeno.` };
      return { l: s.l, v: app.config.ankiMap[s.slot] || 'No se envía', d: `${s.d} ‹ › recorre los campos del tipo de nota (o «No se envía»); A para escribir el nombre.` };
    }
  }
}

/* ---------- acceso al nucleo (o simulado en el navegador) ---------- */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!IS_TAURI) {
    if (cmd === 'anki_test') return { ok: true, deck: 'Japonés::Minado', model: 'Lapis', decks: ['Japonés::Minado', 'Japonés::Novelas'], models: ['Lapis', 'Básico'] } as T;
    if (cmd === 'anki_create_deck') return app.config.ankiDeck as T;
    if (cmd === 'anki_apply_style') return styleOf(app.config.ankiStyle).name as T;
    if (cmd === 'anki_fields') return ['Expression', 'ExpressionReading', 'MainDefinition', 'Sentence', 'SentenceAudio', 'Picture'] as T;
    return { word: 'Expression', reading: 'ExpressionReading', meaning: 'MainDefinition', sentence: 'Sentence', translation: '', shot: 'Picture', audio: 'SentenceAudio' } as T;
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Error desconocido');

async function loadFields(model: string) {
  try { rd.fields = model ? await call<string[]>('anki_fields', { model }) : []; } catch { rd.fields = []; }
}

async function testAnki() {
  rd.status = 'Probando…';
  try {
    const s = await call<{ ok: boolean; decks: string[]; models: string[]; model: string; error?: string }>('anki_test');
    rd.decks = s.decks ?? [];
    rd.models = s.models ?? [];
    rd.status = s.ok ? `Conectado · ${rd.decks.length} mazos` : 'Sin conexión';
    if (s.ok && app.config.ankiModel) await loadFields(app.config.ankiModel);
    toast(s.ok ? 'Anki conectado' : (s.error ?? 'No se pudo conectar con Anki'), s.ok ? 2200 : 4200);
  } catch (e) {
    rd.status = 'Sin conexión';
    toast(errText(e), 4200);
  }
}

/** Cambia el tipo de nota y adivina el mapeo de campos. */
async function setModel(model: string) {
  if (model === OWN_MODEL) model = '';
  await patchConfig({ ankiModel: model });
  if (!model) { rd.fields = []; toast(`Tipo de nota propio «${OWN_MODEL}»: los campos se rellenan solos`, 2800); return; }
  try {
    const map = await call<AnkiMap>('anki_guess_map', { model });
    await patchConfig({ ankiMap: map });
    await loadFields(model);
    toast('Campos asignados solos: revísalos en las filas siguientes', 3200);
  } catch (e) {
    rd.fields = [];
    toast(errText(e), 3600);
  }
}

function cycle(list: string[], cur: string, dir: number, withNone = false): string {
  const all = withNone ? ['', ...list] : list;
  const i = Math.max(0, all.indexOf(cur));
  return all[(i + dir + all.length) % all.length];
}

export function readingAct(id: ReadingId, dir: number) {
  if (id === 'dskey' && dir === 0) {
    openOsk({
      label: 'Clave de API de DeepSeek',
      hint: 'platform.deepseek.com → API keys. Puedes pegarla con Ctrl+V.',
      value: app.config.deepseekKey ?? '',
      maxLen: 80,
      onDone: (v) => {
        const key = v.replace(/\s+/g, '');
        void patchConfig({ deepseekKey: key }).then(() => toast(key ? 'Clave de DeepSeek guardada' : 'Clave quitada'));
      },
    });
  } else if (id === 'dsmodel') {
    if (dir === 0) {
      openOsk({ label: 'Modelo de DeepSeek', value: app.config.deepseekModel ?? '', maxLen: 60, hint: 'Por ejemplo deepseek-chat', onDone: (v) => void patchConfig({ deepseekModel: v.trim() || 'deepseek-chat' }) });
    } else {
      void patchConfig({ deepseekModel: cycle(DS_MODELS, app.config.deepseekModel, dir) });
      blip(880);
    }
  } else if (id === 'ankiurl' && dir === 0) {
    openOsk({ label: 'Dirección de AnkiConnect', value: app.config.ankiUrl, maxLen: 120, hint: 'Por defecto http://127.0.0.1:8765', onDone: (v) => void patchConfig({ ankiUrl: v.trim() || 'http://127.0.0.1:8765' }) });
  } else if (id === 'ankitest' && dir === 0) {
    void testAnki();
  } else if (id === 'ankideck') {
    if (dir === 0) openOsk({ label: 'Mazo de Anki', value: app.config.ankiDeck, maxLen: 120, hint: 'Ejemplo: Japonés::Minado', onDone: (v) => void patchConfig({ ankiDeck: v.trim() }) });
    else if (!rd.decks.length) toast('Pulsa antes «probar conexión» para traer los mazos', 2600);
    else { void patchConfig({ ankiDeck: cycle(rd.decks, app.config.ankiDeck, dir) }); blip(880); }
  } else if (id === 'ankicreate' && dir === 0) {
    void call<string>('anki_create_deck').then((d) => { toast(`Mazo «${d}» listo en Anki`, 2600); void testAnki(); }, (e) => toast(errText(e), 4200));
  } else if (id === 'ankimodel') {
    if (dir === 0) openOsk({ label: 'Tipo de nota de Anki', value: app.config.ankiModel, maxLen: 120, hint: 'Ejemplo: Lapis', onDone: (v) => void setModel(v.trim()) });
    else if (!rd.models.length) toast('Pulsa antes «probar conexión» para traer los tipos de nota', 2600);
    else { void setModel(cycle(rd.models.filter((m) => m !== OWN_MODEL), app.config.ankiModel, dir, true)); blip(880); }
  } else if (id === 'ankistyle') {
    if (dir === 0) {
      void call<string>('anki_apply_style').then((n) => toast(`Diseño «${n}» aplicado en Anki`, 2600), (e) => toast(errText(e), 4200));
    } else {
      void patchConfig({ ankiStyle: cycle(STYLES.map((s) => s.id), app.config.ankiStyle, dir) });
      blip(880);
    }
  } else if (id === 'ankiauto' && dir === 0) {
    void patchConfig({ ankiAutostart: !app.config.ankiAutostart });
    blip(1320);
  } else if (id === 'ankiexe' && dir === 0) {
    openOsk({ label: 'Ruta de anki.exe', value: app.config.ankiExe, maxLen: 260, hint: 'Vacío = buscarlo solo. Ejemplo: C:\\Program Files\\Anki\\anki.exe', onDone: (v) => void patchConfig({ ankiExe: v.trim().replace(/^"|"$/g, '') }) });
  } else if (id === 'phrasesreview' && dir === 0) {
    void openPhrases(null, 'todos los juegos', false);
  } else if (id === 'rechotkey') {
    if (dir === 0) startLearn('key', 'record');
    else { void patchConfig({ recordHotkey: cycle(REC_PRESETS, app.config.recordHotkey, dir) }); blip(880); }
  } else if (id.startsWith('map_')) {
    const slot = SLOT[id].slot;
    const set = (f: string) => void patchConfig({ ankiMap: { ...app.config.ankiMap, [slot]: f.trim() } });
    if (dir === 0) openOsk({ label: SLOT[id].l, value: app.config.ankiMap[slot], maxLen: 60, hint: 'Nombre exacto del campo; vacío = no se envía', onDone: set });
    else if (!rd.fields.length) toast('Pulsa antes «probar conexión» para traer los campos', 2600);
    else { set(cycle(rd.fields, app.config.ankiMap[slot], dir, true)); blip(880); }
  }
}
