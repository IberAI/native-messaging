use serde::Serialize;
use std::io::{self, Read, Write};

const MAX_TO_BROWSER: usize = 1_048_576; // 1 MB (host -> browser)
const MAX_FROM_BROWSER: usize = 64 * 1_048_576; // 64 MB (browser -> host)

#[inline]
fn read_exact_u32_len<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    Ok(u32::from_ne_bytes(len_buf))
}

/// Encode any serde-serializable value into the native-messaging frame:
/// 4-byte native-endian length + JSON bytes.
pub fn encode_message<T: Serialize>(msg: &T) -> io::Result<Vec<u8>> {
    let json = serde_json::to_vec(msg)?;
    if json.len() > MAX_TO_BROWSER {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "outgoing message exceeds 1MB",
        ));
    }
    let mut out = Vec::with_capacity(4 + json.len());
    out.extend_from_slice(&(json.len() as u32).to_ne_bytes());
    out.extend_from_slice(&json);
    Ok(out)
}

/// Decode a single framed message from a reader (useful in tests).
pub fn decode_message<R: Read>(reader: &mut R, max_size: usize) -> io::Result<String> {
    // CHANGED: pass &mut reader (reborrow) to avoid moving it
    let len = read_exact_u32_len(&mut *reader)? as usize;
    let cap = max_size.min(MAX_FROM_BROWSER);
    if len > cap {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "incoming message too large",
        ));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub async fn get_message() -> io::Result<String> {
    tokio::task::spawn_blocking(move || {
        let mut stdin = io::stdin();
        decode_message(&mut stdin, MAX_FROM_BROWSER)
    })
    .await
    .unwrap()
}

pub async fn send_message<T: Serialize>(msg: &T) -> io::Result<()> {
    let frame = encode_message(msg)?;
    tokio::task::spawn_blocking(move || {
        let mut stdout = io::stdout();
        stdout.write_all(&frame)?;
        stdout.flush()?;
        Ok(())
    })
    .await
    .unwrap()
}

pub async fn event_loop<F, Fut>(mut handler: F) -> io::Result<()>
where
    F: FnMut(String) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = io::Result<()>> + Send + 'static,
{
    loop {
        let msg = get_message().await?;
        handler(msg).await?;
    }
}
