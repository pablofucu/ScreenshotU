// Sustituto de `@tauri-apps/plugin-updater`: la tablet no se actualiza sola (se instala el APK nuevo).
export async function check(_options?: unknown): Promise<null> { return null; }
export type Update = never;
