export interface PlatformCfg {
  id: string;
  name: string;
  folder: string;
  accent: number;
  icon: string;
}
export interface Config {
  root: string | null;
  titlesView: 'grid' | 'carousel';
  /** Cuadrícula de capturas continua o agrupada por días. */
  shotsView: 'grid' | 'days';
  /** Orden inicial de toda lista de capturas (el orden propio de un juego, elegido con Y, manda en ese juego). */
  shotsOrder: 'recent' | 'oldest';
  /** art = del juego (automático), rainbow = arcoíris, fixed = color fijo. */
  accentMode: 'art' | 'rainbow' | 'fixed';
  accentFixed: number;
  sound: boolean;
  platforms: PlatformCfg[];
  /** Atajo global de captura ("" = desactivado). */
  captureHotkey: string;
  /** Combinación de mando de captura, `boton+boton` ("off" = desactivada). */
  captureCombo: string;
  /** Botón del ratón que captura: "middle" | "x1" | "x2" ("" = desactivado). */
  captureMouse: string;
  /** Con Magpie: guardar la imagen escalada (lo que se ve) o la original del juego. */
  captureMagpie: 'scaled' | 'source';
  captureSound: boolean;
  /** Miniatura de la captura unos instantes sobre el juego. */
  capturePeek: boolean;
  /** Vibracion corta del mando al capturar con el. */
  captureRumble: boolean;
  /** Capturar solo dentro de un juego (no en el escritorio ni en otras aplicaciones). */
  captureOnlyGames: boolean;
  /** Ruta de Magpie.exe ("" = buscarlo solo). */
  magpieExe: string;
  /** Mostrar atenuados los juegos ocultos. */
  showHidden: boolean;
  /** Clave de la API de SteamGridDB ("" = sin conexión). */
  sgdbKey: string;
  /** Descargar solos los grafismos que falten. */
  sgdbAuto: boolean;
  /** Clave de DeepSeek (traducir y explicar frases bajo demanda). Solo la usa la app. */
  deepseekKey: string;
  deepseekModel: string;
  /** AnkiConnect: URL, mazo y tipo de nota ("" = el propio, «Visor · Japonés») y campo que recibe cada dato en tipos ajenos ("" = no se envia). */
  ankiUrl: string;
  ankiDeck: string;
  ankiModel: string;
  ankiMap: AnkiMap;
  /** Diseño de la tarjeta del tipo de nota propio: limpio, escena, cuaderno, contexto. */
  ankiStyle: string;
  /** Abrir Anki minimizado al lanzar una novela japonesa. */
  ankiAutostart: boolean;
  /** Ruta de anki.exe ("" = buscarlo solo). */
  ankiExe: string;
  /** Atajo global que empieza/para la grabacion del audio del juego ("" = desactivado). */
  recordHotkey: string;
  /** Combinación de mando que cambia la salida de audio de Windows ("off" = desactivada). */
  audioCombo: string;
  /** Atajo global que cambia a la siguiente salida de audio ("" = desactivado). */
  audioHotkey: string;
  /** Copia automática de la base de datos al arrancar (cada 7 días, se conservan 5). */
  autoBackup: boolean;
  /** Abrir el visor en el monitor donde está el cursor al arrancar (en vez de en el último usado). */
  openOnCursor: boolean;
  /** Copiar las partidas guardadas de un juego al cerrarlo (si cambiaron). */
  savesAuto: boolean;
  /** Carpeta de las copias de partidas ("" = la de la configuración). */
  savesDir: string;
  /** Meta diaria de lectura en minutos: novelas visuales y novelas japonesas (anillos de Inicio). */
  goalVn: number;
  goalJvn: number;
  /** Servidor para la app del móvil (misma red): activado, puerto y código de emparejamiento (lo gestiona el núcleo). */
  remoteEnabled: boolean;
  remotePort: number;
  remoteToken: string;
  /** Orden de la lista de juegos por categoría (id de plataforma → orden; ausente = «last»). */
  titlesOrder: Record<string, TitleOrder>;
  /** Tamaño de las portadas: 0 = 5 por fila (por defecto) … 3 = 8 por fila. */
  titlesZoom: number;
  /** Estilo de la pestaña de las colecciones: Aurora o multicolor armonizada. */
  collectionsStyle: 'aurora' | 'multi';
  /** «Mi lista · Juegos parados»: avisar de los juegos en «Jugando» sin jugar desde hace `listStalledDays` días. */
  listStalled: boolean;
  /** Días sin jugar (7 a 180). */
  listStalledDays: number;
  /** Qué hacer con ellos: preguntar, pasarlos a Pendiente o a Abandonado. */
  listStalledAction: 'ask' | 'pending' | 'dropped';
  /** Al lanzar un pendiente desde el visor, pasa a «Jugando». */
  listAutoPlay: boolean;
}
/** Orden de la lista de juegos: última partida, A–Z o por color de la portada. */
export type TitleOrder = 'last' | 'az' | 'color';
export interface AnkiMap { word: string; reading: string; meaning: string; sentence: string; translation: string; shot: string; audio: string }
export interface Shot {
  path: string;
  mtime: number;
  date: number;
  width: number;
  height: number;
  /** Tamaño en bytes. */
  size?: number;
  /** Solo en la vista de una colección: juego de origen (nombre e id). */
  game?: string;
  gid?: string;
}
/** Saga antigua (la usa la app del móvil); en el escritorio son `Collection`. */
export interface Saga { id: string; name: string; members: string[] }
/** Colección: agrupación virtual de juegos de una sola categoría (no mueve ni copia capturas). `platform` null = saga
 *  antigua aún sin convertir; `color` null = automático, si no el índice en `RAINBOW`. */
export interface Collection { id: string; name: string; members: string[]; platform: string | null; color: number | null }
export interface SagaMember { id: string; name: string; cover: string | null }
/** Un color normalizado (`#rrggbb`) y si el texto encima debe ser oscuro. */
export interface Swatch { color: string; dark: boolean }
export interface AccentInfo { accent: Swatch; palette: Swatch[] }
export type SortMode = 'recent' | 'oldest' | 'favs' | 'name';
export type GameSource = 'manual' | 'emulator';
/** Estado de un juego; `none` = sin estado. */
export type GameStatus = 'none' | 'pending' | 'playing' | 'finished' | 'dropped';
export type MagpieMode = 'off' | 'auto' | '2x' | '4x';
export interface MagpieStatus { exe: string; running: boolean; cunny: boolean }
/** Estado del acceso desde el móvil. `error`: último fallo al abrir el puerto (p. ej. en uso). */
export interface RemoteInfo { enabled: boolean; running: boolean; port: number; addresses: string[]; code: string; error: string | null }
export interface Title {
  name: string;
  cover: string | null;
  /** Banner 1920x620 (hero.*) de la carpeta. */
  hero: string | null;
  /** Logo transparente (logo.png|webp) de la carpeta. */
  logo: string | null;
  shots: Shot[];
  /** Nombre real de la carpeta bajo la plataforma. */
  folder: string;
  /** Ruta completa de la carpeta de capturas (puede no existir aún). */
  dir: string;
  hasFolder: boolean;
  gameId: string | null;
  source: GameSource | null;
  launchable: boolean;
  /** Entrada manual: se puede quitar de la lista. */
  removable: boolean;
  lastPlayed: number | null;
  playSeconds: number;
  /** El emulador; null en juegos manuales o sin lanzador. */
  launcher: string | null;
  /** Partidas registradas (0 si solo hay tiempo del formato antiguo). */
  sessions: number;
  firstPlayed: number | null;
  /** Minutos jugados por día en los últimos 7 días locales; el último es hoy. */
  week: number[];
  /** Partidas `[inicio, fin]` en ms reales, de la más antigua a la más reciente. */
  plays: Array<[number, number]>;
  /** Id estable (id del lanzador o plataforma/carpeta). */
  id: string;
  hidden: boolean;
  /** Acento extraído del hero y la portada (null si no hay arte con color). */
  accent: AccentInfo | null;
  /** Acento manual `#rrggbb`. */
  accentOverride: string | null;
  sortMode: SortMode | null;
  /** Escalado con Magpie al lanzarlo (null = nunca se activó). */
  magpieMode?: MagpieMode | null;
  /** Lo último aplicado: «1280×720 → 2x · CuNNy 8x32». */
  magpieSummary?: string | null;
  /** Estado del juego (null = sin estado) y, si está terminado, cuándo (ms desde epoch). */
  status?: Exclude<GameStatus, 'none'> | null;
  finishedMs?: number | null;
  /** Nota de 1 a 5 (solo terminados), sitio entre los pendientes, cuándo se fijó el estado y «dejarlo» ante el aviso de juego parado. */
  rating?: number | null;
  listPos?: number | null;
  statusMs?: number | null;
  snoozeMs?: number | null;
  /** Fecha de lanzamiento (null = sin poner): a mano o sacada de SteamGridDB (`auto`). */
  release?: ReleaseDate | null;
  /** Solo en la vista de capturas de una colección: los juegos que la forman. */
  members?: SagaMember[];
}
export interface HiddenEntry { id: string; name: string; platform: string }
/** Fecha de lanzamiento: año siempre; mes (1-12) y día opcionales. `auto` = sacada de SteamGridDB (se puede sustituir sola). */
export interface ReleaseDate { year: number; month?: number | null; day?: number | null; auto?: boolean }
export interface NewGame {
  kind: 'exe' | 'emulator' | 'captures';
  name: string;
  exe: string;
  args: string;
  rom: string;
  platform: string;
  /** Grafismos opcionales: se copian a la carpeta del juego. */
  hero: string;
  logo: string;
  cover: string;
}
export interface AddReport { notes: string[] }
/** Subcarpeta de un directorio de juegos con los ejecutables que contiene (el más probable primero). */
export interface DirCandidate { folder: string; name: string; exes: string[] }
export interface DirScan { candidates: DirCandidate[]; empty: number; known: number }
export interface BackupResult { path: string; size: number; mediaFiles: number }
export interface RestorePreview { dbVersion: number; size: number; hasConfig: boolean; mediaFiles: number }
export interface StartupNotice { kind: 'error' | 'info'; message: string }
export interface ImportReport { copied: string[]; skipped: string[] }
export interface MoveReport { moved: Array<[string, string]>; failed: string[] }
/** `trashed`: rutas que ya no están (mandadas a la Papelera); `failed`: errores legibles. */
export interface TrashReport { trashed: string[]; failed: string[] }
/** Dispositivo de reproducción de Windows. */
export interface AudioOutput { id: string; name: string; short: string; default: boolean }
export type BackendEvent =
  | { type: 'audio-outputs'; outputs: AudioOutput[] }
  | { type: 'capture-saved'; name: string }
  | { type: 'capture-error'; message: string }
  | { type: 'learn-pad'; combo: string; connected: boolean; done: boolean }
  | { type: 'game-started'; id: string; name: string }
  | { type: 'game-exited'; id: string; name: string; seconds: number }
  | { type: 'magpie-status'; id: string; message: string; error: boolean };
export interface Platform extends PlatformCfg {
  titles: Title[];
}
export interface Library {
  platforms: Platform[];
  /** Juegos ocultos (para la lista de Ajustes). */
  hidden: HiddenEntry[];
}

export interface Backend {
  readonly isTauri: boolean;
  loadConfig(): Promise<Config>;
  saveConfig(c: Config): Promise<void>;
  pickFolder(title?: string): Promise<string | null>;
  scan(): Promise<Library>;
  /** La biblioteca del último escaneo (para pintar al instante al abrir); solo en la app. */
  cachedScan?(): Promise<Library | null>;
  /** URL de la miniatura (480 px) de una imagen. */
  thumb(path: string, signal?: AbortSignal): Promise<string>;
  /** URL de la imagen original (para el visor). */
  fullUrl(path: string): string;
  /** URL de un grafismo (hero, logo) sin miniaturizar; cambia al recargar los grafismos. */
  artUrl(path: string): string;
  /** Fuerza a releer los grafismos (nueva URL). */
  bustArt(): void;
  prefetch(paths: string[]): void;
  getFavorites(): Promise<string[]>;
  setFavorite(path: string, on: boolean): Promise<void>;
  toggleFullscreen(): Promise<void>;
  isFullscreen(): Promise<boolean>;
  /** Monitor donde está el visor («1 de 2 · 2560×1440 · principal»). */
  windowMonitor(): Promise<string>;
  /** Pasa el visor al monitor siguiente (1) o anterior (-1); devuelve el nuevo. */
  nextMonitor(dir: number): Promise<string>;
  exit(): Promise<void>;
  /** Salidas de audio de Windows (la predeterminada marcada). */
  audioOutputs(): Promise<AudioOutput[]>;
  /** Pone una salida como predeterminada; devuelve la lista actualizada. */
  setAudioOutput(id: string): Promise<AudioOutput[]>;
  /** Empieza (o deja) de esperar una combinación del mando; llegan eventos `learn-pad`. */
  learnPad(on: boolean): Promise<void>;
  /** Suelta el atajo global de captura mientras se elige otro (y lo vuelve a registrar). */
  hotkeySuspend(suspend: boolean): Promise<void>;
  /** Lanza un juego añadido (ejecutable o emulador). */
  launch(gameId: string): Promise<void>;
  /** Aviso sobre el juego a la hora `at` (ms) si la partida de `gameId` sigue en marcha (meta de lectura cumplida). */
  goalNotice(gameId: string, at: number, text: string): Promise<void>;
  openFolder(path: string): Promise<void>;
  addGame(g: NewGame): Promise<AddReport>;
  /** Busca juegos (ejecutables) en las subcarpetas de un directorio. */
  scanGamesDir(dir: string): Promise<DirScan>;
  setHidden(e: HiddenEntry, hidden: boolean): Promise<void>;
  setAccentOverride(id: string, color: string | null): Promise<void>;
  setSortPref(id: string, mode: SortMode): Promise<void>;
  /** Escalado con Magpie de un juego; se aplica al lanzarlo. */
  setMagpieMode(id: string, mode: MagpieMode): Promise<void>;
  /** Estado del juego (`none` lo quita); terminado guarda la fecha (`finishedMs`, hoy por defecto) y la nota (1-5; 0 = sin nota). */
  setTitleStatus(id: string, status: GameStatus, finishedMs?: number | null, rating?: number | null): Promise<void>;
  /** Fecha de lanzamiento puesta a mano (null la quita). */
  setReleaseDate(id: string, date: { year: number; month: number | null; day: number | null } | null): Promise<void>;
  /** Orden manual de «Mi lista › Pendientes»: los ids de izquierda a derecha. */
  setPendingOrder(ids: string[]): Promise<void>;
  /** «Dejarlo» ante el aviso de juegos parados: no se vuelve a preguntar hasta que se jueguen otra vez. */
  snoozeStalled(ids: string[]): Promise<void>;
  magpieStatus(): Promise<MagpieStatus>;
  /** Acceso desde el móvil: dirección, código y si el servidor está en marcha. */
  remoteInfo(): Promise<RemoteInfo>;
  /** Genera otro código de emparejamiento (el móvil tendrá que escribirlo de nuevo). */
  remoteNewCode(): Promise<RemoteInfo>;
  /** Colecciones guardadas (las sagas antiguas sin categoría vienen con `platform` null). */
  getCollections(): Promise<Collection[]>;
  saveCollections(list: Collection[]): Promise<Collection[]>;
  /** Una vez, con la biblioteca cargada: pasa las sagas antiguas a colecciones de la categoría donde tienen más juegos
   *  (`platforms` = id de juego → id de categoría). Devuelve la lista final. */
  convertCollections(platforms: Record<string, string>): Promise<Collection[]>;
  /** Elige imágenes (png, jpg, jpeg, webp). */
  pickImages(): Promise<string[]>;
  /** Copia imágenes a la carpeta de un juego. */
  importShots(dir: string, paths: string[], onProgress: (done: number, total: number) => void): Promise<ImportReport>;
  /** Mueve capturas a la carpeta de otro juego (dentro de la biblioteca). */
  moveShots(paths: string[], targetDir: string): Promise<MoveReport>;
  /** Manda capturas a la Papelera de Windows (nunca borrado definitivo). */
  trashShots(paths: string[]): Promise<TrashReport>;
  reveal(path: string): Promise<void>;
  /** Archivos soltados sobre la ventana. */
  onFilesDropped(cb: (paths: string[]) => void): void;
  removeGame(gameId: string): Promise<void>;
  /** Pasa un juego añadido a mano a otra categoría (mueve su carpeta de capturas). */
  moveGamePlatform(gameId: string, platform: string): Promise<void>;
  /** Cambia el ejecutable y/o la carpeta de trabajo de un juego .exe añadido a mano (`undefined` = no se toca; "" = la del ejecutable). */
  setGamePaths(gameId: string, exe?: string, workdir?: string): Promise<void>;
  pickFile(kind: 'exe' | 'rom' | 'image'): Promise<string | null>;
  /** Diálogo de guardar: devuelve la ruta elegida o null. */
  pickSaveZip(defaultName: string): Promise<string | null>;
  /** Diálogo de abrir: un .zip de copia de seguridad. */
  pickZip(): Promise<string | null>;
  /** Crea el .zip de copia de seguridad. */
  backupCreate(dest: string, includeMedia: boolean): Promise<BackupResult>;
  /** Extrae y valida un .zip; no cambia nada hasta `restoreApply`. */
  restorePrepare(zip: string): Promise<RestorePreview>;
  restoreCancel(): Promise<void>;
  /** Reinicia la aplicación; al arrancar se sustituyen la base y la configuración. */
  restoreApply(): Promise<void>;
  /** Avisos nacidos al arrancar (p. ej. la base de datos no se pudo abrir). Se entregan una vez. */
  startupNotices(): Promise<StartupNotice[]>;
  /** Eventos del núcleo: capturas guardadas, juegos que terminan... */
  onEvent(cb: (e: BackendEvent) => void): void;
}
