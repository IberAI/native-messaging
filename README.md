# native_messaging

`native_messaging` is a Rust crate for building browser Native Messaging hosts.

It helps you:

- read and write the Native Messaging wire format over `stdin` / `stdout`
- run a robust async host loop with Tokio
- install, verify, and remove native host manifests for supported browsers
- avoid common protocol mistakes such as logging to `stdout`
- keep manifest JSON shaped correctly for Chromium-family and Firefox-family browsers

The crate targets Chrome, Chromium, Microsoft Edge, Brave, Vivaldi, Firefox, and LibreWolf on
Windows, macOS, and Linux.

Native Messaging is documented by MDN here:
https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging

---

## Native Messaging in one minute

A browser extension talks to a local native program through pipes:

- the browser writes framed JSON to the host process on `stdin`
- the host writes framed JSON replies to `stdout`
- logs must go to `stderr` or a file

Each message is:

1. a 4-byte unsigned length prefix in native byte order
2. followed by that many bytes of UTF-8 JSON

This crate handles the framing, size checks, and structured errors for you.

Important limits:

- host to browser: `1 MiB` enforced by this crate
- browser to host: `64 MiB` default cap, matching Chrome’s documented limit

Firefox and Microsoft Edge document larger browser-to-host limits. The default remains conservative
so a host can run safely across browser families.

---

## Install

```bash
cargo add native_messaging
```

Default features include:

- `tokio`: async host helpers such as `event_loop`, `get_message`, and `send_message`
- `install`: manifest path resolution and installer helpers
- `windows-registry`: Windows registry integration for browser manifest discovery

For the async examples below, your application also needs a Tokio runtime:

```toml
[dependencies]
native_messaging = "0.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

For framing-only usage without Tokio or installer support:

```toml
[dependencies]
native_messaging = { version = "0.3", default-features = false }
serde_json = "1"
```

---

## Example: async host loop

Use `event_loop` for a real host that should stay alive while the extension port is connected.

```rust
use native_messaging::host::{event_loop, NmError, Sender};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
struct Request {
    ping: String,
}

#[derive(Serialize)]
struct Response {
    pong: String,
}

#[tokio::main]
async fn main() -> Result<(), NmError> {
    event_loop(|raw: String, send: Sender| async move {
        let request: Request = match serde_json::from_str(&raw) {
            Ok(request) => request,
            Err(error) => {
                send.send(&json!({
                    "ok": false,
                    "error": "invalid_request",
                    "details": error.to_string(),
                }))
                .await?;
                return Ok(());
            }
        };

        send.send(&Response {
            pong: request.ping,
        })
        .await?;

        Ok(())
    })
    .await
}
```

Do not use `println!()` in a native messaging host. It writes to `stdout` and corrupts the protocol.

```rust
eprintln!("host started"); // safe
```

---

## Example: extension side

Chromium-family extensions typically connect like this:

```js
const port = chrome.runtime.connectNative("com.example.host");

port.onMessage.addListener((message) => {
  console.log("native reply", message);
});

port.onDisconnect.addListener(() => {
  console.log("native disconnected", chrome.runtime.lastError);
});

port.postMessage({ ping: "hello" });
```

Firefox extensions can use the same pattern through the `browser.runtime` API:

```js
const port = browser.runtime.connectNative("com.example.host");

port.onMessage.addListener((message) => {
  console.log("native reply", message);
});

port.postMessage({ ping: "hello" });
```

Your extension manifest must request the `nativeMessaging` permission.

---

## Example: one-shot helper

For small command-style hosts, you can read one message and write one response.
For long-running ports, prefer `event_loop`.

```rust
use native_messaging::{get_message, send_message};
use native_messaging::host::NmError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Request {
    ping: String,
}

#[derive(Serialize)]
struct Response {
    pong: String,
}

#[tokio::main]
async fn main() -> Result<(), NmError> {
    let raw = get_message().await?;
    let request: Request = serde_json::from_str(&raw).map_err(NmError::DeserializeJson)?;

    send_message(&Response {
        pong: request.ping,
    })
    .await?;

    Ok(())
}
```

---

## Example: framing without stdin/stdout

The sync framing functions work with any `Read` / `Write`, which makes protocol tests simple.

```rust
use native_messaging::host::{decode_message, encode_message, MAX_FROM_BROWSER};
use serde_json::json;
use std::io::Cursor;

let message = json!({ "hello": "world" });
let frame = encode_message(&message).unwrap();

let mut input = Cursor::new(frame);
let raw = decode_message(&mut input, MAX_FROM_BROWSER).unwrap();
let decoded: serde_json::Value = serde_json::from_str(&raw).unwrap();

assert_eq!(decoded, message);
```

---

## Installing native host manifests

Browsers discover native hosts through a JSON manifest. The manifest contains:

- the host name used by `connectNative`
- a description
- the path to the native host executable
- `type: "stdio"`
- an allowlist of extensions that may connect

This crate writes the correct manifest shape for each browser family.

Chromium-family browsers use `allowed_origins`:

```text
chrome-extension://<extension-id>/
```

Firefox-family browsers use `allowed_extensions`:

```text
your-addon@example.org
```

Supported browser keys:

- Chromium-family: `chrome`, `chrome_for_testing`, `edge`, `edge_beta`, `edge_dev`,
  `edge_canary`, `chromium`, `brave`, `vivaldi`
- Firefox-family: `firefox`, `librewolf`

Some keys are platform-specific. For example, the Edge channel keys model the macOS user-data
locations documented by Microsoft Edge.

### Install manifests

```rust
use native_messaging::{install, Scope};
use std::path::Path;

fn main() -> std::io::Result<()> {
    let host_name = "com.example.host";
    let description = "Example native messaging host";

    // On macOS and Linux this must be absolute.
    let exe_path = Path::new("/absolute/path/to/host-binary");

    let chromium_origins = vec![
        "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/".to_string(),
    ];

    let firefox_extensions = vec![
        "native-host@example.org".to_string(),
    ];

    install(
        host_name,
        description,
        exe_path,
        &chromium_origins,
        &firefox_extensions,
        &["chrome", "edge", "firefox"],
        Scope::User,
    )
}
```

`Scope::User` is recommended for development and most desktop applications.
`Scope::System` may require elevated permissions.

### Verify installation

```rust
use native_messaging::{verify_installed, Scope};

fn main() -> std::io::Result<()> {
    let ok = verify_installed(
        "com.example.host",
        Some(&["chrome", "firefox"]),
        Scope::User,
    )?;

    assert!(ok);
    Ok(())
}
```

Passing `None` checks every browser key configured for the current OS and scope.

### Inspect manifest paths

```rust
use native_messaging::{manifest_paths, Scope};

fn main() -> std::io::Result<()> {
    for path in manifest_paths("firefox", Scope::System, "com.example.host")? {
        eprintln!("{}", path.display());
    }

    Ok(())
}
```

`manifest_path` returns the primary install path. `manifest_paths` returns every configured lookup
path, including documented alternates such as Firefox’s `/usr/lib64` location on Linux.

### Remove manifests

```rust
use native_messaging::{remove, Scope};

fn main() -> std::io::Result<()> {
    remove("com.example.host", &["chrome", "edge", "firefox"], Scope::User)
}
```

---

## Host names and validation

The installer validates inputs before writing manifest files.

Chromium-family host names:

- lowercase ASCII letters
- digits
- underscores
- dots
- no leading dot
- no trailing dot
- no consecutive dots

Firefox-family host names follow the same rules but may include uppercase ASCII letters.

The installer also checks that:

- Chromium-family allowlists are `chrome-extension://...` origins
- Firefox-family allowlists are non-empty extension IDs without whitespace
- macOS/Linux executable paths are absolute

---

## Recommended message shape

Native Messaging only requires JSON. For real applications, use a stable envelope so requests and
responses can evolve.

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
struct RequestEnvelope {
    #[serde(rename = "type")]
    ty: String,
    id: Option<String>,
    payload: Value,
}

#[derive(Serialize)]
struct ResponseEnvelope<T> {
    #[serde(rename = "type")]
    ty: &'static str,
    id: Option<String>,
    ok: bool,
    payload: T,
}
```

Avoid designs where extension messages can run arbitrary shell commands. Treat the extension as an
untrusted client and validate every request.

---

## Troubleshooting

### “Specified native messaging host not found”

Check:

- the extension uses the exact same host name as the manifest
- the manifest was installed for the right browser key
- user vs system scope matches where the browser is looking
- the manifest filename is `<host_name>.json`
- the manifest JSON is valid

### “Access to the specified native messaging host is forbidden”

Check:

- Chromium-family: `allowed_origins` contains your extension ID
- Firefox-family: `allowed_extensions` contains your add-on ID
- published extension IDs may differ from development/sideloaded IDs

### “Native host has exited” or “Failed to start native messaging host”

Check:

- the manifest `path` points to a real executable
- the executable has run permissions on macOS/Linux
- macOS/Linux manifest `path` is absolute
- the host does not write logs to `stdout`
- the host handles disconnect as normal shutdown

### Extension cannot parse replies

The most common cause is stdout corruption. Do not print logs, prompts, progress bars, or panic
messages to `stdout`. Use `stderr` or a file.

---

## API overview

Common host APIs:

- `native_messaging::host::event_loop`
- `native_messaging::host::Sender`
- `native_messaging::host::NmError`
- `native_messaging::host::encode_message`
- `native_messaging::host::decode_message`
- `native_messaging::host::send_json`
- `native_messaging::host::recv_json`

Common installer APIs:

- `native_messaging::install`
- `native_messaging::verify_installed`
- `native_messaging::remove`
- `native_messaging::manifest_path`
- `native_messaging::manifest_paths`
- `native_messaging::Scope`

---

## Feature flags

```toml
[features]
default = ["tokio", "install", "windows-registry"]
tokio = ["dep:tokio"]
install = ["dep:toml", "dep:once_cell"]
windows-registry = ["install", "dep:winreg"]
```

`windows-registry` is enabled by default and only matters on Windows. Disable default features for
framing-only usage, or enable `install,windows-registry` explicitly when building a custom feature
set that needs Windows registry installation and verification.

---

## Maintainer checks

```bash
cargo fmt --all -- --check
cargo test --all-features --all-targets --no-fail-fast
cargo test --no-default-features --all-targets --no-fail-fast
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features
cargo package
```

---

## License

MIT
