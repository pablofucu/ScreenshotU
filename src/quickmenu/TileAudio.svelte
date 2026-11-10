<script lang="ts">
  // Baldosa de audio directo (volumen, silencio, salidas rápidas) y hoja con mezclador y entradas.
  import type { AudioState } from './data';

  let {
    audio = null,
    view = 'tile',
    onOpenSheet,
    onSetVolume,
    onToggleMute,
    onToggleMicMute,
    onSelectOutput,
    onSelectInput,
    onSetAppVolume,
    onSetAppMute,
  }: {
    audio: AudioState | null;
    view?: 'tile' | 'sheet';
    onOpenSheet?: () => void;
    onSetVolume?: (vol: number) => void;
    onToggleMute?: () => void;
    onToggleMicMute?: () => void;
    onSelectOutput?: (id: string) => void;
    onSelectInput?: (id: string) => void;
    onSetAppVolume?: (pid: number, vol: number) => void;
    onSetAppMute?: (pid: number, mute: boolean) => void;
  } = $props();

  const sortedApps = $derived(
    audio?.apps ? [...audio.apps].sort((a, b) => (b.isGame ? 1 : 0) - (a.isGame ? 1 : 0)) : []
  );

  function handleSliderClick(e: MouseEvent) {
    if (!audio) return;
    const target = e.currentTarget as HTMLElement;
    const vbar = target.querySelector('.vbar') as HTMLElement | null;
    if (!vbar) return;
    const rect = vbar.getBoundingClientRect();
    const x = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
    const pct = Math.round((x / rect.width) * 100);
    onSetVolume?.(pct);
  }

  function handleSliderKey(e: KeyboardEvent) {
    if (!audio) return;
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      onSetVolume?.(Math.max(0, audio.volume - 5));
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      onSetVolume?.(Math.min(100, audio.volume + 5));
    }
  }

  function handleAppSliderKey(e: KeyboardEvent, pid: number, curVol: number) {
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      onSetAppVolume?.(pid, Math.max(0, curVol - 5));
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      onSetAppVolume?.(pid, Math.min(100, curVol + 5));
    }
  }
</script>

{#if view === 'tile'}
  <div class="glass tile t-aud" style="--i:3">
    <div class="lbl">
      <svg class="i" viewBox="0 0 24 24"><path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12"/></svg>
      Audio
      <em>LT / RT desde cualquier sitio</em>
    </div>

    {#if !audio}
      <div class="ach-empty">No disponible</div>
    {:else}
      <div class="arow2">
        <div
          class="ctl mute"
          class:on={audio.mute}
          data-f
          data-id="mute"
          onclick={onToggleMute}
          onkeydown={(e) => { if (e.key === 'Enter') onToggleMute?.(); }}
          role="button"
          tabindex="0"
          title="Silenciar sonido"
        >
          {#if audio.mute}
            <svg class="i" viewBox="0 0 24 24"><path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M17 9l5 6M22 9l-5 6"/></svg>
          {:else}
            <svg class="i" viewBox="0 0 24 24"><path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12"/></svg>
          {/if}
        </div>

        <div
          class="ctl slider"
          data-f
          data-id="vol"
          data-slider
          onclick={handleSliderClick}
          onkeydown={handleSliderKey}
          role="slider"
          aria-valuenow={audio.mute ? 0 : audio.volume}
          aria-valuemin="0"
          aria-valuemax="100"
          tabindex="0"
        >
          <div class="vbar" style="--p:{audio.mute ? 0 : audio.volume}">
            <i></i>
          </div>
          <b>
            {audio.mute ? '—' : audio.volume}<small>{audio.mute ? '' : ' %'}</small>
          </b>
        </div>
      </div>

      <div class="chips">
        {#each audio.outputs.slice(0, 3) as o, i}
          <div
            class="ctl qchip"
            class:on={o.default}
            data-f
            data-id="o{i}"
            onclick={() => onSelectOutput?.(o.id)}
            onkeydown={(e) => { if (e.key === 'Enter') onSelectOutput?.(o.id); }}
            role="button"
            tabindex="0"
          >
            {#if i === 0}
              <svg class="i" viewBox="0 0 24 24"><path d="M4 15v-3a8 8 0 0 1 16 0v3"/><rect x="3" y="14" width="5" height="7" rx="2"/><rect x="16" y="14" width="5" height="7" rx="2"/></svg>
            {:else if i === 1}
              <svg class="i" viewBox="0 0 24 24"><rect x="6" y="3" width="12" height="18" rx="2"/><circle cx="12" cy="14" r="3"/><circle cx="12" cy="7" r="1"/></svg>
            {:else}
              <svg class="i" viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="12" rx="2"/><path d="M8 21h8"/></svg>
            {/if}
            {o.name.split(' (')[0]}
          </div>
        {/each}

        <div
          class="ctl qchip mic"
          class:on={audio.micMute}
          data-f
          data-id="mic"
          onclick={onToggleMicMute}
          onkeydown={(e) => { if (e.key === 'Enter') onToggleMicMute?.(); }}
          role="button"
          tabindex="0"
          title="Silenciar micrófono"
        >
          <svg class="i" viewBox="0 0 24 24"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/></svg>
        </div>

        <div
          class="ctl qchip more"
          data-f
          data-id="amore"
          onclick={onOpenSheet}
          onkeydown={(e) => { if (e.key === 'Enter') onOpenSheet?.(); }}
          role="button"
          tabindex="0"
        >
          Más…
        </div>
      </div>
    {/if}
  </div>
{:else}
  {#if !audio}
    <div class="ach-empty" style="font-size:var(--t-l);text-align:center;padding:40px">Audio no disponible</div>
  {:else}
    <div class="vaud">
      <div class="vcol">
        <div class="lbl">
          <svg class="i" viewBox="0 0 24 24"><path d="M4 15v-3a8 8 0 0 1 16 0v3"/><rect x="3" y="14" width="5" height="7" rx="2"/><rect x="16" y="14" width="5" height="7" rx="2"/></svg>
          Salida de audio
        </div>
        <div class="grp">
          {#each audio.outputs as o, i}
            {@const parts = o.name.split(' (')}
            <div
              class="row opt"
              class:on={o.default}
              data-f
              data-id="sh-out-{o.id}"
              onclick={() => onSelectOutput?.(o.id)}
              onkeydown={(e) => { if (e.key === 'Enter') onSelectOutput?.(o.id); }}
              role="button"
              tabindex="0"
            >
              <svg class="i" viewBox="0 0 24 24">
                {#if i === 0}
                  <path d="M4 15v-3a8 8 0 0 1 16 0v3"/><rect x="3" y="14" width="5" height="7" rx="2"/><rect x="16" y="14" width="5" height="7" rx="2"/>
                {:else if i === 1}
                  <rect x="6" y="3" width="12" height="18" rx="2"/><circle cx="12" cy="14" r="3"/><circle cx="12" cy="7" r="1"/>
                {:else}
                  <rect x="3" y="5" width="18" height="12" rx="2"/><path d="M8 21h8"/>
                {/if}
              </svg>
              <span>
                <b>{parts[0]}</b>
                <small>{parts[1] ? parts[1].slice(0, -1) : ''}</small>
              </span>
              <i class="radio"></i>
            </div>
          {/each}
        </div>

        <div class="lbl" style="margin-top:14px">
          <svg class="i" viewBox="0 0 24 24"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/></svg>
          Entrada (micrófono)
        </div>
        <div class="grp">
          {#each audio.inputs as inp}
            {@const parts = inp.name.split(' (')}
            <div
              class="row opt"
              class:on={inp.default}
              data-f
              data-id="sh-inp-{inp.id}"
              onclick={() => onSelectInput?.(inp.id)}
              onkeydown={(e) => { if (e.key === 'Enter') onSelectInput?.(inp.id); }}
              role="button"
              tabindex="0"
            >
              <svg class="i" viewBox="0 0 24 24"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/></svg>
              <span>
                <b>{parts[0]}</b>
                <small>{parts[1] ? parts[1].slice(0, -1) : ''}</small>
              </span>
              <i class="radio"></i>
            </div>
          {/each}
        </div>
      </div>

      <div class="vcol">
        <div class="lbl">
          <svg class="i" viewBox="0 0 24 24"><path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12"/></svg>
          Mezclador por aplicación
        </div>

        <div
          class="row"
          data-f
          data-id="sh-micmute"
          onclick={onToggleMicMute}
          onkeydown={(e) => { if (e.key === 'Enter') onToggleMicMute?.(); }}
          role="button"
          tabindex="0"
        >
          <svg class="i" viewBox="0 0 24 24"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/></svg>
          <span>
            <b>Silenciar micrófono</b>
            <small>Silencio general de entrada</small>
          </span>
          <i class="tog" class:on={audio.micMute}></i>
        </div>

        <div class="grp" style="margin-top:4px">
          {#each sortedApps as app}
            <div class="row" style="gap:14px;padding:8px 12px">
              <svg class="i" viewBox="0 0 24 24">
                {#if app.isGame}
                  <path d="M7 7h10a5 5 0 0 1 4.8 6.3l-1 3.7a2.6 2.6 0 0 1-4.4 1.1L14 16h-4l-2.4 2.1a2.6 2.6 0 0 1-4.4-1.1l-1-3.7A5 5 0 0 1 7 7z"/><path d="M8 10v4M6 12h4"/><circle cx="16" cy="11" r=".8"/><circle cx="17.5" cy="13" r=".8"/>
                {:else}
                  <path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12"/>
                {/if}
              </svg>
              <span style="flex:1;min-width:110px">
                <b>{app.name}</b>
                <small>{app.isGame ? 'Juego en marcha' : `PID ${app.pid}`}</small>
              </span>
              <div
                class="ctl mute"
                class:on={app.mute}
                style="width:36px;height:36px;--r:10px;border-radius:var(--r-s)"
                data-f
                data-id="app-mute-{app.pid}"
                onclick={() => onSetAppMute?.(app.pid, !app.mute)}
                onkeydown={(e) => { if (e.key === 'Enter') onSetAppMute?.(app.pid, !app.mute); }}
                role="button"
                tabindex="0"
                title="Silenciar {app.name}"
              >
                {#if app.mute}
                  <svg class="i" viewBox="0 0 24 24" style="width:18px;height:18px"><path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M17 9l5 6M22 9l-5 6"/></svg>
                {:else}
                  <svg class="i" viewBox="0 0 24 24" style="width:18px;height:18px"><path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12"/></svg>
                {/if}
              </div>
              <div
                class="ctl slider"
                style="height:38px;padding:0 10px;gap:8px;max-width:170px"
                data-f
                data-id="app-vol-{app.pid}"
                data-slider
                onkeydown={(e) => handleAppSliderKey(e, app.pid, app.volume)}
                role="slider"
                aria-valuenow={app.mute ? 0 : app.volume}
                aria-valuemin="0"
                aria-valuemax="100"
                tabindex="0"
              >
                <div class="vbar" style="--p:{app.mute ? 0 : app.volume}">
                  <i></i>
                </div>
                <b style="font-size:var(--t-m);min-width:44px">{app.mute ? '—' : app.volume}%</b>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
{/if}
