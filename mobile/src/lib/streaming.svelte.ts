// Streaming con Moonlight (solo en tablet): estado del plugin nativo `Streaming` y ajustes del usuario.
// En el móvil (pantalla pequeña) nada de esto se muestra y la app se comporta como antes.
import { registerPlugin } from '@capacitor/core';
import { normCode, type ServerCfg } from './source';

interface StreamingPlugin {
  isTablet(): Promise<{ tablet: boolean }>;
  isMoonlightInstalled(): Promise<{ installed: boolean }>;
  stream(o: { pc: string; app: string }): Promise<void>;
  setConnection(o: { base: string; code: string }): Promise<void>;
  openAccessibilitySettings(): Promise<void>;
  isCaptureServiceEnabled(): Promise<{ enabled: boolean }>;
}
export const Streaming = registerPlugin<StreamingPlugin>('Streaming');

const KEY = 'visor.streaming';
interface Saved { pc: string; app: string }
function load(): Saved {
  try { return { pc: '', app: 'Desktop', ...JSON.parse(localStorage.getItem(KEY) || '{}') }; } catch { return { pc: '', app: 'Desktop' }; }
}

class StreamState {
  /** Pantalla de tablet (ancho mínimo ≥ 600 dp, lo decide el sistema). */
  tablet = $state(false);
  moonlight = $state(false);
  service = $state(false);
  /** Nombre del PC en Moonlight; vacío = el nombre que da el visor en /hello. */
  pc = $state(load().pc);
  /** Aplicación de Sunshine; «Desktop» es la de serie. */
  appName = $state(load().app);

  async init() {
    try { this.tablet = (await Streaming.isTablet()).tablet; } catch { this.tablet = false; }
    if (this.tablet) await this.refresh();
  }
  /** Vuelve a mirar si Moonlight está instalado y si el servicio de la tecla está activo (al volver de Ajustes). */
  async refresh() {
    try { this.moonlight = (await Streaming.isMoonlightInstalled()).installed; } catch { this.moonlight = false; }
    try { this.service = (await Streaming.isCaptureServiceEnabled()).enabled; } catch { this.service = false; }
  }
  save() {
    try { localStorage.setItem(KEY, JSON.stringify({ pc: this.pc.trim(), app: this.appName.trim() || 'Desktop' })); } catch { /* sin almacenamiento */ }
  }
  /** Nombre del PC que se le pasa a Moonlight. */
  pcName(fallback: string) { return this.pc.trim() || fallback; }
  async openService() { await Streaming.openAccessibilitySettings(); }
}
export const stream = new StreamState();
void stream.init();

/** Pasa al servicio nativo dónde está el visor y el código, para que pueda pedir la captura con la tecla de volumen. */
export function pushConnection(s: ServerCfg | 'demo' | null) {
  const o = s && s !== 'demo' ? { base: `http://${s.host}:${s.port}`, code: normCode(s.code) } : { base: '', code: '' };
  Streaming.setConnection(o).catch(() => { /* fuera de Android */ });
}
