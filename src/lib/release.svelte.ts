// Fecha de lanzamiento de un juego: selector con mando (día · mes · año; mes y día opcionales) y textos.
// La fecha puesta a mano nunca la pisa SteamGridDB (eso lo decide el núcleo).
import { app } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { closeOverlay, popOverlay, pushOverlay, toast } from './controller.svelte';
import { patchTitle } from './ops.svelte';
import type { ReleaseDate, Title } from './types';

const MONTHS = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
const MONTHS_LONG = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre'];
export const RELEASE_MIN = 1950;
const maxYear = () => new Date().getFullYear() + 10;
const dim = (y: number, m: number) => new Date(y, m, 0).getDate();

/** «3 mar 2017», «marzo de 2017» o «2017». */
export function fmtRelease(r: ReleaseDate): string {
  if (r.month && r.day) return `${r.day} ${MONTHS[r.month - 1]} ${r.year}`;
  if (r.month) return `${MONTHS_LONG[r.month - 1]} de ${r.year}`;
  return String(r.year);
}

/** Estado del selector. `m` y `d` a 0 = sin mes / sin día. `field`: 0 día · 1 mes · 2 año. */
class ReleaseState {
  title = $state(0);
  field = $state(2);
  y = $state(2000);
  m = $state(0);
  d = $state(0);
}
export const rel = new ReleaseState();
export const MONTH_SHORT = MONTHS;

/** Abre el selector para el título `ti` de la categoría actual. Empieza en la fecha guardada o, si no hay, en el año
 *  de la primera captura (o en el actual). */
export function openRelease(ti: number) {
  const t = app.platform?.titles[ti];
  if (!t || t.members || !t.id) { toast('Este elemento no tiene fecha de lanzamiento'); return; }
  const r = t.release;
  const first = t.shots.length ? Math.min(...t.shots.map((s) => s.date)) : null;
  rel.title = ti;
  rel.field = 2;
  rel.y = r?.year ?? (first ? new Date(first).getFullYear() : new Date().getFullYear());
  rel.m = r?.month ?? 0;
  rel.d = r?.month && r.day ? r.day : 0;
  pushOverlay('release');
  blip(1320);
}

/** Sube (+1) o baja (-1) el campo elegido. Mes y día pasan por «—» (sin poner); el día necesita mes. */
function stepField(dir: number) {
  if (rel.field === 0) {
    if (!rel.m) { toast('Elige antes el mes', 1600); blip(220); return; }
    const n = dim(rel.y, rel.m) + 1;
    rel.d = (rel.d + dir + n) % n;
  } else if (rel.field === 1) {
    rel.m = (rel.m + dir + 13) % 13;
    if (!rel.m) rel.d = 0;
    else rel.d = Math.min(rel.d, dim(rel.y, rel.m));
  } else {
    rel.y = Math.max(RELEASE_MIN, Math.min(maxYear(), rel.y + dir));
    if (rel.m) rel.d = Math.min(rel.d, dim(rel.y, rel.m));
  }
  blip(880);
}

const current = (): Title | undefined => app.platform?.titles[rel.title];

async function save(date: ReleaseDate | null) {
  const t = current();
  if (!t) { closeOverlay(); return; }
  try {
    await backend.setReleaseDate(t.id, date ? { year: date.year, month: date.month ?? null, day: date.day ?? null } : null);
  } catch (e) {
    toast(String(e), 3200);
    return;
  }
  patchTitle(app.p, rel.title, { release: date ? { ...date, auto: false } : null });
  popOverlay();
  toast(date ? `«${t.name}»: salió en ${fmtRelease(date)}` : `«${t.name}»: fecha de lanzamiento quitada`);
  blip(date ? 1320 : 520);
}

export function releaseSave() {
  void save({ year: rel.y, month: rel.m || null, day: rel.m && rel.d ? rel.d : null });
}

export function releaseClear() {
  if (!current()?.release) { toast('Este juego no tiene fecha guardada', 1800); blip(220); return; }
  void save(null);
}

export function releaseCmd(c: string) {
  if (c === 'left') { rel.field = Math.max(0, rel.field - 1); blip(880); }
  else if (c === 'right') { rel.field = Math.min(2, rel.field + 1); blip(880); }
  else if (c === 'up') stepField(1);
  else if (c === 'down') stepField(-1);
  else if (c === 'lb' || c === 'rb') { rel.field = 2; rel.y = Math.max(RELEASE_MIN, Math.min(maxYear(), rel.y + (c === 'lb' ? -10 : 10))); if (rel.m) rel.d = Math.min(rel.d, dim(rel.y, rel.m)); blip(700); }
  else if (c === 'a') releaseSave();
  else if (c === 'x') releaseClear();
  else if (c === 'b') { popOverlay(); blip(520); }
  else if (c === 'menu') { closeOverlay(); blip(520); }
}

/** Para el selector con ratón: elegir campo o subir/bajar uno. */
export function releaseField(f: number) { rel.field = f; }
export function releaseStep(f: number, dir: number) { rel.field = f; stepField(dir); }
