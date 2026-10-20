import { app } from './store.svelte';

export type ImgKind = 'thumb' | 'preview' | 'file';

/** URL de una imagen del servidor según el tamaño pedido. */
export function urlOf(path: string, kind: ImgKind, signal?: AbortSignal): Promise<string> {
  const s = app.source;
  if (!s) return Promise.reject(new Error('sin conexión'));
  return kind === 'thumb' ? s.thumb(path, signal) : kind === 'preview' ? s.preview(path, signal) : s.file(path);
}

/** Acción: avisa una vez cuando el elemento se acerca a la pantalla. */
export function nearView(node: HTMLElement, cb: () => void) {
  const io = new IntersectionObserver((es) => {
    if (es.some((e) => e.isIntersecting)) { io.disconnect(); cb(); }
  }, { rootMargin: '600px 0px' });
  io.observe(node);
  return { destroy: () => io.disconnect() };
}
