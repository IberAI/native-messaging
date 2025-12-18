#![cfg(windows)]

mod common;

use native_messaging::install::manifest::{install, remove, verify_installed};
use native_messaging::install::paths;
use native_messaging::Scope;

use std::path::PathBuf;

fn exe_path() -> PathBuf {
    PathBuf::from(r"C:\Windows\System32\cmd.exe")
}

#[test]
fn verify_is_registry_aware_on_windows() {
    let (_td, _env) = common::sandbox_env();

    let host = "com.example.winregverify";
    let description = "test host";
    let exe = exe_path();

    let allowed_origins = vec!["chrome-extension://test/".to_string()];
    let allowed_extensions = vec!["test@example.org".to_string()];
    let browsers = &["chrome", "firefox", "edge"];

    install(
        host,
        description,
        &exe,
        &allowed_origins,
        &allowed_extensions,
        browsers,
        Scope::User,
    )
    .unwrap();

    // Registry-aware verify should pass.
    assert!(verify_installed(host, Some(browsers), Scope::User).unwrap());

    // Additionally, check the registry pointer resolves to a real file for at least one browser.
    // (This depends on your winreg.rs functions; paths::winreg_key_path must be present.)
    let key_path = paths::winreg_key_path("chrome", Scope::User, host).unwrap();
    let p = native_messaging::install::winreg::read_manifest_path_from_reg(Scope::User, &key_path)
        .unwrap()
        .expect("registry key should exist");
    assert!(
        p.exists(),
        "registry should point to existing manifest: {p:?}"
    );

    remove(host, browsers, Scope::User).unwrap();
    assert!(!verify_installed(host, Some(browsers), Scope::User).unwrap());
}
