<script lang="ts">
  // Hucha de minutos de la pantalla de racha: ocupa el hueco del mapa mientras la categoría tiene un rescate pendiente.
  // Una casilla por minuto que faltó el día fallado (macizas las leídas en días anteriores, claras las de hoy,
  // rayadas las que faltan), las partidas de la ventana (las de menos de 10 min, tachadas) y la cuenta atrás.
  // `wide`: un solo rescate (pestañas LB/RB y columna lateral); si no, mitad del hueco (371×352) para dos a la vez.
  import { GOAL_COL, rescueCls, RESCUE_MAX, RESCUE_MIN_PLAY, type GoalKey, type Rescue } from '../lib/readgoals';
  import { rescueClose, rescueDay, rescueFailed, rescueLeft } from '../lib/readrings.svelte';

  let { r, now, wide = true, tabs = [], style = '' }: { r: Rescue; now: number; wide?: boolean; tabs?: GoalKey[]; style?: string } = $props();

  const C = $derived(GOAL_COL[r.key]);
  const ROWS = $derived(wide ? 6 : 5);
  const rows = $derived(r.plays.slice(-ROWS));
  const mins = (m: number) => `${Math.max(1, Math.round(m))} min`;
  const left = $derived(rescueLeft(r, now));
</script>

<div class="stpanel resc smap {rescueCls(r.key)}" class:half={!wide} style="--c:{C.c};--edge:{C.edge};{style}">
  <div class="ptitle">
    <span class="rdot"></span>
    {#if wide}
      <span>Rescate del {rescueFailed(r)}</span>
      {#if tabs.length > 1}
        <span class="stabs pill">
          <span class="btn sh" data-hact="lb">LB</span>
          {#each tabs as k (k)}<span class="t" class:on={k === r.key} style="--c:{GOAL_COL[k].c}" data-hact={k === r.key ? '' : k === tabs[0] ? 'lb' : 'rb'}>{GOAL_COL[k].full}</span>{/each}
          <span class="btn sh" data-hact="rb">RB</span>
        </span>
      {/if}
    {:else}
      <span class="ttl">{C.full}</span><small>· {rescueDay(r.day).toLowerCase()}</small>
      <span class="cdc">{left}</span>
    {/if}
  </div>

  <div class="jar">
    {#each Array(r.need) as _, i}<i class:d1={i < r.gotBefore} class:d2={i >= r.gotBefore && i < r.got}></i>{/each}
  </div>
  <div class="jarl">
    {#if wide}
      <span>{rescueFailed(r).replace(/^./, (c) => c.toUpperCase())} {r.played} de {r.goal} min · 1 casilla = 1 min{r.repeat ? ' (×1,5)' : ' que faltó'}<span class="sw2"></span>antes<span class="sw2 d2"></span>hoy</span>
      <b>Recuperado {r.got} de {r.need} min</b>
    {:else}
      <span>{r.played} de {r.goal} min</span>
      <b>Recuperado {r.got} de {r.need}</b>
    {/if}
  </div>

  {#if wide}
    <div class="hrow">
      {@render list()}
      <div class="hside">
        <div class="cd">
          <span class="k">Cierra en</span><b>{left}</b>
          <small>{rescueClose(r, true)}</small>
          <small>{r.last ? `Hoy: ${r.next} min más` : `Mañana: ${r.next} min`} en partidas de {RESCUE_MIN_PLAY} min o más</small>
        </div>
        <div class="rcnt"><span class="k">Rescates · 30 días</span><b>{r.used} <small>de {RESCUE_MAX}</small></b></div>
      </div>
    </div>
  {:else}
    {@render list()}
    <div class="hfoot">
      <span>{r.last ? `Hoy ${r.next} min` : `Mañana ${r.next} min`} · cierra {rescueClose(r)}</span>
      <span class="rc2">Rescates <b>{r.used}/{RESCUE_MAX}</b></span>
    </div>
  {/if}
</div>

{#snippet list()}
  <div class="plist">
    <div class="k">Partidas {r.plays.length ? `${rescueDay(r.from).toLowerCase()} – ${rescueDay(r.to).toLowerCase()}` : 'de la ventana'}{r.plays.length > ROWS ? ` · últimas ${ROWS}` : ''}</div>
    {#each rows as p}
      <div class="pr" class:no={!p.counts}>
        <span class="pd">{rescueDay(p.day)}</span>
        <span class="pg"><s>{p.name}</s>{#if !p.counts}<em>{wide ? `no cuenta · menos de ${RESCUE_MIN_PLAY} min` : `< ${RESCUE_MIN_PLAY} min`}</em>{/if}</span>
        <span class="pm">{mins(p.min)}</span>
      </div>
    {:else}
      <div class="pnone">Aún no hay partidas en la ventana.</div>
    {/each}
  </div>
{/snippet}

<style>
  .resc{padding:18px 26px;
    box-shadow:inset 0 1.5px 0 #fff,0 0 0 2px color-mix(in srgb,var(--rc) 60%,white),0 18px 30px -18px color-mix(in srgb,var(--rc) 55%,black)}
  .ptitle{font-size:20px;font-weight:800;color:#4a5260;display:flex;align-items:center;gap:10px;white-space:nowrap}
  .ptitle small{font-size:15px;font-weight:600;color:#8a92a0}
  .rdot{width:14px;height:14px;border-radius:50%;background:var(--rc);flex:none}
  .stabs{display:flex;align-items:center;gap:3px;height:28px;margin:-4px 0 -4px auto;padding:0 3px;border-radius:999px}
  .stabs .t{height:22px;padding:0 11px;border-radius:999px;display:grid;place-items:center;font-size:14px;font-weight:800;color:#5f6776;cursor:pointer}
  .stabs .t.on{background:var(--c);color:#fff;box-shadow:inset 0 1px 0 rgba(255,255,255,.5),0 0 0 1px color-mix(in srgb,var(--c) 70%,black);cursor:default}
  .stabs .btn{cursor:pointer;transform:scale(.78);margin:0 -3px}
  .jar{display:flex;gap:3px;height:36px;margin:12px 0 6px;padding:4px;border-radius:14px;background:#f3f5f8;box-shadow:inset 0 1px 3px rgba(0,0,0,.12)}
  .jar i{flex:1;min-width:0;border-radius:5px;background:repeating-linear-gradient(135deg,#e3e6eb 0 3px,#eef0f3 3px 6px)}
  .jar i.d1{background:var(--rc)}
  .jar i.d2{background:color-mix(in srgb,var(--rc) 60%,white)}
  .jarl{display:flex;justify-content:space-between;align-items:baseline;gap:10px;line-height:20px;font-size:14px;font-weight:800;color:#7a8291}
  .jarl>*{white-space:nowrap}
  .jarl>span{min-width:0;overflow:hidden;text-overflow:ellipsis}
  .jarl b{color:var(--rink);font-size:18px}
  .jarl .sw2{display:inline-block;width:11px;height:11px;border-radius:3px;margin:0 4px 0 10px;vertical-align:-1px;background:var(--rc)}
  .jarl .sw2.d2{background:color-mix(in srgb,var(--rc) 60%,white)}
  .hrow{display:grid;grid-template-columns:minmax(0,1fr) 210px;gap:16px;margin-top:12px}
  .plist{border-radius:18px;padding:8px 16px;background:#f5f7fa;box-shadow:0 0 0 1px rgba(95,110,135,.14);min-width:0;overflow:hidden}
  .plist .k{font-size:13px;font-weight:700;color:#7a8291;text-transform:uppercase;letter-spacing:.05em}
  .pr{display:grid;grid-template-columns:70px minmax(0,1fr) 62px;align-items:center;column-gap:10px;height:24px;font-size:15px;font-weight:700;color:#4a5260;border-top:1px solid #e8ebf0}
  .plist .k + .pr{border-top:0}
  .pr .pd{font-size:13px;font-weight:800;color:#8a92a0}
  .pr .pg{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .pr .pg s{text-decoration:none}
  .pr .pm{text-align:right;font-weight:800;color:#3a414e}
  .pr.no{color:#a3abb8}
  .pr.no .pd,.pr.no .pm,.pr.no .pg>s{color:#a3abb8;text-decoration:line-through;text-decoration-thickness:2px}
  .pr.no .pg>em{font-style:normal;margin-left:8px;font-size:11.5px;font-weight:800;color:var(--rink);background:var(--rsoft);border-radius:999px;padding:2px 8px}
  .pnone{padding:10px 0;font-size:14px;font-weight:700;color:#8a92a0}
  .hside{display:flex;flex-direction:column;gap:10px}
  .cd{flex:1;border-radius:18px;padding:8px 16px;background:var(--rsoft);box-shadow:inset 0 0 0 1.5px color-mix(in srgb,var(--rc) 45%,white);color:var(--rink);font-size:13px;font-weight:800}
  .cd .k{color:var(--rink);font-size:13px;text-transform:uppercase;letter-spacing:.05em}
  .cd b{display:block;font-size:34px;line-height:1.05;margin:1px 0}
  .cd small{display:block;font-size:12.5px;font-weight:700;line-height:1.35;opacity:.9}
  .rcnt{border-radius:18px;padding:8px 16px;background:#f3f5f8;font-size:13px}
  .rcnt .k{font-size:13px;font-weight:700;color:#7a8291;text-transform:uppercase;letter-spacing:.05em}
  .rcnt b{display:block;font-size:22px;font-weight:800;color:#3a414e}
  .rcnt b small{font-size:14px;color:#6a7282}

  /* Mitad del hueco (dos rescates a la vez) */
  .half{padding:14px 18px;display:flex;flex-direction:column}
  .half .ptitle{font-size:17px;gap:8px}
  .half .ptitle small{font-size:13px}
  .half .ttl{overflow:hidden;text-overflow:ellipsis;min-width:0}
  .cdc{margin-left:auto;background:var(--rsoft);color:var(--rink);border-radius:999px;padding:2px 12px;font-size:17px;font-weight:800;box-shadow:inset 0 0 0 1.5px color-mix(in srgb,var(--rc) 45%,white)}
  .half .jar{height:36px;margin:12px 0 4px;gap:2px;padding:3px;border-radius:11px}
  .half .jar i{border-radius:4px}
  .half .jarl b{font-size:16px}
  .half .plist{margin-top:8px;padding:6px 12px;border-radius:14px}
  .half .plist .k{font-size:11.5px}
  .half .pr{grid-template-columns:48px minmax(0,1fr) 48px;column-gap:6px;height:28px;font-size:13.5px}
  .half .pr .pd{font-size:11.5px}
  .half .pr .pg em{margin-left:5px;font-size:10.5px;padding:1px 6px}
  .hfoot{margin-top:auto;display:flex;align-items:center;justify-content:space-between;gap:8px;font-size:12.5px;font-weight:800;color:var(--rink);padding-top:8px}
  .hfoot>span:first-child{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .hfoot .rc2{flex:none;background:#f3f5f8;border-radius:999px;padding:3px 10px;color:#6a7282}
  .hfoot .rc2 b{color:#3a414e}
</style>
