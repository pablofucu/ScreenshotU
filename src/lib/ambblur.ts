// Fondo ambiental difuminado una sola vez en un lienzo pequeño: así el escenario no repinta un
// blur(44px) a pantalla completa (bajo una docena de cristales) cada vez que cambia el juego.
const W = 160;
const H = 90;
const MAX = 40;
const done = new Map<string, HTMLCanvasElement>();
const pending = new Map<string, Promise<HTMLCanvasElement | null>>();

function store(url: string, c: HTMLCanvasElement) {
  done.delete(url);
  done.set(url, c);
  if (done.size > MAX) done.delete(done.keys().next().value!);
}

/** Lienzo ya difuminado de esa URL, si existe (síncrono). */
export const ambBlurNow = (url: string) => done.get(url) ?? null;

/** Carga la imagen y la difumina en un lienzo de 160x90 (una vez por URL); null si no carga. */
export function ambBlur(url: string): Promise<HTMLCanvasElement | null> {
  const hit = done.get(url);
  if (hit) return Promise.resolve(hit);
  let p = pending.get(url);
  if (!p) {
    p = new Promise<HTMLCanvasElement | null>((ok) => {
      const im = new Image();
      im.decoding = 'async';
      im.onload = () => {
        const c = document.createElement('canvas');
        c.width = W;
        c.height = H;
        const ctx = c.getContext('2d');
        if (!ctx || !im.naturalWidth) return ok(null);
        // Con margen para que el difuminado no deje el borde transparente.
        const o = 20;
        const s = Math.max((W + o * 2) / im.naturalWidth, (H + o * 2) / im.naturalHeight);
        const dw = im.naturalWidth * s;
        const dh = im.naturalHeight * s;
        ctx.filter = 'blur(5.5px) saturate(1.4)';
        ctx.drawImage(im, (W - dw) / 2, (H - dh) / 2, dw, dh);
        store(url, c);
        ok(c);
      };
      im.onerror = () => ok(null);
      im.src = url;
    }).finally(() => pending.delete(url));
    pending.set(url, p);
  }
  return p;
}

/** Acción: pinta en el <canvas> el lienzo difuminado de la URL. */
export function ambPaint(node: HTMLCanvasElement, url: string) {
  const paint = (u: string) => {
    const src = done.get(u);
    if (!src) return;
    node.width = W;
    node.height = H;
    node.getContext('2d')?.drawImage(src, 0, 0);
  };
  paint(url);
  return { update: paint };
}
