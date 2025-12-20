//! # native_messaging
//!
//! A batteries-included Rust crate for **browser Native Messaging**:
//!
//! - Build a **native host** that talks to your extension over **stdin/stdout**
//! - Install, verify, and remove **native host manifests** for multiple browsers
//! - Safer, structured errors and correct size limits
//!
//! The goal is to be the “it just works” crate for native messaging—especially on the
//! parts that usually waste hours (manifest placement, allowlists, disconnect handling,
//! and accidentally breaking the protocol with logs).
//!
//! ---
//!
//! ## What is Native Messaging?
//!
//! Native Messaging is the mechanism by which a browser extension talks to a local
//! native process (your “host”) using standard I/O pipes.
//!
//! The wire protocol is:
//!
//! 1. The sender writes a **4-byte length prefix** (`u32`) in **native endianness**.
//! 2. Then writes **that many bytes** of UTF-8 JSON.
//!
//! The host reads from **stdin** and writes replies to **stdout**.
//!
//! ### Most important gotchas (read this first)
//!
//! - **Disconnect is normal:** when the extension disconnects (or browser exits), the browser
//!   typically closes the host’s stdin. Treat [`host::NmError::Disconnected`] as a normal shutdown.
//! - **Message limits:**
//!   - Host → browser: this crate enforces **1 MiB** ([`host::MAX_TO_BROWSER`]).
//!   - Browser → host: this crate enforces **64 MiB** ([`host::MAX_FROM_BROWSER`]) to match Chrome’s documented limit.
//! - **Never log to stdout:** stdout is reserved for framed protocol messages. Use stderr or a file.
//! - **Manifest mismatch is the #1 failure:** “host not found” / “failed to start” is almost always
//!   a manifest path/name/allowlist issue.
//!
//! ---
//!
//! ## Crate layout
//!
//! - [`host`] — framing + stdio helpers + a high-level async event loop.
//! - [`install`] — config-driven browser manifest install/verify/remove.
//!
//! ---
//!
//! ## Cargo setup (recommended)
//!
//! This crate’s async helpers require Tokio. Ensure your `Cargo.toml` includes Tokio features that
//! expose `tokio::sync` and the runtime:
//!
//! ```toml
//! [dependencies]
//! tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread", "sync"] }
//! ```
//!
//! If you don’t want Tokio, you can still use the sync framing helpers in [`host`]
//! with any `Read`/`Write`.
//!
//! ---
//!
//! ## Best practices for hosts
//!
//! ### 1) Logging: don’t corrupt stdout
//!
//! **Do not use `println!()`** in a native messaging host. It writes to stdout and will corrupt
//! the protocol stream. Prefer:
//! - `eprintln!(...)` for simple debugging, or
//! - a logging framework (`tracing`, `log` + `env_logger`), configured to write to **stderr** or a file.
//!
//! Example (stderr is safe):
//!
//! ```no_run
//! # fn main() {
//! eprintln!("host starting"); // ✅ safe: goes to stderr
//! // println!("host starting"); // ❌ unsafe: corrupts stdout protocol
//! # }
//! ```
//!
//! ### 2) Use a message envelope (recommended schema)
//!
//! Raw JSON strings work, but most real apps benefit from a tiny “envelope” pattern:
//!
//! - `type`: which command/event is this?
//! - `id`: optional correlation ID so you can match requests to responses
//! - `payload`: command-specific data
//!
//! Example types:
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use serde_json::Value;
//!
//! #[derive(Deserialize)]
//! struct RequestEnvelope {
//!     #[serde(rename = "type")]
//!     ty: String,
//!     id: Option<String>,
//!     payload: Value,
//! }
//!
//! #[derive(Serialize)]
//! struct ResponseEnvelope<T> {
//!     #[serde(rename = "type")]
//!     ty: &'static str,
//!     id: Option<String>,
//!     ok: bool,
//!     payload: T,
//! }
//! ```
//!
//! This makes your protocol easier to evolve without breaking clients.
//!
//! ### 3) Be strict about what you accept
//!
//! Native messaging is a powerful bridge to the local machine. Best practice:
//! - Only allow known extension IDs in the manifest allowlist
//! - Validate message shapes (don’t `unwrap()` JSON parsing in production)
//! - Avoid “run arbitrary command” designs unless you strongly sandbox/validate inputs
//!
//! ---
//!
//! ## Quick start: robust async host loop (recommended)
//!
//! This is the best default for real hosts: it reads messages continuously and replies.
//! It also demonstrates a common best practice: **don’t crash the host** on bad input—
//! instead, respond with an error message (or ignore invalid messages).
//!
//! ```no_run
//! use native_messaging::host::{event_loop, NmError, Sender};
//! use serde::{Deserialize, Serialize};
//! use serde_json::json;
//!
//! #[derive(Deserialize)]
//! struct In {
//!     ping: String,
//! }
//!
//! #[derive(Serialize)]
//! struct Out {
//!     pong: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), NmError> {
//!     event_loop(|raw: String, send: Sender| async move {
//!         // Best practice: handle parse errors gracefully.
//!         let incoming: In = match serde_json::from_str(&raw) {
//!             Ok(v) => v,
//!             Err(e) => {
//!                 // Option A: reply with an error payload the extension can handle.
//!                 // Keep it simple: JSON object with error info.
//!                 let err_msg = json!({
//!                     "ok": false,
//!                     "error": "invalid_request",
//!                     "details": e.to_string(),
//!                 });
//!                 // Ignore send errors here (disconnect, etc) by propagating them:
//!                 send.send(&err_msg).await?;
//!                 return Ok(());
//!             }
//!         };
//!
//!         // Normal reply
//!         send.send(&Out { pong: incoming.ping }).await?;
//!         Ok(())
//!     })
//!     .await
//! }
//! ```
//!
//! ### What happens on disconnect?
//!
//! When the browser closes stdin, the loop stops and returns `Ok(())`.
//! Disconnect is a normal lifecycle event for native messaging hosts.
//!
//! ---
//!
//! ## Pure framing (runnable example)
//!
//! You can unit-test framing without stdin/stdout by using an in-memory buffer.
//!
//! ```rust
//! use native_messaging::host::{encode_message, decode_message, MAX_FROM_BROWSER};
//! use serde_json::json;
//! use std::io::Cursor;
//!
//! let msg = json!({"hello": "world", "n": 42});
//! let frame = encode_message(&msg).unwrap();
//!
//! // Decode back from a Cursor
//! let mut cur = Cursor::new(frame);
//! let raw = decode_message(&mut cur, MAX_FROM_BROWSER).unwrap();
//! let back: serde_json::Value = serde_json::from_str(&raw).unwrap();
//! assert_eq!(back, msg);
//! ```
//!
//! ---
//!
//! ## One-shot I/O (read one message, send one reply)
//!
//! These are convenience helpers. For production, prefer [`event_loop`].
//!
//! ```no_run
//! use native_messaging::{get_message, send_message};
//! use native_messaging::host::NmError;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct In { ping: String }
//!
//! #[derive(Serialize)]
//! struct Out { pong: String }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), NmError> {
//!     let raw = get_message().await?;
//!     let incoming: In = serde_json::from_str(&raw).map_err(NmError::DeserializeJson)?;
//!     send_message(&Out { pong: incoming.ping }).await?;
//!     Ok(())
//! }
//! ```
//!
//! ---
//!
//! ## Installing the manifest (config-driven browsers)
//!
//! The installer uses an embedded browser configuration (`browsers.toml`) that defines
//! install locations per OS and (on Windows) registry key templates.
//!
//! Supported browser keys in the embedded config include:
//! - Chromium-family: `chrome`, `edge`, `chromium`, `brave`, `vivaldi`
//! - Firefox-family: `firefox`, `librewolf`
//!
//! The manifest allowlist fields differ by browser family:
//!
//! - Chromium-family uses `allowed_origins` (e.g. `chrome-extension://<EXT_ID>/`)
//! - Firefox-family uses `allowed_extensions` (your addon ID, often email-like)
//!
//! ### Scope and permissions
//!
//! - [`Scope::User`] installs into the current user’s profile locations (recommended for development
//!   and for most desktop apps).
//! - System-wide installs may require elevated privileges depending on OS and target locations.
//!
//! ```no_run
//! use std::path::Path;
//! use native_messaging::{install, Scope};
//!
//! let host_name = "com.example.host";
//! let description = "Example native messaging host";
//!
//! // On macOS/Linux, this must be an absolute path.
//! let exe_path = Path::new("/absolute/path/to/host-binary");
//!
//! // Chromium-family allow-list:
//! let allowed_origins = vec![
//!     "chrome-extension://your_extension_id/".to_string(),
//! ];
//!
//! // Firefox-family allow-list:
//! let allowed_extensions = vec![
//!     "your-addon@example.org".to_string(),
//! ];
//!
//! let browsers = &["chrome", "firefox", "edge"];
//!
//! install(
//!     host_name,
//!     description,
//!     exe_path,
//!     &allowed_origins,
//!     &allowed_extensions,
//!     browsers,
//!     Scope::User,
//! ).unwrap();
//! ```
//!
//! ### Verify installation
//!
//! ```no_run
//! use native_messaging::{verify_installed, Scope};
//!
//! let ok = verify_installed("com.example.host", None, Scope::User).unwrap();
//! assert!(ok);
//! ```
//!
//! ### Remove a manifest
//!
//! ```no_run
//! use native_messaging::{remove, Scope};
//!
//! remove("com.example.host", &["chrome", "firefox", "edge"], Scope::User).unwrap();
//! ```
//!
//! ---
//!
//! ## Troubleshooting (read this if “it doesn’t work”)
//!
//! Native Messaging failures are usually configuration issues, not code issues.
//!
//! ### 1) “Specified native messaging host not found”
//! Check:
//! - The extension calls the exact same `host_name` you installed.
//! - The manifest exists at the expected location (user vs system scope).
//! - The manifest JSON is valid.
//!
//! ### 2) “Access to the specified native messaging host is forbidden”
//! Check:
//! - Chromium-family: `allowed_origins` contains the exact `chrome-extension://<id>/` for your extension.
//! - Firefox-family: `allowed_extensions` contains the correct addon ID.
//!
//! ### 3) “Native host has exited” / “Failed to start native messaging host”
//! Check:
//! - The manifest `path` points to a real executable.
//! - On macOS/Linux, manifest `path` is absolute.
//! - Your host does not write logs to stdout.
//! - Your host handles disconnect cleanly (EOF → [`host::NmError::Disconnected`]).
//!
//! ### 4) My host works once and then stops
//! This usually means you read only one message and exited.
//! Prefer [`event_loop`] for hosts meant to stay running.
//!
//! ---
//!
//! ## API re-exports
//!
//! This crate re-exports the most common entry points at the crate root for convenience:
//!
//! - Host helpers: [`encode_message`], [`get_message`], [`send_message`], [`event_loop`]
//! - Installer helpers: [`install`], [`verify_installed`], [`remove`], and [`Scope`]
//!
//! For more advanced control (framing, typed decoding, sender handle, and error variants),
//! see the [`host`] module directly.

pub mod host;
pub mod install;

// -------- Host re-exports --------

#[doc(inline)]
pub use host::{encode_message, event_loop, get_message, send_message};

// -------- Install re-exports --------

// NOTE: These must match your install module’s public symbols.
// If you rename functions in install, update these exports accordingly.
#[doc(inline)]
pub use install::manifest::{install, remove, verify_installed};
#[doc(inline)]
pub use install::paths::Scope;

// Optional: module re-exports for discoverability in docs.rs navigation.
#[doc(inline)]
pub use install::manifest;
#[doc(inline)]
pub use install::paths;
