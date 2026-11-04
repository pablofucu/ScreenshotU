//! Arranque con Windows en segundo plano: una entrada en `HKCU\...\Run` que abre el visor con `--background`
//! (icono en la bandeja, sin ventana) para que la tablet conecte aunque no se haya abierto el visor.
use std::os::windows::process::CommandExt;
use std::process::Command;

const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "VisorCapturas";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// El visor se ha abierto con `--background` (arranque de Windows).
pub fn is_background() -> bool {
    std::env::args().any(|a| a == "--background")
}

/// Pone o quita la entrada de arranque. Al activarla se reescribe siempre, para que apunte al ejecutable actual
/// aunque el visor se haya movido o actualizado.
pub fn sync(on: bool) {
    if on {
        let Ok(exe) = std::env::current_exe() else { return };
        // `/d "\"C:\ruta\visor.exe\" --background"`: el valor lleva sus propias comillas.
        let r = Command::new("reg")
            .arg("add")
            .arg(KEY)
            .args(["/v", NAME, "/t", "REG_SZ", "/f"])
            .raw_arg(format!("/d \"\\\"{}\\\" --background\"", exe.display()))
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        if let Err(e) = r {
            eprintln!("arranque con Windows: {e}");
        }
    } else {
        // Borrar una entrada que no existe da error: no importa.
        let _ = Command::new("reg").args(["delete", KEY, "/v", NAME, "/f"]).creation_flags(CREATE_NO_WINDOW).output();
    }
}
