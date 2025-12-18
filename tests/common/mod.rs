use std::{collections::HashMap, env};
use tempfile::TempDir;

/// Env guard that restores previous env vars on drop.
pub struct EnvGuard {
    old: HashMap<String, Option<String>>,
}

impl EnvGuard {
    pub fn set(vars: &[(&str, String)]) -> Self {
        let mut old = HashMap::new();
        for (k, v) in vars {
            old.insert((*k).to_string(), env::var(k).ok());
            env::set_var(k, v);
        }
        Self { old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, prev) in self.old.drain() {
            match prev {
                Some(v) => env::set_var(k, v),
                None => env::remove_var(k),
            }
        }
    }
}

/// Create a temp sandbox and set env vars so manifest installs go into it.
///
/// This makes tests safe and hermetic:
/// - Linux: uses HOME-based dirs
/// - macOS: uses HOME-based dirs
/// - Windows: uses APPDATA/LOCALAPPDATA/PROGRAMDATA
pub fn sandbox_env() -> (TempDir, EnvGuard) {
    let td = TempDir::new().expect("tempdir");
    let root = td.path().to_path_buf();

    // Use simple subdirectories to match what your templates expect.
    let home = root.join("home");
    let appdata = root.join("appdata_roaming");
    let localappdata = root.join("appdata_local");
    let programdata = root.join("programdata");

    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&appdata).unwrap();
    std::fs::create_dir_all(&localappdata).unwrap();
    std::fs::create_dir_all(&programdata).unwrap();

    let guard = EnvGuard::set(&[
        ("HOME", home.to_string_lossy().to_string()),
        ("APPDATA", appdata.to_string_lossy().to_string()),
        ("LOCALAPPDATA", localappdata.to_string_lossy().to_string()),
        ("PROGRAMDATA", programdata.to_string_lossy().to_string()),
        // Optional: if you later add XDG support, you can set this too.
        // ("XDG_CONFIG_HOME", root.join("xdg").to_string_lossy().to_string()),
    ]);

    (td, guard)
}
