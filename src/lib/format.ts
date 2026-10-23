const MONTHS = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
const z = (n: number) => String(n).padStart(2, '0');
// Las fechas del nombre de archivo son "ingenuas" y el backend las codifica como UTC: se muestran en UTC.
export const fmt = (t: number) => {
  const d = new Date(t);
  return `${z(d.getUTCDate())}/${z(d.getUTCMonth() + 1)}/${d.getUTCFullYear()} ${z(d.getUTCHours())}:${z(d.getUTCMinutes())}`;
};
export const fmtShort = (t: number) => {
  const d = new Date(t);
  return `${d.getUTCDate()} ${MONTHS[d.getUTCMonth()]}`;
};
export const resText = (w: number, h: number) => (w > 0 && h > 0 ? `${w}×${h}` : '');
export const nCapturas = (n: number) => `${n} ${n === 1 ? 'captura' : 'capturas'}`;
export const nGames = (n: number) => `${n} ${n === 1 ? 'juego' : 'juegos'}`;
export const nTitulos = (n: number) => `${n} ${n === 1 ? 'título' : 'títulos'}`;
/** 3720 -> «1 h 02 min»; 2400 -> «40 min»; menos de un minuto -> «menos de 1 min». */
export const fmtDur = (sec: number) => {
  const m = Math.floor(sec / 60);
  if (m < 1) return 'menos de 1 min';
  return m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${z(m % 60)} min`;
};

const WDAYS = ['domingo', 'lunes', 'martes', 'miércoles', 'jueves', 'viernes', 'sábado'];
const MONTHS_L = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre'];
/** «Ahora» con la hora local codificada como UTC, para comparar con las fechas de las capturas. */
export const naiveNow = () => Date.now() - new Date().getTimezoneOffset() * 60000;
/** Día (AAAAMMDD) de una fecha de captura. */
export const dayKey = (t: number) => {
  const d = new Date(t);
  return d.getUTCFullYear() * 10000 + d.getUTCMonth() * 100 + d.getUTCDate();
};
/** «Hoy, viernes 16 de octubre», «Ayer, …», «sábado 6 de septiembre de 2025». */
export const fmtDay = (t: number) => {
  const d = new Date(t);
  const now = naiveNow();
  const base = `${WDAYS[d.getUTCDay()]} ${d.getUTCDate()} de ${MONTHS_L[d.getUTCMonth()]}`;
  const k = dayKey(t);
  if (k === dayKey(now)) return `Hoy, ${base}`;
  if (k === dayKey(now - 864e5)) return `Ayer, ${base}`;
  const s = base[0].toUpperCase() + base.slice(1);
  return d.getUTCFullYear() === new Date(now).getUTCFullYear() ? s : `${s} de ${d.getUTCFullYear()}`;
};
/** Mes corto para la línea de tiempo: «oct», o «oct 25» si es de otro año. */
export const fmtMonth = (t: number) => {
  const d = new Date(t);
  const y = d.getUTCFullYear();
  return y === new Date(naiveNow()).getUTCFullYear() ? MONTHS[d.getUTCMonth()] : `${MONTHS[d.getUTCMonth()]} ${String(y).slice(2)}`;
};
/** Tiempo transcurrido desde una marca real (ms): «Hace 5 min», «Hace 3 h», «Ayer», «El martes», «6 sep». */
export const fmtAgo = (ms: number) => {
  const now = new Date();
  const min = Math.floor((now.getTime() - ms) / 60000);
  if (min < 1) return 'Ahora mismo';
  if (min < 60) return `Hace ${min} min`;
  const d = new Date(ms);
  const days = Math.round((new Date(now.toDateString()).getTime() - new Date(d.toDateString()).getTime()) / 864e5);
  if (days === 0) return `Hace ${Math.floor(min / 60)} h`;
  if (days === 1) return 'Ayer';
  if (days < 7) return `El ${WDAYS[d.getDay()]}`;
  return `${d.getDate()} ${MONTHS[d.getMonth()]}`;
};
/** Versión corta para días que comparten fila: «Hoy», «Ayer», «dom 6 sep», «dom 6 sep 25». */
export const fmtDayShort = (t: number) => {
  const d = new Date(t);
  const now = naiveNow();
  const k = dayKey(t);
  if (k === dayKey(now)) return 'Hoy';
  if (k === dayKey(now - 864e5)) return 'Ayer';
  const s = `${WDAYS[d.getUTCDay()].slice(0, 3)} ${d.getUTCDate()} ${MONTHS[d.getUTCMonth()]}`;
  return d.getUTCFullYear() === new Date(now).getUTCFullYear() ? s : `${s} ${String(d.getUTCFullYear()).slice(2)}`;
};
