import base64
import io
import json
import os
from PIL import Image

covers = {
    'cyberpunk': {
        'name': 'Cyberpunk 2077',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f12910e9163-ddc20bcb/cover-1091500.jpg',
        'c1': '#b0aa00',
        'c2_actual': '#c8635d',
        'c2_real': '#3b82f6',
        'c2_harmonic': '#06b6d4',
        'desc_actual': 'Amarillo (#b0aa00) + Naranja capado a 60° (#c8635d)',
        'desc_real': 'Amarillo (#b0aa00) + Azul cobalto de la portada (#3b82f6)',
        'desc_harmonic': 'Amarillo (#b0aa00) + Cian neón (+110°)'
    },
    'elden': {
        'name': 'ELDEN RING',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f1291ae916d-e8736c63/cover-1245620.jpg',
        'c1': '#d59739',
        'c2_actual': '#9a8d42',
        'c2_real': '#00bdb5',
        'c2_harmonic': '#0ea5e9',
        'desc_actual': 'Oro (#d59739) + Oliva capado a 60° (#9a8d42)',
        'desc_real': 'Oro (#d59739) + Turquesa espectral de la portada (#00bdb5)',
        'desc_harmonic': 'Oro (#d59739) + Azul éter (+110°)'
    },
    'xeno': {
        'name': 'Xenoblade Chronicles 3',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f1298ee91e1-96f46ed4/cover-690563.png',
        'c1': '#3c8bd0',
        'c2_actual': '#00b9d2',
        'c2_real': '#df4d46',
        'c2_harmonic': '#ef4444',
        'desc_actual': 'Azul (#3c8bd0) + Verde azulado capado a 60° (#00b9d2)',
        'desc_real': 'Azul (#3c8bd0) + Rojo espada de Mio (#df4d46)',
        'desc_harmonic': 'Azul (#3c8bd0) + Rojo carmesí (+110°)'
    },
    'mariokart': {
        'name': 'Mario Kart 8 Deluxe',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f12994e91e7-13fad714/cover-421528.png',
        'c1': '#d866db',
        'c2_actual': '#8976cd',
        'c2_real': '#3b82f6',
        'c2_harmonic': '#0284c7',
        'desc_actual': 'Fucsia (#d866db) + Violeta capado a 60° (#8976cd)',
        'desc_real': 'Fucsia (#d866db) + Azul antigravedad (#3b82f6)',
        'desc_harmonic': 'Fucsia (#d866db) + Azul cielo (+110°)'
    },
    'zelda': {
        'name': "Zelda: Link's Awakening",
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f129a0e91f3-8d3be14b/cover-83079.png',
        'c1': '#2ca382',
        'c2_actual': '#488c52',
        'c2_real': '#e09822',
        'c2_harmonic': '#f59e0b',
        'desc_actual': 'Verde (#2ca382) + Verde oliva capado a 60° (#488c52)',
        'desc_real': 'Verde isla (#2ca382) + Ámbar del huevo (#e09822)',
        'desc_harmonic': 'Verde (#2ca382) + Ámbar dorado (+110°)'
    },
    'plague': {
        'name': 'A Plague Tale: Requiem',
        'path': 'P:/ScreenshotUL/Visor de capturas/Grafismos/manual_18e09f128f5e9148-2e168ae9/cover-1182900.jpg',
        'c1': '#c16e2f',
        'c2_actual': '#985828',
        'c2_real': '#2563eb',
        'c2_harmonic': '#3b82f6',
        'desc_actual': 'Cobre (#c16e2f) + Pardo capado a 60° (#985828)',
        'desc_real': 'Fuego (#c16e2f) + Azul noche tormenta (#2563eb)',
        'desc_harmonic': 'Cobre (#c16e2f) + Azul cobalto (+110°)'
    }
}

for k, d in covers.items():
    im = Image.open(d['path']).convert('RGB')
    im.thumbnail((260, 348), Image.Resampling.LANCZOS)
    buf = io.BytesIO()
    im.save(buf, format='JPEG', quality=85)
    d['b64'] = 'data:image/jpeg;base64,' + base64.b64encode(buf.getvalue()).decode('ascii')
    del d['path']

html_content = """<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Prueba de Doble Tono · Carátulas Reales de la BD</title>
<style>
@font-face{font-family:'M PLUS Rounded 1c';font-weight:400;font-display:swap;src:url('../src/assets/fonts/mplus-rounded-1c-400-latin.woff2') format('woff2')}
@font-face{font-family:'M PLUS Rounded 1c';font-weight:700;font-display:swap;src:url('../src/assets/fonts/mplus-rounded-1c-700-latin.woff2') format('woff2')}
@font-face{font-family:'M PLUS Rounded 1c';font-weight:800;font-display:swap;src:url('../src/assets/fonts/mplus-rounded-1c-800-latin.woff2') format('woff2')}

@property --ang {
  syntax: '<angle>';
  inherits: false;
  initial-value: 0deg;
}

:root {
  --round: 'M PLUS Rounded 1c', 'Nunito', 'Segoe UI', system-ui, sans-serif;
  --speed: 3s;
  --theme-bg: #eceef2;
  --theme-text: #2c3340;
  --theme-sub: #657184;
  --card-bg: #e6e9ee;
  color-scheme: light;
}

[data-theme="dark"] {
  --theme-bg: #12151b;
  --theme-text: #f0f3f8;
  --theme-sub: #8e9bb0;
  --card-bg: #1c202a;
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
  background: rgba(255,255,255,.84);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  border-bottom: 1px solid rgba(0,0,0,.08);
  padding: 16px 28px;
}
[data-theme="dark"] header {
  background: rgba(18,21,27,.88);
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
  font-size: 21px;
  font-weight: 800;
  display: flex;
  align-items: center;
  gap: 10px;
}
h1 span {
  font-size: 12px;
  font-weight: 800;
  background: #00A0D8;
  color: #fff;
  padding: 3px 10px;
  border-radius: 999px;
  letter-spacing: .04em;
}

.controls-bar {
  display: flex;
  gap: 10px;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 14px;
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
  font-size: 11.5px;
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

.dot-color {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  display: inline-block;
  box-shadow: 0 0 0 1px rgba(0,0,0,.15);
}

.container {
  max-width: 1420px;
  margin: 0 auto;
  padding: 24px 28px 80px;
}

/* Panel de Diagnóstico de los Colores */
.diag-panel {
  background: rgba(255,255,255,.72);
  border: 1px solid rgba(0,0,0,.08);
  border-radius: 18px;
  padding: 16px 20px;
  margin-bottom: 26px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  flex-wrap: wrap;
}
[data-theme="dark"] .diag-panel {
  background: rgba(24,28,36,.72);
  border-color: rgba(255,255,255,.07);
}

.diag-info {
  display: flex;
  align-items: center;
  gap: 14px;
}
.diag-swatches {
  display: flex;
  gap: 6px;
  align-items: center;
}
.swatch-box {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  box-shadow: inset 0 1px 0 rgba(255,255,255,.5), 0 2px 5px rgba(0,0,0,.15);
  border: 1px solid rgba(0,0,0,.1);
}
.diag-text h3 {
  margin: 0 0 2px;
  font-size: 15px;
  font-weight: 800;
}
.diag-text p {
  margin: 0;
  font-size: 13px;
  color: var(--theme-sub);
}

/* Tarjetas y Rejilla */
.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 24px;
  margin-top: 10px;
}

.variant-box {
  background: rgba(255,255,255,.65);
  border: 1px solid rgba(0,0,0,.08);
  border-radius: 22px;
  padding: 22px 18px 18px;
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;
  box-shadow: 0 4px 18px -6px rgba(40,50,70,.08);
}
[data-theme="dark"] .variant-box {
  background: rgba(24,28,38,.65);
  border-color: rgba(255,255,255,.07);
  box-shadow: 0 8px 24px -10px rgba(0,0,0,.4);
}

.badge-tag {
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
[data-theme="dark"] .badge-tag {
  background: rgba(255,255,255,.08);
}
.badge-tag.highlight {
  background: #00A0D8;
  color: #fff;
}
.badge-tag.warn {
  background: #e67e22;
  color: #fff;
}

.card-stage {
  height: 270px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  margin: 12px 0 6px;
}

/* Tarjeta fiel al visor: 160x214 px */
.tcov {
  position: relative;
  width: 160px;
  height: 214px;
  --r: 16px;
  --bw: 3px;
  border-radius: var(--r);
  padding: var(--bw);
  box-sizing: border-box;
  background: var(--card-bg);
  box-shadow: 0 18px 34px -12px rgba(20,30,50,.55);
  transform: scale(1.06);
  isolation: isolate;
  user-select: none;
}

.tcov .face {
  position: relative;
  z-index: 1;
  width: 100%;
  height: 100%;
  border-radius: calc(var(--r) - var(--bw));
  overflow: hidden;
  background: #11141c;
}
.tcov .face img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

/* Anillo y Halo */
.ring {
  position: absolute;
  inset: -2px;
  z-index: 4;
  border-radius: calc(var(--r) + 2px);
  padding: calc(var(--bw) + 2px);
  box-sizing: border-box;
  -webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
}
.halo {
  position: absolute;
  inset: -12px;
  z-index: 0;
  border-radius: calc(var(--r) + 12px);
  pointer-events: none;
  filter: blur(20px);
  opacity: .85;
}

/* ========================================================
   ESTILOS DE ANIMACIÓN 100% BICOLOR (CERO BRILLO BLANCO)
   ======================================================== */

/* 0. ACTUAL: LINEAR AL 200% (Referencia de salto brusco) */
@keyframes sheen-actual {
  to { background-position: 200% 0; }
}
.v-actual .ring {
  background: linear-gradient(115deg, var(--c1), var(--c2), var(--c1));
  background-size: 200% 100%;
  animation: sheen-actual var(--speed) linear infinite;
}
.v-actual .halo {
  background: linear-gradient(115deg, var(--c1), var(--c2), var(--c1));
  background-size: 200% 100%;
  animation: sheen-actual var(--speed) linear infinite;
}

/* 1. CONIC ROTATORIO CONTINUO (360° fluido, sin cortes) */
@keyframes spin-conic {
  to { --ang: 360deg; }
}
.v-conic .ring {
  background: conic-gradient(from var(--ang), var(--c1), var(--c2), var(--c1));
  animation: spin-conic var(--speed) linear infinite;
}
.v-conic .halo {
  background: conic-gradient(from var(--ang), var(--c1), var(--c2), var(--c1));
  animation: spin-conic var(--speed) linear infinite;
}

/* 2. MAREA OSCILANTE (Ease-in-out alternate sin saltos) */
@keyframes marea-wave {
  0% { background-position: 0% 50%; }
  100% { background-position: 100% 50%; }
}
.v-marea .ring {
  background: linear-gradient(90deg, var(--c1), var(--c2), var(--c1));
  background-size: 250% 100%;
  animation: marea-wave calc(var(--speed) * 0.9) ease-in-out infinite alternate;
}
.v-marea .halo {
  background: linear-gradient(90deg, var(--c1), var(--c2), var(--c1));
  background-size: 250% 100%;
  animation: marea-wave calc(var(--speed) * 0.9) ease-in-out infinite alternate;
}

/* 3. RESPIRACIÓN CROMÁTICA (Fundido suave sin desplazamiento) */
@keyframes hue-breath {
  0%, 100% { opacity: 1; filter: saturate(1); }
  50% { opacity: .82; filter: saturate(1.35); }
}
@keyframes halo-breath {
  0%, 100% { transform: scale(0.96); opacity: .65; }
  50% { transform: scale(1.06); opacity: .95; }
}
.v-breath .ring {
  background: linear-gradient(135deg, var(--c1), var(--c2));
  animation: hue-breath calc(var(--speed) * 0.8) ease-in-out infinite alternate;
}
.v-breath .halo {
  background: radial-gradient(circle, var(--c1), var(--c2), transparent 72%);
  animation: halo-breath calc(var(--speed) * 0.8) ease-in-out infinite alternate;
}

/* 4. ÓRBITA DE DOS POLOS (Mitad y mitad persiguiéndose) */
@keyframes dual-orbit {
  to { --ang: 360deg; }
}
.v-orbit .ring {
  background: conic-gradient(from var(--ang), var(--c1) 0deg, var(--c2) 180deg, var(--c1) 360deg);
  animation: dual-orbit var(--speed) linear infinite;
}
.v-orbit .halo {
  background: conic-gradient(from var(--ang), var(--c1) 0deg, var(--c2) 180deg, var(--c1) 360deg);
  animation: dual-orbit var(--speed) linear infinite;
}

.v-info {
  width: 100%;
  border-top: 1px solid rgba(0,0,0,.07);
  padding-top: 12px;
  margin-top: 10px;
}
[data-theme="dark"] .v-info {
  border-top-color: rgba(255,255,255,.07);
}
.v-title {
  font-size: 15px;
  font-weight: 800;
  margin: 0 0 4px;
}
.v-desc {
  font-size: 12.5px;
  line-height: 1.45;
  color: var(--theme-sub);
  margin: 0 0 10px;
  min-height: 52px;
}
.pros-cons {
  font-size: 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: rgba(0,0,0,.03);
  padding: 8px 10px;
  border-radius: 10px;
}
[data-theme="dark"] .pros-cons {
  background: rgba(255,255,255,.04);
}
.pro { color: #10b981; font-weight: 700; }
.con { color: #ef4444; font-weight: 700; }
</style>
</head>
<body data-theme="light">

<header>
  <div class="header-top">
    <h1>
      Prueba de Doble Tono · Carátulas Reales de la BD
      <span>100% Color Puro</span>
    </h1>
    <div class="c-group">
      <button class="btn-pill" id="themeBtn" onclick="toggleTheme()">☀️ Modo claro</button>
      <button class="btn-pill" id="pauseBtn" onclick="togglePause()">⏸️ Pausar animación</button>
    </div>
  </div>

  <div class="controls-bar">
    <div class="c-group">
      <span class="c-label">Juego Real:</span>
      <button class="btn-pill active" onclick="selectGame('cyberpunk', this)"><span class="dot-color" style="background:#b0aa00"></span> Cyberpunk 2077</button>
      <button class="btn-pill" onclick="selectGame('elden', this)"><span class="dot-color" style="background:#d59739"></span> Elden Ring</button>
      <button class="btn-pill" onclick="selectGame('xeno', this)"><span class="dot-color" style="background:#3c8bd0"></span> Xenoblade 3</button>
      <button class="btn-pill" onclick="selectGame('mariokart', this)"><span class="dot-color" style="background:#d866db"></span> Mario Kart 8</button>
      <button class="btn-pill" onclick="selectGame('zelda', this)"><span class="dot-color" style="background:#2ca382"></span> Zelda: Link's Aw.</button>
      <button class="btn-pill" onclick="selectGame('plague', this)"><span class="dot-color" style="background:#c16e2f"></span> A Plague Tale</button>
    </div>

    <div style="flex:1"></div>

    <div class="c-group">
      <span class="c-label">Modo de Color:</span>
      <button class="btn-pill" onclick="selectColorMode('actual', this)" title="Con el recorte de 60° actual de la app">🔒 Actual (Capado 60°)</button>
      <button class="btn-pill active" onclick="selectColorMode('real', this)" title="Color dominante + 2º color real de la carátula sin recorte">🔓 Paleta Real Desbloqueada</button>
      <button class="btn-pill" onclick="selectColorMode('harmonic', this)" title="Tono 1 + Separación armónica de 110°">🎨 Separación Armónica</button>
    </div>

    <div class="c-group">
      <span class="c-label">Velocidad:</span>
      <button class="btn-pill" onclick="setSpeed('5s', this)">Lenta</button>
      <button class="btn-pill active" onclick="setSpeed('3s', this)">Normal</button>
      <button class="btn-pill" onclick="setSpeed('1.6s', this)">Rápida</button>
    </div>
  </div>
</header>

<div class="container">

  <div class="diag-panel">
    <div class="diag-info">
      <div class="diag-swatches">
        <div class="swatch-box" id="swatch-c1" style="background:#b0aa00" title="Tono 1"></div>
        <div style="font-size:18px;font-weight:800;color:var(--theme-sub)">+</div>
        <div class="swatch-box" id="swatch-c2" style="background:#3b82f6" title="Tono 2"></div>
      </div>
      <div class="diag-text">
        <h3 id="diag-title">Cyberpunk 2077</h3>
        <p id="diag-desc">Amarillo (#b0aa00) + Azul cobalto de la portada (#3b82f6)</p>
      </div>
    </div>
    <div style="font-size:13px;color:var(--theme-sub);max-width:440px;line-height:1.4">
      ✨ <b>100% Color del juego:</b> Cero destellos blancos. Solo el juego puro y su auténtica paleta de portada.
    </div>
  </div>

  <div class="cards-grid">

    <!-- VARIANTE 0: ACTUAL -->
    <div class="variant-box">
      <span class="badge-tag warn">0. Actual</span>
      <div class="card-stage">
        <div class="tcov v-actual">
          <div class="face"><img class="cover-img" src="" alt="" /></div>
          <i class="ring"></i><i class="halo"></i>
        </div>
      </div>
      <div class="v-info">
        <div class="v-title">Linear Trasladado (Actual)</div>
        <div class="v-desc">El degradado se desplaza horizontalmente. Puedes observar el salto brusco cada vez que termina el ciclo.</div>
        <div class="pros-cons">
          <span class="con">✕ Salto brusco en cada vuelta.</span>
          <span class="con">✕ Los laterales verticales apenas tienen cambio.</span>
        </div>
      </div>
    </div>

    <!-- VARIANTE 1: CONIC ROTATORIO BICOLOR -->
    <div class="variant-box">
      <span class="badge-tag highlight">1. Recomendada (Conic)</span>
      <div class="card-stage">
        <div class="tcov v-conic">
          <div class="face"><img class="cover-img" src="" alt="" /></div>
          <i class="ring"></i><i class="halo"></i>
        </div>
      </div>
      <div class="v-info">
        <div class="v-title">Conic-Gradient Bicolor Continuo</div>
        <div class="v-desc">Los dos colores giran 360° perimétricamente alrededor del marco. Movimiento líquido y continuo sin saltos.</div>
        <div class="pros-cons">
          <span class="pro">✓ Cero saltos: matemáticamente continuo.</span>
          <span class="pro">✓ Ambos colores recorren las 4 esquinas de la tarjeta.</span>
        </div>
      </div>
    </div>

    <!-- VARIANTE 2: MAREA OSCILANTE -->
    <div class="variant-box">
      <span class="badge-tag highlight">2. Recomendada (Marea)</span>
      <div class="card-stage">
        <div class="tcov v-marea">
          <div class="face"><img class="cover-img" src="" alt="" /></div>
          <i class="ring"></i><i class="halo"></i>
        </div>
      </div>
      <div class="v-info">
        <div class="v-title">Marea Bicolor Oscilante</div>
        <div class="v-desc">El degradado se mece de izquierda a derecha y regresa frenando suavemente (ease-in-out alternate).</div>
        <div class="pros-cons">
          <span class="pro">✓ Elimina el corte de ciclo: va y viene con suavidad.</span>
          <span class="pro">✓ Efecto muy armónico y calmado.</span>
        </div>
      </div>
    </div>

    <!-- VARIANTE 3: RESPIRACIÓN CROMÁTICA -->
    <div class="variant-box">
      <span class="badge-tag">3. Respiración</span>
      <div class="card-stage">
        <div class="tcov v-breath">
          <div class="face"><img class="cover-img" src="" alt="" /></div>
          <i class="ring"></i><i class="halo"></i>
        </div>
      </div>
      <div class="v-info">
        <div class="v-title">Respiración Cromática</div>
        <div class="v-desc">Sin traslación espacial. El marco funde e intensifica suavemente la interacción entre ambos tonos.</div>
        <div class="pros-cons">
          <span class="pro">✓ Cero sensación de movimiento o distracción.</span>
          <span class="con">✕ Más estático que los modos rotatorios.</span>
        </div>
      </div>
    </div>

    <!-- VARIANTE 4: ÓRBITA DE DOS POLOS -->
    <div class="variant-box">
      <span class="badge-tag">4. Doble Polo</span>
      <div class="card-stage">
        <div class="tcov v-orbit">
          <div class="face"><img class="cover-img" src="" alt="" /></div>
          <i class="ring"></i><i class="halo"></i>
        </div>
      </div>
      <div class="v-info">
        <div class="v-title">Órbita de Dos Polos</div>
        <div class="v-desc">Cada color ocupa un hemisferio opuesto y van persiguiéndose en rotación circular equilibrada.</div>
        <div class="pros-cons">
          <span class="pro">✓ Equilibrio perfecto 50/50 entre ambos tonos.</span>
          <span class="pro">✓ Transición suave en ambos puntos de encuentro.</span>
        </div>
      </div>
    </div>

  </div>

</div>

<script>
const DB_GAMES = """ + json.dumps(covers) + """;

let currentGame = 'cyberpunk';
let currentColorMode = 'real';
let isPaused = false;

function updateUI() {
  const g = DB_GAMES[currentGame];
  const c1 = g.c1;
  let c2 = g.c2_real;
  let desc = g.desc_real;

  if (currentColorMode === 'actual') {
    c2 = g.c2_actual;
    desc = g.desc_actual;
  } else if (currentColorMode === 'harmonic') {
    c2 = g.c2_harmonic;
    desc = g.desc_harmonic;
  }

  document.documentElement.style.setProperty('--c1', c1);
  document.documentElement.style.setProperty('--c2', c2);

  document.getElementById('swatch-c1').style.background = c1;
  document.getElementById('swatch-c2').style.background = c2;
  document.getElementById('diag-title').textContent = g.name;
  document.getElementById('diag-desc').textContent = desc;

  document.querySelectorAll('.cover-img').forEach(img => {
    img.src = g.b64;
  });
}

function selectGame(id, btn) {
  currentGame = id;
  btn.parentNode.querySelectorAll('button').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  updateUI();
}

function selectColorMode(mode, btn) {
  currentColorMode = mode;
  btn.parentNode.querySelectorAll('button').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  updateUI();
}

function setSpeed(spd, btn) {
  document.documentElement.style.setProperty('--speed', spd);
  btn.parentNode.querySelectorAll('button').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
}

function toggleTheme() {
  const body = document.body;
  const isDark = body.getAttribute('data-theme') === 'dark';
  body.setAttribute('data-theme', isDark ? 'light' : 'dark');
  document.getElementById('themeBtn').textContent = isDark ? '☀️ Modo claro' : '🌙 Modo oscuro';
}

function togglePause() {
  isPaused = !isPaused;
  const style = isPaused ? 'paused' : 'running';
  document.querySelectorAll('.ring, .halo').forEach(el => {
    el.style.animationPlayState = style;
  });
  document.getElementById('pauseBtn').textContent = isPaused ? '▶️ Reanudar' : '⏸️ Pausar animación';
}

// Inicializar
updateUI();
</script>
</body>
</html>"""

out_path = 'design/prueba-doble-tono-caratulas-reales.html'
with open(out_path, 'w', encoding='utf-8') as f:
    f.write(html_content)

print('Generated successfully:', out_path, 'Size:', os.path.getsize(out_path))
