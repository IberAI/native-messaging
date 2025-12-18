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

#[derive(Debug, Clone, Copy)]
pub enum Scope {
    User,
    System,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub schema_version: u32,
    pub browsers: HashMap<String, BrowserCfg>,
}

#[derive(Debug, Deserialize)]
pub struct BrowserCfg {
    /// "chromium" or "firefox"
    pub family: String,

    /// Whether Windows registry pointers should be written
    #[serde(default)]
    pub windows_registry: bool,

    pub paths: PathsByOs,

    #[serde(default)]
    pub windows: Option<WindowsCfg>,
}

#[derive(Debug, Deserialize)]
pub struct WindowsCfg {
    #[serde(default)]
    pub registry: Option<RegistryCfg>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryCfg {
    pub hkcu_key_template: Option<String>,
    pub hklm_key_template: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PathsByOs {
    pub macos: Option<Scopes>,
    pub linux: Option<Scopes>,
    pub windows: Option<Scopes>,
}

#[derive(Debug, Deserialize)]
pub struct Scopes {
    pub user: Option<PathEntry>,
    pub system: Option<PathEntry>,
}

#[derive(Debug, Deserialize)]
pub struct PathEntry {
    pub dir: String,
}

static CONFIG: Lazy<Config> = Lazy::new(|| {
    let raw = load_browsers_toml();
    let cfg: Config = toml::from_str(&raw).expect("invalid browsers.toml");
    if cfg.schema_version != 1 {
        panic!("unsupported schema_version {} (expected 1)", cfg.schema_version);
    }
    cfg
});

pub fn config() -> &'static Config {
    &CONFIG
}

pub fn browser_cfg(browser_key: &str) -> io::Result<&'static BrowserCfg> {
    CONFIG
        .browsers
        .get(browser_key)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("unknown browser: {browser_key}")))
}

/// Resolve the full manifest JSON path for this browser+scope+host.
pub fn manifest_path(browser_key: &str, scope: Scope, host_name: &str) -> io::Result<PathBuf> {
    let b = browser_cfg(browser_key)?;

    let scopes = current_os_scopes(&b.paths)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "browser not configured for this OS"))?;

    let entry = match scope {
        Scope::User => scopes.user.as_ref(),
        Scope::System => scopes.system.as_ref(),
    }
    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "scope not configured for this OS"))?;

    let dir = resolve_dir_template(&entry.dir)?;
    Ok(dir.join(format!("{host_name}.json")))
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
            io::Error::new(io::ErrorKind::NotFound, format!("env var {env} not set (needed for {token})"))
        })?;
        *s = s.replace(token, &v);
    }
    Ok(())
}

#[cfg(windows)]
pub fn winreg_key_path(browser_key: &str, scope: Scope, host_name: &str) -> io::Result<String> {
    let b = browser_cfg(browser_key)?;
    if !b.windows_registry {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "registry not enabled for this browser"));
    }

    let reg = b
        .windows
        .as_ref()
        .and_then(|w| w.registry.as_ref())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "missing [browsers.<x>.windows.registry] config"))?;

    let tmpl = match scope {
        Scope::User => reg.hkcu_key_template.as_ref(),
        Scope::System => reg.hklm_key_template.as_ref(),
    }
    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "missing registry template for this scope"))?;

    Ok(tmpl.replace("{name}", host_name))
}
