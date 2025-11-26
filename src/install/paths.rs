use std::path::PathBuf;

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn unix_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .expect("HOME not set")
}

pub fn chrome_user_manifest(name: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        unix_home_dir()
            .join("Library/Application Support/Google/Chrome/NativeMessagingHosts")
            .join(format!("{name}.json"))
    }
    #[cfg(target_os = "linux")]
    {
        unix_home_dir()
            .join(".config/google-chrome/NativeMessagingHosts")
            .join(format!("{name}.json"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Local"))
            .join("NativeMessagingHosts")
            .join(format!("{name}.json"))
    }
}

pub fn chrome_winreg_path(name: &str) -> String {
    format!(r"Software\Google\Chrome\NativeMessagingHosts\{name}")
}

pub fn chrome_system_manifest(name: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from(format!(
            "/Library/Google/Chrome/NativeMessagingHosts/{name}.json"
        ))
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from(format!(
            "/etc/opt/chrome/native-messaging-hosts/{name}.json"
        ))
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(format!(r"C:\ProgramData\NativeMessagingHosts\{name}.json"))
    }
}

pub fn firefox_user_manifest(name: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        unix_home_dir()
            .join("Library/Application Support/Mozilla/NativeMessagingHosts")
            .join(format!("{name}.json"))
    }
    #[cfg(target_os = "linux")]
    {
        unix_home_dir()
            .join(".mozilla/native-messaging-hosts")
            .join(format!("{name}.json"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Roaming"))
            .join("Mozilla\\NativeMessagingHosts")
            .join(format!("{name}.json"))
    }
}

pub fn firefox_system_manifest(name: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from(format!("/Library/Mozilla/NativeMessagingHosts/{name}.json"))
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from(format!(
            "/usr/lib/mozilla/native-messaging-hosts/{name}.json"
        ))
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(format!(
            r"C:\ProgramData\Mozilla\NativeMessagingHosts\{name}.json"
        ))
    }
}

pub fn edge_user_manifest(name: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        unix_home_dir()
            .join("Library/Application Support/Microsoft Edge/NativeMessagingHosts")
            .join(format!("{name}.json"))
    }
    #[cfg(target_os = "linux")]
    {
        unix_home_dir()
            .join(".config/microsoft-edge/NativeMessagingHosts")
            .join(format!("{name}.json"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Local"))
            .join("NativeMessagingHosts")
            .join(format!("{name}.json"))
    }
}

pub fn edge_winreg_path(name: &str) -> String {
    format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{name}")
}

pub fn edge_system_manifest(name: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from(format!(
            "/Library/Microsoft/Edge/NativeMessagingHosts/{name}.json"
        ))
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from(format!("/etc/opt/edge/native-messaging-hosts/{name}.json"))
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(format!(r"C:\ProgramData\NativeMessagingHosts\{name}.json"))
    }
}
