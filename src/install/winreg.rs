use crate::install::paths::chrome_user_manifest;
use std::{io, path::PathBuf};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

/// Write the Chrome native-messaging registry value under HKCU so Chrome
/// can find the manifest file at the given host `name`.
pub fn write_chrome_manifest_reg(name: &str) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_path = format!(r"Software\Google\Chrome\NativeMessagingHosts\{name}");
    let (key, _) = hkcu.create_subkey(&key_path)?;
    let manifest_path: PathBuf = chrome_user_manifest(name);
    key.set_value("", &manifest_path.to_string_lossy().as_ref())?;
    Ok(())
}

/// Remove the HKCU registry value for the Chrome native-messaging host.
pub fn remove_chrome_manifest_reg(name: &str) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_path = format!(r"Software\Google\Chrome\NativeMessagingHosts\{name}");
    hkcu.delete_subkey(&key_path).ok();
    Ok(())
}
