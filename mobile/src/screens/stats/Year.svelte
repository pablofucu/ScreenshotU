<script lang="ts">
  // Resumen anual: selector de año con app.yearSel, cifras clave (tiempo jugado y capturas por año),
  // hitos anuales, distribución mensual, clasificación de juegos, capturas y comparativa.
  import { ICON } from '@shared/icons';
  import {
    MONTH_ABBR,
    MONTH_NAMES,
    WDAYS,
    fmtDate,
    fmtHM,
    playYears,
    yearReview,
  } from '@shared/stats';
  import HeroStrip from '../../components/HeroStrip.svelte';
  import Img from '../../components/Img.svelte';
  import MiniCover from '../../components/MiniCover.svelte';
  import { MI } from '../../lib/icons';
  import { app } from '../../lib/store.svelte';

  const cap = (s: string) => (s ? s[0].toUpperCase() + s.slice(1) : '');

  // Años con partidas o capturas.
  const years = $derived.by(() => {
    const ys = new Set<number>(playYears(app.lib));
    for (const p of app.lib.platforms) {
      for (const t of p.titles) {
        for (const s of t.shots) {
          ys.add(new Date(s.date).getUTCFullYear());
        }
      }
    }
    const arr = [...ys].sort((a, b) => a - b);
    return arr.length ? arr : [new Date(app.now).getFullYear()];
  });

  const curYear = $derived(
    app.yearSel !== null && years.includes(app.yearSel)
      ? app.yearSel
      : (years.at(-1) ?? new Date(app.now).getFullYear())
  );

  const idx = $derived(years.indexOf(curYear));

  const Y = $derived(yearReview(app.lib, curYear, app.favs, app.now));
  const maxGameMs = $derived(Math.max(1, ...Y.games.map((g) => g.ms)));
  const monthMax = $derived(Math.max(1, ...Y.months));
  const currentYear = $derived(new Date(app.now).getFullYear());
  const currentMonth = $derived(new Date(app.now).getMonth());

  const delta = (ms: number, prev: number) =>
    prev === 0
      ? (ms > 0 ? { c: 'up', t: 'nuevo' } : null)
      : ms === prev
        ? { c: 'eq', t: '= igual' }
        : { c: ms > prev ? 'up' : 'down', t: `${ms > prev ? '▲' : '▼'} ${fmtHM(Math.abs(ms - prev))}` };

  const ndelta = (n: number, prev: number) =>
    prev === 0 && n === 0
      ? null
      : n === prev
        ? { c: 'eq', t: '= igual' }
        : { c: n > prev ? 'up' : 'down', t: `${n > prev ? '▲' : '▼'} ${Math.abs(n - prev)}` };

  const dT = $derived(Y.prev ? delta(Y.ms, Y.prev.ms) : null);
  const dS = $derived(Y.prev ? ndelta(Y.shots, Y.prev.shots) : null);
  const dN = $derived(Y.prev ? ndelta(Y.n, Y.prev.n) : null);
  const dD = $derived(Y.prev ? ndelta(Y.days, Y.prev.days) : null);

  const fmtRange = (a: number, b: number) => {
    const f = (x: number) => new Date(x).toLocaleDateString('es', { day: 'numeric', month: 'short' }).replace('.', '');
    return a === b ? f(a) : `del ${f(a)} al ${f(b)}`;
  };

  const pname = (pi: number) => app.lib.platforms[pi]?.name ?? '';

  const cmpRows = $derived.by(() => {
    const p = Y.prev;
    if (!p) return [];
    return [
      { k: 'Tiempo jugado', a: Y.ms, b: p.ms, f: fmtHM },
      { k: 'Capturas', a: Y.shots, b: p.shots, f: String },
      { k: 'Partidas', a: Y.n, b: p.n, f: String },
      { k: 'Días jugados', a: Y.days, b: p.days, f: String },
    ].map((r) => {
      const d = r.a - r.b;
      const m = Math.max(1, r.a, r.b);
      const dt = d === 0 ? '= igual' : `${d > 0 ? '▲' : '▼'} ${r.f(Math.abs(d))}`;
      return { ...r, wa: (r.a / m) * 100, wb: (r.b / m) * 100, d, dt };
    });
  });

  const empty = $derived(Y.ms === 0 && Y.n === 0 && Y.shots === 0);
</script>

{#if years.length > 1}
  <div class="pad yseg">
    <div class="seg pill">
      {#each years as y (y)}
        <button class:on={curYear === y} onclick={() => (app.yearSel = y)}>{y}</button>
      {/each}
    </div>
  </div>
{/if}

<div class="per">
  <button
    class="round pill"
    disabled={idx <= 0}
    onclick={() => (app.yearSel = years[idx - 1])}
    aria-label="Año anterior"
  >
    <i class="dk">{@html ICON.chevL}</i>
  </button>
  <b>{curYear}</b>
  <button
    class="round pill"
    disabled={idx >= years.length - 1 || idx === -1}
    onclick={() => (app.yearSel = years[idx + 1])}
    aria-label="Año siguiente"
  >
    <i class="dk">{@html ICON.chevR}</i>
  </button>
</div>

<div class="kpis">
  <div class="glass panel k">
    <span class="face">
      <small>Tiempo jugado</small>
      <b>{fmtHM(Y.ms)}</b>
      {#if dT}<em class={dT.c}>{dT.t}</em><span class="vs">vs {Y.prev?.year}</span>{/if}
    </span>
  </div>
  <div class="glass panel k">
    <span class="face">
      <small>Capturas</small>
      <b>{Y.shots}</b>
      {#if dS}<em class={dS.c}>{dS.t}</em><span class="vs">vs {Y.prev?.year}</span>{/if}
    </span>
  </div>
  <div class="glass panel k">
    <span class="face">
      <small>Partidas</small>
      <b>{Y.n}</b>
      {#if dN}<em class={dN.c}>{dN.t}</em><span class="vs">vs {Y.prev?.year}</span>{/if}
    </span>
  </div>
  <div class="glass panel k">
    <span class="face">
      <small>Días jugados</small>
      <b>{Y.days}{#if Y.elapsed}<span class="of"> de {Y.elapsed}</span>{/if}</b>
      {#if dD}<em class={dD.c}>{dD.t}</em><span class="vs">vs {Y.prev?.year}</span>{/if}
    </span>
  </div>
</div>

{#if empty}
  <div class="glass panel empty">
    <span class="face">
      <i>{@html MI.trophy}</i>
      <b>Sin actividad en {curYear}</b>
      <p>No hay partidas ni capturas registradas en este año.</p>
    </span>
  </div>
{:else}
  {#if Y.ms > 0}
    <div class="facts">
      {#if Y.streak.n}
        <div class="glass panel f">
          <span class="face">
            <i style="color:#E8384F">{@html MI.flame}</i>
            <small>Mejor racha</small>
            <b>{Y.streak.n} {Y.streak.n === 1 ? 'día' : 'días'}</b>
            <em>{fmtRange(Y.streak.from, Y.streak.to)}</em>
          </span>
        </div>
      {/if}
      {#if Y.bestMonth >= 0}
        <div class="glass panel f">
          <span class="face">
            <i style="color:#F2B807">{@html MI.trophy}</i>
            <small>Mes estrella</small>
            <b>{cap(MONTH_NAMES[Y.bestMonth])}</b>
            <em>{fmtHM(Y.months[Y.bestMonth])}</em>
          </span>
        </div>
      {/if}
      {#if Y.favWd}
        <div class="glass panel f">
          <span class="face">
            <small>Día favorito</small>
            <b>{cap(WDAYS[Y.favWd.wd])}</b>
            <em>{fmtHM(Y.favWd.ms)} en {Y.favWd.days} {Y.favWd.days === 1 ? 'día' : 'días'}</em>
          </span>
        </div>
      {/if}
      {#if Y.longest}
        <div class="glass panel f">
          <span class="face">
            <i style="color:#00A0D8">{@html MI.clock}</i>
            <small>Partida más larga</small>
            <b>{fmtHM(Y.longest.ms)}</b>
            <em>{Y.longest.t.name}</em>
          </span>
        </div>
      {/if}
    </div>

    {#if monthMax > 0}
      <div class="sect">Horas por mes</div>
      <div class="glass panel mon">
        <span class="face">
          {#each Y.months as ms, mIdx}
            {@const isCurYear = curYear === currentYear}
            {@const showMonth = ms > 0 || !isCurYear || mIdx <= currentMonth}
            {#if showMonth}
              <div class="mr">
                <span class="ml">{cap(MONTH_ABBR[mIdx])}</span>
                <span class="bar"><i style="width:{(ms / monthMax) * 100}%"></i></span>
                <span class="mv">{ms ? fmtHM(ms) : '—'}</span>
              </div>
            {/if}
          {/each}
        </span>
      </div>
    {/if}

    {#if Y.games.length}
      <div class="sect">Juegos más jugados<small>{Y.games.length} {Y.games.length === 1 ? 'juego' : 'juegos'}</small></div>
      <div class="list">
        {#each Y.games as r, k (r.t.id)}
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
                <span class="nm">
                  <b>{r.t.name}</b>
                  <span class="bar"><i style="width:{(r.ms / maxGameMs) * 100}%"></i></span>
                </span>
                <span class="tm">{fmtHM(r.ms)}<small>{r.n} part.</small></span>
              </span>
            </button>
          {/if}
        {/each}
      </div>
    {/if}
  {/if}

  {#if Y.shots > 0}
    <div class="sect">Capturas<small>{Y.shots} {Y.shots === 1 ? 'captura' : 'capturas'}</small></div>
    {#if Y.shot}
      {@const a = app.titleAcc(Y.shot.pi, Y.shot.ti)}
      <div class="pad">
        <button class="glass tap fshot" style="--acc:{a.c}" onclick={() => app.openTitle(Y.shot!.t.id)}>
          <span class="face">
            <div class="fimg">
              <Img paths={[Y.shot.path]} alt={Y.shot.t.name} />
              <span class="flbl">{Y.shot.fav ? '♥ Tu favorita' : 'Captura destacada'}</span>
            </div>
            <div class="fmeta">
              <MiniCover t={Y.shot.t} acc={a.c} size={32} />
              <span class="fnm">
                <b>{Y.shot.t.name}</b>
                <small>{fmtDate(Y.shot.date)}</small>
              </span>
            </div>
          </span>
        </button>
      </div>
    {/if}

    {#if Y.shotGames.length}
      <div class="glass panel sgames">
        <span class="face">
          {#each Y.shotGames.slice(0, 5) as sg (sg.t.id)}
            {@const a = app.titleAcc(sg.pi, sg.ti)}
            <button class="srow" onclick={() => app.openTitle(sg.t.id)}>
              <MiniCover t={sg.t} acc={a.c} size={28} />
              <span class="snm">{sg.t.name}</span>
              <span class="scnt">{sg.n} {sg.n === 1 ? 'captura' : 'capturas'}</span>
            </button>
          {/each}
        </span>
      </div>
    {/if}
  {/if}

  {#if Y.prev && cmpRows.length}
    <div class="sect">Comparación<small>frente a {Y.prev.year}</small></div>
    <div class="glass panel cmp">
      <span class="face">
        <div class="cmpleg">
          <span><i class="ca"></i>{curYear}</span>
          <span><i class="cb"></i>{Y.prev.year}</span>
        </div>
        {#each cmpRows as r (r.k)}
          <div class="cmprow">
            <div class="cmptop">
              <span class="ck">{r.k}</span>
              <span class="cdt {r.d > 0 ? 'up' : r.d < 0 ? 'down' : 'eq'}">{r.dt}</span>
            </div>
            <div class="cmpbars">
              <div class="cbar ca"><i style="width:{r.wa}%"></i><span>{r.f(r.a)}</span></div>
              <div class="cbar cb"><i style="width:{r.wb}%"></i><span>{r.f(r.b)}</span></div>
            </div>
          </div>
        {/each}
      </span>
    </div>
  {/if}
{/if}

<style>
  .yseg{padding-bottom:6px}
  .per{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:2px 14px 10px}
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
  .facts{display:grid;grid-template-columns:1fr 1fr;gap:10px;padding:12px 14px 0}
  .f{--r:20px;--bw:6px}
  .f .face{display:flex;flex-direction:column;gap:1px;padding:10px 12px;min-height:84px;position:relative}
  .f i{position:absolute;right:10px;top:10px;width:20px;height:20px}
  .f i :global(svg){width:20px;height:20px;display:block}
  .f small{font-size:11px;font-weight:800;color:var(--dim);text-transform:uppercase;letter-spacing:.04em}
  .f b{font-size:21px;font-weight:800}
  .f em{font-style:normal;font-size:11.5px;font-weight:700;color:var(--dim)}
  .mon{margin:0 14px;--r:22px;--bw:6px}
  .mon .face{padding:10px 12px;display:flex;flex-direction:column;gap:7px}
  .mr{display:flex;align-items:center;gap:10px}
  .ml{width:46px;font-size:12px;font-weight:800;color:var(--dim)}
  .bar{flex:1;height:10px;border-radius:5px;background:rgba(160,168,182,.22);overflow:hidden}
  .bar i{display:block;height:100%;border-radius:5px;background:linear-gradient(90deg,color-mix(in srgb,var(--acc) 55%,white),var(--acc))}
  .mv{width:58px;text-align:right;font-size:12px;font-weight:800;color:var(--ink2)}
  .list{display:flex;flex-direction:column;gap:8px;padding:0 14px}
  .podium{--r:20px;--bw:6px;height:96px;text-align:left;width:100%}
  .podium .face{background:linear-gradient(90deg,#f7f9fc 30%,color-mix(in srgb,var(--acc) 30%,#f7f9fc))}
  .pinfo{position:absolute;z-index:2;left:12px;top:8px;bottom:8px;display:flex;flex-direction:column;justify-content:center;gap:1px}
  .pos{width:30px;height:30px;border-radius:50%;display:grid;place-items:center;font-size:16px;font-weight:800;color:#fff;margin-bottom:2px;box-shadow:inset 0 1px 0 rgba(255,255,255,.6),0 2px 4px rgba(0,0,0,.25)}
  .p1{background:linear-gradient(180deg,#ffe07a,#e2a911)}.p2{background:linear-gradient(180deg,#e6ebf2,#9aa5b5)}.p3{background:linear-gradient(180deg,#f2b98a,#c0723a)}
  .pinfo b{font-size:18px;font-weight:800;color:var(--ink)}
  .pinfo small{font-size:11.5px;font-weight:700;color:var(--dim)}
  .pinfo .pn{color:var(--dim2)}
  .row{--r:18px;--bw:5px;text-align:left;width:100%}
  .row .face{display:flex;align-items:center;gap:10px;padding:8px 12px}
  .num{width:22px;text-align:center;font-size:14px;font-weight:800;color:var(--dim)}
  .nm{flex:1;min-width:0;display:flex;flex-direction:column;gap:5px}
  .nm b{font-size:14px;font-weight:800;color:var(--ink2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .row .bar{height:6px;border-radius:3px;background:rgba(160,168,182,.25);overflow:hidden}
  .row .bar i{display:block;height:100%;border-radius:3px;background:var(--acc)}
  .tm{text-align:right;font-size:14px;font-weight:800;color:var(--ink);white-space:nowrap}
  .tm small{display:block;font-size:10.5px;font-weight:700;color:var(--dim)}
  .fshot{width:100%;--r:20px;--bw:6px;text-align:left;display:block}
  .fshot .face{display:flex;flex-direction:column;padding:0;overflow:hidden}
  .fimg{position:relative;width:100%;aspect-ratio:16/9;background:#1a1d24;overflow:hidden;border-top-left-radius:14px;border-top-right-radius:14px}
  .flbl{position:absolute;top:8px;left:8px;padding:3px 9px;border-radius:999px;font-size:11px;font-weight:800;color:#fff;background:rgba(0,0,0,.6);backdrop-filter:blur(6px)}
  .fmeta{display:flex;align-items:center;gap:10px;padding:10px 12px}
  .fnm{flex:1;min-width:0}
  .fnm b{display:block;font-size:14px;font-weight:800;color:var(--ink2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .fnm small{font-size:11.5px;font-weight:700;color:var(--dim)}
  .sgames{margin:10px 14px 0;--r:20px;--bw:6px}
  .sgames .face{display:flex;flex-direction:column;gap:4px;padding:8px 12px}
  .srow{display:flex;align-items:center;gap:10px;width:100%;padding:6px 0;text-align:left;border-top:1px solid rgba(160,168,182,.2)}
  .srow:first-child{border-top:none}
  .snm{flex:1;min-width:0;font-size:13.5px;font-weight:800;color:var(--ink2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .scnt{font-size:12px;font-weight:800;color:var(--dim)}
  .cmp{margin:0 14px;--r:20px;--bw:6px}
  .cmp .face{display:flex;flex-direction:column;gap:12px;padding:12px 14px}
  .cmpleg{display:flex;justify-content:flex-end;gap:14px;font-size:11.5px;font-weight:800;color:var(--dim)}
  .cmpleg span{display:flex;align-items:center;gap:6px}
  .cmpleg i{width:10px;height:10px;border-radius:3px;display:inline-block}
  .ca{background:var(--acc)}.cb{background:#9aa5b5}
  .cmprow{display:flex;flex-direction:column;gap:5px}
  .cmptop{display:flex;justify-content:space-between;align-items:baseline;font-size:13px;font-weight:800}
  .ck{color:var(--ink2)}
  .cdt{font-size:11.5px}
  .cmpbars{display:flex;flex-direction:column;gap:3px}
  .cbar{display:flex;align-items:center;height:14px;border-radius:7px;background:rgba(160,168,182,.2);overflow:hidden;position:relative}
  .cbar i{height:100%;border-radius:7px;display:block}
  .cbar.ca i{background:linear-gradient(90deg,color-mix(in srgb,var(--acc) 50%,white),var(--acc))}
  .cbar.cb i{background:linear-gradient(90deg,#ccd3de,#9aa5b5)}
  .cbar span{position:absolute;right:8px;font-size:10px;font-weight:800;color:var(--ink)}
  .empty{margin:16px 14px}
  .empty .face{display:flex;flex-direction:column;align-items:center;gap:6px;padding:24px;text-align:center}
  .empty i{width:46px;height:46px;border-radius:14px;padding:9px;background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 40%,white),var(--acc))}
  .empty i :global(svg){width:100%;height:100%}
  .empty b{font-size:17px}
  .empty p{margin:0;color:var(--dim);font-weight:600}
</style>
