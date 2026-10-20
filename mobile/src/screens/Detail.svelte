<script lang="ts">
  // Ficha de un juego: banner con hero y logo, tiempo jugado, semana, partidas, capturas recientes.
  import { fmtAgo, fmtDur, nCapturas } from '@shared/format';
  import { ICON } from '@shared/icons';
  import { evoBuckets, fmtDate, fmtHM, WDAYS } from '@shared/stats';
  import Img from '../components/Img.svelte';
  import MiniCover from '../components/MiniCover.svelte';
  import { logoSrc } from '../components/RecentCard.svelte';
  import { MI } from '../lib/icons';
  import { app } from '../lib/store.svelte';

  let { id }: { id: string } = $props();
  const ref = $derived(app.index.get(id));
  const T = $derived(ref?.t);
  const P = $derived(ref ? app.lib.platforms[ref.pi] : undefined);
  const acc = $derived(ref ? app.titleAcc(ref.pi, ref.ti) : null);
  let logoBad = $state(false);

  const STATUS: Record<string, string> = { pending: 'Pendiente', playing: 'Jugando', finished: 'Terminado', dropped: 'Abandonado' };
  const secs = $derived(T?.playSeconds ?? 0);
  const digits = $derived(String(Math.min(999, Math.floor(secs / 3600))).padStart(3, '0') + String(Math.floor(secs / 60) % 60).padStart(2, '0'));
  const week = $derived(T ? evoBuckets(T, 'day', app.now).slice(-7) : []);
  const wmax = $derived(Math.max(...week.map((b) => b.ms), 1));
  const wtot = $derived(week.reduce((a, b) => a + b.ms, 0));
  const nPlays = $derived(T?.plays?.length || T?.sessions || 0);
  const avg = $derived(T?.plays?.length ? T.plays.reduce((a, p) => a + p[1] - p[0], 0) / T.plays.length : 0);
  const favN = $derived(T ? T.shots.reduce((a, s) => a + (app.favs.has(s.path) ? 1 : 0), 0) : 0);
  const playingNow = $derived(!!T?.gameId && app.playing?.id === T.gameId);
  const live = $derived(playingNow && app.playing ? Math.max(0, Math.floor((app.now - app.playing.since) / 1000)) : 0);
  const recentShots = $derived(T?.shots.slice(0, 9) ?? []);
  const hasPlays = $derived((T?.plays?.length ?? 0) > 0);
</script>

{#if T && ref && acc}
  <div class="wrap" style="--acc:{acc.c};--acc-ink:{acc.dark ? '#3a2f00' : '#fff'}">
    <div class="glass banner">
      <span class="face">
        <Img paths={[T.hero]} kind="preview" eager>
          {#snippet fallback()}<span class="blurbg"><Img paths={[T.cover, T.shots[0]?.path]} eager /></span>{/snippet}
        </Img>
        <span class="shade"></span>
        {#if T.logo && !logoBad}
          <img class="logo" alt={T.name} use:logoSrc={T.logo} onerror={() => (logoBad = true)} />
        {:else}
          <span class="bname">{T.name}</span>
        {/if}
      </span>
    </div>

    <div class="title">
      <h1>{T.name}</h1>
      <div class="tags">
        <span class="tag">{P?.name}</span>
        {#if T.status}<span class="tag soft">{STATUS[T.status]}{T.status === 'finished' && T.finishedMs ? ` · ${fmtDate(T.finishedMs)}` : ''}</span>{/if}
        {#if T.launcher}<span class="tag soft">{T.launcher}</span>{/if}
        {#if playingNow}<span class="tag live">● Jugando ahora · {fmtDur(live)}</span>{/if}
      </div>
    </div>

    {#if T.members}
      <div class="sect">Juegos de la saga<small>{T.members.length}</small></div>
      <div class="members">
        {#each T.members as m (m.id)}
          {@const mr = app.index.get(m.id)}
          {#if mr}
            <button class="glass panel tap mem" style="--acc:{app.titleAcc(mr.pi, mr.ti).c}" onclick={() => app.openTitle(m.id)}>
              <span class="face"><MiniCover t={mr.t} acc={app.titleAcc(mr.pi, mr.ti).c} size={34} /><b>{m.name}</b><small>{nCapturas(mr.t.shots.length)}</small></span>
            </button>
          {/if}
        {/each}
      </div>
    {:else}
      <div class="stats">
        <button class="glass panel tap st big" disabled={!hasPlays} onclick={() => app.go({ v: 'evo', id })}>
          <span class="face">
            <span class="lab"><i>{@html MI.clock}</i>Tiempo jugado</span>
            <span class="counter">
              {#each digits as d, i}{#if i === 3}<em>:</em>{/if}<b>{d}</b>{/each}
            </span>
            <span class="foot">{T.lastPlayed ? `Última partida: ${fmtAgo(T.lastPlayed)}` : 'Sin partidas registradas'}{#if hasPlays}<span class="go">Evolución ›</span>{/if}</span>
          </span>
        </button>
        <button class="glass panel tap st" disabled={!hasPlays} onclick={() => app.go({ v: 'evo', id })}>
          <span class="face">
            <span class="lab">Esta semana</span>
            <span class="val">{wtot > 0 ? fmtHM(wtot) : '—'}</span>
            <span class="bars">
              {#each week as b}<span class="b"><i style="height:{b.ms ? Math.max(10, (b.ms / wmax) * 100) : 6}%" class:z={!b.ms}></i><small>{WDAYS[new Date(b.from).getDay()][0].toUpperCase()}</small></span>{/each}
            </span>
          </span>
        </button>
        <div class="glass panel st">
          <span class="face">
            <span class="lab">Partidas</span>
            <span class="val">{nPlays || '—'}</span>
            <span class="foot">{avg ? `Media ${fmtHM(avg)}` : ''}{T.firstPlayed ? `${avg ? ' · ' : ''}desde ${fmtDate(T.firstPlayed)}` : ''}</span>
          </span>
        </div>
      </div>
    {/if}

    <div class="sect">Capturas<small>{nCapturas(T.shots.length)}{favN ? ` · ${favN} favoritas` : ''}</small>
      {#if T.shots.length}<button class="more" onclick={() => app.go({ v: 'shots', id })}>Ver todas ›</button>{/if}
    </div>
    {#if recentShots.length}
      <div class="shots">
        {#each recentShots as s, i (s.path)}
          <button class="glass tap shot" onclick={() => app.openViewer(T.shots, i, T.name)}>
            <span class="face"><Img paths={[s.path]} />{#if app.favs.has(s.path)}<i class="star">{@html ICON.starY}</i>{/if}</span>
          </button>
        {/each}
      </div>
      {#if T.shots.length > recentShots.length}
        <div class="pad" style="margin-top:12px"><button class="bigbtn" onclick={() => app.go({ v: 'shots', id })}>Ver las {T.shots.length} capturas</button></div>
      {/if}
    {:else}
      <p class="none">Este juego aún no tiene capturas.</p>
    {/if}
  </div>
{:else}
  <p class="none">Este juego ya no está en la biblioteca.</p>
{/if}

<style>
  .banner{margin:0 12px;--r:26px;--bw:8px;aspect-ratio:1920/700}
  .banner .face{background:#2b3140}
  .blurbg{position:absolute;inset:-20px;filter:blur(14px) brightness(.8)}
  .shade{position:absolute;inset:0;z-index:1;background:linear-gradient(180deg,rgba(0,0,0,0) 45%,rgba(0,0,0,.35))}
  .logo{position:absolute;z-index:2;left:10%;right:10%;top:16%;bottom:12%;width:80%;height:72%;object-fit:contain;filter:drop-shadow(0 3px 8px rgba(0,0,0,.55))}
  .bname{position:absolute;z-index:2;left:16px;right:16px;bottom:14px;font-size:24px;font-weight:800;color:#fff;text-shadow:0 2px 8px rgba(0,0,0,.6);line-height:1.1}
  .title{padding:12px 18px 0}
  h1{margin:0;font-size:23px;font-weight:800;color:var(--ink);line-height:1.15}
  .tags{display:flex;flex-wrap:wrap;gap:6px;margin-top:8px}
  .tag.live{background:#3BB54A;color:#fff}
  .stats{display:grid;grid-template-columns:1fr 1fr;gap:10px;padding:14px 14px 0}
  .st{--r:22px;--bw:6px;text-align:left}
  .st:disabled{opacity:1}
  .st.big{grid-column:1/-1}
  .st .face{display:flex;flex-direction:column;gap:4px;padding:12px 14px;min-height:108px}
  .lab{display:flex;align-items:center;gap:6px;font-size:12px;font-weight:800;color:var(--dim);text-transform:uppercase;letter-spacing:.04em}
  .lab i{width:16px;height:16px;border-radius:50%;background:var(--acc);display:grid;place-items:center;padding:2px}
  .lab i :global(svg){width:12px;height:12px}
  .counter{display:flex;align-items:center;gap:4px;margin:4px 0}
  .counter b{width:34px;height:46px;border-radius:9px;display:grid;place-items:center;font-size:30px;font-weight:800;color:#fff;
    background:linear-gradient(180deg,#5b6474,#3a414e 50%,#2e343f 51%,#434b59);box-shadow:inset 0 1px 0 rgba(255,255,255,.35),0 0 0 1px #262b34,0 3px 6px -2px rgba(0,0,0,.4)}
  .counter em{font-style:normal;font-size:26px;font-weight:800;color:var(--ink2);margin:0 1px}
  .foot{font-size:12.5px;font-weight:700;color:var(--dim);display:flex;gap:8px;align-items:center}
  .go{margin-left:auto;color:var(--acc);font-weight:800}
  .val{font-size:26px;font-weight:800;color:var(--ink)}
  .bars{display:flex;gap:4px;align-items:flex-end;height:40px;margin-top:auto}
  .b{flex:1;height:100%;display:flex;flex-direction:column;align-items:center;justify-content:flex-end;gap:2px}
  .b i{display:block;width:100%;border-radius:4px;background:var(--acc)}
  .b i.z{background:#d3d8e0}
  .b small{font-size:9px;font-weight:800;color:var(--dim2)}
  .members{display:flex;flex-direction:column;gap:8px;padding:0 14px}
  .mem{--r:18px;--bw:5px;text-align:left}
  .mem .face{display:flex;align-items:center;gap:12px;padding:8px 12px}
  .mem b{flex:1;font-size:15px;font-weight:800;color:var(--ink2)}
  .mem small{font-size:12px;font-weight:700;color:var(--dim)}
  .shots{display:grid;grid-template-columns:repeat(3,1fr);gap:8px;padding:0 12px}
  .shot{--r:14px;--bw:4px;aspect-ratio:16/10}
  .shot .face{border-radius:11px}
  .star{position:absolute;z-index:5;right:4px;top:4px;width:18px;height:18px}
  .star :global(svg){width:18px;height:18px;display:block}
  .none{text-align:center;color:var(--dim);font-weight:700;padding:20px}
</style>
