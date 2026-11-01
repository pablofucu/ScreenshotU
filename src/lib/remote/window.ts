// Sustituto de `@tauri-apps/api/window`: sin ventana de escritorio. Pantalla completa = la de la app.
const win = {
  isFullscreen: async () => true,
  setFullscreen: async (_on: boolean) => {},
  minimize: async () => {},
  close: async () => {},
};
export function getCurrentWindow() { return win; }
export const getCurrent = getCurrentWindow;
