use native_messaging::install::manifest::{install, remove, verify, Browser, Scope};
use native_messaging::install::paths::{chrome_user_manifest, firefox_user_manifest};
use serial_test::serial;
use std::{env, fs, path::Path};

#[test]
#[serial]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn install_and_remove_user_scope_writes_correct_files() {
    // Spoof HOME so we never touch the real profile
    let td = tempfile::tempdir().unwrap();
    env::set_var("HOME", td.path());

    // Create a dummy "executable" with an absolute path
    let host_exe = td.path().join("host_bin");
    fs::write(&host_exe, b"#!/bin/sh\nexit 0\n").unwrap();

    let name = "com.example.native_echo";
    let desc = "Example host";
    let chrome_origin = "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/".to_string();
    let firefox_id = "native-test@example.com".to_string();

    // Install both manifests at user scope
    install(
        name,
        desc,
        Path::new(&host_exe),
        &[chrome_origin.clone()],
        &[firefox_id.clone()],
        &[Browser::Chrome, Browser::Firefox],
        Scope::User,
    )
    .expect("install");

    // Verify says present
    assert!(verify(name).expect("verify"));

    // Files exist & contain the expected keys
    let chrome_path = chrome_user_manifest(name);
    let firefox_path = firefox_user_manifest(name);

    let chrome_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&chrome_path).unwrap()).unwrap();
    let firefox_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&firefox_path).unwrap()).unwrap();

    // Chrome manifest must have allowed_origins and not allowed_extensions
    assert!(chrome_json.get("allowed_origins").is_some());
    assert!(chrome_json.get("allowed_extensions").is_none());
    assert_eq!(chrome_json["name"], name);
    assert_eq!(chrome_json["description"], desc);
    assert_eq!(chrome_json["type"], "stdio");

    // Firefox manifest must have allowed_extensions and not allowed_origins
    assert!(firefox_json.get("allowed_extensions").is_some());
    assert!(firefox_json.get("allowed_origins").is_none());
    assert_eq!(firefox_json["name"], name);
    assert_eq!(firefox_json["description"], desc);
    assert_eq!(firefox_json["type"], "stdio");

    // Remove & verify false
    remove(name, &[Browser::Chrome, Browser::Firefox], Scope::User).expect("remove");
    assert!(!verify(name).expect("verify after remove"));
}

#[test]
#[serial]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn install_rejects_relative_exe_path_on_unix() {
    let td = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", td.path());

    // Relative path should fail on Linux/macOS
    let rel = Path::new("relative/path/to/host");
    let err = install(
        "com.example.bad",
        "desc",
        rel,
        &["chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/".into()],
        &["native-test@example.com".into()],
        &[Browser::Chrome, Browser::Firefox],
        Scope::User,
    )
    .expect_err("relative exe_path must be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}
