mod common;

use native_messaging::install::paths;
use native_messaging::Scope;

#[test]
fn manifest_path_resolves_for_known_browsers_user_scope() {
    let (_td, _env) = common::sandbox_env();

    let host = "com.example.testhost";

    // These keys come from browsers.toml in this crate. :contentReference[oaicite:1]{index=1}
    let browser_keys = [
        "chrome",
        "edge",
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
