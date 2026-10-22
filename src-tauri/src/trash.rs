//! Mover archivos a la Papelera de Windows (nunca borrado definitivo).

use std::path::Path;

/// Manda un archivo a la Papelera. Sin confirmacion ni ventanas de error: la interfaz ya pidio confirmar.
#[cfg(windows)]
pub fn to_recycle_bin(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};

    // `pFrom` es una lista de rutas separadas por NUL y terminada con un NUL doble. SHFileOperation no entiende '/'.
    let mut from: Vec<u16> = path.as_os_str().encode_wide().map(|c| if c == '/' as u16 { '\\' as u16 } else { c }).collect();
    from.extend([0, 0]);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16,
        ..Default::default()
    };
    // SAFETY: `op` y `from` viven hasta que vuelve la llamada y `from` acaba en doble NUL.
    let code = unsafe { SHFileOperationW(&mut op) };
    if code != 0 {
        return Err(format!("Windows no pudo mandarla a la Papelera (código {code:#x})"));
    }
    if op.fAnyOperationsAborted.as_bool() {
        return Err("Operación cancelada".into());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn to_recycle_bin(_path: &Path) -> Result<(), String> {
    Err("La Papelera solo está disponible en Windows".into())
}
