// Textos de la racha y los días libres, sin dependencias de la app (también los usa el menú sobre el juego).
import { dayDate, type FreeDay } from './readgoals.ts';

const WD_S = ['dom', 'lun', 'mar', 'mié', 'jue', 'vie', 'sáb'];
const WD_L = ['domingo', 'lunes', 'martes', 'miércoles', 'jueves', 'viernes', 'sábado'];
const MON_L = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre'];
/** Día local como «lun 19» (corto) o «lunes 19» (largo). */
export function freeDayName(d: number, long = false) {
  const x = dayDate(d);
  return long ? `${WD_L[x.getUTCDay()]} ${x.getUTCDate()}` : `${WD_S[x.getUTCDay()]} ${x.getUTCDate()}`;
}
/** Plazo que queda para recuperar un día libre: «hoy», «hoy y mañana» o «quedan N días». */
export function freeLeft(f: FreeDay) {
  return f.left <= 1 ? 'hoy' : f.left === 2 ? 'hoy y mañana' : `quedan ${f.left} días`;
}
/** Cuándo cierra el plazo del día libre: «jueves 22 a las 7:00» (largo) o «jue 7:00» (corto). */
export function freeClose(f: FreeDay, long = false) {
  return long ? `${freeDayName(f.deadline + 1, true)} a las 7:00` : `${WD_S[dayDate(f.deadline + 1).getUTCDay()]} 7:00`;
}
export const monthName = (d: number) => MON_L[dayDate(d).getUTCMonth()];

/** Minutos como texto: corto («45m», «1h 05m») o largo («45 min», «1 h 05 min»). */
export function fmtMin(m: number, long = false) {
  const n = Math.floor(m);
  if (long) return n < 60 ? `${n} min` : n % 60 === 0 ? `${n / 60} h` : `${Math.floor(n / 60)} h ${String(n % 60).padStart(2, '0')} min`;
  return n < 60 ? `${n}m` : n % 60 === 0 ? `${n / 60}h` : `${Math.floor(n / 60)}h ${String(n % 60).padStart(2, '0')}m`;
}

/** Meta en minutos como texto corto: «1 h», «1 h 30 min», «45 min». */
export const fmtGoal = (m: number) => (m % 60 === 0 ? `${m / 60} h` : m > 60 ? `${Math.floor(m / 60)} h ${m % 60} min` : `${m} min`);
