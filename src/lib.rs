#![doc = r#"
# Native Messaging — Async Rust host + manifest utilities

This crate helps you build a **Native Messaging host** for WebExtensions and
install/manage the **host manifest** for Chrome/Chromium/Edge and Firefox.

It exposes two areas:

- `host` — encode/read/write native-messaging frames over **stdin/stdout**, and
  an async `event_loop` for handling messages.
- `install::manifest` — create **host manifests**, **verify** their presence,
  and **remove** them for supported browsers.

The examples below mirror common usage: creating a
manifest, sending/receiving JSON messages, and running an async loop.

---

## Quick start

Add deps:

```toml
[dependencies]
native_messaging = { path = "." }   # this crate, if you're developing locally
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

## The wire format (tl;dr)

```text
[0..4)  : 32-bit length (native-endian)
[4..N)  : UTF-8 JSON payload of that length
```

Browsers spawn your program and connect **stdin/stdout**; you read frames from
stdin and write frames to stdout.

---

## Messaging examples

### Read a single message

```no_run
use native_messaging::host::get_message;

#[tokio::main]
async fn main() {
    match get_message().await {
        Ok(json) => println!("Received: {json}"),
        Err(e)  => eprintln!("Error receiving message: {e}"),
    }
}
```

### Send a single message

```no_run
use native_messaging::host::send_message;
use serde::Serialize;

#[derive(Serialize)]
struct MyMessage { content: String }

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let msg = MyMessage { content: "Hello, world!".into() };
    send_message(&msg).await
}
```

### Encode a message to a native-messaging frame

```no_run
use native_messaging::host::encode_message;
use serde::Serialize;

#[derive(Serialize)]
struct Msg { hello: String }

fn main() {
    let frame = encode_message(&Msg { hello: "world".into() })
        .expect("serialize");
    assert!(frame.len() >= 4);
}
```

### Run an async event loop

```no_run
use native_messaging::host::{event_loop, send_message};

#[tokio::main]
async fn main() {
    event_loop(|raw: String| async move {
        // Echo the raw JSON back to the browser
        send_message(&raw).await
    }).await;
}
```

### Structured messages (enums)

```no_run
use native_messaging::host::{event_loop, send_message};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(tag="type", rename_all="snake_case")]
enum In {
  Ping { id: u64, payload: String },
}

#[derive(Serialize)]
struct Out { id: u64, echo: String, ok: bool }

#[tokio::main]
async fn main() {
    event_loop(|raw: String| async move {
        match serde_json::from_str::<In>(&raw) {
            Ok(In::Ping { id, payload }) => {
                send_message(&Out { id, echo: payload, ok: true }).await
            }
            Err(e) => {
                eprintln!("bad json: {e}");
                Ok(())
            }
        }
    }).await;
}
```

---

## Host manifest management

> On macOS/Linux the manifest `path` **must be absolute** (use your built
> binary path). Chrome/Edge use `allowed_origins`; Firefox uses
> `allowed_extensions`. This crate abstracts the placement logic per browser.

### Install for multiple browsers

```ignore
// This is marked `ignore` so rustdoc won't try to compile it, because the exact
// signature of `install` (browsers/enums/scope) depends on your crate version.
// Use this as a template and adjust types to your API.

use std::path::Path;
use native_messaging::install::manifest::{install, Browser, Scope};

fn main() -> std::io::Result<()> {
    let path = Path::new("/absolute/path/to/target/release/host-binary");
    let allowed_origins: Vec<String> = vec![
        "chrome-extension://your_ext_id/".to_string(),
        "chrome-extension://another_id/".to_string(),
    ];

    let browsers = &[Browser::Chrome, Browser::Firefox];

    // Adjust arguments to match your `install` signature exactly.
    install("com.example.host", "Example Host", path, &allowed_origins, browsers, Scope::User)
}
```

### Verify / Remove

```ignore
// Marked `ignore` to avoid signature drift across versions.
// Adapt to your actual function signatures.

use native_messaging::install::manifest::{verify, remove, Browser, Scope};

fn main() -> std::io::Result<()> {
    // Some versions take only `name`, others also take browsers/scope.
    if verify("com.example.host")? {
        // Adjust remove signature as needed (browsers/scope).
        remove("com.example.host", &[Browser::Firefox], Scope::User)?;
    }
    Ok(())
}
```

---

## Extension-side (Chrome/Edge MV3)

```javascript
// background.js
const port = chrome.runtime.connectNative("com.example.host");
port.onMessage.addListener(msg => console.log("host:", msg));
port.postMessage({ type: "ping", payload: "hello", id: 1 });
```

Ensure your host manifest lists the extension (origin for Chrome/Edge or
extension ID for Firefox).

---

## Troubleshooting

- **No response?** Make sure you write to **stdout** (native messaging uses stdio).
- **Absolute path**: on macOS/Linux the `path` in the manifest must be **absolute**.
- Use `verify("com.example.host")` (or your version’s signature) to check install status.

---

## About these docs

This file provides a comprehensive crate-level guide (what you see now) and
then **inlines** the function-level items from your implementation using
`#[doc(inline)] pub use …` so everything appears in one place in `cargo doc`.

Run:

```sh
cargo doc --no-deps --open
```
"#]

// ========= Re-export the public API so it appears inline in these docs =========

// Keep using on-disk modules (e.g., src/host.rs, src/install/manifest.rs)
pub mod host;
pub mod install;

// Re-export host-side functions so they show up on the crate root page.
#[doc(inline)]
pub use host::{encode_message, event_loop, get_message, send_message};

// Re-export manifest utilities (only functions that are public).
#[doc(inline)]
pub use install::manifest::{install, remove, verify};

// Optional: convenience re-export of the submodule for easier navigation.
#[doc(inline)]
pub use install::manifest;
