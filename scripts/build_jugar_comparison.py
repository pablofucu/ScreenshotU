import base64
import io
import json
import os
from PIL import Image

covers = {
    'cyberpunk': {
        'name': 'Cyberpunk 2077',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f12910e9163-ddc20bcb/cover-1091500.jpg',
        'f1': '#9d9700',
        'f2': '#b85850',
        'g1': '#e6dd2a',
        'g2': '#f0857b',
        'via': 'Con Cyber Engine Tweaks',
        'desc': 'Amarillo neón + Rojo fuego'
    },
    'xeno': {
        'name': 'Xenoblade Chronicles 3',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f1298ee91e1-96f46ed4/cover-690563.png',
        'f1': '#2b7ec5',
        'f2': '#00a3bb',
        'g1': '#6cb2f2',
        'g2': '#32ccde',
        'via': 'Con Ryujinx',
        'desc': 'Azul espada + Cian Keves/Agnus'
    },
    'mariokart': {
        'name': 'Mario Kart 8 Deluxe',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f12994e91e7-13fad714/cover-421528.png',
        'f1': '#c24bc6',
        'f2': '#7664b8',
        'g1': '#e977ed',
        'g2': '#a290e2',
        'via': 'Con Yuzu',
        'desc': 'Fucsia caparazón + Violeta antigravedad'
    },
    'zelda': {
        'name': "Zelda: Link's Awakening",
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f129a0e91f3-8d3be14b/cover-83079.png',
        'f1': '#199171',
        'f2': '#387c42',
        'g1': '#43c8a1',
        'g2': '#61b06d',
        'via': 'Con Ryujinx',
        'desc': 'Verde Koholint + Verde bosque'
    },
    'elden': {
        'name': 'ELDEN RING',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f1291ae916d-e8736c63/cover-1245620.jpg',
        'f1': '#c08327',
        'f2': '#887c32',
        'g1': '#e8ad4a',
        'g2': '#b7a94d',
        'via': 'Ejecutable directo',
        'desc': 'Oro rúnico + Bronce antiguo'
    },
    'plague': {
        'name': 'A Plague Tale: Requiem',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f128f5e9148-2e168ae9/cover-1182900.jpg',
        'f1': '#ac5e20',
        'f2': '#854a1e',
        'g1': '#d7823e',
        'g2': '#b06b35',
        'via': 'Ejecutable directo',
        'desc': 'Ámbar hoguera + Cobre noche'
    }
}

for k, d in covers.items():
    p = d['path']
    if os.path.exists(p):
        im = Image.open(p).convert('RGB')
    else:
        im = Image.new('RGB', (160, 214), color=(60, 70, 90))
    im.thumbnail((320, 428), Image.Resampling.LANCZOS)
    buf = io.BytesIO()
    im.save(buf, format='JPEG', quality=85)
    d['b64'] = 'data:image/jpeg;base64,' + base64.b64encode(buf.getvalue()).decode('ascii')
    del d['path']

html_content = """<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Estilo de Borde «Botón Jugar» aplicado a Carátulas</title>
<style>
@font-face{font-family:'M PLUS Rounded 1c';font-weight:400;font-display:swap;src:url('../src/assets/fonts/mplus-rounded-1c-400-latin.woff2') format('woff2')}
@font-face{font-family:'M PLUS Rounded 1c';font-weight:700;font-display:swap;src:url('../src/assets/fonts/mplus-rounded-1c-700-latin.woff2') format('woff2')}
@font-face{font-family:'M PLUS Rounded 1c';font-weight:800;font-display:swap;src:url('../src/assets/fonts/mplus-rounded-1c-800-latin.woff2') format('woff2')}

:root {
  --round: 'M PLUS Rounded 1c', 'Nunito', 'Segoe UI', system-ui, sans-serif;
  --theme-bg: #e6e9f0;
  --theme-text: #2c3340;
  --theme-sub: #657184;
  --card-bg: #eceef2;
  --panel-bg: rgba(255,255,255,.75);
  --panel-border: rgba(0,0,0,.08);
  --f1: #2b7ec5;
  --f2: #00a3bb;
  --g1: #6cb2f2;
  --g2: #32ccde;
  --gf: linear-gradient(115deg, var(--f1), var(--f2));
  --on: #ffffff;
  --face: #ffffff;
  color-scheme: light;
}

[data-theme="dark"] {
  --theme-bg: #14161c;
  --theme-text: #e6e9f0;
  --theme-sub: #8e9bb0;
  --card-bg: #1e222b;
  --panel-bg: rgba(30,34,43,.82);
  --panel-border: rgba(255,255,255,.08);
  --face: #262a33;
  color-scheme: dark;
}

* { box-sizing: border-box; }
html, body {
  margin: 0;
  padding: 0;
  background: var(--theme-bg);
  font-family: var(--round);
  color: var(--theme-text);
  transition: background .25s, color .25s;
}

header {
  position: sticky;
  top: 0;
  z-index: 100;
  background: rgba(255,255,255,.86);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  border-bottom: 1px solid rgba(0,0,0,.08);
  padding: 14px 28px;
}
[data-theme="dark"] header {
  background: rgba(20,22,28,.88);
  border-bottom-color: rgba(255,255,255,.08);
}

.header-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}
h1 {
  margin: 0;
  font-size: 20px;
  font-weight: 800;
  display: flex;
  align-items: center;
  gap: 10px;
}
h1 span {
  font-size: 11.5px;
  font-weight: 800;
  background: #00A0D8;
  color: #fff;
  padding: 3px 9px;
  border-radius: 999px;
  letter-spacing: .04em;
}

.controls-bar {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid rgba(0,0,0,.06);
}
[data-theme="dark"] .controls-bar {
  border-top-color: rgba(255,255,255,.06);
}

.c-group {
  display: flex;
  align-items: center;
  gap: 6px;
}
.c-label {
  font-size: 11px;
  font-weight: 800;
  color: var(--theme-sub);
  text-transform: uppercase;
  letter-spacing: .06em;
  margin-right: 2px;
}

.btn-pill {
  height: 32px;
  padding: 0 13px;
  border-radius: 999px;
  border: 1px solid rgba(0,0,0,.12);
  background: rgba(255,255,255,.65);
  font: 700 13px var(--round);
  color: var(--theme-text);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  transition: all .15s;
}
[data-theme="dark"] .btn-pill {
  border-color: rgba(255,255,255,.14);
  background: rgba(255,255,255,.06);
}
.btn-pill:hover {
  background: #fff;
  transform: translateY(-1px);
}
[data-theme="dark"] .btn-pill:hover {
  background: rgba(255,255,255,.12);
}
.btn-pill.active {
  background: #00A0D8;
  color: #fff;
  border-color: #00A0D8;
  box-shadow: 0 2px 8px rgba(0,160,216,.35);
}

.container {
  max-width: 1420px;
  margin: 0 auto;
  padding: 24px 28px 80px;
}

/* ========================================================
   SECCIÓN BENCHMARK: EL BOTÓN «JUGAR» ORIGINAL
   ======================================================== */
.benchmark-section {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: 24px;
  padding: 22px 28px;
  margin-bottom: 30px;
  box-shadow: 0 6px 20px -8px rgba(30,40,60,.08);
}
.benchmark-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  flex-wrap: wrap;
  gap: 12px;
}
.benchmark-title {
  display: flex;
  align-items: center;
  gap: 10px;
}
.benchmark-title h2 {
  margin: 0;
  font-size: 17px;
  font-weight: 800;
}
.benchmark-tag {
  font-size: 11px;
  font-weight: 800;
  background: #3BB54A;
  color: #fff;
  padding: 2px 8px;
  border-radius: 6px;
  text-transform: uppercase;
}
.benchmark-stage {
  display: flex;
  align-items: center;
  gap: 32px;
  padding: 16px 0 8px;
  flex-wrap: wrap;
}

/* Estilos exactos del Botón «Jugar» extraídos de Detail.svelte y app.css */
.play-btn-wrapper {
  position: relative;
  width: 290px;
  height: 112px;
}
.glass-btn {
  position: absolute;
  inset: 0;
  --r: 30px;
  --bw: 8px;
  border-radius: var(--r);
  padding: var(--bw);
  box-sizing: border-box;
  cursor: pointer;
  isolation: isolate;
  transform: scale(1.06);
  background: linear-gradient(150deg,
    color-mix(in srgb, var(--g1) 55%, white) 0%,
    var(--f1) 50%,
    var(--f2) 100%
  );
  box-shadow:
    inset 0 1.5px 0 rgba(255,255,255,.95),
    inset 0 0 0 1px color-mix(in srgb, white 45%, transparent),
    inset 0 -10px 14px -8px color-mix(in srgb, var(--g2) 40%, white),
    0 0 0 1px color-mix(in srgb, var(--f2) 75%, black),
    0 0 26px 2px color-mix(in srgb, var(--f1) 45%, transparent),
    0 22px 34px -16px color-mix(in srgb, var(--f2) 70%, black);
  transition: transform .2s ease;
}
.glass-btn::before {
  content: '';
  position: absolute;
  z-index: 2;
  left: 2px; right: 2px; top: 2px;
  height: 52%;
  pointer-events: none;
  border-radius: calc(var(--r) - 2px) calc(var(--r) - 2px) 0 0;
  background: linear-gradient(180deg, rgba(255,255,255,.62) 0%, rgba(255,255,255,.20) 55%, rgba(255,255,255,0) 100%);
  -webkit-mask-image: radial-gradient(140% 100% at 50% 0%, #000 64%, transparent 65%);
  mask-image: radial-gradient(140% 100% at 50% 0%, #000 64%, transparent 65%);
  opacity: .85;
}
.glass-btn::after {
  content: '';
  position: absolute;
  z-index: 3;
  left: 12%; top: 3px;
  width: 34%; height: 5px;
  border-radius: 5px;
  pointer-events: none;
  background: linear-gradient(90deg, rgba(255,255,255,0), rgba(255,255,255,1) 40%, rgba(255,255,255,0));
  filter: blur(.6px);
  opacity: .9;
}
.glass-btn .btn-face {
  position: relative;
  z-index: 1;
  width: 100%; height: 100%;
  border-radius: calc(var(--r) - var(--bw) + 2px);
  overflow: hidden;
  background: var(--gf);
  color: var(--on);
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 0 24px;
}
.glass-btn .btn-face::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  box-shadow:
    inset 0 0 0 1px rgba(20,30,50,.20),
    inset 0 3px 6px -2px rgba(0,0,0,.35),
    inset 0 -2px 4px -2px rgba(255,255,255,.5);
}
.glass-btn .pi {
  flex: none;
  width: 50px; height: 50px;
  border-radius: 50%;
  background: linear-gradient(180deg, var(--on), #fff);
  display: grid;
  place-items: center;
  box-shadow: inset 0 -2px 3px rgba(0,0,0,.12), 0 3px 8px rgba(0,0,0,.25);
}
.glass-btn .pi svg {
  width: 22px; height: 22px;
  margin-left: 3px;
}
.glass-btn .pi svg path { fill: var(--f1); }
.glass-btn b {
  font-size: 32px;
  font-weight: 800;
  display: block;
  line-height: 1.05;
  text-shadow: 0 1px 2px rgba(0,0,0,.2);
}
.glass-btn small {
  font-size: 14px;
  font-weight: 600;
  opacity: .92;
  white-space: nowrap;
}

.benchmark-desc {
  flex: 1;
  min-width: 280px;
  font-size: 13.5px;
  line-height: 1.55;
  color: var(--theme-sub);
}
.benchmark-desc b {
  color: var(--theme-text);
}
.formula-badge {
  background: rgba(0,0,0,.06);
  padding: 3px 7px;
  border-radius: 5px;
  font-family: monospace;
  font-size: 12px;
}
[data-theme="dark"] .formula-badge {
  background: rgba(255,255,255,.08);
}

/* ========================================================
   GRID DE COMPARACIÓN DE OPCIONES DE CARÁTULA
   ======================================================== */
.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(255px, 1fr));
  gap: 22px;
  margin-top: 20px;
}

.variant-card {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: 22px;
  padding: 22px 18px 18px;
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;
  box-shadow: 0 4px 18px -6px rgba(40,50,70,.08);
}
[data-theme="dark"] .variant-card {
  box-shadow: 0 8px 24px -10px rgba(0,0,0,.4);
}

.variant-tag {
  position: absolute;
  left: 18px;
  top: 16px;
  font-size: 11px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: .08em;
  padding: 3px 8px;
  border-radius: 6px;
  background: rgba(0,0,0,.06);
  color: var(--theme-sub);
}
[data-theme="dark"] .variant-tag {
  background: rgba(255,255,255,.08);
}
.variant-tag.fav {
  background: #6E56CF;
  color: #fff;
}

.card-stage {
  height: 260px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  margin: 14px 0 6px;
}

/* Base de la carátula */
.cov-base {
  position: relative;
  width: 160px;
  height: 214px;
  border-radius: var(--r, 22px);
  padding: var(--bw, 8px);
  box-sizing: border-box;
  isolation: isolate;
  user-select: none;
  transform: scale(1.06);
}
.cov-base .face {
  position: relative;
  z-index: 1;
  width: 100%;
  height: 100%;
  border-radius: calc(var(--r, 22px) - var(--bw, 8px) + 2px);
  overflow: hidden;
  background: #11141c;
}
.cov-base .face img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

/* ========================================================
   ESTILOS ESPECÍFICOS DE LAS VARIACIONES
   ======================================================== */

/* --- OPCIÓN 1: CRISTAL CANÓNICO FIEL AL BOTÓN «JUGAR» ---
   Exactas 6 capas de box-shadow, degradado 150° y bisel embutido */
.v-fiel {
  --r: 22px;
  --bw: 8px;
  background: linear-gradient(150deg,
    color-mix(in srgb, var(--g1) 55%, white) 0%,
    var(--f1) 50%,
    var(--f2) 100%
  );
  box-shadow:
    inset 0 1.5px 0 rgba(255,255,255,.95),
    inset 0 0 0 1px color-mix(in srgb, white 45%, transparent),
    inset 0 -10px 14px -8px color-mix(in srgb, var(--g2) 40%, white),
    0 0 0 1px color-mix(in srgb, var(--f2) 75%, black),
    0 0 24px 2px color-mix(in srgb, var(--f1) 45%, transparent),
    0 20px 32px -14px color-mix(in srgb, var(--f2) 70%, black);
}
.v-fiel .face::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  box-shadow:
    inset 0 0 0 1px rgba(20,30,50,.20),
    inset 0 3px 6px -2px rgba(0,0,0,.35),
    inset 0 -2px 4px -2px rgba(255,255,255,.5);
}

/* --- OPCIÓN 2: BISEL DE 5PX (PROPORCIÓN PERFECTA PARA CARÁTULA) ---
   Mismo acabado joya de 6 sombras, pero adaptado a 5px para no tapar arte */
.v-proporcional {
  --r: 18px;
  --bw: 5.5px;
  background: linear-gradient(150deg,
    color-mix(in srgb, var(--g1) 60%, white) 0%,
    var(--f1) 45%,
    var(--f2) 100%
  );
  box-shadow:
    inset 0 1.2px 0 rgba(255,255,255,.98),
    inset 0 0 0 1px color-mix(in srgb, white 50%, transparent),
    inset 0 -7px 10px -5px color-mix(in srgb, var(--g2) 45%, white),
    0 0 0 1px color-mix(in srgb, var(--f2) 75%, black),
    0 0 22px 2px color-mix(in srgb, var(--f1) 45%, transparent),
    0 18px 30px -12px color-mix(in srgb, var(--f2) 70%, black);
}
.v-proporcional .face::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  box-shadow:
    inset 0 0 0 1px rgba(20,30,50,.20),
    inset 0 2px 4px -1px rgba(0,0,0,.30);
}

/* --- OPCIÓN 3: CRISTAL CON RESPIRACIÓN CÁUSTICA ---
   La luz en la base (--g2) y el resplandor pulsan suavemente */
@keyframes caustic-breath {
  0%, 100% {
    box-shadow:
      inset 0 1.5px 0 rgba(255,255,255,.95),
      inset 0 0 0 1px color-mix(in srgb, white 45%, transparent),
      inset 0 -8px 12px -7px color-mix(in srgb, var(--g2) 30%, white),
      0 0 0 1px color-mix(in srgb, var(--f2) 75%, black),
      0 0 18px 1px color-mix(in srgb, var(--f1) 35%, transparent),
      0 20px 32px -14px color-mix(in srgb, var(--f2) 70%, black);
  }
  50% {
    box-shadow:
      inset 0 1.8px 0 rgba(255,255,255,1),
      inset 0 0 0 1px color-mix(in srgb, white 60%, transparent),
      inset 0 -14px 18px -7px color-mix(in srgb, var(--g2) 75%, white),
      0 0 0 1px color-mix(in srgb, var(--f2) 80%, black),
      0 0 32px 5px color-mix(in srgb, var(--f1) 65%, transparent),
      0 22px 36px -12px color-mix(in srgb, var(--f2) 75%, black);
  }
}
.v-breath {
  --r: 20px;
  --bw: 6.5px;
  background: linear-gradient(150deg,
    color-mix(in srgb, var(--g1) 55%, white) 0%,
    var(--f1) 50%,
    var(--f2) 100%
  );
  animation: caustic-breath 3.4s ease-in-out infinite;
}
.v-breath .face::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  box-shadow: inset 0 0 0 1px rgba(20,30,50,.20), inset 0 3px 5px -2px rgba(0,0,0,.35);
}

/* --- OPCIÓN 4: CRISTAL CON DESTELLO ESPECULAR (GLINT SWEEP) ---
   Mantiene el marco estático idéntico a «Jugar» y añade un destello viajero */
@keyframes glint-run {
  0% { transform: translateX(-120%) rotate(25deg); opacity: 0; }
  20% { opacity: .75; }
  45% { transform: translateX(240%) rotate(25deg); opacity: 0; }
  100% { transform: translateX(240%) rotate(25deg); opacity: 0; }
}
.v-glint {
  --r: 20px;
  --bw: 7px;
  overflow: hidden;
  background: linear-gradient(150deg,
    color-mix(in srgb, var(--g1) 55%, white) 0%,
    var(--f1) 50%,
    var(--f2) 100%
  );
  box-shadow:
    inset 0 1.5px 0 rgba(255,255,255,.95),
    inset 0 0 0 1px color-mix(in srgb, white 45%, transparent),
    inset 0 -10px 14px -8px color-mix(in srgb, var(--g2) 40%, white),
    0 0 0 1px color-mix(in srgb, var(--f2) 75%, black),
    0 0 24px 2px color-mix(in srgb, var(--f1) 45%, transparent),
    0 20px 32px -14px color-mix(in srgb, var(--f2) 70%, black);
}
.v-glint::before {
  content: '';
  position: absolute;
  inset: -50%;
  width: 60px;
  height: 200%;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,.8), transparent);
  animation: glint-run 4.5s ease-in-out infinite;
  pointer-events: none;
  z-index: 5;
}
.v-glint .face::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  box-shadow: inset 0 0 0 1px rgba(20,30,50,.20), inset 0 3px 5px -2px rgba(0,0,0,.35);
}

/* --- OPCIÓN 5: HÍBRIDO CONIC CON ILUMINACIÓN Y BISEL 3D ---
   Rotación 360° pero con la riqueza cromática y bisel del botón Jugar */
@property --ang-j {
  syntax: '<angle>';
  inherits: false;
  initial-value: 0deg;
}
@keyframes spin-hyb {
  to { --ang-j: 360deg; }
}
.v-hybrid {
  --r: 18px;
  --bw: 5.5px;
  background: conic-gradient(from var(--ang-j),
    color-mix(in srgb, var(--g1) 65%, white) 0deg,
    var(--f1) 90deg,
    var(--f2) 180deg,
    color-mix(in srgb, var(--g2) 65%, white) 270deg,
    color-mix(in srgb, var(--g1) 65%, white) 360deg
  );
  animation: spin-hyb 3.2s linear infinite;
  box-shadow:
    inset 0 1.2px 0 rgba(255,255,255,.98),
    inset 0 0 0 1px color-mix(in srgb, white 45%, transparent),
    inset 0 -8px 12px -6px color-mix(in srgb, var(--g2) 45%, white),
    0 0 0 1px color-mix(in srgb, var(--f2) 75%, black),
    0 0 24px 2px color-mix(in srgb, var(--f1) 45%, transparent),
    0 18px 30px -12px color-mix(in srgb, var(--f2) 70%, black);
}
.v-hybrid .face::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  box-shadow: inset 0 0 0 1px rgba(20,30,50,.20), inset 0 2px 4px -1px rgba(0,0,0,.30);
}

/* Metadatos bajo cada tarjeta */
.card-details {
  width: 100%;
  margin-top: 14px;
  text-align: left;
  border-top: 1px solid rgba(0,0,0,.06);
  padding-top: 12px;
}
[data-theme="dark"] .card-details {
  border-top-color: rgba(255,255,255,.06);
}
.card-details h4 {
  margin: 0 0 4px;
  font-size: 14px;
  font-weight: 800;
}
.card-details p {
  margin: 0;
  font-size: 12px;
  color: var(--theme-sub);
  line-height: 1.45;
}

</style>
</head>
<body>

<header>
  <div class="header-top">
    <h1>
      Adaptación de Borde «Botón Jugar» a Carátulas
      <span>Propuestas Interactivas</span>
    </h1>
    <div style="display:flex;gap:8px;align-items:center;">
      <button class="btn-pill" id="theme-btn" onclick="toggleTheme()">
        🌙 Tema: <b id="theme-lbl">Claro</b>
      </button>
    </div>
  </div>

  <div class="controls-bar">
    <div class="c-group">
      <span class="c-label">Juego Real:</span>
      <div id="game-buttons" style="display:flex;gap:6px;flex-wrap:wrap;"></div>
    </div>
  </div>
</header>

<div class="container">

  <!-- SECCIÓN BENCHMARK: EL BOTÓN JUGAR ORIGINAL -->
  <div class="benchmark-section">
    <div class="benchmark-header">
      <div class="benchmark-title">
        <span class="benchmark-tag">Referencia Canónica</span>
        <h2>Botón «Jugar» original (Detail.svelte) con los colores de este juego</h2>
      </div>
      <div style="font-size:12px;color:var(--theme-sub);">
        Ángulo estático <span class="formula-badge">150°</span> · 6 capas de <span class="formula-badge">box-shadow</span> · Bisel <span class="formula-badge">8px</span>
      </div>
    </div>
    <div class="benchmark-stage">
      <div class="play-btn-wrapper">
        <div class="glass-btn">
          <div class="btn-face">
            <span class="pi">
              <svg viewBox="0 0 24 24"><path d="M8 5.5v13a1 1 0 0 0 1.5.86l10.5-6.5a1 1 0 0 0 0-1.72L9.5 4.64A1 1 0 0 0 8 5.5z"></path></svg>
            </span>
            <span>
              <b>Jugar</b>
              <small id="bench-via">Con Ryujinx</small>
            </span>
          </div>
        </div>
      </div>
      <div class="benchmark-desc">
        <b>¿Por qué destaca tanto este borde?</b><br>
        A diferencia de un marco plano o un anillo lineal, utiliza una dirección de luz estática a <b>150°</b> que proyecta luz arriba-izquierda (<span class="formula-badge">--g1 + blanco</span>) y sombra abajo-derecha (<span class="formula-badge">--f2</span>), rematada con un destello blanco superior tallado de 1.5px y un rebote cáustico en la base. Abajo puedes ver cómo luce adaptado al rectángulo de la carátula:
      </div>
    </div>
  </div>

  <!-- GRID DE OPCIONES PARA LA CARÁTULA -->
  <div class="cards-grid">

    <!-- 1. FIEL AL BOTÓN -->
    <div class="variant-card">
      <span class="variant-tag fav">Recomendado</span>
      <div class="card-stage">
        <div class="cov-base v-fiel">
          <div class="face">
            <img class="cov-img" alt="" draggable="false">
          </div>
        </div>
      </div>
      <div class="card-details">
        <h4>1. Cristal Canónico 150° (Fiel al botón)</h4>
        <p>Misma física: marco de 8px, ángulo a 150°, 6 capas de sombras y luz cáustica en la base. La carátula queda embutida como en una vitrina de cristal.</p>
      </div>
    </div>

    <!-- 2. PROPORCIONAL -->
    <div class="variant-card">
      <span class="variant-tag">Estilizado</span>
      <div class="card-stage">
        <div class="cov-base v-proporcional">
          <div class="face">
            <img class="cov-img" alt="" draggable="false">
          </div>
        </div>
      </div>
      <div class="card-details">
        <h4>2. Bisel Cristal 5.5px (Proporción Óptima)</h4>
        <p>El mismo acabado joya tridimensional de 6 sombras, pero ajustado a 5.5px para dejar visible el 95% del arte de la carátula sin abultar.</p>
      </div>
    </div>

    <!-- 3. RESPIRACIÓN CÁUSTICA -->
    <div class="variant-card">
      <span class="variant-tag">Con Vida</span>
      <div class="card-stage">
        <div class="cov-base v-breath">
          <div class="face">
            <img class="cov-img" alt="" draggable="false">
          </div>
        </div>
      </div>
      <div class="card-details">
        <h4>3. Respiración Cáustica (Pulso de Luz)</h4>
        <p>Estructura física idéntica al botón Jugar, pero la luz que rebota en la base y el aura ambiental respiran suavemente en un ciclo sinusoidal de 3.4s.</p>
      </div>
    </div>

    <!-- 4. DESTELLO VIAJERO (GLINT) -->
    <div class="variant-card">
      <span class="variant-tag">Especular</span>
      <div class="card-stage">
        <div class="cov-base v-glint">
          <div class="face">
            <img class="cov-img" alt="" draggable="false">
          </div>
        </div>
      </div>
      <div class="card-details">
        <h4>4. Destello Viajero sobre Cristal</h4>
        <p>El marco estático idéntico al botón «Jugar» recibe periódicamente (cada 4.5s) una caricia de luz blanca que recorre el bisel de cristal pulido.</p>
      </div>
    </div>

    <!-- 5. HÍBRIDO CONIC CON BISEL 3D -->
    <div class="variant-card">
      <span class="variant-tag">Híbrido</span>
      <div class="card-stage">
        <div class="cov-base v-hybrid">
          <div class="face">
            <img class="cov-img" alt="" draggable="false">
          </div>
        </div>
      </div>
      <div class="card-details">
        <h4>5. Giro Conic con Profundidad 3D</h4>
        <p>Combina la rotación continua 360° perimétrica con los tonos <span class="formula-badge">--g1/--g2</span> más claros y las 6 capas de bisel y cáustica tridimensional.</p>
      </div>
    </div>

  </div>

</div>

<script>
const COVERS = __COVERS_JSON__;
let curKey = 'xeno';
let isDark = false;

function init() {
  const cBar = document.getElementById('game-buttons');
  Object.keys(COVERS).forEach(k => {
    const c = COVERS[k];
    const btn = document.createElement('button');
    btn.className = 'btn-pill' + (k === curKey ? ' active' : '');
    btn.innerHTML = `<span style="width:10px;height:10px;border-radius:50%;background:${c.f1};display:inline-block;"></span> ${c.name}`;
    btn.onclick = () => selectGame(k);
    btn.id = 'btn-' + k;
    cBar.appendChild(btn);
  });
  applyGame(curKey);
}

function selectGame(k) {
  curKey = k;
  document.querySelectorAll('#game-buttons .btn-pill').forEach(b => b.classList.remove('active'));
  document.getElementById('btn-' + k).classList.add('active');
  applyGame(k);
}

function applyGame(k) {
  const g = COVERS[k];
  const r = document.documentElement.style;
  r.setProperty('--f1', g.f1);
  r.setProperty('--f2', g.f2);
  r.setProperty('--g1', g.g1);
  r.setProperty('--g2', g.g2);
  r.setProperty('--gf', `linear-gradient(115deg, ${g.f1}, ${g.f2})`);

  document.getElementById('bench-via').innerText = g.via;

  document.querySelectorAll('.cov-img').forEach(img => {
    img.src = g.b64;
  });
}

function toggleTheme() {
  isDark = !isDark;
  if (isDark) {
    document.documentElement.setAttribute('data-theme', 'dark');
    document.getElementById('theme-lbl').innerText = 'Noche ahumada';
  } else {
    document.documentElement.removeAttribute('data-theme');
    document.getElementById('theme-lbl').innerText = 'Claro';
  }
}

init();
</script>
</body>
</html>
"""

html_content = html_content.replace('__COVERS_JSON__', json.dumps(covers))

out_path = 'design/prueba-estilo-boton-jugar.html'
with open(out_path, 'w', encoding='utf-8') as f:
    f.write(html_content)

print(f"Generated successfully: {out_path} Size: {len(html_content)}")
