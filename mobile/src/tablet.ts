// Modo tablet: la interfaz de escritorio (src/App.svelte) tal cual, hablando con el PC por HTTP (src/lib/remote/).
// Sin servidor guardado se enseña antes la pantalla de conexión del móvil y se recarga al conectar.
import { loadServer, saveServer, setLostNotice } from './lib/cache';
import { unpairedMsg } from './lib/source';
import { pushConnection, Streaming } from './lib/streaming.svelte';
import { relocate } from './lib/locate';
import { parseCloseStatus } from '@shared/remote/session_close';

const saved = loadServer();
// El modo demostración no sirve para la interfaz de escritorio: cuenta como sin servidor.
const server = saved && saved !== 'demo' ? saved : null;
if (!server) {
  await import('./phone');
  const { app } = await import('./lib/store.svelte');
  // Cuando ya hay una instantánea del PC, el token es válido: se recarga y arranca la interfaz de escritorio.
  setInterval(() => { if (app.snap && app.server && app.server !== 'demo') location.reload(); }, 400);
} else {
  // Antes de cargar nada de escritorio: así `IS_TAURI` es cierto y `IS_TABLET` también.
  const w = window as unknown as Record<string, unknown>;
  w.__TAURI_INTERNALS__ = {};
  w.__VISOR_TABLET__ = true;
  w.__VISOR_WAKE__ = (mac: string) => Streaming.wake({ mac }); // Wake-on-LAN para el botón del menú

  pushConnection(server); // el servicio de la tecla de volumen necesita saber dónde está el PC

  await import('@desk/app.css');
  const [{ mount }, { default: App }, { App: Cap }, { setAfterLaunch }, { app: a }, { toast, back, setResumeHandler }] = await Promise.all([
    import('svelte'),
    import('@desk/App.svelte'),
    import('@capacitor/app'),
    import('@desk/lib/remote/core'),
    import('@desk/lib/store.svelte'),
    import('@desk/lib/controller.svelte'),
  ]);
  const { stream } = await import('./lib/streaming.svelte');

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

  // Volver a la partida en marcha: abre el stream otra vez (Apollo retoma la sesión; nunca se relanza el juego).
  let streamApp = '';
  const openStream = async () => {
    await stream.refresh();
    if (!stream.moonlight) throw 'No está instalado Artemis ni Moonlight';
    await Streaming.stream({ pc: stream.pcName(server.name || server.host), app: streamApp || stream.appName.trim() || 'Desktop' });
  };
  setResumeHandler({
    resume: openStream,
    close: async (force) => {
      const ep = endpoint();
      if (!ep) throw 'Sin servidor configurado';
      const r = await fetch(`${ep.base}/api/v1/close`, { method: 'POST', headers: { Authorization: `Bearer ${ep.code}`, 'Content-Type': 'application/json' }, body: force ? '{"force":true}' : '{}' });
      if (!r.ok) throw 'No se pudo cerrar el juego';
      const data = await r.json().catch(() => ({}));
      return parseCloseStatus(data);
    },
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
    if (syncing || document.hidden) return;
    const ep = endpoint();
    if (!ep) return;
    syncing = true;
    try {
      const r = await fetch(`${ep.base}/api/v1/session`, { headers: { Authorization: `Bearer ${ep.code}`, 'X-Visor-Api': '3' }, cache: 'no-store' });
      if (r.status === 401) {
        // El PC quitó este dispositivo: se olvida el servidor y se vuelve al asistente con el aviso.
        setLostNotice(unpairedMsg(server.name));
        saveServer(null);
        location.reload();
        return;
      }
      if (!r.ok) return;
      const j = (await r.json()) as { state?: string; game?: { id: string } | null; app?: string | null; elapsedSeconds?: number };
      if (j.app) streamApp = j.app;
      if (!a.playing && j.game && (j.state === 'starting' || j.state === 'playing')) {
        // La app se reinició (o la partida empezó sin ella): se recupera lo que dice el PC.
        a.playing = { id: j.game.id, since: Date.now() - (j.elapsedSeconds ?? 0) * 1000 };
      } else if (a.playing && (j.state === 'idle' || !j.game || j.game.id !== a.playing.id)) {
        a.playing = null;
        void softRefresh();
      }
    } catch { /* sin conexión: se conserva lo que había */ } finally { syncing = false; }
  };
  void syncPlaying();
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
