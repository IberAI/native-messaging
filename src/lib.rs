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

## Encode a JSON message into a native-messaging frame

```rust
use native_messaging::host::encode_message;
use serde::Serialize;

#[derive(Serialize)]
struct Msg {
    hello: &'static str,
}

// Encode to a 4-byte-length-prefixed frame:
let frame = encode_message(&Msg { hello: "world" }).unwrap();
assert!(frame.len() >= 4);
```

## Read a message and reply on stdout (async)

```no_run
use native_messaging::host::{get_message, send_message};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct In { ping: String }

#[derive(Serialize)]
struct Out { pong: String }

// Create a minimal Tokio runtime just for the example:
let rt = tokio::runtime::Runtime::new().unwrap();
rt.block_on(async {
    // Your API returns a raw String
    let raw = get_message().await.unwrap();

    // Parse to your input type
    let incoming: In = serde_json::from_str(&raw).unwrap();

    // Build and send a reply
    let reply = Out { pong: incoming.ping };
    send_message(&reply).await.unwrap();
});
```

## Run the event loop

```no_run
use native_messaging::host::{event_loop, send_message};
use serde::Serialize;
use std::io;

#[derive(Serialize)]
struct Out { pong: String }

// The event loop receives raw String messages and expects the handler future to
// resolve to io::Result<()>.
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

## Create and install a manifest

```no_run
use std::path::Path;
use native_messaging::install::manifest::{install, Browser, Scope};

let name = "com.example.host";
let description = "Example native messaging host";
let path = Path::new("/absolute/path/to/target/release/host-binary");
let allowed_origins: Vec<String> = vec![
    "chrome-extension://your_ext_id/".to_string(),
    "chrome-extension://another_id/".to_string(),
];
// List of Chrome/Firefox extension IDs that can talk to your host (if applicable)
let allowed_extensions: Vec<String> = vec![];

let browsers = &[Browser::Chrome, Browser::Firefox];

// Adjust arguments to your needs; Scope::User for per-user install.
// NOTE: This matches the signature hinted by the compiler: 
// install(name: &str, description: &str, path: &Path, allowed_origins: &[String],
//         allowed_extensions: &[String], browsers: &[Browser], scope: Scope)
install(
    name,
    description,
    path,
    &allowed_origins,
    &allowed_extensions,
    browsers,
    Scope::User
).unwrap();
```
"#]

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
