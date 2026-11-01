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

@property --ang-j {
  syntax: '<angle>';
  inherits: false;
  initial-value: 0deg;
}

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
   SECCIÓN DE AFINACIÓN EXCLUSIVA DEL HÍBRIDO (RESOLVER LO QUE CHIRRÍA)
   ======================================================== */
.tuner-section {
  background: var(--panel-bg);
  border: 2px solid #6E56CF;
  border-radius: 24px;
  padding: 22px 28px;
  margin-bottom: 30px;
  box-shadow: 0 8px 30px -10px rgba(110,86,207,.25);
}
.tuner-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 16px;
}
.tuner-title h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 800;
  display: flex;
  align-items: center;
  gap: 10px;
  color: #6E56CF;
}
[data-theme="dark"] .tuner-title h2 {
  color: #9d87f5;
}
.tuner-subtitle {
  font-size: 13px;
  color: var(--theme-sub);
  margin-bottom: 20px;
  line-height: 1.5;
}

/* 4 Comparativas del Híbrido */
.hybrid-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 20px;
  margin-top: 14px;
}
.h-box {
  background: rgba(255,255,255,.55);
  border: 1px solid rgba(0,0,0,.08);
  border-radius: 20px;
  padding: 18px 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;
}
[data-theme="dark"] .h-box {
  background: rgba(20,24,32,.55);
  border-color: rgba(255,255,255,.07);
}

.h-tag {
  position: absolute;
  left: 14px;
  top: 14px;
  font-size: 10.5px;
  font-weight: 800;
  text-transform: uppercase;
  padding: 2px 7px;
  border-radius: 6px;
  background: rgba(0,0,0,.07);
  color: var(--theme-sub);
}
[data-theme="dark"] .h-tag {
  background: rgba(255,255,255,.1);
}
.h-tag.alert {
  background: #f59e0b;
  color: #fff;
}
.h-tag.fix {
  background: #10b981;
  color: #fff;
}

.h-stage {
  height: 250px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  margin: 12px 0 6px;
}

/* Animación Conic compartida */
@keyframes spin-hyb {
  to { --ang-j: 360deg; }
}

/* 5. ORIGINAL (EL QUE CHIRRÍA):
   Tiene:
   - Línea exterior negra rígida de 1px (box-shadow 0 0 0 1px)
   - Línea interior blanca rígida de 1px (inset 0 0 0 1px)
   - Sombra y línea interior que muerde la imagen (.face::after)
   - Radio interior con +2px que deforma las esquinas */
.hyb-original {
  position: relative;
  width: 160px;
  height: 214px;
  border-radius: 18px;
  padding: 5.5px;
  box-sizing: border-box;
  transform: scale(1.06);
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
.hyb-original .face {
  width: 100%; height: 100%;
  border-radius: calc(18px - 5.5px + 2px); /* 14.5px: estrecha las esquinas */
  overflow: hidden;
  position: relative;
}
.hyb-original .face::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  box-shadow: inset 0 0 0 1px rgba(20,30,50,.20), inset 0 2px 4px -1px rgba(0,0,0,.30);
}
.hyb-original img { width: 100%; height: 100%; object-fit: cover; display: block; }

/* 5A: SIN MICRO-LÍNEAS (LIMPIEZA TOTAL)
   Eliminadas las 3 líneas duras de 1px:
   - Cero línea exterior negra
   - Cero línea interior blanca
   - Cero línea cortando el arte
   Solo el degradado fluido + sombra física exterior suave */
.hyb-clean {
  position: relative;
  width: 160px;
  height: 214px;
  border-radius: 18px;
  padding: 5.5px;
  box-sizing: border-box;
  transform: scale(1.06);
  background: conic-gradient(from var(--ang-j),
    color-mix(in srgb, var(--g1) 65%, white) 0deg,
    var(--f1) 90deg,
    var(--f2) 180deg,
    color-mix(in srgb, var(--g2) 65%, white) 270deg,
    color-mix(in srgb, var(--g1) 65%, white) 360deg
  );
  animation: spin-hyb 3.2s linear infinite;
  box-shadow:
    inset 0 1.2px 0 rgba(255,255,255,.9),
    inset 0 -8px 12px -6px color-mix(in srgb, var(--g2) 45%, white),
    0 0 24px 2px color-mix(in srgb, var(--f1) 40%, transparent),
    0 18px 30px -12px rgba(20,30,50,.55);
}
.hyb-clean .face {
  width: 100%; height: 100%;
  border-radius: calc(18px - 5.5px); /* 12.5px exactos concéntricos */
  overflow: hidden;
  position: relative;
}
.hyb-clean img { width: 100%; height: 100%; object-fit: cover; display: block; }

/* 5B: BISEL SUAVE CONTINUO (SOLO LUZ)
   Mantiene el volumen tridimensional pero usando degradado de luz difusa
   en lugar de líneas secas de 1px. */
.hyb-bevel {
  position: relative;
  width: 160px;
  height: 214px;
  border-radius: 18px;
  padding: 5px;
  box-sizing: border-box;
  transform: scale(1.06);
  background: conic-gradient(from var(--ang-j),
    color-mix(in srgb, var(--g1) 70%, white) 0deg,
    var(--f1) 90deg,
    var(--f2) 180deg,
    color-mix(in srgb, var(--g2) 70%, white) 270deg,
    color-mix(in srgb, var(--g1) 70%, white) 360deg
  );
  animation: spin-hyb 3.2s linear infinite;
  box-shadow:
    inset 0 1.5px 2px rgba(255,255,255,.85),
    inset 0 -6px 10px -4px color-mix(in srgb, var(--g2) 60%, white),
    0 2px 8px rgba(0,0,0,.15),
    0 18px 30px -12px color-mix(in srgb, var(--f2) 65%, black);
}
.hyb-bevel .face {
  width: 100%; height: 100%;
  border-radius: 13px; /* Concéntrico exacto 18 - 5 = 13 */
  overflow: hidden;
  position: relative;
  box-shadow: 0 0 4px rgba(0,0,0,.35); /* La sombra cae sobre el marco, no sobre la imagen */
}
.hyb-bevel img { width: 100%; height: 100%; object-fit: cover; display: block; }

/* 5C: GROSOR GENEROSO 7.5PX (IGUAL AL BOTÓN JUGAR)
   El problema a veces es que un borde de 5.5px es demasiado estrecho para
   albergar biseles y degradados. Con 7.5px respira con la presencia del botón. */
.hyb-thick {
  position: relative;
  width: 160px;
  height: 214px;
  border-radius: 20px;
  padding: 7.5px;
  box-sizing: border-box;
  transform: scale(1.06);
  background: conic-gradient(from var(--ang-j),
    color-mix(in srgb, var(--g1) 65%, white) 0deg,
    var(--f1) 90deg,
    var(--f2) 180deg,
    color-mix(in srgb, var(--g2) 65%, white) 270deg,
    color-mix(in srgb, var(--g1) 65%, white) 360deg
  );
  animation: spin-hyb 3.2s linear infinite;
  box-shadow:
    inset 0 1.5px 0 rgba(255,255,255,.95),
    inset 0 -10px 14px -8px color-mix(in srgb, var(--g2) 45%, white),
    0 0 26px 2px color-mix(in srgb, var(--f1) 40%, transparent),
    0 20px 32px -14px color-mix(in srgb, var(--f2) 70%, black);
}
.hyb-thick .face {
  width: 100%; height: 100%;
  border-radius: calc(20px - 7.5px); /* 12.5px */
  overflow: hidden;
  position: relative;
}
.hyb-thick img { width: 100%; height: 100%; object-fit: cover; display: block; }

.h-info {
  margin-top: 12px;
  width: 100%;
  border-top: 1px solid rgba(0,0,0,.06);
  padding-top: 10px;
}
[data-theme="dark"] .h-info {
  border-top-color: rgba(255,255,255,.06);
}
.h-info h4 {
  margin: 0 0 4px;
  font-size: 13.5px;
  font-weight: 800;
}
.h-info p {
  margin: 0;
  font-size: 12px;
  color: var(--theme-sub);
  line-height: 1.45;
}

/* ========================================================
   LABORATORIO EN VIVO CON SLIDERS (CONTROLES INTERACTIVOS)
   ======================================================== */
.live-tuner-box {
  background: rgba(255,255,255,.75);
  border: 1px solid rgba(0,0,0,.08);
  border-radius: 20px;
  padding: 18px 24px;
  margin-top: 24px;
  display: flex;
  gap: 30px;
  align-items: center;
  flex-wrap: wrap;
}
[data-theme="dark"] .live-tuner-box {
  background: rgba(24,28,38,.75);
  border-color: rgba(255,255,255,.07);
}
.tuner-preview {
  flex: none;
  width: 220px;
  display: flex;
  justify-content: center;
}
.tuner-sliders {
  flex: 1;
  min-width: 300px;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px;
}
.slider-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.slider-item label {
  font-size: 12px;
  font-weight: 800;
  color: var(--theme-text);
  display: flex;
  justify-content: space-between;
}
.slider-item input[type="range"] {
  accent-color: #6E56CF;
  cursor: pointer;
}
.toggle-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  user-select: none;
}
.toggle-item input[type="checkbox"] {
  accent-color: #6E56CF;
  width: 16px;
  height: 16px;
  cursor: pointer;
}

/* Elemento del laboratorio interactivo */
#interactive-card {
  position: relative;
  width: 160px;
  height: 214px;
  border-radius: 18px;
  padding: 5.5px;
  box-sizing: border-box;
  transform: scale(1.06);
  background: conic-gradient(from var(--ang-j),
    color-mix(in srgb, var(--g1) 65%, white) 0deg,
    var(--f1) 90deg,
    var(--f2) 180deg,
    color-mix(in srgb, var(--g2) 65%, white) 270deg,
    color-mix(in srgb, var(--g1) 65%, white) 360deg
  );
  animation: spin-hyb 3.2s linear infinite;
  box-shadow:
    inset 0 1.2px 0 rgba(255,255,255,.9),
    inset 0 -8px 12px -6px color-mix(in srgb, var(--g2) 45%, white),
    0 0 24px 2px color-mix(in srgb, var(--f1) 40%, transparent),
    0 18px 30px -12px rgba(20,30,50,.55);
}
#interactive-card .face {
  width: 100%; height: 100%;
  border-radius: 12.5px;
  overflow: hidden;
  position: relative;
}
#interactive-card img { width: 100%; height: 100%; object-fit: cover; display: block; }

</style>
</head>
<body>

<header>
  <div class="header-top">
    <h1>
      Afinación del Borde Híbrido
      <span>Laboratorio de Diagnóstico</span>
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

  <!-- SECCIÓN TUNER: DIAGNÓSTICO DEL HÍBRIDO -->
  <div class="tuner-section">
    <div class="tuner-title">
      <h2>🔍 Diagnóstico: ¿Por qué chirriaba el borde pequeño en la Opción 5?</h2>
      <span style="font-size:12px;font-weight:800;color:var(--theme-sub);">Compara las 4 correcciones abajo</span>
    </div>
    <div class="tuner-subtitle">
      En la opción híbrida anterior confluían <b>3 micro-líneas rígidas de 1px</b> (un alambre exterior oscuro, un alambre interior blanco y una sombra que mordía el arte del juego), además de una fórmula de redondeo que estrangulaba las esquinas haciéndolas parecer más delgadas. Abajo tienes las soluciones lado a lado para ver cuál es exactamente la que te sienta bien:
    </div>

    <div class="hybrid-grid">

      <!-- 5. ORIGINAL -->
      <div class="h-box">
        <span class="h-tag alert">El que chirriaba</span>
        <div class="h-stage">
          <div class="hyb-original">
            <div class="face">
              <img class="cov-img" alt="" draggable="false">
            </div>
          </div>
        </div>
        <div class="h-info">
          <h4>5. Original con 3 micro-líneas</h4>
          <p>Lleva línea oscura exterior rígida (1px), línea blanca concéntrica (1px) y sombra que pisa la imagen. Las esquinas se afinan a 3.5px por desajuste de radio.</p>
        </div>
      </div>

      <!-- 5A. LIMPIO -->
      <div class="h-box">
        <span class="h-tag fix">Solución 1: Limpio</span>
        <div class="h-stage">
          <div class="hyb-clean">
            <div class="face">
              <img class="cov-img" alt="" draggable="false">
            </div>
          </div>
        </div>
        <div class="h-info">
          <h4>5A. Sin micro-líneas (100% Fluido)</h4>
          <p><b>Eliminadas las 3 líneas duras de 1px</b>. El degradado va limpio directo a la carátula. Radio concéntrico exacto (12.5px) para que las esquinas no se estrechen.</p>
        </div>
      </div>

      <!-- 5B. BISEL DE LUZ -->
      <div class="h-box">
        <span class="h-tag fix">Solución 2: Solo Luz</span>
        <div class="h-stage">
          <div class="hyb-bevel">
            <div class="face">
              <img class="cov-img" alt="" draggable="false">
            </div>
          </div>
        </div>
        <div class="h-info">
          <h4>5B. Bisel de Luz Difusa (Sin alambres)</h4>
          <p>Mantiene la tridimensionalidad pero sustituye los alambres secos por un bisel de luz suave arriba y cáustica abajo. El arte queda 100% intocado.</p>
        </div>
      </div>

      <!-- 5C. GROSOR 7.5PX -->
      <div class="h-box">
        <span class="h-tag fix">Solución 3: Más cuerpo</span>
        <div class="h-stage">
          <div class="hyb-thick">
            <div class="face">
              <img class="cov-img" alt="" draggable="false">
            </div>
          </div>
        </div>
        <div class="h-info">
          <h4>5C. Grosor Generoso 7.5px (Como Jugar)</h4>
          <p>Aumenta el grosor de 5.5px a 7.5px para darle el mismo empaque físico que tiene el botón de Jugar en la pantalla de detalle.</p>
        </div>
      </div>

    </div>

    <!-- LABORATORIO EN VIVO CON SLIDERS -->
    <div class="live-tuner-box">
      <div class="tuner-preview">
        <div id="interactive-card">
          <div class="face">
            <img class="cov-img" alt="" draggable="false">
          </div>
        </div>
      </div>
      <div class="tuner-sliders">
        <div class="slider-item">
          <label>Grosor del Borde: <span id="val-bw">5.5px</span></label>
          <input type="range" id="range-bw" min="3" max="10" step="0.5" value="5.5" oninput="updateInteractive()">
        </div>
        <div class="slider-item">
          <label>Radio de Esquinas: <span id="val-r">18px</span></label>
          <input type="range" id="range-r" min="12" max="24" step="1" value="18" oninput="updateInteractive()">
        </div>
        <label class="toggle-item">
          <input type="checkbox" id="chk-darkwire" onchange="updateInteractive()">
          Línea oscura exterior de 1px (Alambre)
        </label>
        <label class="toggle-item">
          <input type="checkbox" id="chk-whitewire" onchange="updateInteractive()">
          Línea blanca interior de 1px
        </label>
        <label class="toggle-item">
          <input type="checkbox" id="chk-inshadow" onchange="updateInteractive()">
          Sombra que pisa la carátula
        </label>
        <label class="toggle-item">
          <input type="checkbox" id="chk-caustic" checked onchange="updateInteractive()">
          Cáustica inferior de luz (--g2)
        </label>
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
  updateInteractive();
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

  document.querySelectorAll('.cov-img').forEach(img => {
    img.src = g.b64;
  });
  updateInteractive();
}

function updateInteractive() {
  const bw = parseFloat(document.getElementById('range-bw').value);
  const r = parseFloat(document.getElementById('range-r').value);
  const darkWire = document.getElementById('chk-darkwire').checked;
  const whiteWire = document.getElementById('chk-whitewire').checked;
  const inShadow = document.getElementById('chk-inshadow').checked;
  const caustic = document.getElementById('chk-caustic').checked;

  document.getElementById('val-bw').innerText = bw + 'px';
  document.getElementById('val-r').innerText = r + 'px';

  const card = document.getElementById('interactive-card');
  const face = card.querySelector('.face');

  card.style.padding = bw + 'px';
  card.style.borderRadius = r + 'px';
  face.style.borderRadius = Math.max(4, r - bw) + 'px';

  // Construir box-shadow
  let shadows = [
    'inset 0 1.2px 0 rgba(255,255,255,.95)',
    '0 0 24px 2px color-mix(in srgb, var(--f1) 40%, transparent)',
    '0 18px 30px -12px rgba(20,30,50,.55)'
  ];

  if (whiteWire) shadows.push('inset 0 0 0 1px color-mix(in srgb, white 45%, transparent)');
  if (caustic) shadows.push('inset 0 -8px 12px -6px color-mix(in srgb, var(--g2) 50%, white)');
  if (darkWire) shadows.push('0 0 0 1px color-mix(in srgb, var(--f2) 75%, black)');

  card.style.boxShadow = shadows.join(', ');

  if (inShadow) {
    face.style.boxShadow = 'inset 0 0 0 1px rgba(20,30,50,.20), inset 0 2px 4px -1px rgba(0,0,0,.30)';
  } else {
    face.style.boxShadow = 'none';
  }
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
