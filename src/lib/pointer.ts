// Ayudas del ratón compartidas por las pantallas.

/**
 * Devuelve una función `moved(e)` que dice si el ratón se ha movido de verdad desde la última vez que se llamó.
 * Al abrir un menú o desplazar una lista el navegador lanza `mouseenter` bajo un cursor quieto; sin esta
 * comprobación el ratón robaría el foco a las flechas / al mando. Cada pantalla crea la suya.
 */
export function makeMoved() {
  let last = '';
  return (e: MouseEvent): boolean => {
    const xy = e.clientX + ',' + e.clientY;
    if (xy === last) return false;
    last = xy;
    return true;
  };
}
