#![cfg(windows)]
use native_messaging::install::manifest::{install, remove, verify, Browser, Scope};
use native_messaging::install::paths::chrome_user_manifest;
use serial_test::serial;
use std::{fs, path::Path};

#[test]
#[serial]
fn install_succeeds_and_writes_chrome_manifest_windows() {
    // We won't mutate APPDATA/LOCALAPPDATA envs here—just use defaults.
    let td = tempfile::tempdir().unwrap();
    let host_exe = td.path().join("host.exe");
    fs::write(&host_exe, b"not really an exe").unwrap();

    let name = "com.example.win_host";
    let desc = "Example host";
    let chrome_origin = "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/".to_string();

    install(
        name,
        desc,
        Path::new(&host_exe),
        &[chrome_origin],
        &["native-test@example.com".to_string()],
        &[Browser::Chrome],
        Scope::User,
    )
    .expect("install");

    assert!(verify(name).expect("verify"));

    // manifest file should exist (registry also written by library)
    let chrome_path = chrome_user_manifest(name);
    assert!(chrome_path.exists());

    remove(name, &[Browser::Chrome], Scope::User).expect("remove");
}
