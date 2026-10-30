<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { backend, thumbNow, decodeImg } from '../lib/backend';
  import { ICON } from '../lib/icons';
  import {
    achState,
    cycleCardVariant,
    openAchievementsModal,
    VARIANT_SHORT_NAMES,
    getRankTier,
  } from '../lib/achievements.svelte';
  import type { AchievementSummary } from '../lib/types';
  import Thumb from './Thumb.svelte';

  interface Props {
    gameId: string;
    title: string;
    coverPath?: string | null;
    summary?: AchievementSummary | null;
    focused?: boolean;
  }

  const { gameId, title, coverPath, summary, focused }: Props = $props();

  const variant = $derived(app.config.achievementsCardVariant ?? summary?.variant ?? 1);
  const total = $derived(summary?.total ?? 0);
  const unlocked = $derived(summary?.unlocked ?? 0);
  const pct = $derived(summary?.percentage ?? 0);
  const last = $derived(summary?.lastUnlocked ?? null);

  const rank = $derived(getRankTier(pct));

  // Blur cover fallback for variant background
  let blur = $state('');
  $effect(() => {
    if (!coverPath) { blur = ''; return; }
    const known = thumbNow(coverPath);
    if (known) { blur = known; return; }
    const ctl = new AbortController();
    backend.thumb(coverPath, ctl.signal).then(
      (u) => decodeImg(u).then(() => { if (!ctl.signal.aborted) blur = u; }),
      () => {}
    );
    return () => ctl.abort();
  });

  // SVG Icons
  const trophySvg = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 4h8v5a4 4 0 0 1-8 0z"/><path d="M8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8 20h8M9 17h6"/></svg>`;
  const lockSvg = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></svg>`;

  function onCardClick() {
    openAchievementsModal(gameId, title, coverPath);
  }
</script>

<svg width="0" height="0" style="position:absolute">
  <defs>
    <linearGradient id="card-gold" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#ffe27a" />
      <stop offset="1" stop-color="#e3a400" />
    </linearGradient>
    <linearGradient id="card-silver" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#ffffff" />
      <stop offset="0.45" stop-color="#d5dbe4" />
      <stop offset="1" stop-color="#8f9aab" />
    </linearGradient>
  </defs>
</svg>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="glass dcover fx ach-card"
  class:f={focused}
  data-k="dach"
  data-act="achievements"
  role="button"
  tabindex="-1"
  onclick={onCardClick}
  title="Ver lista de logros (A) · Cambiar diseño (Y)"
>
  <div class="face">
    {#if !summary || total === 0}
      <!-- Sin logros: Portada normal -->
      {#if coverPath}
        <Thumb path={coverPath} />
      {:else}
        <div class="noart"><b>{title}</b></div>
      {/if}
    {:else if variant === 1}
      <!-- VARIANTE 1: Anillo + Último logro -->
      <div class="fc v1">
        <div class="blurc" style={blur ? `background-image:url(${blur})` : ''}></div>
        <div class="lab">
          {@html trophySvg}
          <span>Logros</span>
          <em>Y · cambiar</em>
        </div>

        <div class="rg" style="width:112px;height:112px">
          <svg viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="45" fill="none" stroke="rgba(255,255,255,.18)" stroke-width="9" />
            <circle
              cx="50"
              cy="50"
              r="45"
              fill="none"
              stroke="url(#card-gold)"
              stroke-width="9"
              stroke-linecap="round"
              pathLength="100"
              stroke-dasharray="{(pct).toFixed(1)} 100"
              transform="rotate(-90 50 50)"
            />
          </svg>
          <div class="in">
            <b>{unlocked}<small>/{total}</small></b>
            <i>{pct} %</i>
          </div>
        </div>

        <div class="last">
          <div class="ai" style="--h:45;width:34px;height:34px">
            {#if last?.icon}
              <img src={last.icon} alt={last.displayName} class="card-ico-img" />
            {:else}
              {@html trophySvg}
            {/if}
          </div>
          <div class="last-txt">
            <b>{last ? last.displayName : 'Sin desbloqueos'}</b>
            <small>{last ? (last.unlockedAt ? 'Desbloqueado' : '¡Nuevo!') : 'Juega para empezar'}</small>
          </div>
        </div>
      </div>
    {:else if variant === 2}
      <!-- VARIANTE 2: Portada + Banda inferior -->
      <div class="fc v2">
        <div class="cov">
          {#if coverPath}
            <Thumb path={coverPath} />
          {:else}
            <b>{title}</b>
          {/if}
        </div>
        <span class="pct-chip">
          {@html trophySvg}
          <span>{pct} %</span>
        </span>
        <div class="band dk">
          <div class="r">
            <b>{unlocked} <small>/ {total} logros</small></b>
          </div>
          <div class="gb"><i style="width:{pct}%"></i></div>
          <div class="ics">
            {#each (achState.list && achState.list.length > 0 ? achState.list.filter(a => a.earned).slice(0, 5) : Array(Math.min(5, unlocked)).fill(null)) as ach, i}
              <div class="ai mini" style="--h:{i * 60 + 20};width:20px;height:20px">
                {#if ach?.icon}
                  <img src={ach.icon} alt={ach?.displayName ?? ''} class="card-ico-img" />
                {:else}
                  {@html trophySvg}
                {/if}
              </div>
            {/each}
          </div>
        </div>
      </div>
    {:else if variant === 3}
      <!-- VARIANTE 3: Mosaico de la colección con todos los iconos -->
      <div class="fc v3">
        <div class="blurc" style={blur ? `background-image:url(${blur})` : ''}></div>
        <div class="lab">
          {@html trophySvg}
          <span>Colección</span>
          <em>{unlocked}/{total}</em>
        </div>
        <div class="mos">
          {#each (achState.list && achState.list.length > 0 ? achState.list : Array(Math.min(25, total)).fill(null)).slice(0, 25) as ach, i}
            {@const isEarned = ach ? ach.earned : i < unlocked}
            {@const isSecret = ach ? (ach.hidden && !ach.earned) : false}
            <div
              class="mos-ico"
              class:locked={!isEarned}
              class:secret={isSecret}
              title={ach?.displayName ?? ''}
            >
              {#if ach?.icon}
                <img src={ach.icon} alt={ach?.displayName ?? ''} />
              {:else if isSecret}
                <span>?</span>
              {:else if isEarned}
                {@html trophySvg}
              {:else}
                {@html lockSvg}
              {/if}
            </div>
          {/each}
        </div>
        <div class="ft">
          <small>
            <span>{pct} % completado</span>
            <span>{total - unlocked} restantes</span>
          </small>
          <div class="gb"><i style="width:{pct}%"></i></div>
        </div>
      </div>
    {:else if variant === 4}
      <!-- VARIANTE 4: Medalla de rango -->
      <div class="fc v4">
        <svg viewBox="0 0 120 128" width="112" height="120">
          <path d="M36 0h20l14 42H50z" fill="#3F63E0" />
          <path d="M84 0H64L50 42h20z" fill="#E8384F" />
          <circle cx="60" cy="80" r="44" fill="url(#card-silver)" stroke="#fff" stroke-width="3" />
          <circle cx="60" cy="80" r="35" fill="none" stroke="rgba(255,255,255,.75)" stroke-width="2" stroke-dasharray="3 4" />
          <text x="60" y="88" text-anchor="middle" font-family="'M PLUS Rounded 1c', sans-serif" font-size="24" font-weight="800" fill="#4a5260">
            {pct}%
          </text>
        </svg>
        <div class="t">{rank.name}</div>
        <div class="s">{unlocked} de {total} logros</div>
        <div class="nx">
          <small>
            <span>Siguiente: <b>{rank.next}</b></span>
            <span>{pct >= 100 ? '¡Completado!' : `faltan ${Math.max(1, total - unlocked)}`}</span>
          </small>
          <div class="gb"><i style="width:{pct}%"></i></div>
        </div>
      </div>
    {:else if variant === 7}
      <!-- VARIANTE 7: Contador rodillo de horas -->
      <div class="fc v7">
        <div class="hs">Logros conseguidos</div>
        <div class="odo">
          <span class="c">{Math.floor(unlocked / 10)}</span>
          <span class="c">{unlocked % 10}</span>
          <span class="u">/ {total}</span>
        </div>
        <div class="pc">{pct} % completado</div>
        <div class="wkb">
          <div class="r">
            <span>Esta semana</span>
            <b>+{summary.weekDistribution.reduce((a, b) => a + b, 0)}</b>
          </div>
          <div class="bars">
            {#each summary.weekDistribution as v, i}
              {@const label = ['V', 'S', 'D', 'L', 'M', 'X', 'J'][i]}
              <span>
                <i class:z={!v} style="height:{v ? Math.min(26, v * 8 + 4) : 3}px"></i>
                <small>{label}</small>
              </span>
            {/each}
          </div>
        </div>
      </div>
    {:else if variant === 8}
      <!-- VARIANTE 8: Por rareza -->
      <div class="fc v8">
        <div class="blurc" style={blur ? `background-image:url(${blur})` : ''}></div>
        <div class="lab">
          {@html trophySvg}
          <span>Por rareza</span>
        </div>
        <div class="rg" style="width:110px;height:110px">
          <svg viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="44" fill="none" stroke="rgba(255,255,255,.15)" stroke-width="10" />
            <!-- Segmentos aproximados según distribución -->
            <circle
              cx="50" cy="50" r="44" fill="none" stroke="#9fc0dc" stroke-width="10"
              pathLength="24" stroke-dasharray="{Math.max(1, summary.rarityCommon * (total ? 24 / total : 1))} 24"
              transform="rotate(-90 50 50)"
            />
            <circle
              cx="50" cy="50" r="44" fill="none" stroke="#b67cf0" stroke-width="10"
              pathLength="24" stroke-dasharray="{Math.max(1, summary.rarityRare * (total ? 24 / total : 1))} 24"
              stroke-dashoffset="-{summary.rarityCommon * (total ? 24 / total : 1)}"
              transform="rotate(-90 50 50)"
            />
            <circle
              cx="50" cy="50" r="44" fill="none" stroke="url(#card-gold)" stroke-width="10"
              pathLength="24" stroke-dasharray="{Math.max(1, summary.rarityUltra * (total ? 24 / total : 1))} 24"
              stroke-dashoffset="-{(summary.rarityCommon + summary.rarityRare) * (total ? 24 / total : 1)}"
              transform="rotate(-90 50 50)"
            />
          </svg>
          <div class="in">
            <b>{unlocked}<small>/{total}</small></b>
          </div>
        </div>
        <div class="lg">
          <div><i style="background:#9fc0dc"></i>Comunes<b>{summary.rarityCommon}</b></div>
          <div><i style="background:#b67cf0"></i>Raros<b>{summary.rarityRare}</b></div>
          <div><i style="background:var(--gold)"></i>Ultra raros<b>{summary.rarityUltra}</b></div>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .ach-card {
    cursor: pointer;
    user-select: none;
  }
  .fc {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .fc > * {
    position: relative;
    z-index: 1;
  }
  .blurc {
    position: absolute;
    inset: -20px;
    background-size: cover;
    background-position: center;
    background-color: #1e2638;
    filter: blur(14px) brightness(0.65) saturate(1.3);
    z-index: 0 !important;
  }
  .blurc::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(16, 20, 34, 0.45), rgba(16, 20, 34, 0.85) 75%);
  }

  .card-ico-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: inherit;
  }

  .lab {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #ffe08a;
  }
  .lab :global(svg) {
    width: 14px;
    height: 14px;
  }
  .lab em {
    margin-left: auto;
    font-style: normal;
    letter-spacing: 0;
    text-transform: none;
    color: rgba(255, 255, 255, 0.65);
    font-size: 10.5px;
  }

  .rg {
    position: relative;
    flex: none;
    margin: 4px auto 0;
  }
  .rg svg {
    width: 100%;
    height: 100%;
  }
  .rg .in {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  .ai {
    position: relative;
    flex: none;
    border-radius: 22%;
    display: grid;
    place-items: center;
    color: #fff;
    background: linear-gradient(150deg, #f59e0b, #d97706);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.4), 0 2px 5px rgba(0, 0, 0, 0.3);
  }
  .ai :global(svg) {
    width: 58%;
    height: 58%;
  }

  .gb {
    height: 6px;
    border-radius: 3px;
    background: rgba(120, 130, 150, 0.25);
    overflow: hidden;
  }
  .gb i {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: linear-gradient(90deg, #ffd75a, #F2B807);
  }

  /* 1 · Anillo */
  .v1 {
    color: #fff;
    padding: 12px 12px 10px;
    align-items:center;
  }
  .v1 .rg b {
    font-size: 32px;
    font-weight: 800;
  }
  .v1 .rg b small {
    font-size: 15px;
    opacity: 0.7;
  }
  .v1 .rg i {
    font-style: normal;
    font-size: 12px;
    font-weight: 800;
    color: #ffe08a;
    margin-top: 3px;
  }
  .v1 .last {
    align-self: stretch;
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px;
    border-radius: 13px;
    background: rgba(255, 255, 255, 0.12);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.15);
  }
  .last-txt {
    min-width: 0;
  }
  .last-txt b {
    display: block;
    font-size: 12.5px;
    font-weight: 800;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .last-txt small {
    display: block;
    font-size: 10.5px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.75);
  }

  /* 2 · Portada + Banda */
  .v2 .cov {
    position: absolute;
    inset: 0;
  }
  .v2 .pct-chip {
    position: absolute;
    left: 8px;
    top: 8px;
    z-index: 2;
    height: 22px;
    padding: 0 8px;
    border-radius: 999px;
    background: linear-gradient(180deg, #ffe58a, #F2B807);
    color: #3a2f00;
    font-size: 11px;
    font-weight: 800;
    display: flex;
    align-items: center;
    gap: 4px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
  }
  .v2 .pct-chip :global(svg) {
    width: 12px;
    height: 12px;
  }
  .v2 .band {
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: 6px;
    height: 82px;
    border-radius: 14px;
    padding: 9px 10px;
    color: #fff;
    background: rgba(14, 18, 30, 0.65);
    backdrop-filter: blur(10px);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.25);
  }
  .v2 .band .r {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }
  .v2 .band b {
    font-size: 20px;
    font-weight: 800;
  }
  .v2 .band b small {
    font-size: 12px;
    opacity: 0.7;
    font-weight: 700;
  }
  .v2 .band .ics {
    display: flex;
    gap: 4px;
    margin-top: 8px;
  }

  /* 3 · Mosaico de la colección */
  .v3 {
    background: linear-gradient(180deg, #2c3446, #151a25);
    color: #fff;
    padding: 10px 8px 9px;
    align-items: center;
  }
  .v3 .lab {
    align-self: stretch;
    padding: 0 4px;
    color: #ffe08a;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .v3 .lab em {
    margin-left: auto;
    font-style: normal;
    color: #fff;
    font-size: 13px;
    font-weight: 700;
  }
  .v3 .mos {
    display: grid;
    grid-template-columns: repeat(5, 30px);
    gap: 5px;
    margin-top: 8px;
    justify-content: center;
  }
  .mos-ico {
    width: 30px;
    height: 30px;
    border-radius: 6px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.08);
    display: grid;
    place-items: center;
    font-size: 11px;
    font-weight: 800;
    color: #ffe08a;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }
  .mos-ico img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 6px;
  }
  .mos-ico :global(svg) {
    width: 16px;
    height: 16px;
  }
  .mos-ico.locked {
    opacity: 0.35;
    filter: grayscale(1);
    color: #8a94a6;
  }
  .mos-ico.secret {
    opacity: 0.25;
    color: #94a3b8;
    background: rgba(0, 0, 0, 0.25);
  }
  .v3 .ft {
    align-self: stretch;
    margin-top: auto;
    padding: 0 4px;
  }
  .v3 .ft small {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    font-weight: 800;
    color: rgba(255, 255, 255, 0.7);
    margin-bottom: 5px;
  }

  /* 4 · Medalla */
  .v4 {
    background: radial-gradient(120% 70% at 50% 20%, #fff, #e6eaf0);
    align-items: center;
    padding: 8px 10px 10px;
    color: #3a414e;
    text-align: center;
  }
  .v4 .t {
    font-size: 20px;
    font-weight: 800;
    margin-top: -2px;
  }
  .v4 .s {
    font-size: 12px;
    font-weight: 700;
    color: #7a8291;
  }
  .v4 .nx {
    align-self: stretch;
    margin-top: auto;
    padding: 7px 9px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.85);
    box-shadow: inset 0 0 0 1px rgba(95, 110, 135, 0.18);
    text-align: left;
  }
  .v4 .nx small {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    font-weight: 800;
    color: #6a7282;
    margin-bottom: 5px;
  }
  .v4 .nx small b {
    color: #a67c00;
  }

  /* 7 · Odómetro / Contador */
  .v7 {
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.96), #eef1f5);
    align-items: center;
    padding: 12px 10px 10px;
    color: #3a414e;
  }
  .v7 .hs {
    font-size: 12.5px;
    font-weight: 700;
    color: #5f6776;
    align-self: stretch;
    text-align: center;
  }
  .v7 .odo {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-top: 8px;
  }
  .v7 .odo .c {
    width: 44px;
    height: 56px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    font-size: 40px;
    font-weight: 800;
    color: #c49400;
    background: linear-gradient(180deg, #fffdf3, #f4efdc);
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.18), 0 0 0 1px rgba(110, 125, 150, 0.25);
  }
  .v7 .odo .u {
    font-size: 16px;
    font-weight: 800;
    color: #5a6270;
    margin: 0 0 0 3px;
    align-self: flex-end;
    padding-bottom: 8px;
  }
  .v7 .pc {
    font-size: 11.5px;
    font-weight: 800;
    color: #a67c00;
    margin-top: 6px;
  }
  .v7 .wkb {
    align-self: stretch;
    margin-top: auto;
    padding: 8px 8px 6px;
    border-radius: 12px;
    background: #fff;
    box-shadow: inset 0 0 0 1px rgba(95, 110, 135, 0.15);
  }
  .v7 .wkb .r {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    font-weight: 800;
    color: #6a7282;
    margin-bottom: 6px;
  }
  .v7 .wkb .r b {
    color: #a67c00;
  }
  .v7 .bars {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    height: 28px;
  }
  .v7 .bars span {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    font-size: 9px;
    font-weight: 800;
    color: #9aa1ad;
  }
  .v7 .bars i {
    width: 14px;
    border-radius: 4px 4px 2px 2px;
    background: linear-gradient(180deg, #ffe27a, #e3a400);
  }
  .v7 .bars i.z {
    background: rgba(160, 168, 182, 0.3);
  }

  /* 8 · Rareza */
  .v8 {
    color: #fff;
    padding: 11px 11px 10px;
    align-items: center;
  }
  .v8 .rg b {
    font-size: 28px;
    font-weight: 800;
  }
  .v8 .rg b small {
    font-size: 14px;
    opacity: 0.7;
  }
  .v8 .lg {
    align-self: stretch;
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .v8 .lg div {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 20px;
    padding: 0 8px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.08);
    font-size: 11px;
    font-weight: 700;
  }
  .v8 .lg i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .v8 .lg b {
    margin-left: auto;
    font-weight: 800;
  }
</style>
