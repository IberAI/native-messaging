use native_messaging::install::paths::*;
use std::env;

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn user_paths_use_home_env() {
    let td = tempfile::tempdir().unwrap();
    env::set_var("HOME", td.path());
    let name = "com.example.test";

    let chrome_user = chrome_user_manifest(name);
    let firefox_user = firefox_user_manifest(name);

    // Both should live under our temp HOME
    assert!(chrome_user.starts_with(td.path()));
    assert!(firefox_user.starts_with(td.path()));

    // Both end with the same filename
    assert!(chrome_user
        .to_string_lossy()
        .ends_with("com.example.test.json"));
    assert!(firefox_user
        .to_string_lossy()
        .ends_with("com.example.test.json"));
}

#[test]
#[cfg(target_os = "windows")]
fn windows_user_paths_resolve() {
    // We can't control actual APPDATA/LOCALAPPDATA here, but we can at least
    // assert the filename and parent folder names.
    let name = "com.example.test";
    let chrome_user = chrome_user_manifest(name).to_string_lossy().to_string();
    let firefox_user = firefox_user_manifest(name).to_string_lossy().to_string();

    assert!(chrome_user.ends_with(r"NativeMessagingHosts\com.example.test.json"));
    assert!(firefox_user.ends_with(r"Mozilla\NativeMessagingHosts\com.example.test.json"));
}

#[test]
fn system_paths_have_expected_suffix() {
    let name = "com.example.test";
    let chrome_sys = chrome_system_manifest(name).to_string_lossy().to_string();
    let firefox_sys = firefox_system_manifest(name).to_string_lossy().to_string();

    assert!(chrome_sys.ends_with("com.example.test.json"));
    assert!(firefox_sys.ends_with("com.example.test.json"));
}
