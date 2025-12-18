mod common;

use native_messaging::install::manifest::{install, remove, verify_installed};
use native_messaging::install::paths;
use native_messaging::Scope;

use std::path::PathBuf;

fn dummy_exe_path() -> PathBuf {
    // On Unix, manifest path must be absolute. We'll use a stable absolute placeholder.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        PathBuf::from("/usr/bin/true")
    }

    // On Windows, it can be anything; still use an absolute-looking path.
    #[cfg(windows)]
    {
        PathBuf::from(r"C:\Windows\System32\cmd.exe")
    }
}

#[test]
fn install_then_remove_user_scope_selected_browsers() {
    let (_td, _env) = common::sandbox_env();

    let host = "com.example.installremove";
    let description = "test host";
    let exe = dummy_exe_path();

    let allowed_origins = vec!["chrome-extension://test/".to_string()];
    let allowed_extensions = vec!["test@example.org".to_string()];

    // Keep this set small + representative.
    let browsers = &["chrome", "firefox", "edge"];

    // Install
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

    // File existence check for each browser we installed (on this OS).
    for b in browsers {
        if let Ok(p) = paths::manifest_path(b, Scope::User, host) {
            assert!(p.exists(), "manifest should exist for {b}: {p:?}");
            let raw = std::fs::read_to_string(&p).unwrap();
            let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
            assert_eq!(v.get("name").and_then(|x| x.as_str()), Some(host));
            assert_eq!(v.get("type").and_then(|x| x.as_str()), Some("stdio"));
        }
    }

    // verify_installed: should return true
    assert!(verify_installed(host, Some(browsers), Scope::User).unwrap());

    // Remove
    remove(host, browsers, Scope::User).unwrap();

    // verify_installed: should return false after removal
    assert!(!verify_installed(host, Some(browsers), Scope::User).unwrap());

    // Files should be gone (for those with configured paths on this OS)
    for b in browsers {
        if let Ok(p) = paths::manifest_path(b, Scope::User, host) {
            assert!(!p.exists(), "manifest should be removed for {b}: {p:?}");
        }
    }
}
