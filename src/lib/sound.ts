import { app } from './store.svelte';

let ac: AudioContext | undefined;

/** Clic de navegacion sintetico (sin archivos de audio). Solo suena si el ajuste esta activo. */
export function blip(f: number) {
  if (!app.config.sound) return;
  try {
    ac = ac || new (window.AudioContext || (window as any).webkitAudioContext)();
    const o = ac.createOscillator(), g = ac.createGain();
    o.type = 'sine';
    o.frequency.value = f;
    g.gain.setValueAtTime(0.0001, ac.currentTime);
    g.gain.exponentialRampToValueAtTime(0.12, ac.currentTime + 0.005);
    g.gain.exponentialRampToValueAtTime(0.0001, ac.currentTime + 0.07);
    o.connect(g).connect(ac.destination);
    o.start();
    o.stop(ac.currentTime + 0.08);
  } catch {
    /* sin audio */
  }
}
