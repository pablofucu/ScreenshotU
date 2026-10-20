import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

// `@shared` son los módulos puros de la app de escritorio (tipos, estadísticas, metas, formatos, arte):
// el móvil calcula todo exactamente igual que el visor.
const shared = fileURLToPath(new URL('../src/lib', import.meta.url));
const fonts = fileURLToPath(new URL('../src/assets', import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  resolve: { alias: { '@shared': shared, '@assets': fonts } },
  clearScreen: false,
  server: { port: 1430, strictPort: true, host: '0.0.0.0', fs: { allow: ['..'] } },
  build: { target: 'es2022', chunkSizeWarningLimit: 1000 },
});
