# Native Messaging Helper for WebExtensions

Rust helpers for building **native messaging** hosts for Chrome/Chromium and Firefox. This crate handles:

* Writing **browser-correct host manifests** (Chrome `allowed_origins`, Firefox `allowed_extensions`)
* Installing at **user** or **system** scope (with Windows registry handling for Chrome)
* Verifying/removing manifests
* Correct **message framing** (4-byte length prefix + UTF-8 JSON) with async helpers to read, write, and loop

> Build the parts that matter—leave the manifest details and wire protocol to us.


## Goal of this crate

Make the **native host side** of WebExtensions native messaging **boringly correct** and **portable**:

* **Zero boilerplate for manifests:** Generate the right shape for each browser, write to the correct OS paths, and (on Windows) handle the Chrome registry.
* **Correct-by-default protocol:** Enforce the 4-byte length prefix + UTF-8 JSON framing and sensible size limits.
* **Ergonomic async I/O:** Small, focused helpers (`get_message`, `send_message`, `event_loop`) so you can ship a host quickly.
* **Testability:** Functions are easy to unit/integration test (e.g., in-memory framing, temp HOME paths).

## What this library is and isn’t

**Purpose:** This crate is for the **native host (app) side** of WebExtensions native messaging. It gives you:

* Host **manifest** creation/installation/removal for Chrome & Firefox
* Host-side **message framing** (length-prefixed JSON) and async I/O helpers

**Not included:** It does **not** implement the **extension (browser) side**. You’ll still write normal Chrome/Firefox extension code (`connectNative` / `sendNativeMessage`). The README includes minimal extension snippets purely to help you **test** your host.

---

## Table of Contents

* [Features](#features)
* [Supported platforms](#supported-platforms)
* [Install](#install)
* [Quick Start](#quick-start)

  * [Install manifests (Chrome + Firefox)](#install-manifests-chrome--firefox)
  * [Verify and remove manifests](#verify-and-remove-manifests)
  * [Send/receive messages](#sendreceive-messages)
* [API Overview](#api-overview)

  * [`install::manifest`](#installmanifest)
  * [`host`](#host)
* [Extension-side notes](#extension-side-notes)
* [Testing locally](#testing-locally)
* [Troubleshooting](#troubleshooting)
* [Contributing](#contributing)
* [License](#license)

---

## Features

* ✅ **Cross-browser**: Generates separate host manifests for **Chrome** and **Firefox** with the right keys.
* ✅ **Cross-platform**: Linux, macOS, and Windows (Chrome registry supported).
* ✅ **Scopes**: Write to **user** or **system** locations (system may require elevated privileges).
* ✅ **Protocol helpers**: Encode/decode frames; async `get_message`, `send_message`, and `event_loop`.

---

## Supported platforms

|      OS | Chrome/Chromium | Firefox | Notes                                                              |
| ------: | :-------------: | :-----: | ------------------------------------------------------------------ |
|   Linux |        ✅        |    ✅    | Absolute `path` required in manifests                              |
|   macOS |        ✅        |    ✅    | Absolute `path` required in manifests                              |
| Windows |        ✅        |    ✅    | Chrome requires a **registry** value pointing to the manifest file |

---

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
native_messaging = "0.1.2"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

---

## Quick Start

### Install manifests (Chrome + Firefox)

```rust
use std::path::Path;
use native_messaging::install::manifest::{install, Browser, Scope};

fn main() -> std::io::Result<()> {
    // 1) Absolute path to your host executable (required on macOS/Linux)
    let host_exe = if cfg!(windows) {
        Path::new(r"C:\full\path\to\your_host.exe")
    } else {
        Path::new("/full/path/to/your_host")
    };

    // 2) Chrome requires chrome-extension://<ID>/ origins
    //    (get the ID from chrome://extensions after loading your extension)
    let chrome_origin = "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/".to_string();

    // 3) Firefox requires add-on IDs (set in manifest via browser_specific_settings.gecko.id)
    let firefox_id = "native-test@example.com".to_string();

    // 4) Install both manifests at user scope
    install(
        "com.example.native_host",          // host name used by the extension
        "Example native host",              // description
        host_exe,                           // absolute path to host
        &[chrome_origin],                   // Chrome allowed_origins
        &[firefox_id],                      // Firefox allowed_extensions
        &[Browser::Chrome, Browser::Firefox],
        Scope::User,
    )
}
```

### Verify and remove manifests

```rust
use native_messaging::install::manifest::{verify, remove, Browser, Scope};

fn main() -> std::io::Result<()> {
    // Check if either Chrome/Firefox user-scope manifest exists
    let present = verify("com.example.native_host")?;
    println!("present? {present}");

    // Remove both manifests (also removes Chrome registry value on Windows)
    remove("com.example.native_host",
           &[Browser::Chrome, Browser::Firefox],
           Scope::User)?;
    Ok(())
}
```

### Send/receive messages

> Hosts talk to the browser via **length-prefixed JSON**. Use the helpers below.

**Read one message from stdin:**

```rust
use native_messaging::host::get_message;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let msg = get_message().await?;
    eprintln!("got: {msg}");
    Ok(())
}
```

**Send one JSON message to stdout:**

```rust
use native_messaging::host::send_message;
use serde::Serialize;

#[derive(Serialize)]
struct Greeting { hello: String }

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let g = Greeting { hello: "from host".into() };
    send_message(&g).await
}
```

**Run an async event loop:**

```rust
use native_messaging::host::{event_loop, send_message};
use serde_json::json;
use std::io;

async fn handle_message(message: String) -> io::Result<()> {
    // echo back in a wrapper JSON
    let reply = json!({ "echo": message });
    send_message(&reply).await
}

#[tokio::main]
async fn main() -> io::Result<()> {
    event_loop(handle_message).await
}
```

---

## API Overview

### `install::manifest`

* `install(name, description, exe_path, chrome_allowed_origins, firefox_allowed_extensions, browsers, scope)`
  Writes the correct manifests for requested browsers. On Windows+Chrome, also writes the registry value pointing at the Chrome manifest file.

* `verify(name) -> io::Result<bool>`
  Returns `true` if **user-scope** Chrome or Firefox manifest exists.

* `remove(name, browsers, scope)`
  Deletes the manifest files for the given browsers and scope. Also removes Chrome’s registry value on Windows.

**Types**

* `enum Browser { Chrome, Firefox }`
* `enum Scope { User, System }`

> ⚠️ On macOS/Linux, `exe_path` **must** be absolute. On Windows, absolute paths are strongly recommended (and used by default in examples).

### `host`

* `encode_message<T: Serialize>(&T) -> io::Result<Vec<u8>>`
  Build a framed message (`len:u32` + JSON bytes). Enforces a **1 MB** limit host→browser.

* `decode_message<R: Read>(&mut R, max_size: usize) -> io::Result<String>`
  Read and decode one frame from a reader (defaults elsewhere to **≤64 MB** browser→host).

* `get_message() -> io::Result<String>` *(async)*
  Read a single framed message from **stdin**.

* `send_message<T: Serialize>(&T) -> io::Result<()>` *(async)*
  Frame and write a JSON message to **stdout**.

* `event_loop(handler) -> io::Result<()>` *(async)*
  Call `handler(String)` for each incoming message forever.

---

## Extension-side notes

* **Chrome**:

  * Manifest key is `allowed_origins` with entries like `chrome-extension://<ID>/`.
  * Your extension must declare `"permissions": ["nativeMessaging"]` and call `chrome.runtime.connectNative("com.example.native_host")`.
  * MV3 service workers: a live native messaging port keeps the worker alive, but only while connected—handle `onDisconnect` and reconnect as needed.

* **Firefox**:

  * Manifest key is `allowed_extensions` with your **add-on IDs**.
  * Set a stable ID in your extension’s `manifest.json`:

    ```json
    {
      "browser_specific_settings": { "gecko": { "id": "native-test@example.com" } }
    }
    ```
  * Use `browser.runtime.connectNative("com.example.native_host")`.

---

## Testing locally

**Protocol sanity (no browser):**

```rust
use native_messaging::host::{encode_message, decode_message};
use serde_json::json;

fn main() -> std::io::Result<()> {
    // Encode a message like the browser would
    let frame = encode_message(&json!({"hello": "world"}))?;

    // Read it back via an in-memory cursor (like host would do)
    let mut cursor = std::io::Cursor::new(frame);
    let s = decode_message(&mut cursor, 64 * 1024 * 1024)?; // 64MB cap
    assert!(s.contains("hello"));
    Ok(())
}
```

**End-to-end with a tiny host:**

* Build a small echo host that reads with `get_message()` and replies with `send_message()`.
* Install manifests (user scope).
* Load a minimal test extension in Chrome/Firefox and connect using `connectNative()`.

---

## Troubleshooting

* **“Specified native messaging host not found”**

  * Host `name` mismatch between extension & manifest
  * Manifest written to the wrong location (check OS paths)
  * Windows + Chrome: missing registry value (re-run install on Windows)
  * `exe_path` not absolute (macOS/Linux)

* **Disconnects / No messages**

  * Host crashed: run the host binary manually to see stderr logs
  * Message too large: host enforces **1 MB** host→browser
  * Chrome MV3: service worker stopped—reconnect on `onDisconnect`

* **Multiple Chrome channels/profiles**

  * Use stable Chrome and the default profile for first validation; paths vary per channel/profile.

---

## Contributing

Contributions welcome!

1. Fork
2. Branch (`feature/my-feature`)
3. Commit (`feat: add X`)
4. PR

Please follow the [Contributor Covenant](https://www.contributor-covenant.org/version/2/0/code_of_conduct.html).

---

## License

MIT — see [LICENSE](LICENSE).

---

**Build great WebExtensions. We’ll handle the plumbing.**
