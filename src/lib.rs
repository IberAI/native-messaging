#![doc = r#"
# Native Messaging — Async Rust host + manifest installer

This crate helps you build a **Native Messaging host** for WebExtensions and
install/manage the **host manifest** for multiple browsers across platforms.

It exposes two main areas:

- `host` — encode/read/write native-messaging frames over **stdin/stdout**, plus
  an async `event_loop` for handling messages.
- `install` — create **host manifests**, **verify** their presence, and **remove**
  them for supported browsers.

The installer side is **config-driven** via an embedded `browsers.toml`. The
embedded config includes the following browser keys (as of this release):  
`"chrome"`, `"edge"`, `"chromium"`, `"brave"`, `"vivaldi"`, `"firefox"`, `"librewolf"`. 

## Encode a JSON message into a native-messaging frame

```rust
use native_messaging::encode_message;
use serde::Serialize;

#[derive(Serialize)]
struct Msg {
    hello: &'static str,
}

// Encode to a 4-byte-length-prefixed frame:
let frame = encode_message(&Msg { hello: "world" }).unwrap();
assert!(frame.len() >= 4);
````

## Read one message and reply (async)

```no_run
use native_messaging::{get_message, send_message};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct In { ping: String }

#[derive(Serialize)]
struct Out { pong: String }

let rt = tokio::runtime::Runtime::new().unwrap();
rt.block_on(async {
    // Incoming messages are framed and decoded as a raw JSON string:
    let raw = get_message().await.unwrap();

    // Parse to your input type:
    let incoming: In = serde_json::from_str(&raw).unwrap();

    // Build and send a reply:
    let reply = Out { pong: incoming.ping };
    send_message(&reply).await.unwrap();
});
```

## Run the event loop

```no_run
use native_messaging::{event_loop, send_message};
use serde::Serialize;
use std::io;

#[derive(Serialize)]
struct Out { pong: String }

let rt = tokio::runtime::Runtime::new().unwrap();
rt.block_on(async {
    event_loop(|msg: String| async move {
        // In real code, you'd parse `msg` into a structured type first.
        let reply = Out { pong: msg };
        send_message(&reply).await?;
        Ok::<(), io::Error>(())
    })
    .await;
});
```

## Install a manifest (config-driven browsers)

Browsers are selected by **string keys** from the embedded `browsers.toml`. 

* `allowed_origins` is used for **Chromium-family** browsers
  (`chrome`, `edge`, `chromium`, `brave`, `vivaldi`).
* `allowed_extensions` is used for **Firefox-family** browsers
  (`firefox`, `librewolf`).

```no_run
use std::path::Path;
use native_messaging::install::{install, Scope};

let host_name = "com.example.host";
let description = "Example native messaging host";

// IMPORTANT: On macOS/Linux, this must be an absolute path.
let exe_path = Path::new("/absolute/path/to/host-binary");

// Chromium-family allow-list:
let allowed_origins: Vec<String> = vec![
    "chrome-extension://your_ext_id/".to_string(),
];

// Firefox-family allow-list:
let allowed_extensions: Vec<String> = vec![
    "your-addon@example.org".to_string(),
];

// Pick which browsers to install for (keys from browsers.toml):
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

## Verify installation

On Windows, verification is **registry-aware** for browsers that require registry
pointers; on macOS/Linux, verification checks the expected manifest file path.

```no_run
use native_messaging::install::{verify_installed, Scope};

let host_name = "com.example.host";

// Verify for all configured browsers (pass None) in user scope:
let ok = verify_installed(host_name, None, Scope::User).unwrap();
assert!(ok);
```

## Remove a manifest

```no_run
use native_messaging::install::{remove, Scope};

let host_name = "com.example.host";
let browsers = &["chrome", "firefox", "edge"];

remove(host_name, browsers, Scope::User).unwrap();
```

"#]

pub mod host;
pub mod install;

// Re-export host-side functions on the crate root.
#[doc(inline)]
pub use host::{encode_message, event_loop, get_message, send_message};

// Re-export install API (functions + Scope) on the crate root.
//
// NOTE: These must match the public symbols defined in `src/install/manifest.rs`.
// If you rename the functions there, update these exports accordingly.
#[doc(inline)]
pub use install::manifest::{install, remove, verify_installed};
#[doc(inline)]
pub use install::paths::Scope;

// Optional: convenience re-export of modules for navigation.
#[doc(inline)]
pub use install::manifest;
#[doc(inline)]
pub use install::paths;
