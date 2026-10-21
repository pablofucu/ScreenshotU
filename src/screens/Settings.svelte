<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { ICON, RAINBOW } from '../lib/icons';
  import {
    accentState, closeSettings, dropRestore, setSection, settingsAct, SETTING_SECTIONS, type SettingId,
  } from '../lib/controller.svelte';
  import { comboLabel, hotkeyLabel, mouseLabel } from '../lib/learn.svelte';
  import { sg, sgdbKeyLabel } from '../lib/sgdb.svelte';
  import { ludusaviLabel } from '../lib/saves.svelte';
  import { lb } from '../lib/library.svelte';
  import { READING_CYCLE, isReadingId, readingRow } from '../lib/reading.svelte';
  import { updateRow } from '../lib/updater.svelte';
  import { REMOTE_CYCLE, isRemoteId, remoteRow } from '../lib/remote.svelte';
  import Learn from '../components/Learn.svelte';
  import Hints from '../components/Hints.svelte';

  const yn = (b: boolean) => (b ? 'Sí' : 'No');

  // Filas que cambian de valor con izquierda/derecha (se marcan con ‹ › al enfocarlas).
  const CYCLE = new Set<SettingId>(['showhidden', 'view', 'shotsview', 'accent', 'sound', 'fullscreen', 'monitor', 'oncursor', 'audioout', 'audiocombo', 'audiohotkey', 'hotkey', 'combo', 'mouse', 'magpie', 'magpieexe', 'capsound', 'sgdbauto', 'backupmedia', 'autobackup', 'savesauto']);

  function row(id: SettingId): { l: string; v: string; d: string } {
    if (isReadingId(id)) return readingRow(id);
    if (isRemoteId(id)) return remoteRow(id);
    switch (id) {
      case 'root': return { l: 'Carpeta de la biblioteca', v: lb.info?.dir || 'Sin elegir', d: 'Todo lo del visor está en esta carpeta: Capturas (una subcarpeta por plataforma y otra por juego), Partidas, Grafismos, Glosario, Diccionarios, la base de datos y los ajustes. Copiarla es una copia completa. Pulsa A para moverla a otra carpeta u otro disco: se copia entera y la anterior se queda donde está.' };
      case 'rescan': return { l: 'Volver a escanear la biblioteca', v: '', d: 'Vuelve a leer todas las carpetas: capturas nuevas, juegos añadidos a mano y grafismos cambiados.' };
      case 'addgame': return { l: 'Añadir juego…', v: '', d: 'Añade un ejecutable, un emulador con su ROM, una carpeta de juegos o solo una carpeta de capturas.' };
      case 'hidden': return { l: 'Juegos ocultos', v: app.lib.hidden.length ? `${app.lib.hidden.length}` : 'Ninguno', d: 'Lista de juegos ocultos desde su ficha (Y → Ocultar juego) para volver a mostrarlos. Sus archivos no se tocan.' };
      case 'showhidden': return { l: 'Mostrar ocultos (atenuados)', v: yn(app.config.showHidden), d: 'Enseña los juegos ocultos en las listas, atenuados y con una marca, en vez de quitarlos.' };
      case 'accent': return { l: 'Color de acento', v: accentState() === 0 ? 'Del juego (automático)' : accentState() === 1 ? 'Arcoíris' : RAINBOW[app.config.accentFixed].n, d: '«Del juego» saca el color del hero y la portada de cada juego. También puedes usar el arcoíris o un color fijo. Cada juego puede tener el suyo desde su ficha (Y → Color de acento).' };
      case 'view': return { l: 'Vista de títulos', v: app.config.titlesView === 'grid' ? 'Cuadrícula' : 'Carrusel', d: 'Cuadrícula paginada de 5×2 o carrusel con la portada enfocada en grande. También se cambia con Y en la lista.' };
      case 'shotsview': return { l: 'Cuadrícula de capturas', v: app.config.shotsView === 'days' ? 'Agrupada por días' : 'Continua', d: 'Agrupada por días pone una cabecera por día (con lo que jugaste ese día) y una línea de tiempo por meses; LT/RT saltan de día. Solo con orden por fecha. También se cambia con el botón View.' };
      case 'sound': return { l: 'Sonido de navegación', v: yn(app.config.sound), d: 'Pequeños sonidos al moverte por los menús y al confirmar.' };
      case 'fullscreen': return { l: 'Pantalla completa (F11)', v: yn(app.fullscreen), d: 'La interfaz se diseña a 1280×720 y se escala para llenar la pantalla.' };
      case 'monitor': return { l: 'Monitor del visor', v: app.monitorLabel || '…', d: 'Pulsa A o ‹ › para llevar el visor a otro monitor; se abre ahí la próxima vez. Los juegos se lanzan en el monitor donde esté el visor: si no es el principal de Windows, pasa a serlo mientras dura la partida (también para el escalado de Magpie) y al salir se restaura.' };
      case 'audioout': {
        const cur = app.audioOutputs.find((o) => o.default);
        return { l: 'Salida de audio', v: cur ? cur.name : app.audioOutputs.length ? 'Sin salida' : 'No disponible', d: 'Dispositivo de reproducción de Windows (altavoces, auriculares, monitor…). Pulsa A o ‹ › para pasar al siguiente: es el mismo cambio que en Configuración › Sonido, así que también afecta a los demás programas. En la barra superior hay un icono para elegir uno concreto.' };
      }
      case 'audiocombo': return { l: 'Cambiar audio con el mando', v: comboLabel(app.config.audioCombo), d: 'Pulsa A y luego el botón o la combinación (Xbox/XInput). Pasa a la siguiente salida de audio aunque estés en el juego; el mando vibra tantos pulsos como la posición de la salida elegida (1 la primera, 2 la segunda…) y sale un cartel con su nombre. ‹ › recorre combinaciones de serie o lo desactiva. Con el visor delante no actúa.' };
      case 'audiohotkey': return { l: 'Cambiar audio con el teclado', v: hotkeyLabel(app.config.audioHotkey), d: 'Atajo global (funciona con el juego delante) que pasa a la siguiente salida de audio. Pulsa A y luego la tecla o combinación (p. ej. Ctrl+Alt+A). ‹ › recorre algunas de serie. Si otro programa ya la usa, te avisa y se queda la anterior.' };
      case 'combo': return { l: 'Capturar con el mando', v: comboLabel(app.config.captureCombo), d: 'Pulsa A y luego el botón o la combinación que quieras en el mando (Xbox/XInput, incluido el botón Xbox y los gatillos). ‹ › recorre combinaciones de serie o lo desactiva. Con el visor delante no captura.' };
      case 'mouse': return { l: 'Capturar con el ratón', v: mouseLabel(app.config.captureMouse), d: 'Pulsa A (o haz clic) y luego el botón central o uno lateral del ratón. El juego sigue recibiendo el clic. ‹ › lo cambia o lo desactiva.' };
      case 'hotkey': return { l: 'Capturar con el teclado', v: hotkeyLabel(app.config.captureHotkey), d: 'Pulsa A y luego la tecla o combinación (p. ej. Ctrl+F12). Funciona aunque el visor esté minimizado; si otro programa ya la usa, te avisa y se queda la anterior.' };
      case 'magpie': return { l: 'Capturas de juegos escalados', v: app.config.captureMagpie === 'source' ? 'Imagen original' : 'Imagen escalada', d: 'Si Magpie está escalando el juego, guarda lo que ves (escalada) o la imagen original del juego, sin escalar. Si Magpie no deja capturar su ventana, se guarda la original.' };
      case 'magpieexe': {
        const s = app.magpieStatus;
        const v = !s ? 'Buscando…' : !s.exe ? 'No encontrado' : (app.config.magpieExe ? '' : 'Auto · ') + s.exe.split(/[\\/]/).slice(-2).join('\\');
        const extra = !s?.exe ? '' : s.cunny ? ' Tiene los modelos CuNNy2 (los mejores para novelas visuales).' : ' No tiene los modelos CuNNy2: se usará Anime4K.';
        return { l: 'Programa Magpie', v, d: 'Magpie escala solo los juegos que tengan activado «Escalado con Magpie» en su ficha (Y). El visor mide la ventana del juego al lanzarlo, elige 2x o 4x según su resolución y el monitor, y reinicia Magpie con el perfil nuevo (pide permiso de administrador cuando cambia algo). Pulsa A para elegir Magpie.exe a mano; ‹ › vuelve a buscarlo solo.' + extra };
      }
      case 'capsound': return { l: 'Sonido al capturar', v: yn(app.config.captureSound), d: 'Suena un aviso corto cada vez que se guarda una captura.' };
      case 'cappeek': return { l: 'Miniatura al capturar', v: yn(app.config.capturePeek), d: 'Enseña la captura en pequeño, un segundo y medio, en la esquina superior derecha del juego. No sale en las capturas ni recibe clics. No se ve sobre juegos en pantalla completa exclusiva (sí en ventana o sin bordes).' };
      case 'caprumble': return { l: 'Vibración al capturar', v: yn(app.config.captureRumble), d: 'Al capturar con el mando, vibra un instante el mando que hizo la combinación. El mando no se bloquea: el juego lo sigue recibiendo igual.' };
      case 'backup': return { l: 'Crear copia de seguridad…', v: app.backupBusy ? 'Creando…' : '', d: 'Guarda en un .zip (nombre visor-copia-AAAA-MM-DD.zip) tu base de datos (favoritas, juegos, horas y partidas, estados, preferencias, sagas, novelas y glosario) y la configuración. Te pide dónde guardarlo.' };
      case 'backupmedia': return { l: 'Incluir medios del glosario', v: yn(app.backupMedia), d: 'Añade a la copia la carpeta Glosario (capturas y audio de las frases guardadas). Puede hacerla bastante más grande. Las capturas de tus juegos y las partidas no se incluyen: para una copia completa, copia la carpeta de la biblioteca.' };
      case 'restore': return { l: app.restoreReady ? 'Pulsa A otra vez para restaurar' : 'Restaurar copia de seguridad…', v: app.restoreReady, d: 'Elige un .zip creado aquí. Se comprueba que es válido y que no es de una versión más nueva; después, con A otra vez, el visor se reinicia y la sustituye. Tu base actual se guarda antes como visor.db.antes-de-restaurar (y config.json.antes-de-restaurar). Los medios del glosario se mezclan con los que ya tengas.' };
      case 'oncursor': return { l: 'Abrir en el monitor del cursor', v: yn(app.config.openOnCursor), d: 'Al arrancar, el visor se abre en el monitor donde está el cursor (normalmente el del icono que pulsaste). Si está en No, se abre en el último monitor usado.' };
      case 'autobackup': return { l: 'Copia automática', v: yn(app.config.autoBackup), d: 'Al arrancar, si la última copia automática tiene 7 días o más, guarda la base de datos en la carpeta «Copias de la base» de la biblioteca (visor-AAAA-MM-DD.db) y conserva las 5 últimas.' };
      case 'savesauto': return { l: 'Copiar partidas al cerrar el juego', v: yn(app.config.savesAuto), d: 'Al cerrar un juego lanzado desde el visor, guarda en un .zip sus partidas si cambiaron desde la última copia. Se conservan las 10 últimas automáticas de cada juego; las manuales no se borran. Las ubicaciones de cada juego se revisan en su ficha (Y › Partidas guardadas).' };
      case 'ludusavi': return { l: 'Base de partidas de Ludusavi', v: ludusaviLabel(), d: 'Base libre de PCGamingWiki y Ludusavi con dónde guarda sus partidas cada juego. Pulsa A para descargarla o actualizarla (unos 18 MB). Para novelas visuales y juegos que no aparecen se usan además reglas por motor (Ren’Py, RPG Maker, KiriKiri, Unity, Unreal…).' };
      case 'update': return updateRow();
      case 'quit': return { l: 'Salir de la aplicación', v: '', d: 'Cierra el visor. Las capturas con el atajo dejan de funcionar hasta que lo vuelvas a abrir.' };
      case 'sgdbkey': return { l: 'Clave de API', v: sgdbKeyLabel(), d: 'Clave gratuita de steamgriddb.com (inicia sesión → Preferencias → API). Pulsa A para escribirla o pegarla con Ctrl+V.' };
      case 'sgdbauto': return { l: 'Descargar grafismos que falten', v: yn(app.config.sgdbAuto), d: 'Al escanear, busca cada juego por su nombre y descarga la portada, el hero y el logo que falten. Se guardan en la carpeta Grafismos de la biblioteca: la carpeta de capturas del juego no se toca y sus grafismos propios siempre ganan.' };
      case 'sgdbnow': return { l: 'Buscar ahora los que faltan', v: sg.status, d: 'Vuelve a buscar los grafismos que faltan en todos los juegos, también en los que ya se revisaron antes.' };
      case 'sgdbgame': return { l: 'Cambiar grafismos de un juego…', v: '', d: 'Elige otra portada, hero o logo de SteamGridDB o una imagen del PC (View) para un juego, o corrige el juego que se encontró si no era el bueno.' };
    }
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
  const showArrows = (id: SettingId) => CYCLE.has(id) || READING_CYCLE.has(id as never) || REMOTE_CYCLE.has(id as never);
  function wheel(e: WheelEvent) {
    if (rows.length <= VIS) return;
    e.preventDefault();
    const d = e.deltaY > 0 ? 1 : -1;
    off = Math.max(0, Math.min(rows.length - VIS, off + d));
  }

  let lastXY = '';
  function hover(e: MouseEvent, i: number) {
    const xy = e.clientX + ',' + e.clientY;
    if (xy !== lastXY) { lastXY = xy; if (app.setRow !== i) dropRestore(); app.setRow = i; }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closeSettings(); }}>
  <div class="glass setpanel secpanel" style="--acc:{RAINBOW[SETTING_SECTIONS[app.setSec].color].c};--acc-ink:{RAINBOW[SETTING_SECTIONS[app.setSec].color].dark ? '#3a2f00' : '#fff'}">
    <div class="face">
      <div class="scats">
        <h2>Ajustes</h2>
        {#each SETTING_SECTIONS as s, k (s.name)}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="scat" class:on={app.setSec === k} style="--c:{RAINBOW[s.color].c}" onclick={() => setSection(k)}>
            <i>{@html ICON[s.icon]}</i>{s.name}
          </div>
        {/each}
      </div>
      <div class="sbody">
        <h3>{sec.name}</h3>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="srows" style="height:{VIS * RH}px" onwheel={wheel}>
        <div class="srtrack" style="transform:translateY({-off * RH}px)">
        {#each rows as r, i (r.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="srow" class:f={app.setRow === i} class:danger={r.id === 'quit' || (r.id === 'restore' && !!app.restoreReady)} data-set={r.id} onmousemove={(e) => hover(e, i)} onclick={() => { app.setRow = i; settingsAct(r.id, 0); }}>
            <span class="sl">{r.l}</span>
            {#if r.id === 'accent'}
              <span class="sw">
                <i class="art" title="Del juego" class:sel={accentState() === 0}></i>
                <i class="rb" title="Arcoíris" class:sel={accentState() === 1}></i>
                {#each RAINBOW as c, k}<i style="background:{c.c}" class:sel={accentState() === k + 2}></i>{/each}
              </span>
            {/if}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            {#if showArrows(r.id)}<span class="sarr" class:hid={app.setRow !== i} onclick={(e) => { e.stopPropagation(); settingsAct(r.id, -1); }}>‹</span>{/if}
            <span class="sv">{r.v}</span>
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            {#if showArrows(r.id)}<span class="sarr" class:hid={app.setRow !== i} onclick={(e) => { e.stopPropagation(); settingsAct(r.id, 1); }}>›</span>{/if}
          </div>
        {/each}
        </div>
        </div>
        {#if rows.length > VIS}
          <div class="sscroll2" style="top:{26 + 34 + 4}px;height:{VIS * RH}px"><i style="height:{(VIS / rows.length) * 100}%;top:{(off / rows.length) * 100}%"></i></div>
        {/if}
        {#if focused}<div class="sdesc"><b>{focused.l}.</b> {focused.d}</div>{/if}
      </div>
    </div>
  </div>
  <Hints list={[['LB/RB', 'Sección', 'rb'], ['A', 'Seleccionar', 'a'], ['B', 'Cerrar', 'b']]} />
  {#if app.learn}<Learn />{/if}
</div>
