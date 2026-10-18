<script lang="ts">
  import { app, SORT_LABEL } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { fmt, fmtDay, fmtDayShort, fmtDur, fmtMonth, fmtShort, naiveNow, resText, nCapturas } from '../lib/format';
  import { GRID_VIEW_H, curLayout, daysOn } from '../lib/controller.svelte';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import Thumb from '../components/Thumb.svelte';

  const T = $derived(app.cur);
  const shots = $derived(T.shots);
  const L = $derived(curLayout());
  // Desplazamiento virtual: solo existen en el DOM las miniaturas (y cabeceras) cercanas a la zona visible.
  const lo = $derived(app.goff - L.h - 60);
  const hi = $derived(app.goff + GRID_VIEW_H + 60);
  const idx = $derived.by(() => {
    const out: number[] = [];
    for (let i = 0; i < L.y.length; i++) if (L.y[i] > lo && L.y[i] < hi) out.push(i);
    return out;
  });
  const heads = $derived(L.heads.filter((h) => h.y > lo && h.y < hi));
  const ratio = $derived(Math.min(1, GRID_VIEW_H / L.total));
  const cur = $derived(shots[app.s]);
  const crumbs = $derived(app.favView ? ['Inicio', 'Favoritas'] : ['Inicio', app.platform?.name ?? '', T.name]);
  const coverPath = $derived(T.cover ?? shots[0]?.path);
  const title = $derived(!app.favView ? app.platform?.titles[app.t] : undefined);
  const members = $derived(title?.members);
  const days = $derived(L.days);
  const wantDays = $derived(app.config.shotsView === 'days');

  // Los filtros de una saga con muchos juegos no caben: el activo se mantiene a la vista.
  let sfEl = $state<HTMLElement>();
  $effect(() => {
    void app.sagaFilter;
    const on = sfEl?.querySelector<HTMLElement>('.sf.on');
    if (sfEl && on) sfEl.scrollLeft = on.offsetLeft - sfEl.clientWidth / 2 + on.offsetWidth / 2;
  });

  // Minutos jugados ese día (solo de los últimos 7 días, que es lo que guarda la ficha).
  function played(date: number) {
    if (!title || members) return 0;
    const ago = Math.floor(naiveNow() / 864e5) - Math.floor(date / 864e5);
    return ago >= 0 && ago < 7 ? (title.week?.[6 - ago] ?? 0) : 0;
  }

  // Línea de tiempo: una marca por mes (la primera cabecera de cada mes), sin solapar etiquetas.
  const TRACK = 440;
  const months = $derived.by(() => {
    const out: Array<{ y: number; l: string }> = [];
    let last = '', lastY = -99;
    for (const h of L.heads) {
      const l = fmtMonth(h.date);
      if (l === last) continue;
      last = l;
      const y = (h.y / L.total) * TRACK;
      if (y - lastY < 16) continue;
      out.push({ y, l });
      lastY = y;
    }
    return out;
  });
  const knob = $derived(L.total > GRID_VIEW_H ? (app.goff / L.total) * TRACK : 0);
</script>

<TopBar {crumbs} />
<div class="ghead">
  <div class="glass mini">
    <div class="face">
      {#if T.isFavs}<span class="mstar">{@html ICON.starY}</span>{:else if coverPath}<Thumb path={coverPath} />{/if}
    </div>
  </div>
  <h3>{T.name}</h3>
  <span class="chip">{nCapturas(shots.length)}</span>
  {#if members}
    <div class="sfilter" bind:this={sfEl}>
      <span class="sf" class:on={app.sagaFilter < 0}>Todos</span>
      {#each members as m, k (m.id)}<span class="sf" class:on={app.sagaFilter === k}>{m.name}</span>{/each}
    </div>
  {:else}
    <span class="soft">{SORT_LABEL[app.sortMode]}</span>
  {/if}
  {#if wantDays && !days}<span class="soft">Por días: solo con orden por fecha</span>{/if}
  {#if app.selMode}<span class="chip selchip">Selección · {app.sel.size}</span>{/if}
  <span class="date">{cur ? [fmt(cur.date), resText(cur.width, cur.height)].filter(Boolean).join(' · ') : ''}</span>
</div>
<div class="gview" class:days>
  <div class="gtrack" style="height:{L.total}px;transform:translateY({-app.goff}px)">
    {#each heads as h (h.first)}
      {@const m = played(h.date)}
      {@const wide = h.cols >= 3}
      <div class="dhead" class:mid={h.x > 68} style="top:{h.y}px;left:{h.x}px;width:{h.cols * 224 - 18}px">{wide ? fmtDay(h.date) : fmtDayShort(h.date)}<span>{wide && m > 0 ? `· jugado ${fmtDur(m * 60)} ` : ''}· {wide ? nCapturas(h.n) : h.n}</span></div>
    {/each}
    {#each idx as i (shots[i].path)}
      <div class="glass shot fx" class:sm={days} class:f={app.focus === 'shot' + i} class:fav={app.favs.has(shots[i].path)} class:checked={app.sel.has(shots[i].path)} data-k="shot{i}" data-act="shot:{i}" data-i={i}
        style="left:{L.x[i]}px;top:{L.y[i]}px;width:{L.w}px;height:{L.h}px">
        <div class="face"><Thumb path={shots[i].path} /><span class="star">{@html ICON.starY}</span>{#if members && shots[i].game}<span class="gbadge">{shots[i].game}</span>{/if}{#if app.selMode}<span class="chk">{@html app.sel.has(shots[i].path) ? ICON.check : ''}</span>{/if}</div>
      </div>
    {/each}
  </div>
</div>
{#if days}
  <div class="tline" style="height:{TRACK}px">
    <div class="ln"></div>
    {#each months as m (m.l)}<span class="mo" style="top:{m.y}px">{m.l}</span><span class="tk" style="top:{m.y}px"></span>{/each}
    <span class="kn" style="top:{knob}px"></span>
    {#if cur}<span class="kl" style="top:{knob}px">{fmtShort(cur.date)}</span>{/if}
  </div>
{:else}
  <div class="sbar"><i style="height:{440 * ratio}px;top:{L.total > GRID_VIEW_H ? (app.goff / (L.total - GRID_VIEW_H)) * (440 - 440 * ratio) : 0}px"></i></div>
{/if}
<Hints list={[
  ['A', app.selMode ? 'Marcar' : 'Ver', 'a'],
  ['B', app.selMode ? 'Salir de la selección' : 'Volver', 'b'],
  ...(members ? [['LB/RB', 'Juego', 'rb'] as [string, string, string]] : []),
  ...(daysOn() ? [['LT/RT', 'Día', 'rt'] as [string, string, string]] : []),
  ['X', 'Favorita', 'x'],
  ['Y', 'Ordenar', 'y'],
  ['View', wantDays ? 'Continua' : 'Por días', 'view'],
  ['Start', members ? 'Saga' : 'Menú', 'menu'],
]} />
