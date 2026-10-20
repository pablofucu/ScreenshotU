<script lang="ts">
  // Inicio: metas diarias (lo primero), «Seguir jugando», los últimos 7 días y las categorías.
  import { fmtAgo, fmtDur, nCapturas, nTitulos } from '@shared/format';
  import { ICON } from '@shared/icons';
  import { addDays, dayStart, fmtHM } from '@shared/stats';
  import GoalRings from '../components/GoalRings.svelte';
  import RecentCard from '../components/RecentCard.svelte';
  import { app } from '../lib/store.svelte';

  const hasGoals = $derived(app.lib.platforms.some((p) => p.id === 'vn' || p.id === 'jvn'));

  // «Seguir jugando»: jugados o capturados más recientemente, de todas las categorías.
  const recent = $derived.by(() => {
    const out: Array<{ pi: number; ti: number; k: number }> = [];
    app.lib.platforms.forEach((p, pi) => {
      if (p.id === 'sagas') return;
      p.titles.forEach((t, ti) => {
        const k = Math.max(t.lastPlayed ?? 0, t.shots[0]?.date ?? 0);
        if (k > 0) out.push({ pi, ti, k });
      });
    });
    return out.sort((a, b) => b.k - a.k).slice(0, 6).map((r) => ({ ...r, t: app.lib.platforms[r.pi].titles[r.ti] }));
  });

  // Últimos 7 días: minutos por día (todas las partidas) y capturas.
  const week = $derived.by(() => {
    const today = dayStart(app.now);
    const days = Array.from({ length: 7 }, (_, i) => ({ from: addDays(today, i - 6), ms: 0, shots: 0 }));
    const a = days[0].from, b = addDays(today, 1);
    // Fechas de capturas: hora local codificada como UTC.
    const off = (ms: number) => ms - new Date(ms).getTimezoneOffset() * 60000;
    for (const p of app.lib.platforms) {
      if (p.id === 'sagas') continue;
      for (const t of p.titles) {
        for (const [s, e] of t.plays ?? []) {
          if (e <= a || s >= b) continue;
          for (const d of days) d.ms += Math.max(0, Math.min(e, addDays(d.from, 1)) - Math.max(s, d.from));
        }
        for (const sh of t.shots) {
          if (sh.date < off(a)) break;
          const d = days.find((x) => sh.date >= off(x.from) && sh.date < off(addDays(x.from, 1)));
          if (d) d.shots++;
        }
      }
    }
    const max = Math.max(...days.map((d) => d.ms), 1);
    return { days, max, ms: days.reduce((s, d) => s + d.ms, 0), shots: days.reduce((s, d) => s + d.shots, 0) };
  });
  const WD = ['D', 'L', 'M', 'X', 'J', 'V', 'S'];

  const count = (pi: number) => app.lib.platforms[pi].titles.reduce((a, t) => a + t.shots.length, 0);
  function meta(t: (typeof recent)[number]['t']) {
    const when = t.lastPlayed ? fmtAgo(t.lastPlayed) : '';
    return [when, t.playSeconds > 60 ? fmtDur(t.playSeconds) : nCapturas(t.shots.length)].filter(Boolean).join(' · ');
  }
  function openPlat(pi: number) {
    app.gamesPlat = pi;
    app.gamesFilter = 'all';
    app.setTab('games');
  }
  const live = $derived(app.playing ? Math.max(0, Math.floor((app.now - app.playing.since) / 1000)) : 0);
</script>

{#if app.playing}
  <button class="glass blur tap nowplay" style="--acc:{app.accById(app.playing.id).c}" onclick={() => app.index.has(app.playing!.id) ? app.openTitle(app.playing!.id) : null}>
    <span class="face"><i>{@html ICON.play}</i><b>Jugando ahora en el PC</b><span>{app.playing.name} · {fmtDur(live)}</span></span>
  </button>
{/if}

{#if hasGoals}
  <div class="sect">Metas diarias<small>lectura</small></div>
  <GoalRings />
{/if}

{#if recent.length}
  <div class="sect">Seguir jugando</div>
  <div class="hscroll">
    {#each recent as r (r.t.id)}
      <button class="rcard" onclick={() => app.openTitle(r.t.id)}>
        <span class="glass tap card" style="--acc:{app.titleAcc(r.pi, r.ti).c}"><RecentCard t={r.t} playing={!!r.t.gameId && app.playing?.id === r.t.gameId} /></span>
        <span class="rmeta">{meta(r.t)}</span>
      </button>
    {/each}
  </div>
{/if}

<div class="sect">Últimos 7 días<small>{week.ms > 0 ? fmtHM(week.ms) : 'sin partidas'} · {nCapturas(week.shots)}</small><button class="more" onclick={() => app.setTab('stats')}>Estadísticas</button></div>
<button class="glass panel tap wk" onclick={() => app.setTab('stats')}>
  <span class="face">
    {#each week.days as d, i}
      <span class="col" class:today={i === 6}>
        <span class="bar"><i style="height:{d.ms ? Math.max(8, (d.ms / week.max) * 100) : 4}%"></i></span>
        <small>{WD[new Date(d.from).getDay()]}</small>
        <em>{d.ms ? fmtHM(d.ms).replace(' min', 'm') : ''}</em>
      </span>
    {/each}
  </span>
</button>

<div class="sect">Biblioteca<small>{nCapturas(app.totalShots)}</small></div>
<div class="plats">
  {#each app.lib.platforms as p, pi (p.id)}
    {@const a = app.platAcc(pi)}
    <button class="plat" onclick={() => openPlat(pi)} style="--pc:{a.c};--acc:{a.c}">
      <span class="glass tap icon"><span class="face">{@html ICON[p.icon] ?? ICON.pc}</span></span>
      <span class="pl">{p.name}<small>{p.id === 'sagas' ? `${p.titles.length} ${p.titles.length === 1 ? 'saga' : 'sagas'}` : `${nTitulos(p.titles.length)} · ${nCapturas(count(pi))}`}</small></span>
    </button>
  {/each}
</div>

<style>
  .nowplay{margin:4px 14px 0;--r:22px;--bw:6px;width:calc(100% - 28px);text-align:left}
  .nowplay .face{display:grid;grid-template-columns:34px 1fr;grid-template-rows:auto auto;column-gap:10px;align-items:center;padding:10px 14px;
    background:linear-gradient(180deg,color-mix(in srgb,#3BB54A 18%,white),color-mix(in srgb,#3BB54A 34%,white))}
  .nowplay i{grid-row:span 2;width:34px;height:34px;border-radius:50%;display:grid;place-items:center;background:#3BB54A;box-shadow:0 0 0 1px #24802f,0 0 14px #3BB54A;animation:pulse 1.6s infinite}
  .nowplay i :global(svg){width:18px;height:18px}
  .nowplay b{font-size:12px;font-weight:800;color:#24802f;text-transform:uppercase;letter-spacing:.04em}
  .nowplay span span,.nowplay .face>span{font-size:15px;font-weight:800;color:var(--ink)}
  .hscroll{display:flex;gap:12px;overflow-x:auto;padding:4px 14px 8px;scroll-snap-type:x mandatory;scrollbar-width:none}
  .hscroll::-webkit-scrollbar{display:none}
  .rcard{flex:none;width:min(78vw,330px);scroll-snap-align:start;text-align:left}
  .card{--r:22px;--bw:7px;height:calc(min(78vw,330px) * .43)}
  .rmeta{display:block;margin:6px 6px 0;font-size:13px;font-weight:700;color:var(--dim);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .wk{margin:0 14px;width:calc(100% - 28px)}
  .wk .face{display:flex;gap:6px;padding:12px 12px 8px;height:130px}
  .col{flex:1;display:flex;flex-direction:column;align-items:center;gap:3px;min-width:0}
  .bar{flex:1;width:100%;max-width:30px;display:flex;align-items:flex-end;border-radius:8px;background:rgba(160,168,182,.18)}
  .bar i{display:block;width:100%;border-radius:8px;background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 55%,white),var(--acc));box-shadow:inset 0 1px 0 rgba(255,255,255,.6);transition:height .6s cubic-bezier(.3,.8,.3,1)}
  .col small{font-size:11px;font-weight:800;color:var(--dim)}
  .col.today small{color:var(--acc)}
  .col em{font-style:normal;font-size:9.5px;font-weight:700;color:var(--dim2);height:12px;white-space:nowrap}
  .plats{display:grid;grid-template-columns:repeat(2,1fr);gap:14px 10px;padding:4px 14px 0}
  .plat{display:flex;flex-direction:column;align-items:center;gap:6px}
  .icon{--r:30px;--bw:9px;width:100%;aspect-ratio:1.25}
  .icon .face{display:grid;place-items:center;background:radial-gradient(120% 100% at 30% 10%,color-mix(in srgb,var(--pc) 22%,white) 0%,var(--pc) 70%,color-mix(in srgb,var(--pc) 80%,black) 100%)}
  .icon .face :global(svg){width:52%;height:62%;filter:drop-shadow(0 2px 0 rgba(0,0,0,.10)) drop-shadow(0 6px 10px rgba(0,0,0,.22))}
  .pl{font-size:16px;font-weight:800;color:var(--ink2);text-align:center;line-height:1.2}
  .pl small{display:block;font-size:12px;font-weight:600;color:var(--dim)}
</style>
