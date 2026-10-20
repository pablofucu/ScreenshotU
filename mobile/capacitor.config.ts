import type { CapacitorConfig } from '@capacitor/cli';

// La app habla por HTTP sin cifrar con el visor del PC (red local): esquema http y tráfico en claro.
const config: CapacitorConfig = {
  appId: 'app.visorcapturas.movil',
  appName: 'Visor',
  webDir: 'dist',
  backgroundColor: '#eceef2',
  server: { androidScheme: 'http', cleartext: true },
  android: { allowMixedContent: true },
  plugins: {
    SystemBars: { insetsHandling: 'css', style: 'LIGHT', initialViewportFitValueHint: 'cover' },
  },
};

export default config;
