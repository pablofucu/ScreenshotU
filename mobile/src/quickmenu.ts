// Menú sobre el juego de la tablet: la página del overlay del PC (src/quickmenu) dentro de un WebView que abre el
// servicio de accesibilidad (CaptureKeyService) encima de Artemis. Habla con el PC por HTTP como el resto de la tablet.
const w = window as unknown as Record<string, unknown> & { VisorQm?: { config(): string } };
try { if (w.VisorQm) window.localStorage.setItem('visor.m.server', w.VisorQm.config()); } catch { /* sin almacenamiento */ }
// Antes de cargar nada de escritorio, igual que en tablet.ts: `isTauri()` y la interfaz de la tablet.
w.__TAURI_INTERNALS__ = {};
w.__VISOR_TABLET__ = true;
w.__QM_TABLET__ = true;

await import('@desk/quickmenu/main');
