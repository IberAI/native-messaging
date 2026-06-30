//! Browser manifest path resolution.
//!
//! The installer uses an embedded `browsers.toml` file to resolve native
//! messaging host manifest locations for each supported browser key, scope, and
//! operating system. Set `NATIVE_MESSAGING_BROWSERS_CONFIG` to point at a custom
//! TOML file if your application needs to override or extend the embedded
//! browser table.
//!
//! Use [`manifest_path`] for the primary install target and [`manifest_paths`]
//! when you need every configured lookup location.

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{collections::HashMap, fs, io, path::PathBuf};

const DEFAULT_BROWSERS_TOML: &str = include_str!("browsers.toml");

/// Optional override:
/// - If NATIVE_MESSAGING_BROWSERS_CONFIG is set, load config from that path.
/// - Otherwise use the embedded browsers.toml.
fn load_browsers_toml() -> String {
    if let Ok(p) = std::env::var("NATIVE_MESSAGING_BROWSERS_CONFIG") {
        if let Ok(s) = fs::read_to_string(&p) {
            return s;
        }
    }
    DEFAULT_BROWSERS_TOML.to_string()
}

/// Installation scope for a native messaging host manifest.
#[derive(Debug, Clone, Copy)]
pub enum Scope {
    /// Install for the current user only.
    ///
    /// This is the recommended scope for development and for applications that
    /// do not need machine-wide availability.
    User,

    /// Install system-wide for all users.
    ///
    /// System locations typically require elevated permissions on Linux,
    /// macOS, and Windows.
    System,
}

/// Parsed browser path configuration.
///
/// This is primarily useful for diagnostics or custom installer tooling. Most
/// applications should use [`manifest_path`] or [`manifest_paths`] instead of
/// reading this structure directly.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Version of the embedded browser configuration schema.
    pub schema_version: u32,

    /// Browser entries keyed by names such as `chrome`, `edge`, or `firefox`.
    pub browsers: HashMap<String, BrowserCfg>,
}

/// Configuration for one browser key.
#[derive(Debug, Deserialize)]
pub struct BrowserCfg {
    /// Browser manifest family, currently `chromium` or `firefox`.
    pub family: String,

    /// Whether Windows registry pointers should be written and read.
    #[serde(default)]
    pub windows_registry: bool,

    /// OS-specific manifest path templates.
    pub paths: PathsByOs,

    /// Optional Windows registry template configuration.
    #[serde(default)]
    pub windows: Option<WindowsCfg>,
}

/// Windows-only browser configuration.
#[derive(Debug, Deserialize)]
pub struct WindowsCfg {
    /// Registry key templates for user and system installs.
    #[serde(default)]
    pub registry: Option<RegistryCfg>,
}

/// Windows registry key templates for a browser key.
#[derive(Debug, Deserialize)]
pub struct RegistryCfg {
    /// `HKEY_CURRENT_USER` key template.
    ///
    /// The `{name}` token is replaced with the native messaging host name.
    pub hkcu_key_template: Option<String>,

    /// `HKEY_LOCAL_MACHINE` key template.
    ///
    /// The `{name}` token is replaced with the native messaging host name.
    pub hklm_key_template: Option<String>,
}

/// Manifest path templates grouped by operating system.
#[derive(Debug, Deserialize)]
pub struct PathsByOs {
    /// macOS manifest locations.
    pub macos: Option<Scopes>,

    /// Linux manifest locations.
    pub linux: Option<Scopes>,

    /// Windows manifest locations.
    pub windows: Option<Scopes>,
}

/// Manifest path templates grouped by install scope.
#[derive(Debug, Deserialize)]
pub struct Scopes {
    /// Current-user manifest location.
    pub user: Option<PathEntry>,

    /// System-wide manifest location.
    pub system: Option<PathEntry>,
}

/// Manifest directory templates for one browser/scope/OS combination.
#[derive(Debug, Deserialize)]
pub struct PathEntry {
    /// Primary install directory for this browser/scope/OS.
    pub dir: String,

    /// Additional browser lookup directories.
    ///
    /// `manifest_path` returns `dir` for backward compatibility. Use
    /// `manifest_paths` when you need every documented lookup location, such
    /// as Firefox's `/usr/lib64` Linux fallback.
    #[serde(default)]
    pub alternate_dirs: Vec<String>,
}

static CONFIG: Lazy<Config> = Lazy::new(|| {
    let raw = load_browsers_toml();
    let cfg: Config = toml::from_str(&raw).expect("invalid browsers.toml");
    if cfg.schema_version != 1 {
        panic!(
            "unsupported schema_version {} (expected 1)",
            cfg.schema_version
        );
    }
    cfg
});

/// Return the loaded browser configuration.
///
/// The configuration is loaded once on first use. If
/// `NATIVE_MESSAGING_BROWSERS_CONFIG` points at a readable file, that file is
/// used; otherwise the crate's embedded `browsers.toml` is used.
pub fn config() -> &'static Config {
    &CONFIG
}

/// Return configuration for one browser key.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidInput`] when `browser_key` is not present in
/// the loaded browser configuration.
pub fn browser_cfg(browser_key: &str) -> io::Result<&'static BrowserCfg> {
    CONFIG.browsers.get(browser_key).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown browser: {browser_key}"),
        )
    })
}

/// Return whether a browser key has a manifest location for this OS and scope.
///
/// This is useful when iterating through all configured browser keys, because
/// some keys intentionally model platform-specific locations. For example,
/// Edge Beta/Dev/Canary entries currently model the macOS user-data locations
/// documented by Microsoft Edge.
///
/// # Errors
///
/// Returns an error if the browser key is unknown or the operating system is
/// unsupported by the crate.
pub fn browser_supports_scope(browser_key: &str, scope: Scope) -> io::Result<bool> {
    let b = browser_cfg(browser_key)?;
    let Some(scopes) = current_os_scopes(&b.paths)? else {
        return Ok(false);
    };

    Ok(match scope {
        Scope::User => scopes.user.is_some(),
        Scope::System => scopes.system.is_some(),
    })
}

/// Resolve the primary manifest JSON path for this browser+scope+host.
///
/// Some browsers document more than one lookup path for a platform/scope. This
/// function intentionally returns the primary install path for backward
/// compatibility. Use [`manifest_paths`] when you need every configured lookup
/// path.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::{manifest_path, Scope};
///
/// let path = manifest_path("chrome", Scope::User, "com.example.host").unwrap();
/// eprintln!("{}", path.display());
/// ```
///
/// # Errors
///
/// Returns an error if the browser key is unknown, the current OS/scope is not
/// configured, or a required environment variable such as `HOME` is missing.
pub fn manifest_path(browser_key: &str, scope: Scope, host_name: &str) -> io::Result<PathBuf> {
    manifest_paths(browser_key, scope, host_name)?
        .into_iter()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no manifest paths configured"))
}

/// Resolve all configured manifest JSON paths for this browser+scope+host.
///
/// The first item is always the primary path returned by [`manifest_path`].
/// Additional items are browser-documented alternate lookup paths for the same
/// OS/scope.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::{manifest_paths, Scope};
///
/// let paths = manifest_paths("firefox", Scope::System, "com.example.host").unwrap();
/// for path in paths {
///     eprintln!("{}", path.display());
/// }
/// ```
///
/// # Errors
///
/// Returns an error if the browser key is unknown, the current OS/scope is not
/// configured, or a required environment variable such as `HOME` is missing.
pub fn manifest_paths(
    browser_key: &str,
    scope: Scope,
    host_name: &str,
) -> io::Result<Vec<PathBuf>> {
    let b = browser_cfg(browser_key)?;

    let scopes = current_os_scopes(&b.paths)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "browser not configured for this OS",
        )
    })?;

    let entry = match scope {
        Scope::User => scopes.user.as_ref(),
        Scope::System => scopes.system.as_ref(),
    }
    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "scope not configured for this OS"))?;

    let file_name = format!("{host_name}.json");
    let mut paths = Vec::with_capacity(1 + entry.alternate_dirs.len());
    paths.push(resolve_dir_template(&entry.dir)?.join(&file_name));

    for dir in &entry.alternate_dirs {
        paths.push(resolve_dir_template(dir)?.join(&file_name));
    }

    Ok(paths)
}

fn current_os_scopes(paths: &PathsByOs) -> io::Result<Option<&Scopes>> {
    #[cfg(target_os = "macos")]
    {
        Ok(paths.macos.as_ref())
    }
    #[cfg(target_os = "linux")]
    {
        Ok(paths.linux.as_ref())
    }
    #[cfg(target_os = "windows")]
    {
        Ok(paths.windows.as_ref())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(io::Error::new(io::ErrorKind::Other, "unsupported OS"))
    }
}

fn resolve_dir_template(t: &str) -> io::Result<PathBuf> {
    let mut s = t.to_string();

    // Only replace if referenced; error if referenced but env missing.
    replace_var(&mut s, "{HOME}", "HOME")?;
    replace_var(&mut s, "{LOCALAPPDATA}", "LOCALAPPDATA")?;
    replace_var(&mut s, "{APPDATA}", "APPDATA")?;
    replace_var(&mut s, "{PROGRAMDATA}", "PROGRAMDATA")?;

    Ok(PathBuf::from(s))
}

fn replace_var(s: &mut String, token: &str, env: &str) -> io::Result<()> {
    if s.contains(token) {
        let v = std::env::var(env).map_err(|_| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("env var {env} not set (needed for {token})"),
            )
        })?;
        *s = s.replace(token, &v);
    }
    Ok(())
}

#[cfg(all(windows, feature = "windows-registry"))]
/// Resolve the Windows registry key path for a browser+scope+host.
///
/// The returned string is relative to either `HKEY_CURRENT_USER` or
/// `HKEY_LOCAL_MACHINE`, depending on [`Scope`].
///
/// # Errors
///
/// Returns an error when the browser key is unknown, registry support is not
/// enabled for that browser key, or the selected scope has no registry template.
pub fn winreg_key_path(browser_key: &str, scope: Scope, host_name: &str) -> io::Result<String> {
    let b = browser_cfg(browser_key)?;
    if !b.windows_registry {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "registry not enabled for this browser",
        ));
    }

    let reg = b
        .windows
        .as_ref()
        .and_then(|w| w.registry.as_ref())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "missing [browsers.<x>.windows.registry] config",
            )
        })?;

    let tmpl = match scope {
        Scope::User => reg.hkcu_key_template.as_ref(),
        Scope::System => reg.hklm_key_template.as_ref(),
    }
    .ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "missing registry template for this scope",
        )
    })?;

    Ok(tmpl.replace("{name}", host_name))
}
