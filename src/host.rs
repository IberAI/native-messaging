//! Native Messaging host-side protocol helpers.
//!
//! This module provides:
//! - Framing: 4-byte native-endian length prefix + UTF-8 JSON payload
//! - Size limits: 1 MiB outgoing (host->browser), 64 MiB incoming (browser->host)
//! - Structured errors with actionable messages
//! - Sync helpers for Read/Write
//! - Tokio helpers (requires tokio features: rt, sync, macros, rt-multi-thread)

use serde::{de::DeserializeOwned, Serialize};
use std::{
    error::Error as StdError,
    fmt,
    io::{self, Read, Write},
};

/// 1 MB (host -> browser)
pub const MAX_TO_BROWSER: usize = 1_048_576;

/// 64 MB (browser -> host)
pub const MAX_FROM_BROWSER: usize = 64 * 1_048_576;
/// Rich, actionable error type.
#[derive(Debug)]
pub enum NmError {
    /// Browser closed stdin / disconnected (clean shutdown).
    Disconnected,

    /// Outgoing payload too large for the browser’s limit.
    OutgoingTooLarge { len: usize, max: usize },

    /// Incoming payload too large (either user max_size or hard cap).
    IncomingTooLarge { len: usize, max: usize },

    /// Invalid UTF-8 in the incoming JSON bytes.
    IncomingNotUtf8(std::string::FromUtf8Error),

    /// JSON serialization failed.
    SerializeJson(serde_json::Error),

    /// JSON deserialization failed.
    DeserializeJson(serde_json::Error),

    /// Tokio join error (if awaiting a JoinHandle).
    TokioJoin(tokio::task::JoinError),

    /// Oneshot receive error (sender dropped before sending).
    OneshotRecv(tokio::sync::oneshot::error::RecvError),

    /// Underlying I/O error.
    Io(io::Error),
}

impl fmt::Display for NmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use NmError::*;
        match self {
            Disconnected => write!(f, "native messaging disconnected (stdin closed)"),
            OutgoingTooLarge { len, max } => write!(
                f,
                "outgoing native message is {len} bytes (max {max}); \
                 reduce size (chunk/compress) before sending"
            ),
            IncomingTooLarge { len, max } => write!(
                f,
                "incoming native message is {len} bytes (max {max}); \
                 extension must send smaller messages (chunk/compress)"
            ),
            IncomingNotUtf8(e) => write!(f, "incoming native message is not valid UTF-8: {e}"),
            SerializeJson(e) => write!(f, "failed to serialize JSON: {e}"),
            DeserializeJson(e) => write!(f, "failed to deserialize JSON: {e}"),
            TokioJoin(e) => write!(f, "internal task join error: {e}"),
            OneshotRecv(e) => write!(f, "internal oneshot receive error: {e}"),
            Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl StdError for NmError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        use NmError::*;
        match self {
            IncomingNotUtf8(e) => Some(e),
            SerializeJson(e) => Some(e),
            DeserializeJson(e) => Some(e),
            TokioJoin(e) => Some(e),
            OneshotRecv(e) => Some(e),
            Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for NmError {
    fn from(e: io::Error) -> Self {
        NmError::Io(e)
    }
}

/// Encode any serde-serializable value into the native-messaging frame:
/// 4-byte native-endian length + JSON bytes.
///
/// Returns the full frame bytes (len prefix + payload).
pub fn encode_message<T: Serialize>(msg: &T) -> Result<Vec<u8>, NmError> {
    let json = serde_json::to_vec(msg).map_err(NmError::SerializeJson)?;
    if json.len() > MAX_TO_BROWSER {
        return Err(NmError::OutgoingTooLarge {
            len: json.len(),
            max: MAX_TO_BROWSER,
        });
    }

    let mut out = Vec::with_capacity(4 + json.len());
    out.extend_from_slice(&(json.len() as u32).to_ne_bytes());
    out.extend_from_slice(&json);
    Ok(out)
}

/// Decode a single framed message from a reader.
///
/// - `Ok(Some(String))` => got a message
/// - `Ok(None)` => clean EOF (browser disconnected)
/// - `Err(_)` => malformed frame or I/O failure
pub fn decode_message_opt<R: Read>(
    reader: &mut R,
    max_size: usize,
) -> Result<Option<String>, NmError> {
    let cap = max_size.min(MAX_FROM_BROWSER);

    // Read len prefix with clean EOF handling:
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(NmError::Io(e)),
    }

    let len = u32::from_ne_bytes(len_buf) as usize;
    if len > cap {
        return Err(NmError::IncomingTooLarge { len, max: cap });
    }

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).map_err(NmError::Io)?;

    String::from_utf8(buf)
        .map(Some)
        .map_err(NmError::IncomingNotUtf8)
}

/// Convenience: decode a required message; returns `Disconnected` on EOF.
pub fn decode_message<R: Read>(reader: &mut R, max_size: usize) -> Result<String, NmError> {
    decode_message_opt(reader, max_size)?.ok_or(NmError::Disconnected)
}

/// Decode and parse JSON into a typed value.
pub fn recv_json<T: DeserializeOwned, R: Read>(
    reader: &mut R,
    max_size: usize,
) -> Result<T, NmError> {
    let s = decode_message(reader, max_size)?;
    serde_json::from_str::<T>(&s).map_err(NmError::DeserializeJson)
}

/// Encode and write a message frame to a writer.
pub fn send_frame<W: Write>(writer: &mut W, frame: &[u8]) -> Result<(), NmError> {
    writer.write_all(frame).map_err(NmError::Io)?;
    writer.flush().map_err(NmError::Io)?;
    Ok(())
}

/// Encode and write a JSON message directly to a writer.
pub fn send_json<T: Serialize, W: Write>(writer: &mut W, msg: &T) -> Result<(), NmError> {
    let frame = encode_message(msg)?;
    send_frame(writer, &frame)
}

// -----------------------------
// Tokio async API
// -----------------------------
//
// Requires tokio features: "rt", "sync", "macros" (and often "rt-multi-thread").
//
// NOTE: This async API uses stdin/stdout directly, as expected for native messaging hosts.
// If you want fully deterministic integration tests, add an injectable Host<R,W> wrapper.

/// Spawn a background reader that produces raw JSON strings.
///
/// Returns a receiver that yields `Ok(String)` messages, or `Err(NmError)` on failure.
/// On EOF, it sends `Err(NmError::Disconnected)` and then ends.
pub fn spawn_reader(max_size: usize) -> tokio::sync::mpsc::Receiver<Result<String, NmError>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, NmError>>(32);

    tokio::task::spawn_blocking(move || {
        let mut stdin = io::stdin();
        loop {
            match decode_message_opt(&mut stdin, max_size) {
                Ok(Some(msg)) => {
                    if tx.blocking_send(Ok(msg)).is_err() {
                        // Receiver dropped; shut down.
                        break;
                    }
                }
                Ok(None) => {
                    let _ = tx.blocking_send(Err(NmError::Disconnected));
                    break;
                }
                Err(e) => {
                    let _ = tx.blocking_send(Err(e));
                    break;
                }
            }
        }
    });

    rx
}

/// Spawn a background writer that accepts already-framed bytes.
pub fn spawn_writer() -> tokio::sync::mpsc::Sender<Vec<u8>> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);

    tokio::task::spawn_blocking(move || {
        let mut stdout = io::stdout();

        while let Some(frame) = rx.blocking_recv() {
            // If writing fails, the browser likely disconnected.
            if stdout.write_all(&frame).is_err() {
                break;
            }
            if stdout.flush().is_err() {
                break;
            }
        }
    });

    tx
}

/// Read one message asynchronously (spawns one blocking task).
///
/// If you call this in a loop, prefer `spawn_reader` to avoid spawn-per-message overhead.
pub async fn get_message() -> Result<String, NmError> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, NmError>>();

    tokio::task::spawn_blocking(move || {
        let mut stdin = io::stdin();
        let res = decode_message(&mut stdin, MAX_FROM_BROWSER);
        let _ = tx.send(res);
    });

    rx.await.map_err(NmError::OneshotRecv)?
}

/// Send one message asynchronously (spawns one blocking task).
///
/// If you send frequently, prefer `spawn_writer` and use `Sender`.
pub async fn send_message<T: Serialize>(msg: &T) -> Result<(), NmError> {
    let frame = encode_message(msg)?;
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), NmError>>();

    tokio::task::spawn_blocking(move || {
        let mut stdout = io::stdout();
        let res = send_frame(&mut stdout, &frame);
        let _ = tx.send(res);
    });

    rx.await.map_err(NmError::OneshotRecv)?
}

/// A handle you can clone and use inside handlers to send replies safely.
#[derive(Clone)]
pub struct Sender {
    pub writer: tokio::sync::mpsc::Sender<Vec<u8>>,
}

impl Sender {
    /// Send any JSON-serializable value to the browser.
    pub async fn send<T: Serialize>(&self, msg: &T) -> Result<(), NmError> {
        let frame = encode_message(msg)?;
        self.writer
            .send(frame)
            .await
            .map_err(|_| NmError::Disconnected)?;
        Ok(())
    }
}

/// Robust event loop using the dedicated reader and writer.
///
/// - Stops cleanly on `Disconnected`.
/// - Never panics.
/// - Uses bounded channels to avoid unbounded memory growth.
///
/// `handler` receives:
/// - the raw JSON string
/// - a `Sender` to respond
pub async fn event_loop<F, Fut>(mut handler: F) -> Result<(), NmError>
where
    F: FnMut(String, Sender) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), NmError>> + Send,
{
    let mut rx = spawn_reader(MAX_FROM_BROWSER);
    let writer = spawn_writer();
    let sender = Sender { writer };

    while let Some(item) = rx.recv().await {
        match item {
            Ok(msg) => handler(msg, sender.clone()).await?,
            Err(NmError::Disconnected) => return Ok(()),
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
