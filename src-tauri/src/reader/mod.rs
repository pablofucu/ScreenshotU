//! Pagina de lectura: servidor HTTP + WebSocket en 127.0.0.1:6677, navegador en modo app y monitores.
//! (Agente C.) Ver `docs/novelas-japonesas.md`.
//!
//! - `server`: `start`, `url`, `broadcast`, `set_session`, `send_prefs`, `bye`, `on_page_prefs`.
//! - `browser`: `open` (Edge/Chrome en `--app` con perfil propio), `browser_path`.
//! - `monitor`: `reading_monitor` (monitor a la izquierda del juego).
#![allow(dead_code)]
pub mod browser;
pub mod monitor;
pub mod server;

use visor_core::jp::ReaderLine;

/// Envia una linea a todas las paginas de lectura conectadas.
pub fn broadcast(line: &ReaderLine) {
    server::broadcast(line);
}
