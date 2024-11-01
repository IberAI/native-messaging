use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
};

/// Stores information about browser-specific paths and registries for native messaging.
#[derive(Serialize, Deserialize, Debug)]
pub struct BrowserInfo {
    pub registry: Option<String>,
    pub linux: Option<PathBuf>,
    pub darwin: Option<PathBuf>,
}

/// Represents a native messaging manifest.
#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_origins: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_extensions: Option<Vec<String>>,
}

/// Gets information about supported browsers, such as paths for native messaging hosts.
///
/// # Examples
///
/// ```
/// use native_messaging::install::get_browser_info;
///
/// let browser_info = get_browser_info();
/// assert!(browser_info.contains_key("chrome"));
/// assert!(browser_info.contains_key("firefox"));
/// ```
pub fn get_browser_info() -> HashMap<String, BrowserInfo> {
    let home_dir = env::var("HOME").unwrap_or_else(|_| String::new());
    let mut browser_info = HashMap::new();

    browser_info.insert(
        "chrome".to_string(),
        BrowserInfo {
            registry: Some("Software\\Google\\Chrome\\NativeMessagingHosts".to_string()),
            linux: Some(PathBuf::from(format!(
                "{}/.config/google-chrome/NativeMessagingHosts",
                home_dir
            ))),
            darwin: Some(PathBuf::from(format!(
                "{}/Library/Application Support/Google/Chrome/NativeMessagingHosts",
                home_dir
            ))),
        },
    );

    browser_info.insert(
        "firefox".to_string(),
        BrowserInfo {
            registry: Some("Software\\Mozilla\\NativeMessagingHosts".to_string()),
            linux: Some(PathBuf::from(format!(
                "{}/.mozilla/native-messaging-hosts",
                home_dir
            ))),
            darwin: Some(PathBuf::from(format!(
                "{}/Library/Application Support/Mozilla/NativeMessagingHosts",
                home_dir
            ))),
        },
    );

    browser_info
}

/// Reads the contents of a file into a `String`.
///
/// # Arguments
///
/// * `filename` - Path to the file to be read.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use native_messaging::install::read_file;
///
/// let path = PathBuf::from("example.txt");
/// match read_file(&path) {
///     Ok(contents) => println!("File contents: {}", contents),
///     Err(e) => eprintln!("Failed to read file: {}", e),
/// }
/// ```
fn read_file(filename: &PathBuf) -> io::Result<String> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Writes a `String` to a file.
///
/// # Arguments
///
/// * `filename` - Path to the file to be written to.
/// * `contents` - The content to be written.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use native_messaging::install::write_file;
///
/// let path = PathBuf::from("example.txt");
/// let content = "Hello, world!";
/// write_file(&path, content).expect("Failed to write file");
/// ```
fn write_file(filename: &PathBuf, contents: &str) -> io::Result<()> {
    let mut file = File::create(filename)?;
    file.write_all(contents.as_bytes())
}

/// Writes a manifest file for a specified browser.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use native_messaging::install::{Manifest, write_manifest};
///
/// let mut manifest = Manifest {
///     name: "my_extension".to_string(),
///     description: "An example extension".to_string(),
///     path: PathBuf::from("/path/to/extension"),
///     allowed_origins: Some(vec!["*".to_string()]),
///     allowed_extensions: None,
/// };
///
/// let path = PathBuf::from("manifest.json");
/// write_manifest("chrome", &path, &mut manifest).expect("Failed to write manifest");
/// ```
fn write_manifest(browser: &str, path: &PathBuf, manifest: &mut Manifest) -> io::Result<()> {
    match browser {
        "firefox" => manifest.allowed_origins = None,
        "chrome" => manifest.allowed_extensions = None,
        _ => {}
    }

    let manifest_json = serde_json::to_string_pretty(manifest)?;
    write_file(path, &manifest_json)
}

/// Installs the manifest file on Unix-based systems for specified browsers.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::install::{install_unix, Manifest};
/// use std::path::PathBuf;
///
/// let mut manifest = Manifest {
///     name: "my_extension".to_string(),
///     description: "An example extension".to_string(),
///     path: PathBuf::from("/path/to/extension"),
///     allowed_origins: Some(vec!["*".to_string()]),
///     allowed_extensions: None,
/// };
///
/// install_unix(&["chrome", "firefox"], &mut manifest).expect("Installation failed");
/// ```
fn install_unix(browsers: &[&str], manifest: &mut Manifest) -> io::Result<()> {
    let browser_info = get_browser_info();
    for &browser in browsers {
        if let Some(info) = browser_info.get(browser) {
            if let Some(manifest_path) = &info.linux {
                if !manifest_path.exists() {
                    fs::create_dir_all(manifest_path)?;
                }
                let manifest_file = manifest_path.join(format!("{}.json", manifest.name));
                write_manifest(browser, &manifest_file, manifest)?;
            }
        }
    }
    Ok(())
}

/// Installs the manifest file for the given browsers.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::install::install;
///
/// install("my_extension", "An example extension", "/path/to/extension", &["chrome", "firefox"])
///     .expect("Failed to install extension");
/// ```
pub fn install(name: &str, description: &str, path: &str, browsers: &[&str]) -> io::Result<()> {
    let manifest = Manifest {
        name: name.to_string(),
        description: description.to_string(),
        path: PathBuf::from(path),
        allowed_origins: None,
        allowed_extensions: None,
    };
    let mut manifest = manifest;
    manifest.path = fs::canonicalize(&manifest.path)?;
    install_unix(browsers, &mut manifest)
}

/// Verifies if the manifest file is installed for the specified browsers.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::install::verify;
///
/// let is_installed = verify("my_extension").expect("Verification failed");
/// if is_installed {
///     println!("Manifest is installed.");
/// } else {
///     println!("Manifest is not installed.");
/// }
/// ```
pub fn verify(name: &str) -> io::Result<bool> {
    let browser_info = get_browser_info();
    for (_, info) in &browser_info {
        if let Some(manifest_path) = &info.linux {
            let manifest_file = manifest_path.join(format!("{}.json", name));
            if manifest_file.exists() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Removes the manifest file for specified browsers.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::install::verify;
///
/// remove("my_extension", &["chrome", "firefox"]).expect("Failed to remove extension");
/// ```
pub fn remove(name: &str, browsers: &[&str]) -> io::Result<()> {
    let browser_info = get_browser_info();
    for &browser in browsers {
        if let Some(info) = browser_info.get(browser) {
            if let Some(manifest_path) = &info.linux {
                let manifest_file = manifest_path.join(format!("{}.json", name));
                if manifest_file.exists() {
                    fs::remove_file(manifest_file)?;
                }
            }
        }
    }
    Ok(())
}
