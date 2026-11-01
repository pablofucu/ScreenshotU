// Sustituto de `@tauri-apps/plugin-dialog`: en la tablet no hay diálogos de archivos del PC.
export async function open(_options?: unknown): Promise<null> { return null; }
export async function save(_options?: unknown): Promise<null> { return null; }
export async function message(_msg: string, _options?: unknown): Promise<void> { /* nada */ }
export async function ask(_msg: string, _options?: unknown): Promise<boolean> { return false; }
export async function confirm(_msg: string, _options?: unknown): Promise<boolean> { return false; }
