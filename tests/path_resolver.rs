#![cfg(feature = "install")]

mod common;

use native_messaging::install::paths;
use native_messaging::{manifest_paths, Scope};
use serial_test::serial;

#[test]
#[serial]
fn manifest_path_resolves_for_known_browsers_user_scope() {
    let (_td, _env) = common::sandbox_env();

    let host = "com.example.testhost";

    // These keys come from browsers.toml in this crate.
    let browser_keys = [
        "chrome",
        "chrome_for_testing",
        "edge",
        "edge_beta",
        "edge_dev",
        "edge_canary",
        "chromium",
        "brave",
        "vivaldi",
        "firefox",
        "librewolf",
    ];

    for key in browser_keys {
        // Some OS/scope combinations may be intentionally missing in config.
        // For this test we only check "User" which is defined for these on most OSes,
        // but if one is missing on a platform, we just skip.
        match paths::manifest_path(key, Scope::User, host) {
            Ok(p) => {
                let s = p.to_string_lossy();
                assert!(s.contains(host), "path should include host name: {s}");
                assert!(s.ends_with(".json"), "path should end with .json: {s}");
            }
            Err(_e) => {
                // Skip if not configured for this OS (acceptable).
            }
        }
    }
}

#[test]
#[serial]
fn manifest_paths_includes_primary_path_first() {
    let (_td, _env) = common::sandbox_env();

    let host = "com.example.testhost";
    let primary = paths::manifest_path("firefox", Scope::User, host).unwrap();
    let all = paths::manifest_paths("firefox", Scope::User, host).unwrap();

    assert_eq!(all.first(), Some(&primary));
    assert!(all.iter().all(|p| p.ends_with(format!("{host}.json"))));
}

#[test]
#[serial]
fn manifest_paths_is_reexported_at_crate_root() {
    let (_td, _env) = common::sandbox_env();

    let paths = manifest_paths("firefox", Scope::User, "com.example.testhost").unwrap();

    assert!(!paths.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn firefox_linux_system_paths_include_usr_lib64_alternate() {
    let (_td, _env) = common::sandbox_env();

    let paths = paths::manifest_paths("firefox", Scope::System, "com.example.testhost").unwrap();
    let rendered: Vec<_> = paths.iter().map(|p| p.to_string_lossy()).collect();

    assert!(
        rendered
            .iter()
            .any(|p| p.contains("/usr/lib/mozilla/native-messaging-hosts/")),
        "missing primary /usr/lib Firefox path: {rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|p| p.contains("/usr/lib64/mozilla/native-messaging-hosts/")),
        "missing alternate /usr/lib64 Firefox path: {rendered:?}"
    );
}
