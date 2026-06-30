//! Native messaging host manifest writer, verifier, and remover.
//!
//! Browser native messaging requires a small JSON manifest that tells the
//! browser where the native host executable lives and which extensions may
//! connect to it. This module generates family-correct manifests for
//! Chromium-family browsers and Firefox-family browsers using the paths resolved
//! by [`crate::install::paths`].

use serde::Serialize;
use serde_json::Value;
use std::{fs, io, path::Path};

use crate::install::paths;

#[derive(Serialize)]
struct ChromiumHostManifest<'a> {
    name: &'a str,
    description: &'a str,
    path: &'a str,
    #[serde(rename = "type")]
    ty: &'a str, // "stdio"
    allowed_origins: Vec<String>, // chrome-extension://<id>/
}

#[derive(Serialize)]
struct FirefoxHostManifest<'a> {
    name: &'a str,
    description: &'a str,
    path: &'a str,
    #[serde(rename = "type")]
    ty: &'a str, // "stdio"
    allowed_extensions: Vec<String>, // Firefox add-on IDs
}

fn ensure_absolute_path(exe_path: &Path) -> io::Result<()> {
    // On macOS/Linux the manifest "path" MUST be absolute.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        if !exe_path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Manifest `path` must be absolute on macOS/Linux",
            ));
        }
    }

    #[cfg(windows)]
    {
        let _ = exe_path;
    }

    Ok(())
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn is_valid_host_name(host_name: &str, allow_uppercase: bool) -> bool {
    if host_name.is_empty() || host_name.starts_with('.') || host_name.ends_with('.') {
        return false;
    }

    let mut previous_dot = false;
    for b in host_name.bytes() {
        let valid_char = b.is_ascii_lowercase()
            || b.is_ascii_digit()
            || b == b'_'
            || b == b'.'
            || (allow_uppercase && b.is_ascii_uppercase());

        if !valid_char {
            return false;
        }
        if b == b'.' && previous_dot {
            return false;
        }
        previous_dot = b == b'.';
    }

    true
}

fn validate_chromium_origin(origin: &str) -> bool {
    let Some(extension_id) = origin.strip_prefix("chrome-extension://") else {
        return false;
    };
    let extension_id = extension_id.strip_suffix('/').unwrap_or(extension_id);

    !extension_id.is_empty() && !extension_id.contains('/')
}

fn validate_firefox_extension_id(extension_id: &str) -> bool {
    !extension_id.is_empty() && !extension_id.chars().any(char::is_whitespace)
}

fn validate_install_inputs(
    browser_key: &str,
    family: &str,
    host_name: &str,
    chrome_allowed_origins: &[String],
    firefox_allowed_extensions: &[String],
) -> io::Result<()> {
    match family {
        "chromium" => {
            if !is_valid_host_name(host_name, false) {
                return Err(invalid_input(format!(
                    "invalid host_name '{host_name}' for Chromium-family browser '{browser_key}': \
                     use lowercase ASCII letters, digits, underscores, and dots; do not start or \
                     end with a dot or use consecutive dots"
                )));
            }
            if chrome_allowed_origins.is_empty() {
                return Err(invalid_input(format!(
                    "chrome_allowed_origins must contain at least one chrome-extension:// origin \
                     for Chromium-family browser '{browser_key}'"
                )));
            }
            if let Some(origin) = chrome_allowed_origins
                .iter()
                .find(|origin| !validate_chromium_origin(origin))
            {
                return Err(invalid_input(format!(
                    "invalid Chromium allowed origin '{origin}' for browser '{browser_key}': \
                     expected chrome-extension://<extension-id> or chrome-extension://<extension-id>/"
                )));
            }
        }
        "firefox" => {
            if !is_valid_host_name(host_name, true) {
                return Err(invalid_input(format!(
                    "invalid host_name '{host_name}' for Firefox-family browser '{browser_key}': \
                     use ASCII letters, digits, underscores, and dots; do not start or end with \
                     a dot or use consecutive dots"
                )));
            }
            if firefox_allowed_extensions.is_empty() {
                return Err(invalid_input(format!(
                    "firefox_allowed_extensions must contain at least one extension ID for \
                     Firefox-family browser '{browser_key}'"
                )));
            }
            if let Some(extension_id) = firefox_allowed_extensions
                .iter()
                .find(|extension_id| !validate_firefox_extension_id(extension_id))
            {
                return Err(invalid_input(format!(
                    "invalid Firefox extension ID '{extension_id}' for browser '{browser_key}': \
                     extension IDs must be non-empty and must not contain whitespace"
                )));
            }
        }
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown browser family '{other}' for browser '{browser_key}'"),
            ));
        }
    }

    Ok(())
}

/// Install manifests for the given browser keys.
///
/// Browser keys are looked up in the embedded `browsers.toml` configuration, or
/// in the file pointed to by `NATIVE_MESSAGING_BROWSERS_CONFIG` when that
/// environment variable is set.
///
/// The allowlist parameter used depends on the browser family:
/// - `chrome_allowed_origins` is used for `family = "chromium"` browsers.
/// - `firefox_allowed_extensions` is used for `family = "firefox"` browsers.
///
/// The installer validates host names and allowlists before writing files:
/// - Chromium-family host names must use lowercase ASCII letters, digits,
///   underscores, and dots.
/// - Firefox-family host names may also use uppercase ASCII letters.
/// - Names may not start or end with a dot, and dots may not be consecutive.
///
/// On Linux and macOS, `exe_path` must be absolute because browsers require an
/// absolute manifest `path` value on those platforms.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use native_messaging::{install, Scope};
///
/// install(
///     "com.example.host",
///     "Example native messaging host",
///     Path::new("/absolute/path/to/host"),
///     &["chrome-extension://abcdefghijklmnopabcdefghijklmnop/".to_string()],
///     &["native-host@example.org".to_string()],
///     &["chrome", "firefox", "edge"],
///     Scope::User,
/// )?;
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// # Errors
///
/// Returns an error when a browser key is unknown, a host name or allowlist is
/// invalid for the selected browser family, the current OS/scope is not
/// configured, a path template references a missing environment variable, or the
/// manifest/registry cannot be written.
pub fn install(
    host_name: &str,
    description: &str,
    exe_path: &Path,
    chrome_allowed_origins: &[String],
    firefox_allowed_extensions: &[String],
    browsers: &[&str],
    scope: paths::Scope,
) -> io::Result<()> {
    ensure_absolute_path(exe_path)?;

    for browser_key in browsers {
        let cfg = paths::browser_cfg(browser_key)?;
        validate_install_inputs(
            browser_key,
            &cfg.family,
            host_name,
            chrome_allowed_origins,
            firefox_allowed_extensions,
        )?;
        let manifest_path = paths::manifest_path(browser_key, scope, host_name)?;

        if let Some(dir) = manifest_path.parent() {
            fs::create_dir_all(dir)?;
        }

        match cfg.family.as_str() {
            "chromium" => {
                let m = ChromiumHostManifest {
                    name: host_name,
                    description,
                    path: exe_path.to_str().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "exe_path is not valid UTF-8")
                    })?,
                    ty: "stdio",
                    allowed_origins: chrome_allowed_origins.to_vec(),
                };
                fs::write(&manifest_path, serde_json::to_vec_pretty(&m)?)?;
            }
            "firefox" => {
                let m = FirefoxHostManifest {
                    name: host_name,
                    description,
                    path: exe_path.to_str().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "exe_path is not valid UTF-8")
                    })?,
                    ty: "stdio",
                    allowed_extensions: firefox_allowed_extensions.to_vec(),
                };
                fs::write(&manifest_path, serde_json::to_vec_pretty(&m)?)?;
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown browser family '{other}' for browser '{browser_key}'"),
                ));
            }
        }

        // On Windows, write registry pointer if configured.
        #[cfg(all(windows, feature = "windows-registry"))]
        {
            if cfg.windows_registry {
                let key_path = paths::winreg_key_path(browser_key, scope, host_name)?;
                crate::install::winreg::write_manifest_path_to_reg(
                    scope,
                    &key_path,
                    &manifest_path,
                )?;
            }
        }
    }

    Ok(())
}

/// Remove manifests and registry keys for the given browser keys.
///
/// File removal is best-effort for missing manifest files: absent files are not
/// treated as errors. On non-Windows platforms this checks all configured
/// manifest lookup paths, including alternate paths such as Firefox's Linux
/// `/usr/lib64` location. On Windows, registry keys are removed when registry
/// support is configured for the selected browser key.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::{remove, Scope};
///
/// remove("com.example.host", &["chrome", "firefox"], Scope::User)?;
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// # Errors
///
/// Returns an error when a browser key is unknown, the current OS/scope is not
/// configured, a path template references a missing environment variable, or an
/// existing manifest file cannot be removed.
pub fn remove(host_name: &str, browsers: &[&str], scope: paths::Scope) -> io::Result<()> {
    for browser_key in browsers {
        // Remove file (best-effort if missing)
        for manifest_path in paths::manifest_paths(browser_key, scope, host_name)? {
            if manifest_path.exists() {
                fs::remove_file(&manifest_path)?;
            }
        }

        // Remove registry pointer if configured.
        #[cfg(all(windows, feature = "windows-registry"))]
        {
            let cfg = paths::browser_cfg(browser_key)?;
            if cfg.windows_registry {
                let key_path = paths::winreg_key_path(browser_key, scope, host_name)?;
                crate::install::winreg::remove_manifest_reg(scope, &key_path).ok();
            }
        }
    }
    Ok(())
}
/// Verify whether a host manifest is installed for one or more browsers.
///
/// If `browsers` is `None`, every configured browser key is checked. If a slice
/// is provided, only those browser keys are checked. The function returns
/// `Ok(true)` as soon as one valid manifest is found.
///
/// Verification checks that the manifest file exists, is valid JSON, has the
/// expected `name`, has `type = "stdio"`, contains a `path`, uses the correct
/// allowlist field for the browser family, and satisfies the same host-name and
/// allowlist validation rules used by [`install`].
///
/// On Windows, browser keys with `windows_registry = true` are verified through
/// the registry pointer. On non-Windows platforms, all configured lookup paths
/// for the selected browser key are checked.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::{verify_installed, Scope};
///
/// let installed = verify_installed("com.example.host", Some(&["firefox"]), Scope::User)?;
/// # let _ = installed;
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// # Errors
///
/// Returns an error when a browser key is unknown, the current OS/scope is not
/// configured, a path template references a missing environment variable, a
/// manifest exists but cannot be read, or a manifest contains invalid JSON.
pub fn verify_installed(
    host_name: &str,
    browsers: Option<&[&str]>,
    scope: paths::Scope,
) -> io::Result<bool> {
    let keys: Vec<&str> = match browsers {
        Some(list) => list.to_vec(),
        None => paths::config()
            .browsers
            .keys()
            .filter(|k| paths::browser_supports_scope(k, scope).unwrap_or(false))
            .map(|k| k.as_str())
            .collect(),
    };

    for browser_key in keys {
        if verify_one(browser_key, host_name, scope)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn verify_one(browser_key: &str, host_name: &str, scope: paths::Scope) -> io::Result<bool> {
    let cfg = paths::browser_cfg(browser_key)?;

    // Determine manifest path
    #[cfg(all(windows, feature = "windows-registry"))]
    let manifest_path = if cfg.windows_registry {
        let key_path = paths::winreg_key_path(browser_key, scope, host_name)?;
        match crate::install::winreg::read_manifest_path_from_reg(scope, &key_path)? {
            Some(p) => p,
            None => return Ok(false),
        }
    } else {
        paths::manifest_path(browser_key, scope, host_name)?
    };

    #[cfg(all(windows, not(feature = "windows-registry")))]
    let manifest_path = paths::manifest_path(browser_key, scope, host_name)?;

    #[cfg(not(windows))]
    let manifest_paths = paths::manifest_paths(browser_key, scope, host_name)?;

    #[cfg(windows)]
    {
        if !manifest_path.exists() {
            return Ok(false);
        }

        return verify_manifest_file(&manifest_path, &cfg.family, host_name);
    }

    #[cfg(not(windows))]
    {
        for manifest_path in manifest_paths {
            if !manifest_path.exists() {
                continue;
            }

            if verify_manifest_file(&manifest_path, &cfg.family, host_name)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn verify_manifest_file(manifest_path: &Path, family: &str, host_name: &str) -> io::Result<bool> {
    let data = fs::read_to_string(manifest_path)?;
    let v: Value = serde_json::from_str(&data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid JSON manifest: {e}"),
        )
    })?;

    validate_manifest_json(&v, family, host_name)
}

fn validate_manifest_json(v: &Value, family: &str, expected_name: &str) -> io::Result<bool> {
    let obj = match v.as_object() {
        Some(o) => o,
        None => return Ok(false),
    };

    if obj.get("name").and_then(|x| x.as_str()) != Some(expected_name) {
        return Ok(false);
    }
    if obj.get("type").and_then(|x| x.as_str()) != Some("stdio") {
        return Ok(false);
    }
    if obj.get("path").and_then(|x| x.as_str()).is_none() {
        return Ok(false);
    }

    match family {
        "chromium" => {
            if !is_valid_host_name(expected_name, false) {
                return Ok(false);
            }
            let Some(origins) = obj.get("allowed_origins").and_then(|x| x.as_array()) else {
                return Ok(false);
            };
            if origins.is_empty()
                || origins
                    .iter()
                    .any(|x| !x.as_str().is_some_and(validate_chromium_origin))
            {
                return Ok(false);
            }
            if obj.contains_key("allowed_extensions") {
                return Ok(false);
            }
        }
        "firefox" => {
            if !is_valid_host_name(expected_name, true) {
                return Ok(false);
            }
            let Some(extensions) = obj.get("allowed_extensions").and_then(|x| x.as_array()) else {
                return Ok(false);
            };
            if extensions.is_empty()
                || extensions
                    .iter()
                    .any(|x| !x.as_str().is_some_and(validate_firefox_extension_id))
            {
                return Ok(false);
            }
            if obj.contains_key("allowed_origins") {
                return Ok(false);
            }
        }
        _ => return Ok(false),
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let exe = obj.get("path").and_then(|x| x.as_str()).unwrap_or("");
        if !std::path::Path::new(exe).is_absolute() {
            return Ok(false);
        }
    }

    Ok(true)
}
