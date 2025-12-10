use serde::Serialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy)]
pub enum Browser {
    Chrome,
    Firefox,
    Edge,
}

#[derive(Debug, Clone, Copy)]
pub enum Scope {
    User,
    System,
}

#[derive(Serialize)]
struct ChromeHostManifest<'a> {
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

    // On Windows we accept any (absolute recommended). Touch exe_path
    // to avoid an unused-variable warning in Windows builds.
    #[cfg(windows)]
    {
        let _ = exe_path;
    }

    Ok(())
}

pub fn install(
    name: &str,
    description: &str,
    exe_path: &Path,
    chrome_allowed_origins: &[String],
    firefox_allowed_extensions: &[String],
    browsers: &[Browser],
    scope: Scope,
) -> io::Result<()> {
    ensure_absolute_path(exe_path)?;
    for b in browsers {
        match b {
            Browser::Chrome => write_chrome_manifest(
                name,
                description,
                exe_path,
                chrome_allowed_origins,
                &manifest_path(name, Browser::Chrome, scope)?,
                &winreg_path(name, Browser::Chrome)?,
            )?,
            Browser::Firefox => write_firefox_manifest(
                name,
                description,
                exe_path,
                firefox_allowed_extensions,
                &manifest_path(name, Browser::Firefox, scope)?,
            )?,
            Browser::Edge => write_chrome_manifest(
                name,
                description,
                exe_path,
                chrome_allowed_origins,
                &manifest_path(name, Browser::Edge, scope)?,
                &winreg_path(name, Browser::Edge)?,
            )?,
        }
    }
    Ok(())
}

pub fn remove(name: &str, browsers: &[Browser], scope: Scope) -> io::Result<()> {
    for b in browsers {
        let path = manifest_path(name, *b, scope)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    #[cfg(windows)]
    {
        // also remove Windows registry key for Chrome
        use crate::install::winreg::remove_chrome_manifest_reg;
        remove_chrome_manifest_reg(name).ok();
    }
    Ok(())
}

pub fn verify(name: &str) -> io::Result<bool> {
    let chrome_user = manifest_path(name, Browser::Chrome, Scope::User)?;
    let firefox_user = manifest_path(name, Browser::Firefox, Scope::User)?;
    let edge_user = manifest_path(name, Browser::Edge, Scope::User)?;
    Ok(chrome_user.exists() || firefox_user.exists() || edge_user.exists())
}

fn write_chrome_manifest(
    name: &str,
    description: &str,
    exe_path: &Path,
    allowed_origins: &[String],
    manifest_path: &PathBuf,
    #[allow(unused_variables)] winreg_path: &str,
) -> io::Result<()> {
    let m = ChromeHostManifest {
        name,
        description,
        path: exe_path.to_str().unwrap(),
        ty: "stdio",
        allowed_origins: allowed_origins.to_vec(),
    };
    if let Some(dir) = manifest_path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(manifest_path, serde_json::to_vec_pretty(&m)?)?;

    #[cfg(windows)]
    {
        // Chrome on Windows requires a registry entry pointing to the manifest path. :contentReference[oaicite:4]{index=4}
        use crate::install::winreg::write_chrome_manifest_reg;
        write_chrome_manifest_reg(name, winreg_path)?;
    }
    Ok(())
}

fn write_firefox_manifest(
    name: &str,
    description: &str,
    exe_path: &Path,
    allowed_extensions: &[String],
    out: &PathBuf,
) -> io::Result<()> {
    let m = FirefoxHostManifest {
        name,
        description,
        path: exe_path.to_str().unwrap(),
        ty: "stdio",
        allowed_extensions: allowed_extensions.to_vec(),
    };
    // let out = manifest_path(name, Browser::Firefox, scope)?;
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(out, serde_json::to_vec_pretty(&m)?)?;
    Ok(())
}

fn manifest_path(name: &str, browser: Browser, scope: Scope) -> io::Result<PathBuf> {
    use crate::install::paths::*;
    match (browser, scope) {
        (Browser::Chrome, Scope::User) => Ok(chrome_user_manifest(name)),
        (Browser::Chrome, Scope::System) => Ok(chrome_system_manifest(name)),
        (Browser::Firefox, Scope::User) => Ok(firefox_user_manifest(name)),
        (Browser::Firefox, Scope::System) => Ok(firefox_system_manifest(name)),
        (Browser::Edge, Scope::User) => Ok(edge_user_manifest(name)),
        (Browser::Edge, Scope::System) => Ok(edge_system_manifest(name)),
    }
}

fn winreg_path(name: &str, browser: Browser) -> io::Result<String> {
    use crate::install::paths::*;
    match browser {
        Browser::Chrome => Ok(chrome_winreg_path(name)),
        Browser::Firefox => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no registry for Firefox",
        )),
        Browser::Edge => Ok(edge_winreg_path(name)),
    }
}
