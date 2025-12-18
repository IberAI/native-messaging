use std::{
    io,
    path::{Path, PathBuf},
};
use winreg::{enums::*, RegKey};

use crate::install::paths::Scope;

pub fn read_manifest_path_from_reg(scope: Scope, key_path: &str) -> io::Result<Option<PathBuf>> {
    let root = match scope {
        Scope::User => RegKey::predef(HKEY_CURRENT_USER),
        Scope::System => RegKey::predef(HKEY_LOCAL_MACHINE),
    };

    let key = match root.open_subkey(key_path) {
        Ok(k) => k,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };

    let s: String = key.get_value("")?;
    Ok(Some(PathBuf::from(s)))
}

/// Create/update the registry key and set its (Default) value to the manifest JSON path.
pub fn write_manifest_path_to_reg(
    scope: Scope,
    key_path: &str,
    manifest_path: &Path,
) -> io::Result<()> {
    let root = match scope {
        Scope::User => RegKey::predef(HKEY_CURRENT_USER),
        Scope::System => RegKey::predef(HKEY_LOCAL_MACHINE),
    };

    let (key, _) = root.create_subkey(key_path)?;
    let s = manifest_path.to_string_lossy().to_string();
    key.set_value("", &s)?;
    Ok(())
}

/// Remove the registry key (best-effort if missing).
pub fn remove_manifest_reg(scope: Scope, key_path: &str) -> io::Result<()> {
    let root = match scope {
        Scope::User => RegKey::predef(HKEY_CURRENT_USER),
        Scope::System => RegKey::predef(HKEY_LOCAL_MACHINE),
    };

    match root.delete_subkey_all(key_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
