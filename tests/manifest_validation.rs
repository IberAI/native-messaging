#![cfg(feature = "install")]

mod common;

use native_messaging::{install, Scope};
use serial_test::serial;
use std::{io, path::PathBuf};

fn dummy_exe_path() -> PathBuf {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        PathBuf::from("/usr/bin/true")
    }

    #[cfg(windows)]
    {
        PathBuf::from(r"C:\Windows\System32\cmd.exe")
    }
}

#[test]
#[serial]
fn chromium_install_rejects_invalid_host_name() {
    let (_td, _env) = common::sandbox_env();
    let err = install(
        "Com.Example.Host",
        "test host",
        &dummy_exe_path(),
        &["chrome-extension://test/".to_string()],
        &["test@example.org".to_string()],
        &["chrome"],
        Scope::User,
    )
    .expect_err("uppercase host names are invalid for Chromium-family browsers");

    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
}

#[test]
#[serial]
fn chromium_install_rejects_bad_origin_shape() {
    let (_td, _env) = common::sandbox_env();
    let err = install(
        "com.example.host",
        "test host",
        &dummy_exe_path(),
        &["moz-extension://test/".to_string()],
        &["test@example.org".to_string()],
        &["edge"],
        Scope::User,
    )
    .expect_err("Chromium-family browsers require chrome-extension:// origins");

    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
}

#[test]
#[serial]
fn chromium_install_accepts_origin_without_trailing_slash() {
    let (_td, _env) = common::sandbox_env();

    install(
        "com.example.host",
        "test host",
        &dummy_exe_path(),
        &["chrome-extension://test".to_string()],
        &["test@example.org".to_string()],
        &["edge"],
        Scope::User,
    )
    .expect("Edge documentation shows chrome-extension origins without a trailing slash");
}

#[test]
#[serial]
fn firefox_install_allows_uppercase_host_name() {
    let (_td, _env) = common::sandbox_env();

    install(
        "Com.Example_Host",
        "test host",
        &dummy_exe_path(),
        &["chrome-extension://test/".to_string()],
        &["test@example.org".to_string()],
        &["firefox"],
        Scope::User,
    )
    .expect("Firefox-family native messaging names allow uppercase ASCII letters");
}

#[test]
#[serial]
fn firefox_install_rejects_empty_extension_allowlist() {
    let (_td, _env) = common::sandbox_env();
    let err = install(
        "com.example.host",
        "test host",
        &dummy_exe_path(),
        &["chrome-extension://test/".to_string()],
        &[],
        &["firefox"],
        Scope::User,
    )
    .expect_err("Firefox-family browsers need allowed_extensions");

    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
}
