// Lógica compartida de los anillos de lectura: la tarjeta «Lectura de hoy» de Inicio (ReadRings) y la pantalla
// completa de racha (Streak). Cada una conserva su aspecto y sus animaciones; aquí vive lo común: las pistas
// (anillos de novelas visuales / japonesas), la geometría de los anillos, qué racha está «pendiente de encender»,
// el color de las casillas del mapa y el esqueleto de la animación «la racha se enciende».
import { onDestroy, onMount, tick } from 'svelte';
import { app } from './store.svelte';
import { GOAL_COL, dayDate, goalLit, goalTrack, localDay, markGoalLit, type GoalKey, type GoalTrack, type Rescue } from './readgoals';
import { blip } from './sound';

const WD_S = ['dom', 'lun', 'mar', 'mié', 'jue', 'vie', 'sáb'];
const WD_L = ['domingo', 'lunes', 'martes', 'miércoles', 'jueves', 'viernes', 'sábado'];
const MON_L = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre'];
/** Día local como «Lun 19» (corto) o «lunes 19» (largo). */
export function rescueDay(d: number, long = false) {
  const x = dayDate(d);
  return long ? `${WD_L[x.getUTCDay()]} ${x.getUTCDate()}` : `${WD_S[x.getUTCDay()][0].toUpperCase()}${WD_S[x.getUTCDay()].slice(1)} ${x.getUTCDate()}`;
}
/** Cuenta atrás del rescate: «32 h» (o «45 min» en la última hora). */
export function rescueLeft(r: Rescue, now: number) {
  const ms = Math.max(0, r.closeAt - now);
  return ms < 3600000 ? `${Math.max(1, Math.ceil(ms / 60000))} min` : `${Math.floor(ms / 3600000)} h`;
}
/** Cuándo cierra el rescate: «jueves 22 a las 7:00» (largo) o «jue 7:00» (corto). */
export function rescueClose(r: Rescue, long = false) {
  return long ? `${rescueDay(r.to + 1, true)} a las 7:00` : `${WD_S[dayDate(r.to + 1).getUTCDay()]} 7:00`;
}
/** Nombre largo del día fallado: «domingo 18». */
export const rescueFailed = (r: Rescue) => rescueDay(r.day, true);
export const monthName = (d: number) => MON_L[dayDate(d).getUTCMonth()];

/** Minutos como texto: corto («45m», «1h 05m») o largo («45 min», «1 h 05»). */
export function fmtMin(m: number, long = false) {
  const n = Math.floor(m);
  if (long) return n < 60 ? `${n} min` : `${Math.floor(n / 60)} h ${String(n % 60).padStart(2, '0')}`;
  return n < 60 ? `${n}m` : `${Math.floor(n / 60)}h ${String(n % 60).padStart(2, '0')}m`;
}

export interface ReadRingsOpts {
  /** Semanas del mapa; sin valor, el de por defecto de `goalTrack` (MAP_WEEKS). */
  weeks?: number;
  /** Diámetro del conjunto de anillos y grosor de cada uno. */
  size: number;
  stroke: number;
  /** Separación entre anillos, en múltiplos del grosor. */
  gap: number;
  /** Color de una casilla del mapa sin lectura. */
  empty: string;
}

/** Cómo se anima el número de la racha al encenderse (lo único que cambia entre pantallas). */
export interface LightSpec {
  /** Selector de la medalla dentro de `root` y del número dentro de la medalla. */
  box: string;
  num: string;
  /** Animación de salida del número viejo. */
  out: Keyframe[];
  outOpts: KeyframeAnimationOptions;
  /** Si devuelve true no se anima: la racha se enciende de golpe. */
  skip?: () => boolean;
  /** Animaciones de entrada del número nuevo y del resto de la medalla. */
  enter: (box: HTMLElement, num: HTMLElement) => void;
}

/** Debe llamarse durante la inicialización de un componente (usa onMount/onDestroy). */
export function createReadRings(o: ReadRingsOpts) {
  const now = $derived(app.now.getTime());
  const today = $derived(localDay(now));
  const goalOf = (k: GoalKey) => (k === 'vn' ? app.config.goalVn : app.config.goalJvn);
  const tracks = $derived(
    (['vn', 'jvn'] as GoalKey[])
      .filter((k) => app.lib.platforms.some((p) => p.id === k))
      .map((k) => goalTrack(k, goalOf(k), app.lib.platforms, app.playing, now, o.weeks)),
  );
  const rings = $derived(tracks.map((t, i) => {
    const r = o.size / 2 - o.stroke / 2 - 1 - i * o.stroke * o.gap;
    return { t, r, C: 2 * Math.PI * r };
  }));

  // Racha ya celebrada (día local) por anillo: se guarda para no repetirla al volver a la pantalla.
  let lit = $state<Partial<Record<GoalKey, number>>>({});
  /** Meta cumplida hoy cuya racha aún no se ha «encendido»: se enseña el día anterior hasta la animación. */
  const pending = (t: GoalTrack) => t.done && lit[t.key] !== today && !goalLit(t.key, today);
  const shown = (t: GoalTrack) => (pending(t) ? t.streak - 1 : t.streak);

  /** Degradado continuo: de muy pálido (poco leído) a tono medio al cumplir la meta (60 %), y de ahí a color pleno a 3× la meta. */
  function cellBg(t: GoalTrack, r: number, isToday: boolean) {
    if (isToday && pending(t)) r = Math.min(r, 0.6);
    if (!(r > 0)) return o.empty;
    const p = r < 1 ? 15 + 45 * r : 60 + 40 * Math.min(1, (r - 1) / 2);
    return `color-mix(in srgb, ${GOAL_COL[t.key].c} ${p.toFixed(1)}%, transparent)`;
  }

  // Entrada: los anillos se dibujan al montar (dos fotogramas después, para que la transición arranque).
  let drawn = $state(false);
  let mountedAt = 0;
  const timers: ReturnType<typeof setTimeout>[] = [];
  onMount(() => {
    mountedAt = Date.now();
    requestAnimationFrame(() => requestAnimationFrame(() => (drawn = true)));
  });
  onDestroy(() => timers.forEach(clearTimeout));

  /** «La racha se enciende»: gira el número y la medalla; `spec.enter` pone el resto de animaciones. */
  function light(k: GoalKey, root: HTMLElement | undefined, spec: LightSpec) {
    const day = today;
    const done = () => {
      lit[k] = day;
      markGoalLit(k, day);
    };
    const box = root?.querySelector<HTMLElement>(spec.box.replace('{k}', k));
    const num = box?.querySelector<HTMLElement>(spec.num);
    if (!box || !num || spec.skip?.()) { done(); return; }
    num.animate(spec.out, spec.outOpts).onfinish = async () => {
      done();
      await tick();
      num.getAnimations().forEach((a) => a.cancel());
      spec.enter(box, num);
      blip(1320);
      setTimeout(() => blip(1760), 110);
    };
  }

  return {
    get now() { return now; },
    get today() { return today; },
    get tracks() { return tracks; },
    get rings() { return rings; },
    get drawn() { return drawn; },
    get mountedAt() { return mountedAt; },
    timers,
    goalOf,
    pending,
    shown,
    cellBg,
    light,
  };
}
