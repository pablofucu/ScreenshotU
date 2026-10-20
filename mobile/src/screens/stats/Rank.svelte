<script lang="ts">
  // Clasificación de juegos por tiempo en una semana, mes, año o «Siempre», comparada con el periodo anterior.
  import { ICON } from '@shared/icons';
  import { PERIOD_KINDS, addDays, fmtHM, prevName, rankPeriod } from '@shared/stats';
  import HeroStrip from '../../components/HeroStrip.svelte';
  import MiniCover from '../../components/MiniCover.svelte';
  import { MI } from '../../lib/icons';
  import { app } from '../../lib/store.svelte';

  const R = $derived(rankPeriod(app.lib, app.statsKind, app.statsOff, app.now));
  const rows = $derived(R.rows);
  const maxMs = $derived(Math.max(1, ...rows.map((r) => r.ms)));
  const all = $derived(app.statsKind === 'all');
  const prevN = $derived(prevName(app.statsKind, app.statsOff, app.now));
  const avg = $derived(R.n ? R.ms / R.n : 0);
  const elapsed = $derived.by(() => {
    if (all) return 0;
    const to = Math.min(R.range.to, addDays(new Date(app.now).setHours(0, 0, 0, 0), 1));
    return Math.max(0, Math.round((to - R.range.from) / 864e5));
  });
  const delta = (ms: number, prev: number) =>
    all ? null : prev === 0 ? (ms > 0 ? { c: 'up', t: 'nuevo' } : null) : ms === prev ? { c: 'eq', t: '= igual' } : { c: ms > prev ? 'up' : 'down', t: `${ms > prev ? '▲' : '▼'} ${fmtHM(Math.abs(ms - prev))}` };
  const ndelta = (n: number, prev: number) =>
    all || (prev === 0 && n === 0) ? null : n === prev ? { c: 'eq', t: '= igual' } : { c: n > prev ? 'up' : 'down', t: `${n > prev ? '▲' : '▼'} ${Math.abs(n - prev)}` };
  const pname = (pi: number) => app.lib.platforms[pi]?.name ?? '';
  const setKind = (k: typeof app.statsKind) => { app.statsKind = k; app.statsOff = 0; };
  const dT = $derived(delta(R.ms, R.prevMs));
  const dN = $derived(ndelta(R.n, R.prevN));
</script>

<div class="pad"><div class="seg pill">
  {#each PERIOD_KINDS as [k, l] (k)}<button class:on={app.statsKind === k} onclick={() => setKind(k)}>{l}</button>{/each}
</div></div>
{#if !all}
  <div class="per">
    <button class="round pill" disabled={!R.older} onclick={() => app.statsOff--} aria-label="Anterior"><i class="dk">{@html ICON.chevL}</i></button>
    <b>{R.range.label}</b>
    <button class="round pill" disabled={app.statsOff >= 0} onclick={() => app.statsOff++} aria-label="Siguiente"><i class="dk">{@html ICON.chevR}</i></button>
  </div>
{/if}

<div class="kpis">
  <div class="glass panel k"><span class="face"><small>Tiempo jugado</small><b>{fmtHM(R.ms)}</b>{#if dT}<em class={dT.c}>{dT.t}</em><span class="vs">vs {prevN}</span>{/if}</span></div>
  <div class="glass panel k"><span class="face"><small>Partidas</small><b>{R.n}</b>{#if dN}<em class={dN.c}>{dN.t}</em><span class="vs">vs {prevN}</span>{/if}</span></div>
  <div class="glass panel k"><span class="face"><small>Media por partida</small><b>{avg ? fmtHM(avg) : '—'}</b></span></div>
  <div class="glass panel k"><span class="face"><small>Días jugados</small><b>{R.days}{#if elapsed}<span class="of"> de {elapsed}</span>{/if}</b></span></div>
</div>

{#if rows.length}
  <div class="sect">Clasificación<small>{rows.length} {rows.length === 1 ? 'juego' : 'juegos'}</small></div>
  <div class="list">
    {#each rows as r, k (r.t.id)}
      {@const a = app.titleAcc(r.pi, r.ti)}
      {#if k < 3}
        <button class="glass tap podium" style="--acc:{a.c}" onclick={() => app.openTitle(r.t.id)}>
          <span class="face">
            <HeroStrip t={r.t} />
            <span class="pinfo">
              <span class="pos p{k + 1}">{k + 1}</span>
              <b>{fmtHM(r.ms)}</b>
              <small>{r.n} {r.n === 1 ? 'partida' : 'partidas'}</small>
              <small class="pn">{pname(r.pi)}</small>
            </span>
          </span>
        </button>
      {:else}
        <button class="glass panel tap row" style="--acc:{a.c}" onclick={() => app.openTitle(r.t.id)}>
          <span class="face">
            <span class="num">{k + 1}</span>
            <MiniCover t={r.t} acc={a.c} size={32} />
            <span class="nm"><b>{r.t.name}</b><span class="bar"><i style="width:{(r.ms / maxMs) * 100}%"></i></span></span>
            <span class="tm">{fmtHM(r.ms)}<small>{r.n} part.</small></span>
          </span>
        </button>
      {/if}
    {/each}
  </div>
{:else}
  <div class="glass panel empty"><span class="face"><i>{@html MI.trophy}</i><b>Sin partidas en este periodo</b><p>Las partidas se registran al jugar desde el visor del PC.</p></span></div>
{/if}

<style>
  .per{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:10px 14px 0}
  .per b{font-size:17px;font-weight:800;color:var(--ink)}
  .per button:disabled{opacity:.35}
  .dk{width:18px;height:18px;display:block;filter:invert(.4)}
  .dk :global(svg){width:18px;height:18px;display:block}
  .kpis{display:grid;grid-template-columns:1fr 1fr;gap:10px;padding:12px 14px 0}
  .k{--r:20px;--bw:6px}
  .k .face{display:flex;flex-direction:column;gap:2px;padding:10px 12px;min-height:92px}
  .k small{font-size:11px;font-weight:800;color:var(--dim);text-transform:uppercase;letter-spacing:.04em}
  .k b{font-size:24px;font-weight:800;color:var(--ink)}
  .k .of{font-size:14px;color:var(--dim)}
  .k em{font-style:normal;font-size:12.5px;font-weight:800}
  .k .vs{font-size:11px;font-weight:700;color:var(--dim2)}
  .up{color:#2f9a3c}.down{color:#d6334a}.eq{color:var(--dim)}
  .list{display:flex;flex-direction:column;gap:8px;padding:0 14px}
  .podium{--r:20px;--bw:6px;height:96px;text-align:left}
  .podium .face{background:linear-gradient(90deg,#f7f9fc 30%,color-mix(in srgb,var(--acc) 30%,#f7f9fc))}
  .pinfo{position:absolute;z-index:2;left:12px;top:8px;bottom:8px;display:flex;flex-direction:column;justify-content:center;gap:1px}
  .pos{width:30px;height:30px;border-radius:50%;display:grid;place-items:center;font-size:16px;font-weight:800;color:#fff;margin-bottom:2px;box-shadow:inset 0 1px 0 rgba(255,255,255,.6),0 2px 4px rgba(0,0,0,.25)}
  .p1{background:linear-gradient(180deg,#ffe07a,#e2a911)}.p2{background:linear-gradient(180deg,#e6ebf2,#9aa5b5)}.p3{background:linear-gradient(180deg,#f2b98a,#c0723a)}
  .pinfo b{font-size:18px;font-weight:800;color:var(--ink)}
  .pinfo small{font-size:11.5px;font-weight:700;color:var(--dim)}
  .pinfo .pn{color:var(--dim2)}
  .row{--r:18px;--bw:5px;text-align:left}
  .row .face{display:flex;align-items:center;gap:10px;padding:8px 12px}
  .num{width:22px;text-align:center;font-size:14px;font-weight:800;color:var(--dim)}
  .nm{flex:1;min-width:0;display:flex;flex-direction:column;gap:5px}
  .nm b{font-size:14px;font-weight:800;color:var(--ink2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .bar{height:6px;border-radius:3px;background:rgba(160,168,182,.25);overflow:hidden}
  .bar i{display:block;height:100%;border-radius:3px;background:var(--acc)}
  .tm{text-align:right;font-size:14px;font-weight:800;color:var(--ink);white-space:nowrap}
  .tm small{display:block;font-size:10.5px;font-weight:700;color:var(--dim)}
  .empty{margin:16px 14px}
  .empty .face{display:flex;flex-direction:column;align-items:center;gap:6px;padding:24px;text-align:center}
  .empty i{width:46px;height:46px;border-radius:14px;padding:9px;background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 40%,white),var(--acc))}
  .empty i :global(svg){width:100%;height:100%}
  .empty b{font-size:17px}
  .empty p{margin:0;color:var(--dim);font-weight:600}
</style>
