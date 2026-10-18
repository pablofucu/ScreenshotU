<script lang="ts">
  // Estadisticas de lectura: cifras clave, graficos sencillos de los ultimos 30 dias, palabras nuevas por semana y
  // totales por juego. Los datos los calcula la app (`readstats`); aqui solo se piden y se pintan.
  import { onMount } from 'svelte';
  import type { ReadStats } from '../lib/novel-types';
  import { ICON, dayLabel, fmtDur, fmtNum } from './util';

  let { stats, onclose, send }: { stats: ReadStats | null; onclose: () => void; send: (m: Record<string, unknown>) => void } = $props();
  onMount(() => send({ type: 'statsGet' }));

  const maxChars = $derived(Math.max(1, ...(stats?.days.map((d) => d.chars) ?? [1])));
  const maxKnown = $derived(Math.max(1, ...(stats?.weeks.map((w) => w.newKnown) ?? [1])));
  const todayChars = $derived(stats?.days.at(-1)?.chars ?? 0);
  const weekKnown = $derived(stats?.weeks.at(-1)?.newKnown ?? 0);
  const monthChars = $derived(stats?.days.reduce((s, d) => s + d.chars, 0) ?? 0);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="veil" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal smodal" role="dialog" aria-label="Estadísticas de lectura">
    <div class="mh">
      <span class="logo" style="background:linear-gradient(180deg,#5fcf6c,var(--ok))">{@html ICON.chart}</span>
      <span>Estadísticas de lectura</span>
      <span class="sp"></span>
      <button class="ib" onclick={onclose} aria-label="Cerrar">✕</button>
    </div>
    {#if !stats}
      <div class="mut" style="padding:24px 4px;font-weight:700">Cargando…</div>
    {:else if stats.total.chars === 0 && stats.knownDated === 0}
      <div class="mut" style="padding:24px 4px;font-weight:700">Aún no hay lectura registrada. Las cifras aparecen al leer líneas del juego.</div>
    {:else}
      <div class="stiles">
        <div class="stile"><b>{fmtNum(stats.total.cph)}</b><span>caracteres / hora</span></div>
        <div class="stile"><b>{fmtNum(todayChars)}</b><span>caracteres hoy</span></div>
        <div class="stile"><b>{fmtNum(stats.charsPerDay)}</b><span>caracteres / día leído</span></div>
        <div class="stile"><b>{stats.streak} {stats.streak === 1 ? 'día' : 'días'}</b><span>racha · mejor {stats.bestStreak}</span></div>
        <div class="stile"><b>{fmtNum(stats.total.chars)}</b><span>caracteres en total</span></div>
        <div class="stile"><b>{fmtDur(stats.total.activeMs)}</b><span>tiempo activo</span></div>
        <div class="stile"><b>{weekKnown}</b><span>palabras nuevas esta semana</span></div>
        <div class="stile"><b>{fmtNum(stats.knownTotal)}</b><span>palabras conocidas</span></div>
      </div>

      <div class="stitle">Caracteres por día · últimos 30 días <em>{fmtNum(monthChars)} en total</em></div>
      <div class="sbars" role="img" aria-label="Caracteres leídos cada día">
        {#each stats.days as d (d.day)}
          <div class="sbar" class:today={d.day === stats.today} title="{dayLabel(d.day)}: {fmtNum(d.chars)} caracteres · {fmtDur(d.activeMs)}">
            <i style="height:{d.chars ? Math.max(4, (d.chars / maxChars) * 100) : 0}%"></i>
          </div>
        {/each}
      </div>
      <div class="saxis"><span>{dayLabel(stats.days[0].day)}</span><span>hoy</span></div>

      <div class="stitle">Palabras conocidas nuevas por semana <em>últimas 8 semanas</em></div>
      <div class="sbars wk" role="img" aria-label="Palabras nuevas cada semana">
        {#each stats.weeks as w, i (w.start)}
          <div class="sbar" class:today={i === stats.weeks.length - 1} title="Semana del {dayLabel(w.start)}: {w.newKnown} palabras">
            <em>{w.newKnown || ''}</em>
            <i class="k" style="height:{w.newKnown ? Math.max(6, (w.newKnown / maxKnown) * 70) : 0}%"></i>
          </div>
        {/each}
      </div>
      <div class="saxis"><span>{dayLabel(stats.weeks[0].start)}</span><span>esta semana</span></div>
      {#if stats.knownDated < stats.knownTotal}
        <p class="mut snote">Solo se cuentan las palabras marcadas desde que existe el registro ({stats.knownDated} de {stats.knownTotal}).</p>
      {/if}

      {#if stats.games.length}
        <div class="stitle">Por juego</div>
        <div class="sgames">
          {#each stats.games as g (g.gameId)}
            <div class="sgame">
              <b class="jp">{g.gameName}</b>
              <span>{fmtNum(g.chars)} car.</span>
              <span>{fmtDur(g.activeMs)}</span>
              <span>{fmtNum(g.cph)} car/h</span>
              <span>{g.days} {g.days === 1 ? 'día' : 'días'}</span>
              <span>{g.phrases} {g.phrases === 1 ? 'frase' : 'frases'}</span>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>
