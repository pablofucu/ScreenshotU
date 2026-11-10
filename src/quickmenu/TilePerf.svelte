<script lang="ts">
  // Baldosa y hoja de rendimiento: gráficas en vivo de CPU y GPU, VRAM y temperatura.
  import type { PerfState } from './data';

  let {
    perf = null,
    paused = false,
    view = 'tile',
    onOpenSheet,
  }: {
    perf: PerfState | null;
    paused?: boolean;
    view?: 'tile' | 'sheet';
    onOpenSheet?: () => void;
  } = $props();

  const lastVal = (arr: number[]) => (arr.length > 0 ? Math.round(arr[arr.length - 1]) : 0);
  const avgVal = (arr: number[]) => (arr.length > 0 ? Math.round(arr.reduce((a, b) => a + b, 0) / arr.length) : 0);
  const maxVal = (arr: number[]) => (arr.length > 0 ? Math.round(Math.max(...arr)) : 0);

  function makeChartPaths(arr: number[], w: number, h: number) {
    if (!arr || arr.length < 2) {
      return { line: `M0 ${h - 4} L${w} ${h - 4}`, area: `M0 ${h - 4} L${w} ${h - 4} L${w} ${h} L0 ${h} Z` };
    }
    const st = w / (arr.length - 1);
    let d = '';
    arr.forEach((v, i) => {
      const clamped = Math.max(0, Math.min(100, v));
      const y = h - 4 - (clamped / 100) * (h - 8);
      d += (i === 0 ? 'M' : 'L') + (i * st).toFixed(1) + ' ' + y.toFixed(1);
    });
    return {
      line: d,
      area: d + ` L${w} ${h} L0 ${h} Z`,
    };
  }

  const cpuChartTile = $derived(perf ? makeChartPaths(perf.cpu, 240, 72) : null);
  const gpuChartTile = $derived(perf ? makeChartPaths(perf.gpu, 240, 72) : null);
  const cpuChartSheet = $derived(perf ? makeChartPaths(perf.cpu, 480, 120) : null);
  const gpuChartSheet = $derived(perf ? makeChartPaths(perf.gpu, 480, 120) : null);
</script>

{#if view === 'tile'}
  <div
    class="glass tile t-perf"
    style="--i:0"
    data-f
    data-id="perf"
    onclick={onOpenSheet}
    onkeydown={(e) => { if (e.key === 'Enter') onOpenSheet?.(); }}
    role="button"
    tabindex="0"
  >
    <div class="lbl">
      <svg class="i" viewBox="0 0 24 24"><path d="M13 2L4 14h7l-1 8 9-12h-7z"/></svg>
      Rendimiento
      <em>en vivo</em>
    </div>
    {#if !perf}
      <div class="ach-empty">Midiendo…</div>
    {:else}
      <div class="pp">
        <div>
          <div class="pv">
            <span>CPU</span>
            <b>{lastVal(perf.cpu)}</b>%
            <em>{perf.cpuName.split(' ')[0] || 'CPU'}</em>
          </div>
          {#if cpuChartTile}
            <svg class="ch" viewBox="0 0 240 72" preserveAspectRatio="none" style="height:72px">
              <defs>
                <linearGradient id="c_t_cpu_g" x1="0" y1="0" x2="1" y2="0">
                  <stop offset="0%" style="stop-color:var(--arc-rojo)"/>
                  <stop offset="17%" style="stop-color:var(--arc-naranja)"/>
                  <stop offset="33%" style="stop-color:var(--arc-amarillo)"/>
                  <stop offset="50%" style="stop-color:var(--arc-verde)"/>
                  <stop offset="67%" style="stop-color:var(--arc-azul)"/>
                  <stop offset="83%" style="stop-color:var(--arc-anil)"/>
                  <stop offset="100%" style="stop-color:var(--arc-violeta)"/>
                </linearGradient>
              </defs>
              <path class="af" fill="url(#c_t_cpu_g)" fill-opacity="0.18" d={cpuChartTile.area}/>
              <path class="ln" stroke="url(#c_t_cpu_g)" d={cpuChartTile.line}/>
            </svg>
          {/if}
        </div>
        <div>
          <div class="pv">
            <span>GPU</span>
            <b>{lastVal(perf.gpu)}</b>%
            <em>{perf.gpuTemp != null ? `${perf.gpuTemp} °C` : 'GPU'}</em>
          </div>
          {#if gpuChartTile}
            <svg class="ch" viewBox="0 0 240 72" preserveAspectRatio="none" style="height:72px">
              <defs>
                <linearGradient id="c_t_gpu_g" x1="0" y1="0" x2="1" y2="0">
                  <stop offset="0%" style="stop-color:var(--arc-rojo)"/>
                  <stop offset="17%" style="stop-color:var(--arc-naranja)"/>
                  <stop offset="33%" style="stop-color:var(--arc-amarillo)"/>
                  <stop offset="50%" style="stop-color:var(--arc-verde)"/>
                  <stop offset="67%" style="stop-color:var(--arc-azul)"/>
                  <stop offset="83%" style="stop-color:var(--arc-anil)"/>
                  <stop offset="100%" style="stop-color:var(--arc-violeta)"/>
                </linearGradient>
              </defs>
              <path class="af" fill="url(#c_t_gpu_g)" fill-opacity="0.18" d={gpuChartTile.area}/>
              <path class="ln" stroke="url(#c_t_gpu_g)" d={gpuChartTile.line}/>
            </svg>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{:else}
  {#if !perf}
    <div class="ach-empty" style="font-size:var(--t-l);text-align:center;padding:40px">Midiendo…</div>
  {:else}
    <div class="vperf">
      <div class="pblk">
        <div class="phead">
          <span class="lbl"><svg class="i" viewBox="0 0 24 24"><rect x="6" y="6" width="12" height="12" rx="2"/><path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4"/></svg>CPU</span>
          <span class="sub">{perf.cpuName} · {perf.cpuThreads} hilos</span>
          <b class="pbig">{lastVal(perf.cpu)}<small>%</small></b>
        </div>
        {#if cpuChartSheet}
          <svg class="ch" viewBox="0 0 480 120" preserveAspectRatio="none" style="height:120px">
            <defs>
              <linearGradient id="c_s_cpu_g" x1="0" y1="0" x2="1" y2="0">
                <stop offset="0%" style="stop-color:var(--arc-rojo)"/>
                <stop offset="17%" style="stop-color:var(--arc-naranja)"/>
                <stop offset="33%" style="stop-color:var(--arc-amarillo)"/>
                <stop offset="50%" style="stop-color:var(--arc-verde)"/>
                <stop offset="67%" style="stop-color:var(--arc-azul)"/>
                <stop offset="83%" style="stop-color:var(--arc-anil)"/>
                <stop offset="100%" style="stop-color:var(--arc-violeta)"/>
              </linearGradient>
            </defs>
            <line class="gl" x1="0" x2="480" y1="30" y2="30"/>
            <line class="gl" x1="0" x2="480" y1="60" y2="60"/>
            <line class="gl" x1="0" x2="480" y1="90" y2="90"/>
            <path class="af" fill="url(#c_s_cpu_g)" fill-opacity="0.22" d={cpuChartSheet.area}/>
            <path class="ln" stroke="url(#c_s_cpu_g)" d={cpuChartSheet.line}/>
          </svg>
        {/if}
        <div class="pfoot">Media: <b>{avgVal(perf.cpu)} %</b> · Pico: <b>{maxVal(perf.cpu)} %</b></div>
      </div>

      <div class="pblk">
        <div class="phead">
          <span class="lbl"><svg class="i" viewBox="0 0 24 24"><path d="M13 2L4 14h7l-1 8 9-12h-7z"/></svg>GPU</span>
          <span class="sub">{perf.gpuName}</span>
          <b class="pbig">{lastVal(perf.gpu)}<small>%</small></b>
        </div>
        {#if gpuChartSheet}
          <svg class="ch" viewBox="0 0 480 120" preserveAspectRatio="none" style="height:120px">
            <defs>
              <linearGradient id="c_s_gpu_g" x1="0" y1="0" x2="1" y2="0">
                <stop offset="0%" style="stop-color:var(--arc-rojo)"/>
                <stop offset="17%" style="stop-color:var(--arc-naranja)"/>
                <stop offset="33%" style="stop-color:var(--arc-amarillo)"/>
                <stop offset="50%" style="stop-color:var(--arc-verde)"/>
                <stop offset="67%" style="stop-color:var(--arc-azul)"/>
                <stop offset="83%" style="stop-color:var(--arc-anil)"/>
                <stop offset="100%" style="stop-color:var(--arc-violeta)"/>
              </linearGradient>
            </defs>
            <line class="gl" x1="0" x2="480" y1="30" y2="30"/>
            <line class="gl" x1="0" x2="480" y1="60" y2="60"/>
            <line class="gl" x1="0" x2="480" y1="90" y2="90"/>
            <path class="af" fill="url(#c_s_gpu_g)" fill-opacity="0.22" d={gpuChartSheet.area}/>
            <path class="ln" stroke="url(#c_s_gpu_g)" d={gpuChartSheet.line}/>
          </svg>
        {/if}
        <div class="pfoot">
          {#if perf.gpuTemp != null}<b>{perf.gpuTemp} °C</b> · {/if}
          {#if perf.vramUsed != null && perf.vramTotal != null}VRAM <b>{perf.vramUsed.toFixed(1)} / {perf.vramTotal.toFixed(0)} GB</b> · {/if}
          Media: <b>{avgVal(perf.gpu)} %</b>
        </div>
      </div>

      <div class="sub">
        Últimos 2 minutos
        {#if paused}
          · Con el juego en pausa el uso de GPU cae casi al 0 %
        {/if}
      </div>
    </div>
  {/if}
{/if}
