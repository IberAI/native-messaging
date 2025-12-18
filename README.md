# Native Messaging Helper for WebExtensions

Rust helpers for building **native messaging hosts** for WebExtensions, with a **config-driven installer** and **correct-by-default protocol handling**.

This crate handles:

* Writing **browser-correct native messaging manifests**

  * Chromium family → `allowed_origins`
  * Firefox family → `allowed_extensions`
* Installing manifests at **user** or **system** scope
* **Windows registry handling** (for Chromium *and* Firefox-family browsers)
* Verifying and removing installations
* Correct **message framing** (4-byte length prefix + UTF-8 JSON)
* Async helpers for reading, writing, and running a host loop

> Build the host logic that matters — leave manifests, paths, and wire protocol details to this crate.

---

## Goals of this crate

Make the **native host side** of WebExtensions native messaging:

* **Boringly correct**
* **Portable across browsers and OSes**
* **Easy to test and reason about**

### Design principles

* **Config-driven browser support**
  Browsers are defined in an embedded `browsers.toml`. Adding support for new browsers (Brave, Vivaldi, LibreWolf, etc.) does **not** require code changes — only config.

* **Correct manifests by construction**
  The installer always emits the right JSON shape per browser family.

* **Registry-aware on Windows**
  Verification and removal correctly handle registry-only installs.

* **Async-first host helpers**
  Minimal, focused helpers to ship a production host quickly.

---

## What this library is (and isn’t)

### ✅ This crate **is** for:

* The **native host application**
* Writing / installing / verifying native messaging manifests
* Framing and exchanging JSON messages with the browser

### ❌ This crate **is not**:

* A browser extension SDK
* A replacement for `chrome.runtime.connectNative` / `browser.runtime.connectNative`

You still write standard extension code — this crate handles the **native side**.

---

## Features

* ✅ **Cross-browser**
  Chrome, Edge, Chromium, Brave, Vivaldi, Firefox, LibreWolf (via config)

* ✅ **Cross-platform**
  Linux, macOS, Windows

* ✅ **Registry-aware verification on Windows**

* ✅ **User & system scope installs**

* ✅ **Correct native messaging protocol helpers**

---

## Supported platforms

|      OS | Chromium family | Firefox family | Notes                            |
| ------: | :-------------: | :------------: | -------------------------------- |
|   Linux |        ✅        |        ✅       | Absolute `path` required         |
|   macOS |        ✅        |        ✅       | Absolute `path` required         |
| Windows |        ✅        |        ✅       | Registry keys used for discovery |

---

## Install

```toml
[dependencies]
native_messaging = "0.1.3"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

---

## Quick Start

### Install manifests (config-driven browsers)

Browsers are selected using **string keys** from the embedded `browsers.toml`
(e.g. `"chrome"`, `"firefox"`, `"edge"`, `"brave"`, `"vivaldi"`).

```rust
use std::path::Path;
use native_messaging::install::{install, Scope};

fn main() -> std::io::Result<()> {
    let host_exe = if cfg!(windows) {
        Path::new(r"C:\full\path\to\your_host.exe")
    } else {
        Path::new("/full/path/to/your_host")
    };

    let chromium_origins = vec![
        "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/".to_string(),
    ];

    let firefox_extensions = vec![
        "native-test@example.com".to_string(),
    ];

    install(
        "com.example.native_host",
        "Example native messaging host",
        host_exe,
        &chromium_origins,
        &firefox_extensions,
        &["chrome", "firefox", "edge"],
        Scope::User,
    )
}
```

---

### Verify installation

Verification is **registry-aware on Windows** and filesystem-based on Unix.

```rust
use native_messaging::install::{verify_installed, Scope};

fn main() -> std::io::Result<()> {
    let installed = verify_installed(
        "com.example.native_host",
        None,            // check all configured browsers
        Scope::User,
    )?;

    println!("installed? {installed}");
    Ok(())
}
```

---

### Remove manifests

```rust
use native_messaging::install::{remove, Scope};

fn main() -> std::io::Result<()> {
    remove(
        "com.example.native_host",
        &["chrome", "firefox", "edge"],
        Scope::User,
    )
}
```

---

## Send / receive messages

Native messaging uses **length-prefixed UTF-8 JSON**.

### Read one message

```rust
use native_messaging::get_message;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let msg = get_message().await?;
    eprintln!("received: {msg}");
    Ok(())
}
```

---

### Send one message

```rust
use native_messaging::send_message;
use serde::Serialize;

#[derive(Serialize)]
struct Greeting {
    hello: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    send_message(&Greeting {
        hello: "from host".into(),
    }).await
}
```

---

### Run an async event loop

```rust
use native_messaging::{event_loop, send_message};
use serde_json::json;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    event_loop(|msg: String| async move {
        send_message(&json!({ "echo": msg })).await
    }).await
}
```

---

## API Overview

### `install`

* `install(...)`
* `verify_installed(...)`
* `remove(...)`
* `Scope::{User, System}`

The installer is **config-driven**. Browser behavior comes from `browsers.toml`, not Rust enums.

---

### `host`

* `encode_message`
* `decode_message`
* `get_message`
* `send_message`
* `event_loop`

Protocol rules enforced:

* 4-byte length prefix
* UTF-8 JSON
* 1 MB host → browser limit
* configurable browser → host limit

---

## Extension-side notes

### Chromium family

* Uses `allowed_origins`
* Requires `"permissions": ["nativeMessaging"]`
* Call `chrome.runtime.connectNative("com.example.native_host")`

### Firefox family

* Uses `allowed_extensions`
* Requires a stable add-on ID:

```json
{
  "browser_specific_settings": {
    "gecko": { "id": "native-test@example.com" }
  }
}
```

* Call `browser.runtime.connectNative("com.example.native_host")`

---

## Testing locally

* Unit tests validate:

  * framing correctness
  * manifest JSON shape
  * path resolution
  * install/remove/verify behavior
* Windows registry logic is exercised on Windows CI
* No real browser required for tests

---

## Troubleshooting

**Host not found**

* Name mismatch between extension & manifest
* Wrong scope (User vs System)
* Windows: registry key missing
* Non-absolute path on macOS/Linux

**Disconnects**

* Host crashed (run manually for stderr)
* Message size exceeded
* MV3 service worker reconnect needed

---

## Contributing

PRs welcome. Please keep changes:

* Config-driven
* Cross-platform
* Well-tested

---

## License

MIT

---

**Build great WebExtensions — this crate handles the native side, correctly.**
