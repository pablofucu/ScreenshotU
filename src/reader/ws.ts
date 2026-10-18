// Conexion WebSocket con la app (mismo host que la pagina) y reconexion automatica.
import type { ReaderMsg } from '../lib/novel-types';

export interface Link {
  send(msg: unknown): void;
  close(): void;
}

export function connect(onMsg: (m: ReaderMsg) => void, onState: (online: boolean) => void): Link {
  let ws: WebSocket | null = null;
  let timer: number | undefined;
  let wait = 800;
  let closed = false;

  const open = () => {
    if (closed) return;
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    try {
      ws = new WebSocket(`${proto}://${location.host}/ws`);
    } catch {
      retry();
      return;
    }
    ws.onopen = () => {
      wait = 800;
      onState(true);
    };
    ws.onmessage = (e) => {
      try {
        onMsg(JSON.parse(String(e.data)) as ReaderMsg);
      } catch {
        /* mensaje ilegible: se ignora */
      }
    };
    ws.onclose = () => {
      onState(false);
      retry();
    };
    ws.onerror = () => ws?.close();
  };
  const retry = () => {
    if (closed) return;
    clearTimeout(timer);
    timer = window.setTimeout(open, wait);
    wait = Math.min(wait * 1.5, 5000);
  };
  open();

  return {
    send(msg) {
      if (ws && ws.readyState === WebSocket.OPEN) ws.send(JSON.stringify(msg));
    },
    close() {
      closed = true;
      clearTimeout(timer);
      ws?.close();
    },
  };
}
