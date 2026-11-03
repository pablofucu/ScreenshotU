<script lang="ts">
  // Baldosa y hoja de tiempo de juego y racha de novelas visuales o récord de sesión.
  import { onDestroy } from 'svelte';
  import type { QuickMenuState } from './data';
  import { goalTrack, readDetail, GOAL_COL, RESCUE_COL, localDay, type GoalKey } from '../lib/readgoals';
  import type { Platform, Title } from '../lib/types';
  import { rescueDay, rescueLeft, rescueClose, rescueFailed, fmtMin } from '../lib/readfmt';
  import { titleStats } from '../lib/stats';
  import { fmtDur, z } from '../lib/format';

  let {
    context = null,
    game = null,
    goals = { vn: 60, jvn: 45 },
    view = 'tile',
    onOpenSheet,
  }: {
    context: QuickMenuState['context'];
    game: QuickMenuState['game'];
    goals: QuickMenuState['goals'];
    view?: 'tile' | 'sheet';
    onOpenSheet?: () => void;
  } = $props();

  let now = $state(Date.now());
  const timer = setInterval(() => { now = Date.now(); }, 1000);
  onDestroy(() => clearInterval(timer));

  const gameId = $derived(game?.id ?? context?.gameId ?? 'game');
  const since = $derived(game?.since ?? now);
  const sessionSec = $derived(Math.max(0, Math.floor((now - since) / 1000)));

  const sesHours = $derived(Math.floor(sessionSec / 3600));
  const sesMins = $derived(Math.floor((sessionSec % 3600) / 60));
  const sesStr = $derived(sesHours > 0 ? `${sesHours}:${z(sesMins)}` : `${sesMins}:${z(sessionSec % 60)}`);

  const sinceTime = $derived(
    new Date(since).toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' })
  );

  const totalSec = $derived((context?.title.playSeconds ?? game?.playSeconds ?? 0) + sessionSec);
  const totalStr = $derived(fmtDur(totalSec));

  const isReading = $derived(!!context?.key && (context.key === 'vn' || context.key === 'jvn'));
  const goalKey = $derived((context?.key ?? 'vn') as GoalKey);
  const goalMinutes = $derived(goals[goalKey] ?? 60);

  const track = $derived.by(() => {
    if (!isReading || !context) return null;
    const live = { id: gameId, since };
    return goalTrack(goalKey, goalMinutes, [{ id: goalKey, titles: context.titles as unknown as Title[] }] as Platform[], live, now);
  });

  const detail = $derived.by(() => {
    if (!isReading || !context) return null;
    const live = { id: gameId, since };
    return readDetail(goalKey, goalMinutes, [{ id: goalKey, titles: context.titles as unknown as Title[] }] as Platform[], live, now);
  });

  const todayStr = $derived.by(() => {
    if (track) return fmtMin(track.today, true);
    return fmtDur(sessionSec);
  });

  const timeNote = $derived.by(() => {
    if (!track) return '';
    if (track.paidDay === localDay(now)) {
      return `¡Hucha llena! El ${track.rescue ? rescueFailed(track.rescue) : 'día fallado'} cuenta · ${track.streak} días`;
    }
    if (track.done) {
      const atStr = detail?.doneAt
        ? new Date(detail.doneAt).toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' })
        : '';
      return `✓ Cumplida${atStr ? ` a las ${atStr}` : ''}${track.rescue ? ` · +${track.rescue.gotToday || 0} min a la hucha` : ''}`;
    }
    const leftM = Math.ceil(track.goal - track.today);
    return track.streak > 0 ? `Faltan ${leftM} min para seguir la racha` : `Faltan ${leftM} min`;
  });

  const tStats = $derived.by(() => {
    if (!context?.title) return null;
    return titleStats(context.title as unknown as Title);
  });

  const longestMs = $derived(tStats?.longest?.ms ?? 0);
  const longestStr = $derived(fmtDur(Math.round(longestMs / 1000)));
  const beatsRecord = $derived(sessionSec * 1000 > longestMs);
  const leftToBeat = $derived(beatsRecord ? '' : fmtDur(Math.round((longestMs - sessionSec * 1000) / 1000)));

  // Hucha mini casillas
  const miniCells = $derived.by(() => {
    if (!track?.rescue) return [];
    const r = track.rescue;
    const need = Math.max(1, r.need);
    const unit = need <= 12 ? 1 : need <= 60 ? 5 : 10 * Math.ceil(need / 120);
    return Array.from({ length: Math.ceil(need / unit) }, (_, i) => {
      const lo = i * unit, w = Math.min(need, lo + unit) - lo;
      const pct = (v: number) => Math.round(Math.max(0, Math.min(1, (v - lo) / w)) * 100);
      return { a: pct(r.gotBefore), b: pct(r.got) };
    });
  });

  const weekBarsList = $derived.by(() => {
    const raw = context?.title.week ?? [0, 0, 0, 0, 0, 0, 0];
    const mx = Math.max(...raw, 0.1);
    const dayLetters = ['L', 'M', 'X', 'J', 'V', 'S', 'D'];
    return raw.map((v, i) => ({
      val: v,
      pct: Math.min(100, Math.max(4, Math.round((v / mx) * 100))),
      letter: dayLetters[i] ?? '',
      isToday: i === raw.length - 1,
    }));
  });

  function ringDash(frac: number, size: number, sw: number) {
    const r = size / 2 - sw / 2 - 1;
    const c = 2 * Math.PI * r;
    return {
      r,
      c,
      offset: c * (1 - Math.max(0, Math.min(1, frac))),
      m: size / 2,
    };
  }
</script>

{#if view === 'tile'}
  <div
    class="glass tile t-time"
    class:r-vn={isReading && goalKey === 'vn'}
    class:r-jv={isReading && goalKey === 'jvn'}
    style="--i:1;{isReading ? `--c:${GOAL_COL[goalKey].c}` : ''}"
    data-f
    data-id="time"
    onclick={onOpenSheet}
    onkeydown={(e) => { if (e.key === 'Enter') onOpenSheet?.(); }}
    role="button"
    tabindex="0"
  >
    <div class="lbl">
      <svg class="i" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg>
      {isReading ? 'Tiempo y racha' : 'Tiempo de juego'}
      <em class="rnote" class:ok={track?.done || beatsRecord}>
        {#if isReading}
          {timeNote}
        {:else}
          {game?.platformName || 'Juegos'} · desde las {sinceTime}
        {/if}
      </em>
    </div>

    <div class="tt">
      <div class="tses">
        <span class="sub">Esta sesión</span>
        <b class="big">{sesStr}</b>
        <small class="sub">Hoy&nbsp;<b>{todayStr}</b></small>
        <small class="sub">Total&nbsp;<b>{totalStr}</b></small>
      </div>

      {#if isReading && track}
        {@const rd = ringDash(track.today / track.goal, 104, 11)}
        <div class="gring" style="width:104px;height:104px">
          <svg width="104" height="104" viewBox="0 0 104 104">
            <circle cx={rd.m} cy={rd.m} r={rd.r} fill="none" stroke={GOAL_COL[goalKey].c} stroke-opacity=".25" stroke-width="11"/>
            <circle
              cx={rd.m} cy={rd.m} r={rd.r}
              fill="none"
              stroke={GOAL_COL[goalKey].c}
              stroke-width="11"
              stroke-linecap="round"
              stroke-dasharray={rd.c}
              stroke-dashoffset={rd.offset}
              transform="rotate(-90 {rd.m} {rd.m})"
              style="filter:drop-shadow(0 0 5px {GOAL_COL[goalKey].c})"
            />
          </svg>
          <div class="gin">
            <b>{Math.round(track.today)}m</b>
            <small>de {goalMinutes / 60} h</small>
          </div>
        </div>

        <div class="rside">
          <div class="rmed">
            <svg viewBox="0 0 24 24">
              <path
                fill={GOAL_COL[goalKey].c}
                fill-opacity=".25"
                stroke={GOAL_COL[goalKey].c}
                stroke-width="2.2"
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235"
              />
            </svg>
            <span class="n">{track.streak}</span>
            <span class="l">
              {GOAL_COL[goalKey].name}
              <small class={track.rescue ? 'rp' : ''}>
                {track.rescue ? 'pendiente' : track.streak >= track.best && track.streak > 0 ? 'récord' : `mejor ${track.best}`}
              </small>
            </span>
          </div>

          {#if track.rescue}
            <div class="hm">
              <div class="mh">
                <span class="mk">Rescate</span>
                <span class="mn"><b>{track.rescue.got}</b>/{track.rescue.need}<small> min</small></span>
              </div>
              <div class="mbar">
                {#each miniCells as c}
                  <i style="--a:{c.a}%;--b:{c.b}%"><s class="fb"></s><s class="fa"></s></i>
                {/each}
              </div>
              <div class="mf">
                <span>quedan <b>{rescueLeft(track.rescue, now)}</b></span>
                <span>{rescueClose(track.rescue, false)}</span>
              </div>
            </div>
          {:else}
            <div class="rrow">
              <div class="rmap">
                {#each track.weeks as col}
                  <div>
                    {#each col as cell}
                      <span
                        class="cell"
                        class:st={cell.streak}
                        class:td={cell.today}
                        style="background:{cell.future ? 'rgba(255,255,255,.05)' : cell.v <= 0 ? 'rgba(255,255,255,.1)' : `color-mix(in srgb, ${GOAL_COL[goalKey].c} ${(cell.r < 1 ? 15 + 45 * cell.r : Math.min(100, 60 + 40 * (cell.r - 1))).toFixed(0)}%, transparent)`}"
                      ></span>
                    {/each}
                  </div>
                {/each}
              </div>
              <span class="rnote ok">
                {track.streak} {track.streak === 1 ? 'día seguido' : 'días seguidos'}<br>
                <span style="opacity:.8;color:#fff">sin rescates pendientes</span>
              </span>
            </div>
          {/if}
        </div>
      {:else}
        <div class="wk">
          {#each weekBarsList as b, i}
            <div class={b.isToday ? 'td' : ''}>
              <i style="height:{b.pct}%;background:linear-gradient(180deg, var(--r{((i % 6) + 1)}), var(--r{(((i + 1) % 6) + 1)}))"></i>
              <small>{b.letter}</small>
            </div>
          {/each}
        </div>
        <div class="recc">
          <span class="kk">Sesión récord</span>
          <b>{longestStr}</b>
          {#if beatsRecord}
            ¡Nueva sesión récord!
          {:else}
            Te falta {leftToBeat} para batirla
          {/if}
        </div>
      {/if}
    </div>
  </div>
{:else}
  <div class="vtime">
    <div class="tbig">
      <div>
        <span class="sub">Esta sesión</span>
        <b class="big">{sesStr}</b>
        <small class="sub">desde las {sinceTime}</small>
      </div>
      <div>
        <span class="sub">Hoy</span>
        <b class="big">{todayStr}</b>
        <small class="sub">{detail?.month.elapsed ? `${detail.month.elapsed} días mes` : '1 sesión'}</small>
      </div>
      <div>
        <span class="sub">Total</span>
        <b class="big">{totalStr}</b>
        <small class="sub">{context?.title.sessions || 1} sesiones</small>
      </div>
    </div>

    {#if isReading && track}
      {#if track.rescue}
        {@const r = track.rescue}
        <div class="resc r-vn" style="--c:{GOAL_COL[goalKey].c};--rc:{RESCUE_COL[goalKey].rc};--rmid:{RESCUE_COL[goalKey].rmid}">
          <div class="rtitle">
            <span class="rdot"></span>
            Rescate del {rescueFailed(r)}
            <span style="margin-left:auto;display:flex;gap:10px;align-items:center">
              <div class="rmed" style="height:38px">
                <svg viewBox="0 0 24 24"><path fill={GOAL_COL[goalKey].c} fill-opacity=".25" stroke={GOAL_COL[goalKey].c} stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235"/></svg>
                <span class="n" style="font-size:22px">{track.streak}</span>
                <span class="l">{GOAL_COL[goalKey].name}<small class="rp">pendiente</small></span>
              </div>
            </span>
          </div>

          <div class="jar">
            {#each Array(r.need) as _, i}
              <i class:d1={i < r.gotBefore} class:d2={i >= r.gotBefore && i < r.got}></i>
            {/each}
          </div>

          <div class="jarl">
            <span>{rescueDay(r.day, true)}: {r.played} de {r.goal} min · 1 casilla = 1 min que faltó</span>
            <b>Recuperado {r.got} de {r.need} min</b>
          </div>

          <div class="hrow">
            <div class="plist">
              <div class="kk">Partidas de la ventana</div>
              {#each r.plays as p}
                <div class="pr" class:no={!p.counts}>
                  <span>{rescueDay(p.day)}</span>
                  <span>
                    {#if !p.counts}<s>{p.name}</s><em>no cuenta · menos de 10 min</em>{:else}{p.name}{/if}
                  </span>
                  <span class="pm">{Math.round(p.min)} min</span>
                </div>
              {/each}
            </div>

            <div>
              <div class="cd">
                <span class="kk">Cierra en</span>
                <b>{rescueLeft(r, now)}</b>
                <small>{rescueClose(r, true)}</small>
                <small>Hoy: {r.next} min más en partidas de 10 min o más</small>
              </div>
              <div class="rcnt">
                <span style="opacity:.7;text-transform:uppercase;letter-spacing:.05em">Rescates · 30 días</span>
                <b>{r.used} <small style="opacity:.7">de 2</small></b>
              </div>
            </div>
          </div>
        </div>
      {:else}
        {@const rd = ringDash(track.today / track.goal, 140, 14)}
        <div style="display:flex;gap:24px;align-items:center;--c:{GOAL_COL[goalKey].c};margin-top:10px">
          <div class="gring" style="width:140px;height:140px">
            <svg width="140" height="140" viewBox="0 0 140 140">
              <circle cx={rd.m} cy={rd.m} r={rd.r} fill="none" stroke={GOAL_COL[goalKey].c} stroke-opacity=".25" stroke-width="14"/>
              <circle
                cx={rd.m} cy={rd.m} r={rd.r}
                fill="none"
                stroke={GOAL_COL[goalKey].c}
                stroke-width="14"
                stroke-linecap="round"
                stroke-dasharray={rd.c}
                stroke-dashoffset={rd.offset}
                transform="rotate(-90 {rd.m} {rd.m})"
                style="filter:drop-shadow(0 0 5px {GOAL_COL[goalKey].c})"
              />
            </svg>
            <div class="gin">
              <b style="font-size:30px">{Math.round(track.today)}m</b>
              <small>de {goalMinutes / 60} h</small>
            </div>
          </div>

          <div style="display:flex;flex-direction:column;gap:10px;flex:1">
            <div class="rmed" style="height:60px">
              <svg viewBox="0 0 24 24"><path fill={GOAL_COL[goalKey].c} fill-opacity=".25" stroke={GOAL_COL[goalKey].c} stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235"/></svg>
              <span class="n" style="font-size:34px">{track.streak}</span>
              <span class="l" style="font-size:15px">
                días de racha de {GOAL_COL[goalKey].full.toLowerCase()}
                <small>{track.streak >= track.best && track.streak > 0 ? 'récord' : `mejor ${track.best}`}</small>
              </span>
            </div>
            <div class="rnote ok" style="font-size:15px">{timeNote}</div>
          </div>

          <div class="rmap" style="transform:scale(1.4);transform-origin:right center;margin-left:30px">
            {#each track.weeks as col}
              <div>
                {#each col as cell}
                  <span
                    class="cell"
                    class:st={cell.streak}
                    class:td={cell.today}
                    style="background:{cell.future ? 'rgba(255,255,255,.05)' : cell.v <= 0 ? 'rgba(255,255,255,.1)' : `color-mix(in srgb, ${GOAL_COL[goalKey].c} ${(cell.r < 1 ? 15 + 45 * cell.r : Math.min(100, 60 + 40 * (cell.r - 1))).toFixed(0)}%, transparent)`}"
                  ></span>
                {/each}
              </div>
            {/each}
          </div>
        </div>
        <div class="sub" style="margin-top:14px">
          Lo mismo que «Lectura de hoy» de Inicio: anillo de hoy, racha y mapa de las últimas semanas. Solo de esta categoría.
        </div>
      {/if}
    {:else}
      <div class="week">
        {#each weekBarsList as b, i}
          <div class={b.isToday ? 'td' : ''}>
            <i style="height:{b.pct}%;background:linear-gradient(180deg, var(--r{((i % 6) + 1)}), var(--r{(((i + 1) % 6) + 1)}))"></i>
            <small>{b.letter}</small>
          </div>
        {/each}
      </div>
      <div class="sub">
        Horas por día esta semana · Sesión récord: <b>{longestStr}</b> ({beatsRecord ? '¡Superada en esta partida!' : `faltan ${leftToBeat}`})
      </div>
    {/if}
  </div>
{/if}
