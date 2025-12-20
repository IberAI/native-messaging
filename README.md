# native_messaging

A batteries-included Rust crate for **browser Native Messaging**:

- Build a **Native Messaging host** that talks to your extension over **stdin/stdout**
- Install, verify, and remove the **native host manifest** for multiple browsers
- Safe framing + size caps + structured errors (`NmError`)
- Cross-platform: **Windows / macOS / Linux**

This crate aims to be the “it just works” choice for [native messaging](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging).

---

## What is Native Messaging?

Native Messaging is how a browser extension talks to a local native process (your “host”).
The protocol is:

1. A 4-byte length prefix (`u32`, **native endianness**)
2. Followed by that many bytes of **UTF-8 JSON**

Your host reads from **stdin** and writes to **stdout**.

### Big gotchas (read this first)

- **Disconnect is normal:** when the extension disconnects or browser exits, stdin usually closes.
  Treat `NmError::Disconnected` as a normal shutdown.
- **Never log to stdout:** stdout is reserved for framed messages. Logging to stdout corrupts the stream.
  Log to **stderr** or a file.
- **Size limits are real:**
  - host → browser: **1 MiB** (enforced)
  - browser → host: **64 MiB** (enforced)

---

## Install

```bash
cargo add native_messaging
````

Tokio is required for the async host helpers (recommended). Use these features:

```toml
tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread", "sync"] }
```

---

## Quickstart: robust async host loop (recommended)

This is the easiest correct way to run a host continuously and reply to messages.

```rust
use native_messaging::host::{event_loop, NmError, Sender};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct In {
    ping: String,
}

#[derive(Serialize)]
struct Out {
    pong: String,
}

#[tokio::main]
async fn main() -> Result<(), NmError> {
    event_loop(|raw: String, send: Sender| async move {
        let incoming: In = serde_json::from_str(&raw).map_err(NmError::DeserializeJson)?;
        send.send(&Out { pong: incoming.ping }).await?;
        Ok(())
    })
    .await
}
```

### Logging (important)

Do **not** use `println!()` in a host. It writes to stdout and breaks the protocol.
Use stderr:

```rust
eprintln!("host starting"); // ✅ safe
// println!("host starting"); // ❌ unsafe
```

---

## JS extension example (Chrome/Chromium)

This is what the extension side typically looks like:

```js
const port = chrome.runtime.connectNative("com.example.host");

port.onMessage.addListener((msg) => {
  console.log("native reply:", msg);
});

port.onDisconnect.addListener(() => {
  console.log("native disconnected:", chrome.runtime.lastError);
});

port.postMessage({ ping: "hello" });
```

---

## Pure framing (unit-test friendly)

You can test framing without stdin/stdout using an in-memory buffer:

```rust
use native_messaging::host::{encode_message, decode_message, MAX_FROM_BROWSER};
use serde_json::json;
use std::io::Cursor;

let msg = json!({"hello": "world"});
let frame = encode_message(&msg).unwrap();

let mut cur = Cursor::new(frame);
let raw = decode_message(&mut cur, MAX_FROM_BROWSER).unwrap();

let back: serde_json::Value = serde_json::from_str(&raw).unwrap();
assert_eq!(back, msg);
```

---

## One-shot read/write helpers (convenience)

These helpers read one message from stdin and write one reply to stdout.
For production hosts, prefer `event_loop`.

```rust
use native_messaging::{get_message, send_message};
use native_messaging::host::NmError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct In { ping: String }

#[derive(Serialize)]
struct Out { pong: String }

#[tokio::main]
async fn main() -> Result<(), NmError> {
    let raw = get_message().await?;
    let incoming: In = serde_json::from_str(&raw).map_err(NmError::DeserializeJson)?;
    send_message(&Out { pong: incoming.ping }).await?;
    Ok(())
}
```

---

## Installing a manifest (config-driven browsers)

This crate includes an installer for writing/verifying/removing manifests for supported browsers.

**Browser allowlists differ by family:**

* Chromium-family uses `allowed_origins` like: `chrome-extension://<EXT_ID>/`
* Firefox-family uses `allowed_extensions` like: `your-addon@example.org`

```rust
use std::path::Path;
use native_messaging::{install, Scope};

let host_name = "com.example.host";
let description = "Example native messaging host";

// On macOS/Linux, this must be an absolute path.
let exe_path = Path::new("/absolute/path/to/host-binary");

// Chromium-family allow-list:
let allowed_origins = vec![
    "chrome-extension://your_extension_id/".to_string(),
];

// Firefox-family allow-list:
let allowed_extensions = vec![
    "your-addon@example.org".to_string(),
];

// Install for selected browsers by key:
let browsers = &["chrome", "firefox", "edge"];

install(
    host_name,
    description,
    exe_path,
    &allowed_origins,
    &allowed_extensions,
    browsers,
    Scope::User,
).unwrap();
```

### Verify installation

```rust
use native_messaging::{verify_installed, Scope};

let ok = verify_installed("com.example.host", None, Scope::User).unwrap();
assert!(ok);
```

### Remove a manifest

```rust
use native_messaging::{remove, Scope};

remove("com.example.host", &["chrome", "firefox", "edge"], Scope::User).unwrap();
```

---

## Troubleshooting

Native messaging failures are usually **manifest** issues, not code.

### “Specified native messaging host not found”

Check:

* The extension calls the exact same `host_name` you installed (case-sensitive).
* The manifest exists at the expected location (User vs System scope).
* The manifest JSON is valid.

### “Access to the specified native messaging host is forbidden”

Check:

* Chromium-family: `allowed_origins` contains exact `chrome-extension://<id>/`
* Firefox-family: `allowed_extensions` contains your addon ID

### “Native host has exited” / “Failed to start native messaging host”

Check:

* The manifest `path` points to a real executable.
* On macOS/Linux, manifest `path` is absolute.
* Your host does not log to stdout.
* Your host handles disconnect cleanly (EOF → `NmError::Disconnected`).

### Host prints weird JSON / extension can’t parse

This almost always means **stdout was corrupted** by logs.
Switch logging to stderr/file.

---

## API overview

Most users only need:

* Host:

  * `native_messaging::host::event_loop`
  * `native_messaging::host::Sender`
  * `native_messaging::host::NmError`
* Installer:

  * `native_messaging::install`
  * `native_messaging::verify_installed`
  * `native_messaging::remove`
  * `native_messaging::Scope`

---

## Notes for crate maintainers / contributors

### Run tests (including docs)

```bash
cargo test
cargo test --doc
```

### Clippy (strict)

```bash
cargo clippy -- -D warnings
```

---

## License

MIT
