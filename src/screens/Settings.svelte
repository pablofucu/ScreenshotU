<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { pair, variablesCSS } from '../lib/atmos';
  import {
    closeSettings, dropRestore, rowHasArrows, setNav, setSection, settingsAct, SETTING_SECTIONS, type SettingId,
  } from '../lib/controller.svelte';
  import { comboLabel, hotkeyLabel, mouseLabel } from '../lib/learn.svelte';
  import { sg, sgdbKeyLabel } from '../lib/sgdb.svelte';
  import { ludusaviLabel } from '../lib/saves.svelte';
  import { lb } from '../lib/library.svelte';
  import { isReadingId, readingRow } from '../lib/reading.svelte';
  import { updateRow } from '../lib/updater.svelte';
  import { isRemoteId, remoteRow } from '../lib/remote.svelte';
  import Learn from '../components/Learn.svelte';
  import Hints from '../components/Hints.svelte';
  import { makeMoved } from '../lib/pointer';
  import { VARIANT_NAMES } from '../lib/achievements.svelte';

  const yn = (b: boolean) => (b ? 'Sí' : 'No');

  /** Contenido de cada fila (etiqueta, valor y ayuda). Se evalúa al pintar: depende del estado vivo de la app. */
  type Row = { l: string; v: string; d: string };
  const ROWS: Partial<Record<SettingId, () => Row>> = {
    root: () => ({ l: 'Carpeta de la biblioteca', v: lb.info?.dir || 'Sin elegir', d: 'Todo lo del visor está en esta carpeta: Capturas (una subcarpeta por plataforma y otra por juego), Partidas, Grafismos, Glosario, Diccionarios, la base de datos y los ajustes. Copiarla es una copia completa. Pulsa A para moverla a otra carpeta u otro disco: se copia entera y la anterior se queda donde está.' }),
    rescan: () => ({ l: 'Volver a escanear la biblioteca', v: '', d: 'Vuelve a leer todas las carpetas: capturas nuevas, juegos añadidos a mano y grafismos cambiados.' }),
    addgame: () => ({ l: 'Añadir juego…', v: '', d: 'Añade un ejecutable, un emulador con su ROM, una carpeta de juegos o solo una carpeta de capturas.' }),
    hidden: () => ({ l: 'Juegos ocultos', v: app.lib.hidden.length ? `${app.lib.hidden.length}` : 'Ninguno', d: 'Lista de juegos ocultos desde su ficha (Y → Ocultar juego) para volver a mostrarlos. Sus archivos no se tocan.' }),
    showhidden: () => ({ l: 'Mostrar ocultos (atenuados)', v: yn(app.config.showHidden), d: 'Enseña los juegos ocultos en las listas, atenuados y con una marca, en vez de quitarlos.' }),
    achcardvariant: () => {
      const v = app.config.achievementsCardVariant ?? 1;
      return { l: 'Diseño de logros en la ficha', v: `${v} · ${VARIANT_NAMES[v] ?? 'Anillo'}`, d: 'Diseño visual con el que se muestra la tarjeta de logros en la ficha de todos tus juegos: 1 · Anillo dorado + último logro, 2 · Portada + banda inferior, 3 · Mosaico de la colección (todos los iconos), 4 · Medalla de rango, 7 · Contador rodillo u 8 · Por rareza. Cambia para todos los juegos a la vez.' };
    },
    theme: () => ({ l: 'Tema', v: app.config.theme === 'dark' ? 'Noche ahumada' : 'Claro', d: 'Claro o Noche ahumada (oscuro). Cambia toda la interfaz a la vez; el color de cada sección y de cada juego se mantiene.' }),
    view: () => ({ l: 'Vista de títulos', v: app.config.titlesView === 'grid' ? 'Cuadrícula' : 'Carrusel', d: 'Cuadrícula paginada de 5×2 o carrusel con la portada enfocada en grande. También se cambia con Y en la lista.' }),
    shotsview: () => ({ l: 'Cuadrícula de capturas', v: app.config.shotsView === 'days' ? 'Agrupada por días' : 'Continua', d: 'Agrupada por días pone una cabecera por día (con lo que jugaste ese día) y una línea de tiempo por meses; LT/RT saltan de día. Solo con orden por fecha. También se cambia con el botón View.' }),
    shotsorder: () => ({ l: 'Orden de las capturas', v: app.config.shotsOrder === 'oldest' ? 'Más antiguas primero' : 'Más recientes primero', d: 'Orden con el que se abre cualquier lista de capturas: la de un juego, una saga, las favoritas o las agrupadas por días. Si un juego tiene su propio orden (Y en su lista de capturas), ese manda en ese juego.' }),
    collstyle: () => ({ l: 'Estilo de las colecciones', v: app.config.collectionsStyle === 'multi' ? 'Multicolor armonizada' : 'Aurora', d: 'Cómo se ve la pestaña de cada colección. Aurora: la pestaña lleva el color de la colección y por dentro flotan manchas difuminadas con los colores de las portadas. Multicolor armonizada: un degradado con los colores de las portadas. En los dos, los colores se igualan en luz e intensidad para que combinen.' }),
    sound: () => ({ l: 'Sonido de navegación', v: yn(app.config.sound), d: 'Pequeños sonidos al moverte por los menús y al confirmar.' }),
    fullscreen: () => ({ l: 'Pantalla completa (F11)', v: yn(app.fullscreen), d: 'La interfaz se diseña a 1280×720 y se escala para llenar la pantalla.' }),
    monitor: () => ({ l: 'Monitor del visor', v: app.monitorLabel || '…', d: 'Pulsa A o ‹ › para llevar el visor a otro monitor; se abre ahí la próxima vez. Los juegos se lanzan en el monitor donde esté el visor: si no es el principal de Windows, pasa a serlo mientras dura la partida (también para el escalado de Magpie) y al salir se restaura.' }),
    audioout: () => {
      const cur = app.audioOutputs.find((o) => o.default);
      return { l: 'Salida de audio', v: cur ? cur.name : app.audioOutputs.length ? 'Sin salida' : 'No disponible', d: 'Dispositivo de reproducción de Windows (altavoces, auriculares, monitor…). Pulsa A o ‹ › para pasar al siguiente: es el mismo cambio que en Configuración › Sonido, así que también afecta a los demás programas. En la barra superior hay un icono para elegir uno concreto.' };
    },
    audiocombo: () => ({ l: 'Cambiar audio con el mando', v: comboLabel(app.config.audioCombo), d: 'Pulsa A y luego el botón o la combinación (Xbox/XInput). Pasa a la siguiente salida de audio aunque estés en el juego; el mando vibra tantos pulsos como la posición de la salida elegida (1 la primera, 2 la segunda…) y sale un cartel con su nombre. ‹ › recorre combinaciones de serie o lo desactiva. Con el visor delante no actúa.' }),
    audiohotkey: () => ({ l: 'Cambiar audio con el teclado', v: hotkeyLabel(app.config.audioHotkey), d: 'Atajo global (funciona con el juego delante) que pasa a la siguiente salida de audio. Pulsa A y luego la tecla o combinación (p. ej. Ctrl+Alt+A). ‹ › recorre algunas de serie. Si otro programa ya la usa, te avisa y se queda la anterior.' }),
    combo: () => ({ l: 'Capturar con el mando', v: comboLabel(app.config.captureCombo), d: 'Pulsa A y luego el botón o la combinación que quieras en el mando (Xbox/XInput, incluido el botón Xbox y los gatillos). ‹ › recorre combinaciones de serie o lo desactiva. Con el visor delante no captura.' }),
    mouse: () => ({ l: 'Capturar con el ratón', v: mouseLabel(app.config.captureMouse), d: 'Pulsa A (o haz clic) y luego el botón central o uno lateral del ratón. El juego sigue recibiendo el clic. ‹ › lo cambia o lo desactiva.' }),
    hotkey: () => ({ l: 'Capturar con el teclado', v: hotkeyLabel(app.config.captureHotkey), d: 'Pulsa A y luego la tecla o combinación (p. ej. Ctrl+F12). Funciona aunque el visor esté minimizado; si otro programa ya la usa, te avisa y se queda la anterior.' }),
    magpie: () => ({ l: 'Capturas de juegos escalados', v: app.config.captureMagpie === 'source' ? 'Imagen original' : 'Imagen escalada', d: 'Si Magpie está escalando el juego, guarda lo que ves (escalada) o la imagen original del juego, sin escalar. Si Magpie no deja capturar su ventana, se guarda la original.' }),
    magpieexe: () => {
      const s = app.magpieStatus;
      const v = !s ? 'Buscando…' : !s.exe ? 'No encontrado' : (app.config.magpieExe ? '' : 'Auto · ') + s.exe.split(/[\\/]/).slice(-2).join('\\');
      const extra = !s?.exe ? '' : s.cunny ? ' Tiene los modelos CuNNy2 (los mejores para novelas visuales).' : ' No tiene los modelos CuNNy2: se usará Anime4K.';
      return { l: 'Programa Magpie', v, d: 'Magpie escala solo los juegos que tengan activado «Escalado con Magpie» en su ficha (Y). El visor mide la ventana del juego al lanzarlo, elige 2x o 4x según su resolución y el monitor, y reinicia Magpie con el perfil nuevo (pide permiso de administrador cuando cambia algo). Pulsa A para elegir Magpie.exe a mano; ‹ › vuelve a buscarlo solo.' + extra };
    },
    capsound: () => ({ l: 'Sonido al capturar', v: yn(app.config.captureSound), d: 'Suena un aviso corto cada vez que se guarda una captura.' }),
    caponly: () => ({ l: 'Capturar solo en juegos', v: yn(app.config.captureOnlyGames), d: 'Solo se captura con un juego abierto desde el visor, una ventana de un juego conocido o una ventana a pantalla completa. En el escritorio u otras aplicaciones no se hace la captura.' }),
    cappeek: () => ({ l: 'Miniatura al capturar', v: yn(app.config.capturePeek), d: 'Enseña la captura en pequeño, un segundo y medio, en la esquina superior derecha del juego. No sale en las capturas ni recibe clics. No se ve sobre juegos en pantalla completa exclusiva (sí en ventana o sin bordes).' }),
    caprumble: () => ({ l: 'Vibración al capturar', v: yn(app.config.captureRumble), d: 'Al capturar con el mando, vibra un instante el mando que hizo la combinación. El mando no se bloquea: el juego lo sigue recibiendo igual.' }),
    backup: () => ({ l: 'Crear copia de seguridad…', v: app.backupBusy ? 'Creando…' : '', d: 'Guarda en un .zip (nombre visor-copia-AAAA-MM-DD.zip) tu base de datos (favoritas, juegos, horas y partidas, estados, preferencias, sagas, novelas y glosario) y la configuración. Te pide dónde guardarlo.' }),
    backupmedia: () => ({ l: 'Incluir medios del glosario', v: yn(app.backupMedia), d: 'Añade a la copia la carpeta Glosario (capturas y audio de las frases guardadas). Puede hacerla bastante más grande. Las capturas de tus juegos y las partidas no se incluyen: para una copia completa, copia la carpeta de la biblioteca.' }),
    restore: () => ({ l: app.restoreReady ? 'Pulsa A otra vez para restaurar' : 'Restaurar copia de seguridad…', v: app.restoreReady, d: 'Elige un .zip creado aquí. Se comprueba que es válido y que no es de una versión más nueva; después, con A otra vez, el visor se reinicia y la sustituye. Tu base actual se guarda antes como visor.db.antes-de-restaurar (y config.json.antes-de-restaurar). Los medios del glosario se mezclan con los que ya tengas.' }),
    oncursor: () => ({ l: 'Abrir en el monitor del cursor', v: yn(app.config.openOnCursor), d: 'Al arrancar, el visor se abre en el monitor donde está el cursor (normalmente el del icono que pulsaste). Si está en No, se abre en el último monitor usado.' }),
    autobackup: () => ({ l: 'Copia automática', v: yn(app.config.autoBackup), d: 'Al arrancar, si la última copia automática tiene 7 días o más, guarda la base de datos en la carpeta «Copias de la base» de la biblioteca (visor-AAAA-MM-DD.db) y conserva las 5 últimas.' }),
    savesauto: () => ({ l: 'Copiar partidas al cerrar el juego', v: yn(app.config.savesAuto), d: 'Al cerrar un juego lanzado desde el visor, guarda en un .zip sus partidas si cambiaron desde la última copia. Se conservan las 10 últimas automáticas de cada juego; las manuales no se borran. Las ubicaciones de cada juego se revisan en su ficha (Y › Partidas guardadas).' }),
    inputmouse: () => ({ l: 'Contar ratón en las estadísticas', v: yn(app.config.inputMouse), d: 'Durante las partidas lanzadas desde el visor cuenta clics, muescas de rueda y distancia del cursor, solo con el juego delante. Se ve en Estadísticas de juego › Periféricos. Se aplica a la siguiente partida.' }),
    inputkeys: () => ({ l: 'Contar teclado en las estadísticas', v: yn(app.config.inputKeys), d: 'Cuenta cuántas teclas se pulsan, nunca cuáles, y solo con el juego delante. Algunos antivirus pueden avisar porque lee el teclado en segundo plano; por eso está desactivado de serie. Se aplica a la siguiente partida.' }),
    ludusavi: () => ({ l: 'Base de partidas de Ludusavi', v: ludusaviLabel(), d: 'Base libre de PCGamingWiki y Ludusavi con dónde guarda sus partidas cada juego. Pulsa A para descargarla o actualizarla (unos 18 MB). Para novelas visuales y juegos que no aparecen se usan además reglas por motor (Ren’Py, RPG Maker, KiriKiri, Unity, Unreal…).' }),
    liststalled: () => ({ l: 'Avisar de juegos parados', v: yn(app.config.listStalled), d: 'Al arrancar, revisa los juegos en «Jugando» que llevan tiempo sin jugarse y te pregunta qué hacer con ellos (una sola tarjeta para todos). «Jugar» cuenta como la última partida registrada o, si el juego no tiene partidas, su captura más reciente.' }),
    liststalleddays: () => ({ l: 'Días sin jugar', v: `${app.config.listStalledDays} días`, d: 'Un juego en «Jugando» se considera parado cuando pasan estos días sin jugarlo (de 7 a 180). Pulsa ‹ › para cambiarlo de uno en uno, o A para sumar una semana.' }),
    liststalledact: () => ({ l: 'Qué hacer con ellos', v: app.config.listStalledAction === 'pending' ? 'Pasar a Pendiente' : app.config.listStalledAction === 'dropped' ? 'Pasar a Abandonado' : 'Preguntar', d: 'Preguntar enseña una tarjeta al arrancar con Pendiente, Abandonado o Dejarlo para cada juego; «Dejarlo» no se vuelve a preguntar hasta que lo juegues otra vez. Las otras opciones los cambian solas y avisan con un mensaje.' }),
    listautoplay: () => ({ l: 'Al lanzar un pendiente, pasa a Jugando', v: yn(app.config.listAutoPlay), d: 'Cuando lanzas desde el visor un juego marcado como Pendiente, pasa solo a Jugando.' }),
    update: () => updateRow(),
    quit: () => ({ l: 'Salir de la aplicación', v: '', d: 'Cierra el visor. Las capturas con el atajo dejan de funcionar hasta que lo vuelvas a abrir.' }),
    sgdbkey: () => ({ l: 'Clave de API', v: sgdbKeyLabel(), d: 'Clave gratuita de steamgriddb.com (inicia sesión → Preferencias → API). Pulsa A para escribirla o pegarla con Ctrl+V.' }),
    sgdbauto: () => ({ l: 'Descargar grafismos que falten', v: yn(app.config.sgdbAuto), d: 'Al escanear, busca cada juego por su nombre y descarga la portada, el hero y el logo que falten. Se guardan en la carpeta Grafismos de la biblioteca: la carpeta de capturas del juego no se toca y sus grafismos propios siempre ganan.' }),
    sgdbnow: () => ({ l: 'Buscar ahora los que faltan', v: sg.status, d: 'Vuelve a buscar los grafismos que faltan en todos los juegos, también en los que ya se revisaron antes.' }),
    sgdbgame: () => ({ l: 'Cambiar grafismos de un juego…', v: '', d: 'Elige otra portada, hero o logo de SteamGridDB o una imagen del PC (View) para un juego, o corrige el juego que se encontró si no era el bueno.' }),
  };

  function row(id: SettingId): Row {
    if (isReadingId(id)) return readingRow(id);
    if (isRemoteId(id)) return remoteRow(id);
    return ROWS[id]?.() ?? { l: '', v: '', d: '' };
  }

  const sec = $derived(SETTING_SECTIONS[app.setSec]);
  const rows = $derived(sec.rows.map((id) => ({ id, ...row(id) })));
  const focused = $derived(rows[app.setRow]);

  // Lista con desplazamiento: la fila enfocada siempre queda dentro de las VIS visibles.
  const VIS = 6, RH = 56;
  let off = $state(0);
  $effect(() => {
    const n = rows.length, i = app.setRow;
    let o = Math.min(off, Math.max(0, n - VIS));
    if (i < o) o = i;
    else if (i >= o + VIS) o = i - VIS + 1;
    if (o !== off) off = o;
  });
  const showArrows = (id: SettingId) => rowHasArrows(id);
  const vars = (t: number) => { const p = pair(t); return Object.entries(variablesCSS(p.h1, p.h2)).map(([k, v]) => `${k}:${v}`).join(';'); };
  const dark = $derived(app.config.theme === 'dark');
  function wheel(e: WheelEvent) {
    if (rows.length <= VIS) return;
    e.preventDefault();
    const d = e.deltaY > 0 ? 1 : -1;
    off = Math.max(0, Math.min(rows.length - VIS, off + d));
  }

  const moved = makeMoved();
  function hover(e: MouseEvent, i: number) {
    if (moved(e)) { if (app.setRow !== i) dropRestore(); app.setRow = i; setNav.zone = 'body'; }
  }
</script>

<div class="scrim st-scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closeSettings(); }}>
  <div class="st-wrap" style={vars(sec.t)}>
    <div class="glass st-menu">
      <div class="face">
        <h2>Ajustes</h2>
        {#each SETTING_SECTIONS as s, k (s.name)}
          <button type="button" class="st-mrow" class:on={app.setSec === k} class:f={app.setSec === k && setNav.zone === 'menu'} style={vars(s.t)} onclick={() => { setSection(k); setNav.zone = 'menu'; }} onmousemove={() => { if (app.setSec !== k) setSection(k); setNav.zone = 'menu'; }}>
            <i></i><span>{s.name}</span>
          </button>
        {/each}
      </div>
    </div>
    <div class="glass st-body">
      <div class="face">
        <h3>{sec.name}</h3>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="st-rows" style="height:{VIS * RH}px" onwheel={wheel}>
        <div class="st-track" style="transform:translateY({-off * RH}px)">
        {#each rows as r, i (r.id)}
          <button type="button" class="st-row" class:f={app.setRow === i && setNav.zone === 'body'} class:cur={app.setRow === i} class:danger={r.id === 'quit' || (r.id === 'restore' && !!app.restoreReady)} data-set={r.id} onmousemove={(e) => hover(e, i)} onclick={() => { app.setRow = i; setNav.zone = 'body'; settingsAct(r.id, 0); }}>
            <span class="sl">{r.l}</span>
            {#if r.id === 'theme'}
              <span class="seg"><span class:on={!dark}>Claro</span><span class:on={dark}>Noche ahumada</span></span>
            {:else}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              {#if showArrows(r.id)}<span class="sarr" class:hid={app.setRow !== i} onclick={(e) => { e.stopPropagation(); settingsAct(r.id, -1); }}>‹</span>{/if}
              <span class="sv">{r.v}</span>
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              {#if showArrows(r.id)}<span class="sarr" class:hid={app.setRow !== i} onclick={(e) => { e.stopPropagation(); settingsAct(r.id, 1); }}>›</span>{/if}
            {/if}
          </button>
        {/each}
        </div>
        </div>
        {#if rows.length > VIS}
          <div class="st-scroll" style="height:{VIS * RH}px"><i style="height:{(VIS / rows.length) * 100}%;top:{(off / rows.length) * 100}%"></i></div>
        {/if}
        {#if focused}<div class="st-desc"><b>{focused.l}.</b> {focused.d}</div>{/if}
      </div>
    </div>
  </div>
  <Hints list={[['LB/RB', 'Sección', 'rb'], ['A', 'Seleccionar', 'a'], ['B', setNav.zone === 'body' ? 'Menú' : 'Cerrar', 'b']]} />
  {#if app.learn}<Learn />{/if}
</div>

<style>
  .st-scrim { background: var(--scrim); }
  .st-wrap { position: absolute; inset: 0; }
  .st-menu { left: 56px; top: 40px; width: 290px; height: 640px; --r: 36px; cursor: default; }
  .st-body { left: 366px; top: 40px; width: 858px; height: 640px; --r: 36px; cursor: default; }
  .st-menu .face, .st-body .face { background: var(--face-hi); color: var(--ink); display: flex; flex-direction: column; padding: 24px 22px; position: relative; }
  .st-menu h2 { margin: 0 8px 16px; font-size: var(--t-2xl); font-weight: 800; color: var(--ink); }
  .st-mrow { position: relative; flex: none; display: flex; align-items: center; gap: 14px; height: 56px; margin-bottom: 6px; padding: 0 16px; border: 0; border-radius: var(--r-m); background: transparent; font: inherit; font-size: var(--t-xl); font-weight: 700; color: var(--ink-2); text-align: left; cursor: pointer; transition: background .15s, color .15s; }
  .st-mrow i { flex: none; width: 14px; height: 14px; border-radius: 50%; background: var(--grad); box-shadow: 0 0 0 2px var(--line); }
  .st-mrow.on { background: var(--gf); color: var(--on); }
  .st-mrow.on i { background: var(--on); box-shadow: none; }
  .st-mrow.f { box-shadow: var(--ring); }
  .st-body h3 { margin: 0 0 10px; font-size: var(--t-2xl); font-weight: 800; color: var(--ink); }
  .st-rows { position: relative; overflow: hidden; flex: none; box-sizing: content-box; margin: -4px -8px 0; padding: 4px 8px 0; }
  .st-track { transition: transform .2s cubic-bezier(.25, 1, .4, 1); }
  .st-row { display: flex; align-items: center; gap: 12px; width: 100%; height: 52px; margin-bottom: 4px; padding: 0 20px; border: 0; border-radius: var(--r-m); background: var(--glass-in); font: inherit; font-size: var(--t-l); font-weight: 700; color: var(--ink); text-align: left; cursor: pointer; transition: background .15s, color .15s; }
  .st-row .sl { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .st-row .sv { max-width: 340px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; color: var(--ink-2); }
  .st-row .sarr { flex: none; font-size: 22px; line-height: 1; padding: 0 4px; opacity: .85; }
  .st-row .sarr.hid { visibility: hidden; }
  .st-row.cur { background: var(--glass-chip); }
  .st-row.f { background: var(--gf); color: var(--on); box-shadow: var(--ring); position: relative; z-index: 1; }
  .st-row.f .sv { color: inherit; opacity: .92; }
  .st-row.danger.f { background: var(--bad); color: var(--on); }
  .seg { display: flex; padding: 3px; border-radius: var(--r-pill); background: var(--line); }
  .seg span { padding: 5px 16px; border-radius: var(--r-pill); font-size: var(--t-m); font-weight: 700; color: var(--ink-2); }
  .seg span.on { background: var(--gf); color: var(--on); }
  .st-row.f .seg { background: var(--glass-in); }
  .st-scroll { position: absolute; right: 8px; top: 98px; width: 5px; border-radius: 3px; background: var(--line); }
  .st-scroll i { position: absolute; left: 0; right: 0; border-radius: 3px; background: var(--g2); }
  .st-desc { flex: none; margin: auto 0 0; height: 124px; overflow: hidden; border-radius: var(--r-m); padding: 12px 18px; background: var(--glass-in); font-size: var(--t-s); font-weight: 500; line-height: 1.45; color: var(--ink-2); }
  .st-desc b { color: var(--ink); font-weight: 800; }
</style>
