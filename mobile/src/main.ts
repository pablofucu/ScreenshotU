// Arranque: el móvil muestra su interfaz de siempre; la tablet monta la del visor de escritorio (ver tablet.ts).
import { Streaming } from './lib/streaming.svelte';

/** Tablet = pantalla grande (lo decide el sistema con el ancho mínimo ≥ 600 dp); fuera de Android, por el tamaño de pantalla. */
async function isTablet(): Promise<boolean> {
  const q = new URLSearchParams(location.search);
  if (q.has('phone')) return false;
  if (q.has('tablet')) return true;
  try { return (await Streaming.isTablet()).tablet; } catch { return Math.min(screen.width, screen.height) >= 600; }
}

if (await isTablet()) await import('./tablet');
else await import('./phone');
