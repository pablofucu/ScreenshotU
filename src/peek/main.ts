// Miniatura de la captura recien hecha (ver src-tauri/src/peek.rs). La app la cambia con `window.__peek(dataUrl)`.
const f = document.getElementById('f')!;
const img = document.getElementById('img') as HTMLImageElement;

function set(src: string) {
  img.src = src;
  // Reinicia la animacion aunque llegue otra captura mientras se ve la anterior.
  f.classList.remove('on');
  void f.offsetWidth;
  f.classList.add('on');
}

(window as unknown as { __peek: typeof set }).__peek = set;
const first = location.hash.slice(1);
if (first) set(first);
