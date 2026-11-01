// Sustituto de `@tauri-apps/api/webview`: no hay arrastrar y soltar archivos del sistema.
const view = {
  onDragDropEvent: async (_cb: (e: { payload: { type: string; paths: string[] } }) => void) => () => {},
};
export function getCurrentWebview() { return view; }
