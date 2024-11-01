use serde::Serialize;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::select;

/// Encodes a message according to the native messaging protocol.
/// <https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging#App_side>
///
/// The message is serialized into compact JSON (without extra whitespace),
/// prefixed with a 4-byte length in native byte order.
///
/// # Examples
///
/// ```
/// use native_messaging::host::encode_message;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct MyMessage {
///     key: String,
/// }
///
/// let message = MyMessage { key: "value".to_string() };
/// let encoded = encode_message(&message).expect("Encoding failed");
/// assert!(encoded.len() > 4);  // Message contains 4-byte length + content.
/// ```
///
/// # Errors
/// This function returns a `serde_json::Error` if serialization fails.
pub fn encode_message<T>(message_content: &T) -> Result<Vec<u8>, serde_json::Error>
where
    T: Serialize,
{
    let encoded_content = serde_json::to_vec(message_content)?;
    let content_length = encoded_content.len() as u32;
    let mut encoded_message = Vec::with_capacity(4 + encoded_content.len());
    encoded_message.extend_from_slice(&content_length.to_ne_bytes());
    encoded_message.extend_from_slice(&encoded_content);

    Ok(encoded_message)
}

/// Asynchronously reads a message from stdin according to the native messaging protocol.
///
/// Each message is prefixed with a 4-byte length in native byte order,
/// followed by the UTF-8 encoded JSON message content.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::host::get_message;
/// use tokio;
///
/// #[tokio::main]
/// async fn main() {
///     match get_message().await {
///         Ok(message) => println!("Received message: {}", message),
///         Err(e) => eprintln!("Error reading message: {}", e),
///     }
/// }
/// ```
///
/// # Errors
/// Returns an `io::Error` if reading from stdin fails.
pub async fn get_message() -> io::Result<String> {
    let mut stdin = io::stdin();
    let mut length_bytes = [0u8; 4];
    stdin.read_exact(&mut length_bytes).await?;
    let message_length = u32::from_ne_bytes(length_bytes) as usize;
    let mut content_bytes = vec![0u8; message_length];
    stdin.read_exact(&mut content_bytes).await?;
    let message = String::from_utf8(content_bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(message)
}

/// Asynchronously encodes a message and writes it to stdout according to the native messaging protocol.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::host::send_message;
/// use serde::Serialize;
/// use tokio;
///
/// #[derive(Serialize)]
/// struct MyMessage {
///     content: String,
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let message = MyMessage { content: "Hello, world!".to_string() };
///     if let Err(e) = send_message(&message).await {
///         eprintln!("Failed to send message: {}", e);
///     }
/// }
/// ```
///
/// # Errors
/// This function returns an `io::Error` if writing to stdout fails.
pub async fn send_message<T>(message_content: &T) -> io::Result<()>
where
    T: Serialize,
{
    let encoded_message = encode_message(message_content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut stdout = io::stdout();
    stdout.write_all(&encoded_message).await?;
    stdout.flush().await?;

    Ok(())
}

/// Asynchronously runs the event loop, reading messages from stdin and handling them using a callback function.
///
/// # Examples
///
/// ```no_run
/// use native_messaging::host::{event_loop, send_message};
/// use tokio;
///
/// async fn handle_message(message: String) -> io::Result<()> {
///     println!("Handling message: {}", message);
///     Ok(())
/// }
///
/// #[tokio::main]
/// async fn main() {
///     event_loop(handle_message).await;
/// }
/// ```
///
/// # Errors
/// Prints an error message if reading from stdin fails or if the callback function returns an error.
pub async fn event_loop<F, Fut>(callback: F)
where
    F: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = io::Result<()>> + Send,
{
    loop {
        select! {
            result = get_message() => {
                match result {
                    Ok(message) => {
                        if let Err(e) = callback(message).await {
                            eprintln!("Failed to handle message: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read message: {}", e);
                        break;
                    }
                }
            }
        }
    }
}
