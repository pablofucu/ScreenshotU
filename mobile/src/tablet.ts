// Modo tablet: la interfaz de escritorio (src/App.svelte) tal cual, hablando con el PC por HTTP (src/lib/remote/).
// Sin servidor guardado se enseña antes la pantalla de conexión del móvil y se recarga al conectar.
import { loadServer } from './lib/cache';
import { pushConnection } from './lib/streaming.svelte';

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
  setAfterLaunch(async () => {
    await stream.refresh();
    if (!stream.moonlight) { toast('No está instalado Artemis ni Moonlight'); return; }
    await Streaming.stream({ pc: stream.pcName(server.name || server.host), app: stream.appName.trim() || 'Desktop' });
  });

  // Botón atrás de Android = atrás de la interfaz de escritorio (B del mando / Esc); en Inicio sin paneles, la app pasa a segundo plano.
  void Cap.addListener('backButton', () => {
    if (a.view === 'home' && !a.overlay && !a.audioMenu && !a.osk.open) void Cap.minimizeApp();
    else back();
  });

  mount(App, { target: document.getElementById('app')! });
}
