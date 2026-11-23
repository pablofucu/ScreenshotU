import { app } from './store.svelte';
import { moveTone, panFor } from './juice';

let ac: AudioContext | undefined;
let noiseBuf: AudioBuffer | undefined;
let lastMove = 0;
let streak = 0;
let modalAt = -999;

function ctx(): AudioContext | null {
  if (!app.config.sound) return null;
  try {
    const Ctor = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    if (!ac) ac = new Ctor();
    if (ac.state === 'suspended') void ac.resume();
    return ac;
  } catch {
    return null;
  }
}

function panOut(c: AudioContext, pan: number): AudioNode {
  if (Math.abs(pan) > 0.01 && typeof c.createStereoPanner === 'function') {
    try {
      const p = c.createStereoPanner();
      p.pan.setValueAtTime(Math.max(-1, Math.min(1, pan)), c.currentTime);
      p.connect(c.destination);
      return p;
    } catch {
      /* sin StereoPannerNode */
    }
  }
  return c.destination;
}

interface ToneOpts {
  type?: OscillatorType;
  f0: number;
  f1?: number;
  delay?: number;
  dur: number;
  gain: number;
}

function tone(c: AudioContext, dest: AudioNode, o: ToneOpts) {
  const t0 = c.currentTime + (o.delay ?? 0);
  const osc = c.createOscillator();
  const g = c.createGain();
  osc.type = o.type ?? 'sine';
  osc.frequency.setValueAtTime(Math.max(20, o.f0), t0);
  if (o.f1 && o.f1 !== o.f0) {
    osc.frequency.exponentialRampToValueAtTime(Math.max(20, o.f1), t0 + o.dur);
  }
  g.gain.setValueAtTime(0.0001, t0);
  g.gain.exponentialRampToValueAtTime(o.gain, t0 + 0.004);
  g.gain.exponentialRampToValueAtTime(0.0001, t0 + o.dur);
  osc.connect(g).connect(dest);
  osc.start(t0);
  osc.stop(t0 + o.dur + 0.01);
}

interface WhooshOpts {
  delay?: number;
  dur: number;
  f0: number;
  f1: number;
  q?: number;
  gain: number;
}

function whoosh(c: AudioContext, dest: AudioNode, o: WhooshOpts) {
  if (!noiseBuf || noiseBuf.sampleRate !== c.sampleRate) {
    const len = Math.max(1, Math.ceil(c.sampleRate * 0.08));
    noiseBuf = c.createBuffer(1, len, c.sampleRate);
    const ch = noiseBuf.getChannelData(0);
    for (let i = 0; i < len; i++) ch[i] = Math.random() * 2 - 1;
  }
  const t0 = c.currentTime + (o.delay ?? 0);
  const src = c.createBufferSource();
  src.buffer = noiseBuf;
  src.loop = true;
  const filt = c.createBiquadFilter();
  filt.type = 'bandpass';
  filt.Q.setValueAtTime(o.q ?? 2.2, t0);
  filt.frequency.setValueAtTime(Math.max(60, o.f0), t0);
  filt.frequency.exponentialRampToValueAtTime(Math.max(60, o.f1), t0 + o.dur);
  const g = c.createGain();
  g.gain.setValueAtTime(0.0001, t0);
  g.gain.exponentialRampToValueAtTime(o.gain, t0 + o.dur * 0.25);
  g.gain.exponentialRampToValueAtTime(0.0001, t0 + o.dur);
  src.connect(filt).connect(g).connect(dest);
  src.start(t0);
  src.stop(t0 + o.dur + 0.01);
}

function run(pan: number, fn: (c: AudioContext, dest: AudioNode) => void) {
  const c = ctx();
  if (!c) return;
  try {
    fn(c, panOut(c, pan));
  } catch {
    /* sin audio */
  }
}

/** Vocabulario sonoro sintético estilo Wii U (cristal, dirección y estéreo). */
export const sfx = {
  move(dx = 0, dy = 0, normX = 0.5) {
    const now = performance.now();
    streak = now - lastMove < 210 ? Math.min(6, streak + 1) : 0;
    lastMove = now;
    const f = moveTone({ dx, dy, streak });
    run(panFor(dx, normX), (c, d) => {
      tone(c, d, { type: 'triangle', f0: f * 1.03, f1: f, dur: 0.044, gain: 0.075 });
      tone(c, d, { type: 'sine', f0: f * 2, dur: 0.026, gain: 0.022 });
    });
  },
  confirm(normX = 0.5) {
    run(panFor(0, normX), (c, d) => {
      tone(c, d, { type: 'triangle', f0: 659, f1: 698, dur: 0.052, gain: 0.08 });
      tone(c, d, { type: 'sine', f0: 988, delay: 0.036, dur: 0.095, gain: 0.085 });
      tone(c, d, { type: 'sine', f0: 1976, delay: 0.036, dur: 0.045, gain: 0.022 });
    });
  },
  back() {
    run(0, (c, d) => {
      tone(c, d, { type: 'sine', f0: 600, f1: 405, dur: 0.085, gain: 0.08 });
      tone(c, d, { type: 'triangle', f0: 440, f1: 310, dur: 0.07, gain: 0.025 });
    });
  },
  tab(dir = 1) {
    const p = Math.sign(dir) * 0.22;
    run(p, (c, d) => {
      whoosh(c, d, { dur: 0.042, f0: dir >= 0 ? 900 : 1700, f1: dir >= 0 ? 1700 : 900, q: 2.4, gain: 0.035 });
      tone(c, d, { type: 'triangle', f0: dir >= 0 ? 820 : 690, f1: dir >= 0 ? 940 : 620, dur: 0.042, gain: 0.065 });
    });
  },
  page(dir = 1) {
    const p = Math.sign(dir) * 0.25;
    run(p, (c, d) => {
      whoosh(c, d, { dur: 0.048, f0: dir >= 0 ? 700 : 1350, f1: dir >= 0 ? 1350 : 700, q: 2.2, gain: 0.04 });
      tone(c, d, { type: 'triangle', f0: dir >= 0 ? 740 : 620, f1: dir >= 0 ? 860 : 540, dur: 0.048, gain: 0.07 });
    });
  },
  bump(dir = 0) {
    run(Math.sign(dir) * 0.2, (c, d) => {
      tone(c, d, { type: 'triangle', f0: 155, f1: 82, dur: 0.072, gain: 0.11 });
      tone(c, d, { type: 'sine', f0: 110, f1: 72, dur: 0.065, gain: 0.06 });
    });
  },
  fav(on = true) {
    run(0, (c, d) => {
      if (on) {
        tone(c, d, { type: 'sine', f0: 880, dur: 0.065, gain: 0.07 });
        tone(c, d, { type: 'sine', f0: 1175, delay: 0.03, dur: 0.075, gain: 0.07 });
        tone(c, d, { type: 'sine', f0: 1568, delay: 0.06, dur: 0.105, gain: 0.075 });
      } else {
        tone(c, d, { type: 'sine', f0: 820, dur: 0.055, gain: 0.06 });
        tone(c, d, { type: 'sine', f0: 587, delay: 0.028, dur: 0.07, gain: 0.055 });
      }
    });
  },
  play() {
    run(0, (c, d) => {
      const notes = [523, 659, 784, 1046];
      notes.forEach((f, i) => {
        tone(c, d, { type: i === 0 ? 'triangle' : 'sine', f0: f, delay: i * 0.024, dur: 0.21, gain: 0.065 });
      });
    });
  },
  modal(open = true) {
    modalAt = performance.now();
    run(0, (c, d) => {
      if (open) {
        whoosh(c, d, { dur: 0.032, f0: 650, f1: 1400, q: 1.9, gain: 0.025 });
        tone(c, d, { type: 'sine', f0: 460, f1: 680, dur: 0.062, gain: 0.065 });
      } else {
        tone(c, d, { type: 'sine', f0: 520, f1: 360, dur: 0.058, gain: 0.055 });
      }
    });
  },
  tick(normX = 0.5, pitch = 1) {
    const f = Math.round(920 * Math.max(0.5, Math.min(2, pitch)));
    run(panFor(0, normX), (c, d) => {
      tone(c, d, { type: 'sine', f0: f, dur: 0.026, gain: 0.05 });
    });
  },
};

/** Clic de navegacion sintetico (compatible con llamadas legadas por frecuencia). */
export function blip(f: number) {
  if (!app.config.sound) return;
  if ((f >= 1181 || f <= 560) && performance.now() - modalAt < 25) return;
  if (f <= 260) sfx.bump();
  else if (f <= 560) sfx.back();
  else if (f <= 760) sfx.tab(1);
  else if (f <= 960) sfx.move();
  else if (f <= 1180) sfx.fav(true);
  else sfx.confirm();
}
