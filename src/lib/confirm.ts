import { blip } from './sound';

/**
 * Confirmación en dos pulsaciones («Pulsa A otra vez…»), la única máquina de estados
 * para las acciones destructivas de los paneles:
 *
 *   desarmado --A--> armado --A--> (acción)
 *       ^              |
 *       +-- mover / B / CONFIRM_MS sin pulsar --+
 *
 * El estado vive en el campo `confirm` del panel (las vistas lo leen para cambiar el texto).
 * Solo hay un armado a la vez: armar otro panel cancela la caducidad del anterior.
 */
export const CONFIRM_MS = 3500;

let timer: ReturnType<typeof setTimeout> | undefined;

/** Primera pulsación: arma y devuelve false. Segunda (aún armado): devuelve true y la acción sigue. */
export function confirmTwice(s: { confirm: boolean }): boolean {
  // Armado: la acción sigue. La caducidad no se cancela, así un panel que siga abierto
  // (p. ej. tras un error) no se queda armado para siempre.
  if (s.confirm) return true;
  s.confirm = true;
  blip(520);
  clearTimeout(timer);
  timer = setTimeout(() => { s.confirm = false; }, CONFIRM_MS);
  return false;
}
