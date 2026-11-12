// Solicitudes de emparejamiento pendientes en el PC principal. El modal (components/PairRequest.svelte) enseña la
// más antigua; mientras esté abierto, `pairCmd` (enganchado al principio de `cmd`) recoge A/B y traga el resto.
import { backend } from './backend';
import { clientMode } from './ipc';
import { toast } from './controller.svelte';
import { blip } from './sound';
import type { PairRequestInfo } from './types';

export const pr = $state<{ list: PairRequestInfo[]; busy: boolean; now: number }>({ list: [], busy: false, now: Date.now() });

/** La más antigua (las que ya han caducado no se enseñan). */
export const current = () => pr.list[0] ?? null;

export async function refreshPairs() {
  try {
    const l = await backend.pairPending();
    pr.list = l.filter((r) => r.expiresMs > Date.now()).sort((a, b) => a.expiresMs - b.expiresMs);
  } catch { pr.list = []; }
}

export async function answerPair(ok: boolean) {
  const r = current();
  if (!r || pr.busy) return;
  pr.busy = true;
  try {
    if (ok) { await backend.pairApprove(r.id); toast(`${r.name} conectado`); }
    else await backend.pairReject(r.id);
    blip(ok ? 1320 : 520);
  } catch (e) {
    toast(`No se pudo responder: ${e}`);
  } finally {
    pr.list = pr.list.filter((x) => x.id !== r.id);
    pr.busy = false;
    void refreshPairs();
  }
}

/** Gancho del mando: true = la orden ya está atendida (con el modal abierto no llega a la app de debajo). */
export function pairCmd(c: string): boolean {
  if (!current()) return false;
  if (c === 'a') void answerPair(true);
  else if (c === 'b') void answerPair(false);
  return true;
}

let started = false;
/** Solo en el escritorio y sin ser cliente de otro PC. */
export async function startPairs() {
  if (started) return;
  started = true;
  const { ready, listen } = await import('./ipc');
  await ready;
  if (clientMode || !('__TAURI_INTERNALS__' in window)) return;
  await refreshPairs();
  try {
    await listen<PairRequestInfo>('pair-request', (e) => {
      const r = e.payload;
      if (r && !pr.list.some((x) => x.id === r.id)) pr.list = [...pr.list, r].sort((a, b) => a.expiresMs - b.expiresMs);
    });
    await listen('pair-requests-changed', () => { void refreshPairs(); });
  } catch { /* sin Tauri (navegador): no hay eventos */ }
  setInterval(() => {
    pr.now = Date.now();
    if (pr.list.length && pr.list[0].expiresMs <= pr.now) pr.list = pr.list.filter((r) => r.expiresMs > pr.now);
  }, 1000);
}
