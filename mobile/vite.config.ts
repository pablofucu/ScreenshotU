import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

// `@shared` son los módulos puros de la app de escritorio (tipos, estadísticas, metas, formatos, arte):
// el móvil calcula todo exactamente igual que el visor.
const shared = fileURLToPath(new URL('../src/lib', import.meta.url));
const fonts = fileURLToPath(new URL('../src/assets', import.meta.url));
// `@desk` es el código de la interfaz de escritorio entero: en la tablet se monta tal cual (modo tablet de src/main.ts).
const desk = fileURLToPath(new URL('../src', import.meta.url));
// Dentro del APK no hay Tauri: los paquetes `@tauri-apps/*` que usa la interfaz de escritorio se sustituyen por los
// equivalentes de `src/lib/remote/` (HTTP contra el PC, sin conexión con caché).
const remote = (f: string) => fileURLToPath(new URL(`../src/lib/remote/${f}`, import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: [
      { find: '@tauri-apps/api/core', replacement: remote('core.ts') },
      { find: '@tauri-apps/api/event', replacement: remote('event.ts') },
      { find: '@tauri-apps/api/window', replacement: remote('window.ts') },
      { find: '@tauri-apps/api/webview', replacement: remote('webview.ts') },
      { find: '@tauri-apps/plugin-dialog', replacement: remote('dialog.ts') },
      { find: '@tauri-apps/plugin-updater', replacement: remote('updater.ts') },
      { find: '@shared', replacement: shared },
      { find: '@assets', replacement: fonts },
      { find: '@desk', replacement: desk },
    ],
    // Una sola copia de Svelte aunque el código de ../src la pida desde otra carpeta.
    dedupe: ['svelte'],
  },
  clearScreen: false,
  server: { port: 1430, strictPort: true, host: '0.0.0.0', fs: { allow: ['..'] } },
  build: { target: 'es2022', chunkSizeWarningLimit: 1000 },
});
