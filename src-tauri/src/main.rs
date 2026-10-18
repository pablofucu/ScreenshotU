// Evita la ventana de consola extra en Windows en release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Modo auxiliar elevado para reiniciar Magpie: no abre la interfaz.
    if let Some(code) = visor_capturas_lib::helper_main() {
        std::process::exit(code);
    }
    visor_capturas_lib::run()
}
