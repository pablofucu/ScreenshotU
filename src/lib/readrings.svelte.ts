// Lógica compartida de los anillos de lectura: la tarjeta «Lectura de hoy» de Inicio (ReadRings) y la pantalla
// completa de racha (Streak). Cada una conserva su aspecto y sus animaciones; aquí vive lo común: las pistas
// (anillos de novelas visuales / japonesas), la geometría de los anillos, qué racha está «pendiente de encender»,
// el color de las casillas del mapa y el esqueleto de la animación «la racha se enciende».
import { onDestroy, onMount, tick } from 'svelte';
import { app } from './store.svelte';
import { GOAL_COL, goalLit, goalTrack, localDay, markGoalLit, type GoalKey, type GoalTrack } from './readgoals';
import { blip } from './sound';

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

  function cellBg(t: GoalTrack, v: number, isToday: boolean) {
    if (isToday && pending(t)) v = Math.min(v, 0.6);
    const c = GOAL_COL[t.key].c;
    return v >= 1 ? c : v >= 0.5 ? c + 'aa' : v > 0 ? c + '55' : o.empty;
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
