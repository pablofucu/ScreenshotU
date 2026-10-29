// Estado reactivo y utilidades para el sistema de logros
import { app } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { closeOverlay, pushOverlay, toast } from './controller.svelte';
import type { Achievement, AchievementSummary } from './types';

export const VALID_VARIANTS = [1, 2, 4, 7, 8] as const;
export type AchievementVariant = (typeof VALID_VARIANTS)[number];

export const VARIANT_NAMES: Record<number, string> = {
  1: 'Anillo + último logro',
  2: 'Portada + banda',
  4: 'Medalla de rango',
  7: 'Contador de horas',
  8: 'Por rareza',
};

export const VARIANT_SHORT_NAMES: Record<number, string> = {
  1: 'Anillo',
  2: 'Banda',
  4: 'Medalla',
  7: 'Contador',
  8: 'Rareza',
};

export function nextVariant(v: number): number {
  const idx = VALID_VARIANTS.indexOf(v as AchievementVariant);
  if (idx === -1) return 1;
  return VALID_VARIANTS[(idx + 1) % VALID_VARIANTS.length];
}

class AchievementsState {
  summary = $state<AchievementSummary | null>(null);
  list = $state<Achievement[]>([]);
  loading = $state(false);
  gameId = $state<string | null>(null);
  gameTitle = $state<string>('');
  coverPath = $state<string | null>(null);

  // Modal interactivo
  tab = $state<'all' | 'unlocked' | 'locked'>('all');
  selectedIdx = $state(0);
  revealedSecrets = $state<Set<string>>(new Set());
}

export const achState = new AchievementsState();

export function getFilteredAchievements(): Achievement[] {
  if (achState.tab === 'unlocked') {
    return achState.list.filter((a) => a.earned);
  }
  if (achState.tab === 'locked') {
    return achState.list.filter((a) => !a.earned);
  }
  return achState.list;
}

export async function loadGameAchievements(gameId: string, appId?: string) {
  achState.gameId = gameId;
  achState.summary = null;
  achState.list = [];
  achState.loading = true;
  try {
    const [summary, list] = await Promise.all([
      backend.getAchievementsSummary(gameId, appId),
      backend.getGameAchievements(gameId, appId),
    ]);
    if (achState.gameId === gameId) {
      achState.summary = summary;
      achState.list = list;
      if (achState.selectedIdx >= list.length) {
        achState.selectedIdx = 0;
      }
    }
  } catch (err) {
    console.error('Error al cargar logros:', err);
  } finally {
    if (achState.gameId === gameId) {
      achState.loading = false;
    }
  }
}

export async function cycleCardVariant(gameId: string) {
  const cur = achState.summary?.variant ?? 1;
  const nxt = nextVariant(cur);
  if (achState.summary) {
    achState.summary.variant = nxt;
  }
  blip(1200);
  toast(`Variante de logros: ${VARIANT_NAMES[nxt]}`, 1500);
  try {
    await backend.setAchievementsCardVariant(gameId, nxt);
  } catch (err) {
    console.error('Error al guardar preferencia de variante:', err);
  }
}

export async function setCardVariant(gameId: string, variant: number) {
  if (achState.summary) {
    achState.summary.variant = variant;
  }
  blip(1200);
  try {
    await backend.setAchievementsCardVariant(gameId, variant);
  } catch (err) {
    console.error('Error al guardar preferencia de variante:', err);
  }
}

export function openAchievementsModal(gameId: string, titleName: string, cover?: string | null) {
  achState.gameId = gameId;
  achState.gameTitle = titleName;
  achState.coverPath = cover ?? null;
  achState.tab = 'all';
  achState.selectedIdx = 0;
  pushOverlay('achievements');
  blip(1320);
  void loadGameAchievements(gameId);
}

export function closeAchievementsModal() {
  closeOverlay();
  blip(800);
}

export function toggleRevealSecret(apiName: string) {
  const next = new Set(achState.revealedSecrets);
  if (next.has(apiName)) {
    next.delete(apiName);
    blip(800);
  } else {
    next.add(apiName);
    blip(1400);
    toast('Logro secreto revelado', 1500);
  }
  achState.revealedSecrets = next;
}

export function formatUnlockDate(epoch?: number | null): string {
  if (!epoch) return 'Bloqueado';
  const d = new Date(epoch * 1000);
  const now = new Date();
  const diffDays = Math.floor((now.getTime() - d.getTime()) / (1000 * 60 * 60 * 24));
  if (diffDays === 0) {
    return `Hoy a las ${d.toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' })}`;
  }
  if (diffDays === 1) {
    return `Ayer a las ${d.toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' })}`;
  }
  return d.toLocaleDateString('es-ES', { day: 'numeric', month: 'short', year: 'numeric' });
}

export function getRankTier(percentage: number): { name: string; color: string; next: string; nextTarget: number } {
  if (percentage >= 100) return { name: 'Platino', color: '#00A0D8', next: 'Máximo', nextTarget: 100 };
  if (percentage >= 75) return { name: 'Oro', color: '#F2B807', next: 'Platino', nextTarget: 100 };
  if (percentage >= 25) return { name: 'Plata', color: '#d5dbe4', next: 'Oro', nextTarget: 75 };
  return { name: 'Bronce', color: '#cd7f32', next: 'Plata', nextTarget: 25 };
}

/** Controlador para navegar el modal de logros con mando o teclado */
export function achievementsCmd(c: string) {
  const filtered = getFilteredAchievements();
  if (c === 'up') {
    if (achState.selectedIdx > 0) {
      achState.selectedIdx--;
      blip(880);
    }
  } else if (c === 'down') {
    if (achState.selectedIdx < filtered.length - 1) {
      achState.selectedIdx++;
      blip(880);
    }
  } else if (c === 'lb' || c === 'lt' || c === 'left') {
    if (achState.tab === 'locked') achState.tab = 'unlocked';
    else if (achState.tab === 'unlocked') achState.tab = 'all';
    else achState.tab = 'locked';
    achState.selectedIdx = 0;
    blip(700);
  } else if (c === 'rb' || c === 'rt' || c === 'right') {
    if (achState.tab === 'all') achState.tab = 'unlocked';
    else if (achState.tab === 'unlocked') achState.tab = 'locked';
    else achState.tab = 'all';
    achState.selectedIdx = 0;
    blip(700);
  } else if (c === 'x') {
    const cur = filtered[achState.selectedIdx];
    if (cur && cur.hidden && !cur.earned) {
      toggleRevealSecret(cur.apiName);
    }
  } else if (c === 'y') {
    if (achState.gameId) {
      void cycleCardVariant(achState.gameId);
    }
  } else if (c === 'b' || c === 'menu') {
    closeAchievementsModal();
  }
}
