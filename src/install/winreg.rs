//! Windows registry helpers for native messaging host manifests.
//!
//! Chromium-family browsers and Firefox locate native messaging manifests on
//! Windows through registry keys whose default value points at a manifest JSON
//! file. These helpers are used by the high-level installer when the
//! `windows-registry` feature is enabled on Windows.

use std::{
    io,
    path::{Path, PathBuf},
};
use winreg::{enums::*, RegKey};

use crate::install::paths::Scope;

/// Read the manifest path stored in a registry key's default value.
///
/// `key_path` is relative to either `HKEY_CURRENT_USER` or
/// `HKEY_LOCAL_MACHINE`, selected by `scope`.
///
/// # Errors
///
/// Returns registry I/O errors except missing keys, which are reported as
/// `Ok(None)`.
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

/// Create or update a registry key and set its default value to a manifest path.
///
/// `key_path` is relative to either `HKEY_CURRENT_USER` or
/// `HKEY_LOCAL_MACHINE`, selected by `scope`.
///
/// # Errors
///
/// Returns an error if the key cannot be created or the default value cannot be
/// written.
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

/// Remove a registry key.
///
/// Missing keys are treated as success.
///
/// # Errors
///
/// Returns an error if the key exists but cannot be removed.
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
