use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    io,
    path::Path,
};

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

/// Install manifests for the given browser keys (from browsers.toml).
///
/// - `chrome_allowed_origins` is used for `family="chromium"` browsers
/// - `firefox_allowed_extensions` is used for `family="firefox"` browsers
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
        let manifest_path = paths::manifest_path(browser_key, scope, host_name)?;

        if let Some(dir) = manifest_path.parent() {
            fs::create_dir_all(dir)?;
        }

        match cfg.family.as_str() {
            "chromium" => {
                let m = ChromiumHostManifest {
                    name: host_name,
                    description,
                    path: exe_path
                        .to_str()
                        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "exe_path is not valid UTF-8"))?,
                    ty: "stdio",
                    allowed_origins: chrome_allowed_origins.to_vec(),
                };
                fs::write(&manifest_path, serde_json::to_vec_pretty(&m)?)?;
            }
            "firefox" => {
                let m = FirefoxHostManifest {
                    name: host_name,
                    description,
                    path: exe_path
                        .to_str()
                        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "exe_path is not valid UTF-8"))?,
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
        #[cfg(windows)]
        {
            if cfg.windows_registry {
                let key_path = paths::winreg_key_path(browser_key, scope, host_name)?;
                crate::install::winreg::write_manifest_path_to_reg(scope, &key_path, &manifest_path)?;
            }
        }
    }

    Ok(())
}

/// Remove manifests + registry keys for the given browser keys.
pub fn remove(host_name: &str, browsers: &[&str], scope: paths::Scope) -> io::Result<()> {
    for browser_key in browsers {
        let cfg = paths::browser_cfg(browser_key)?;

        // Remove file (best-effort if missing)
        let manifest_path = paths::manifest_path(browser_key, scope, host_name)?;
        if manifest_path.exists() {
            fs::remove_file(&manifest_path)?;
        }

        // Remove registry pointer if configured.
        #[cfg(windows)]
        {
            if cfg.windows_registry {
                let key_path = paths::winreg_key_path(browser_key, scope, host_name)?;
                crate::install::winreg::remove_manifest_reg(scope, &key_path).ok();
            }
        }
    }
    Ok(())
}

/// Verify installation for a host across browsers.
/// - If `browsers` is None, checks all configured browsers in `browsers.toml`.
/// - On Windows, if `windows_registry=true`, verification is registry-aware.
pub fn verify_installed(
    host_name: &str,
    browsers: Option<&[&str]>,
    scope: paths::Scope,
) -> io::Result<bool> {
    let keys: Vec<&str> = match browsers {
        Some(list) => list.to_vec(),
        None => paths::config().browsers.keys().map(|k| k.as_str()).collect(),
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
    #[cfg(windows)]
    let manifest_path = if cfg.windows_registry {
        let key_path = paths::winreg_key_path(browser_key, scope, host_name)?;
        match crate::install::winreg::read_manifest_path_from_reg(scope, &key_path)? {
            Some(p) => p,
            None => return Ok(false),
        }
    } else {
        paths::manifest_path(browser_key, scope, host_name)?
    };

    #[cfg(not(windows))]
    let manifest_path = paths::manifest_path(browser_key, scope, host_name)?;

    if !manifest_path.exists() {
        return Ok(false);
    }

    let data = fs::read_to_string(&manifest_path)?;
    let v: Value = serde_json::from_str(&data).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("invalid JSON manifest: {e}"))
    })?;

    validate_manifest_json(&v, &cfg.family, host_name)
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
            if obj.get("allowed_origins").and_then(|x| x.as_array()).is_none() {
                return Ok(false);
            }
            if obj.contains_key("allowed_extensions") {
                return Ok(false);
            }
        }
        "firefox" => {
            if obj.get("allowed_extensions").and_then(|x| x.as_array()).is_none() {
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
