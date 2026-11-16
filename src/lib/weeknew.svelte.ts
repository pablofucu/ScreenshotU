// Aviso de semana nueva en Inicio: la primera vez que se abre la app desde el lunes, si la semana pasada tuvo
// actividad y aún no se ha visto, sale una tarjeta con sus cifras. Se marca vista por equipo (localStorage), así
// que el PC y la tablet la ven una vez cada uno. El mando lo atiende `weekNewCmd`, en el controlador.
import { weekStart } from './stats';

const KEY = 'visor.weekSeen';

export const wn = $state({ open: false, from: 0 });

/** Inicio (ms) de la última semana vista; 0 si ninguna. */
function seen(): number {
  try { return Number(localStorage.getItem(KEY) ?? 0) || 0; } catch { return 0; }
}
export function weekSeen(from: number) { return seen() >= from; }
export function markWeekSeen(from: number) {
  try { localStorage.setItem(KEY, String(from)); } catch { /* sin almacenamiento */ }
}

/** Inicio de la semana pasada (lunes 00:00 local). */
export function lastWeekFrom(now = Date.now()) {
  const d = new Date(weekStart(now));
  d.setDate(d.getDate() - 7);
  return d.getTime();
}

/** Abre el aviso si toca: semana pasada sin ver y con actividad (`ms` > 0). Solo una vez por semana. */
export function offerWeek(from: number, ms: number) {
  // Sin actividad no se marca vista: puede que la biblioteca aún se esté cargando.
  if (wn.open || ms <= 0 || weekSeen(from)) return;
  wn.from = from;
  wn.open = true;
}

export function closeWeek() {
  if (!wn.open) return;
  markWeekSeen(wn.from);
  wn.open = false;
}
