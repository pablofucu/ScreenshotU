import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Tauri espera un puerto fijo en desarrollo.
// Paginas: la app (index), la ventana de texthook (hook), la pagina de lectura (reader) y los avisos sobre el juego
// (rec: indicador de grabacion; peek: miniatura de la ultima captura).
export default defineConfig({
  plugins: [
    svelte({
      inspector: {
        toggleKeyCombo: 'control-alt',
        holdMode: true,
        showToggleButton: 'always',
        toggleButtonPos: 'bottom-right',
      },
    }),
  ],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: '127.0.0.1' },
  build: {
    target: 'es2022',
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      input: {
        index: 'index.html',
        hook: 'hook.html',
        reader: 'reader.html',
        rec: 'rec.html',
        peek: 'peek.html',
      },
    },
  },
});
