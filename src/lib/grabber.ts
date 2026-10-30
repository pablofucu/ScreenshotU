// Herramienta estilo react-grab para Svelte 5 (modo DEV)
// Permite copiar elementos, componentes y HTML directamente al portapapeles.

export function setupGrabber() {
  if (typeof window === 'undefined' || !import.meta.env.DEV) return;

  let active = false; // Persistent toggle mode
  let isAltHeld = false;
  let hoveredEl: HTMLElement | null = null;

  // Highlight overlay
  const overlay = document.createElement('div');
  overlay.id = '__svelte_grab_overlay';
  overlay.style.cssText = `
    position: fixed;
    pointer-events: none;
    z-index: 999999;
    border: 2px solid #38bdf8;
    background: rgba(56, 189, 248, 0.12);
    border-radius: 6px;
    box-shadow: 0 0 16px rgba(56, 189, 248, 0.4);
    transition: all 0.08s ease-out;
    display: none;
  `;
  document.body.appendChild(overlay);

  // Tooltip
  const tooltip = document.createElement('div');
  tooltip.id = '__svelte_grab_tooltip';
  tooltip.style.cssText = `
    position: fixed;
    pointer-events: none;
    z-index: 1000000;
    padding: 4px 10px;
    background: #0f172a;
    color: #f8fafc;
    border: 1px solid rgba(56, 189, 248, 0.5);
    border-radius: 6px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    font-weight: 600;
    line-height: 1.4;
    white-space: nowrap;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    display: none;
  `;
  document.body.appendChild(tooltip);

  // Toast
  const toast = document.createElement('div');
  toast.id = '__svelte_grab_toast';
  toast.style.cssText = `
    position: fixed;
    top: 24px;
    left: 50%;
    transform: translateX(-50%) translateY(-20px);
    z-index: 1000001;
    padding: 10px 20px;
    background: #0f172a;
    color: #38bdf8;
    border: 1px solid #38bdf8;
    border-radius: 30px;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 13px;
    font-weight: 700;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.6), 0 0 15px rgba(56, 189, 248, 0.3);
    opacity: 0;
    pointer-events: none;
    transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  `;
  document.body.appendChild(toast);

  // Bottom-left toggle button
  const btn = document.createElement('button');
  btn.id = '__svelte_grab_btn';
  btn.title = 'Activar modo Copiar Elemento (o mantén pulsado Alt + Click)';
  btn.innerHTML = `<span style="font-size:14px">📋</span> <span style="font-size:11px;letter-spacing:0.5px">Copiar elemento <kbd style="background:rgba(255,255,255,0.15);padding:1px 5px;border-radius:3px;font-size:10px">Alt</kbd></span>`;
  btn.style.cssText = `
    position: fixed;
    bottom: 12px;
    left: 12px;
    z-index: 999998;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: rgba(15, 23, 42, 0.85);
    backdrop-filter: blur(8px);
    color: #94a3b8;
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 20px;
    cursor: pointer;
    user-select: none;
    transition: all 0.15s ease;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  `;
  btn.onmouseenter = () => { if (!active) btn.style.color = '#f8fafc'; };
  btn.onmouseleave = () => { if (!active) btn.style.color = '#94a3b8'; };
  btn.onclick = () => {
    active = !active;
    updateBtnStyle();
    if (!active) hide();
  };
  document.body.appendChild(btn);

  function updateBtnStyle() {
    if (active) {
      btn.style.background = '#0284c7';
      btn.style.color = '#ffffff';
      btn.style.borderColor = '#38bdf8';
      btn.style.boxShadow = '0 0 15px rgba(56, 189, 248, 0.6)';
    } else {
      btn.style.background = 'rgba(15, 23, 42, 0.85)';
      btn.style.color = '#94a3b8';
      btn.style.borderColor = 'rgba(148, 163, 184, 0.2)';
      btn.style.boxShadow = '0 4px 12px rgba(0, 0, 0, 0.3)';
    }
  }

  function isGrabMode() {
    return active || isAltHeld;
  }

  function findSvelteMeta(el: HTMLElement | null): { file?: string; line?: number; component?: string } | null {
    let curr: any = el;
    while (curr) {
      if (curr.__svelte_meta) {
        const meta = curr.__svelte_meta;
        let comp = '';
        let p = meta;
        while (p) {
          if (p.componentTag) { comp = p.componentTag; break; }
          p = p.parent;
        }
        return {
          file: meta.loc?.file,
          line: meta.loc?.line,
          component: comp || (meta.loc?.file ? meta.loc.file.split(/[/\\]/).pop().replace(/\.svelte$/, '') : '')
        };
      }
      curr = curr.parentElement;
    }
    return null;
  }

  function getSelector(el: HTMLElement): string {
    const tag = el.tagName.toLowerCase();
    const id = el.id ? `#${el.id}` : '';
    const classes = Array.from(el.classList).filter(c => !c.startsWith('svelte-') && !c.startsWith('__svelte')).map(c => `.${c}`).join('');
    return `${tag}${id}${classes}`;
  }

  function show(el: HTMLElement) {
    hoveredEl = el;
    const r = el.getBoundingClientRect();
    overlay.style.display = 'block';
    overlay.style.top = `${r.top}px`;
    overlay.style.left = `${r.left}px`;
    overlay.style.width = `${r.width}px`;
    overlay.style.height = `${r.height}px`;

    const meta = findSvelteMeta(el);
    const sel = getSelector(el);
    const compLabel = meta?.component ? `<span style="color:#a78bfa">&lt;${meta.component}&gt;</span> ` : '';
    const fileLabel = meta?.file ? `<span style="color:#94a3b8;font-size:10px"> (${meta.file.split(/[/\\]/).pop()}:${meta.line ?? ''})</span>` : '';

    tooltip.innerHTML = `${compLabel}<span style="color:#38bdf8">${sel}</span>${fileLabel}`;
    tooltip.style.display = 'block';

    const ttY = r.top > 32 ? r.top - 28 : r.bottom + 6;
    const ttX = Math.max(8, Math.min(window.innerWidth - 300, r.left));
    tooltip.style.top = `${ttY}px`;
    tooltip.style.left = `${ttX}px`;
  }

  function hide() {
    overlay.style.display = 'none';
    tooltip.style.display = 'none';
    hoveredEl = null;
  }

  function showToast(msg: string) {
    toast.innerHTML = msg;
    toast.style.opacity = '1';
    toast.style.transform = 'translateX(-50%) translateY(0)';
    setTimeout(() => {
      toast.style.opacity = '0';
      toast.style.transform = 'translateX(-50%) translateY(-20px)';
    }, 2400);
  }

  function copyElement(el: HTMLElement) {
    const meta = findSvelteMeta(el);
    const sel = getSelector(el);
    
    // Clean HTML snippet
    const clone = el.cloneNode(true) as HTMLElement;
    const all = clone.querySelectorAll('*');
    for (const item of [clone, ...Array.from(all)]) {
      for (const attr of Array.from(item.attributes)) {
        if (attr.name.startsWith('data-svelte') || attr.name.startsWith('__svelte')) {
          item.removeAttribute(attr.name);
        }
      }
    }
    let html = clone.outerHTML;
    if (html.length > 2500) {
      html = html.slice(0, 2500) + '\n... (truncado por longitud)';
    }

    const payload = [
      `[Elemento copiado con Svelte-Grab]`,
      meta?.component ? `Componente: <${meta.component}>` : null,
      meta?.file ? `Archivo: ${meta.file}${meta.line ? `:${meta.line}` : ''}` : null,
      `Selector: ${sel}`,
      `HTML:\n\`\`\`html\n${html}\n\`\`\``
    ].filter(Boolean).join('\n');

    navigator.clipboard.writeText(payload).then(() => {
      showToast(`✨ ¡Copiado al portapapeles! 📋 Listo para pegar en el chat`);
    }).catch(() => {
      showToast(`⚠️ No se pudo copiar automáticamente. Revisa permisos.`);
    });
  }

  // Event Listeners
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Alt') {
      isAltHeld = true;
      if (hoveredEl) show(hoveredEl);
    }
  });

  window.addEventListener('keyup', (e) => {
    if (e.key === 'Alt') {
      isAltHeld = false;
      if (!active) hide();
    }
  });

  window.addEventListener('mousemove', (e) => {
    if (!isGrabMode()) {
      if (hoveredEl) hide();
      return;
    }
    const target = e.target as HTMLElement;
    if (!target || target === overlay || target === tooltip || target === btn || target.id?.startsWith('__svelte_grab')) return;
    show(target);
  });

  window.addEventListener('click', (e) => {
    if (!isGrabMode()) return;
    const target = e.target as HTMLElement;
    if (!target || target === btn || target.id?.startsWith('__svelte_grab')) return;

    e.preventDefault();
    e.stopPropagation();
    copyElement(target);

    if (active) {
      active = false;
      updateBtnStyle();
      hide();
    }
  }, true);
}
