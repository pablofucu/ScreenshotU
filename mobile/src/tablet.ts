// Modo tablet: la interfaz de escritorio (src/App.svelte) tal cual, hablando con el PC por HTTP (src/lib/remote/).
// Sin servidor guardado se enseña antes la pantalla de conexión del móvil y se recarga al conectar.
import { loadServer } from './lib/cache';
import { pushConnection } from './lib/streaming.svelte';
import { relocate } from './lib/locate';

const saved = loadServer();
// El modo demostración no sirve para la interfaz de escritorio: cuenta como sin servidor.
const server = saved && saved !== 'demo' ? saved : null;
if (!server) {
  await import('./phone');
  const { app } = await import('./lib/store.svelte');
  // Cuando ya hay una instantánea del PC, el código es válido: se recarga y arranca la interfaz de escritorio.
  setInterval(() => { if (app.snap && app.server && app.server !== 'demo') location.reload(); }, 400);
} else {
  // Antes de cargar nada de escritorio: así `IS_TAURI` es cierto y `IS_TABLET` también.
  const w = window as unknown as Record<string, unknown>;
  w.__TAURI_INTERNALS__ = {};
  w.__VISOR_TABLET__ = true;
  pushConnection(server); // el servicio de la tecla de volumen necesita saber dónde está el PC

  await import('@desk/app.css');
  const [{ mount }, { default: App }, { App: Cap }, { setAfterLaunch }, { app: a }, { toast, back }] = await Promise.all([
    import('svelte'),
    import('@desk/App.svelte'),
    import('@capacitor/app'),
    import('@desk/lib/remote/core'),
    import('@desk/lib/store.svelte'),
    import('@desk/lib/controller.svelte'),
  ]);
  const { Streaming, stream } = await import('./lib/streaming.svelte');

  // «Jugar»: el PC ya ha lanzado el juego; la tablet abre Moonlight con el stream (igual que «Jugar en el PC» del móvil).
  // Si el PC tiene la app «Juego actual» de Apollo, la responde en `app`; si no, vale el ajuste o «Desktop».
  setAfterLaunch(async (_args, res) => {
    const { invoke } = await import('@desk/lib/remote/core');
    const cancel = (msg: string) => { void invoke('stream_cancel').catch(() => {}); toast(msg); };
    await stream.refresh();
    if (!stream.moonlight) { cancel('No está instalado Artemis ni Moonlight'); return; }
    try {
      await Streaming.stream({ pc: stream.pcName(server.name || server.host), app: (res as { app?: string | null } | null)?.app || stream.appName.trim() || 'Desktop' });
    } catch (e) {
      cancel(typeof e === 'string' ? e : e instanceof Error ? e.message : 'No se pudo abrir Artemis');
    }
  });

  // Resumen al acabar la partida: tiempo, capturas y meta de hoy (la meta se lee tras refrescar la biblioteca).
  const { listen } = await import('@desk/lib/remote/event');
  type Summary = { name: string; platform: string; seconds: number; shots: number };
  void listen<Summary>('session-summary', (ev) => {
    const s = ev.payload;
    setTimeout(async () => {
      let goal = '';
      if (s.platform === 'vn' || s.platform === 'jvn') {
        const { goalTrack } = await import('@desk/lib/readgoals');
        const g = s.platform === 'vn' ? a.config.goalVn : a.config.goalJvn;
        const t = goalTrack(s.platform, g, a.lib.platforms, null, Date.now());
        goal = ` · Meta de hoy ${Math.round(t.today)}/${g} min`;
      }
      const mins = Math.max(1, Math.round(s.seconds / 60));
      toast(`Has jugado ${s.name} ${mins} min · ${s.shots} ${s.shots === 1 ? 'captura' : 'capturas'}${goal}`, 5000);
    }, 1500);
  });

  // El PC manda: si la partida acabó (cerrada desde fuera, evento perdido con la app en segundo plano), se corrige aquí.
  const { endpoint } = await import('@desk/lib/remote/config');
  const { softRefresh } = await import('@desk/lib/controller.svelte');
  let syncing = false;
  const syncPlaying = async () => {
    if (syncing || document.hidden || !a.playing) return;
    const ep = endpoint();
    if (!ep) return;
    syncing = true;
    try {
      const r = await fetch(`${ep.base}/api/v1/session`, { headers: { Authorization: `Bearer ${ep.code}` }, cache: 'no-store' });
      if (!r.ok) return;
      const j = (await r.json()) as { state?: string; game?: { id: string } | null };
      if (j.state === 'idle' || !j.game || (a.playing && j.game.id !== a.playing.id)) {
        a.playing = null;
        void softRefresh();
      }
    } catch { /* sin conexión: se conserva lo que había */ } finally { syncing = false; }
  };
  document.addEventListener('visibilitychange', () => { void syncPlaying(); });
  setInterval(() => { void syncPlaying(); }, 5000);

  // Botón atrás de Android = atrás de la interfaz de escritorio (B del mando / Esc); en Inicio sin paneles, la app pasa a segundo plano.
  void Cap.addListener('backButton', () => {
    if (a.view === 'home' && !a.overlay && !a.audioMenu && !a.osk.open) void Cap.minimizeApp();
    else back();
  });

  mount(App, { target: document.getElementById('app')! });

  // Si el PC ha cambiado de IP, se busca por su identificador y se recarga con la dirección nueva (sin bloquear el arranque).
  void relocate(server).then((n) => { if (n.host !== server.host || n.port !== server.port) location.reload(); });
}
