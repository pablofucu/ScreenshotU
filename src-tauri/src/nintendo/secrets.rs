//! Secretos de Nintendo en el Administrador de credenciales de Windows (`CRED_TYPE_GENERIC`,
//! persistencia `LOCAL_MACHINE`). Nunca van a `visor.db`, `config.json` ni a las copias de seguridad.
//! Al restaurar una copia en otro PC hay que volver a conectar la cuenta, y es lo correcto.

use anyhow::{anyhow, Result};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{ERROR_NOT_FOUND, FILETIME};
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

/// Que secreto se guarda. Cada uno tiene su propia entrada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    /// Session token de la cuenta del historial de juego.
    History,
    /// Session token del Control Parental (Moon), si se conecta.
    #[allow(dead_code)] // lo usara el Control Parental (SW-19)
    Parental,
}

impl Secret {
    pub fn target(self) -> &'static str {
        match self {
            Secret::History => "VisorCapturas/nintendo/history",
            Secret::Parental => "VisorCapturas/nintendo/parental",
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Guarda (o sustituye) el secreto. El valor se guarda como UTF-8.
pub fn set(which: Secret, value: &str) -> Result<()> {
    let mut target = wide(which.target());
    let mut user = wide("nintendo");
    let mut blob = value.as_bytes().to_vec();
    let cred = CREDENTIALW {
        Flags: CRED_FLAGS(0),
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target.as_mut_ptr()),
        Comment: PWSTR::null(),
        LastWritten: FILETIME::default(),
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: PWSTR::null(),
        UserName: PWSTR(user.as_mut_ptr()),
    };
    // SAFETY: `cred` y los buffers a los que apunta viven hasta que vuelve la llamada.
    unsafe { CredWriteW(&cred, 0) }.map_err(|e| anyhow!("no se pudo guardar el secreto en el Administrador de credenciales: {e}"))
}

/// Lee el secreto; `None` si no existe.
pub fn get(which: Secret) -> Result<Option<String>> {
    let target = wide(which.target());
    let mut out: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: `target` termina en 0; `out` recibe un bloque que se libera con `CredFree`.
    let res = unsafe { CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None, &mut out) };
    match res {
        Ok(()) => {
            // SAFETY: tras un Ok, `out` apunta a una CREDENTIALW valida con su blob.
            let value = unsafe {
                let c = &*out;
                let bytes = if c.CredentialBlob.is_null() || c.CredentialBlobSize == 0 {
                    &[][..]
                } else {
                    std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize)
                };
                let v = String::from_utf8_lossy(bytes).into_owned();
                CredFree(out as *const _);
                v
            };
            Ok(Some(value))
        }
        Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(None),
        Err(e) => Err(anyhow!("no se pudo leer el secreto del Administrador de credenciales: {e}")),
    }
}

/// Borra el secreto. Si no existia no es un error.
pub fn delete(which: Secret) -> Result<()> {
    let target = wide(which.target());
    // SAFETY: `target` termina en 0.
    match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
        Err(e) => Err(anyhow!("no se pudo borrar el secreto del Administrador de credenciales: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Usa una entrada de prueba (no toca las reales) y la limpia al acabar.
    #[test]
    fn roundtrip_set_get_delete() {
        // Se prueba con `Parental` solo si no hay un secreto real guardado, para no pisarlo.
        if get(Secret::Parental).unwrap().is_some() {
            return;
        }
        assert_eq!(get(Secret::Parental).unwrap(), None);
        set(Secret::Parental, "token-de-prueba-ñ").unwrap();
        assert_eq!(get(Secret::Parental).unwrap().as_deref(), Some("token-de-prueba-ñ"));
        set(Secret::Parental, "otro").unwrap();
        assert_eq!(get(Secret::Parental).unwrap().as_deref(), Some("otro"));
        delete(Secret::Parental).unwrap();
        assert_eq!(get(Secret::Parental).unwrap(), None);
        delete(Secret::Parental).unwrap(); // borrar dos veces no falla
    }
}
