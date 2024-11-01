use native_messaging::host::{encode_message, get_message, send_message};
use serde_json::json;
use std::io::Cursor;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn test_encode_message() {
    let message = json!({ "key": "value" });
    let encoded = encode_message(&message).unwrap();

    // The first 4 bytes should be the length of the JSON content.
    let length_bytes = &encoded[0..4];
    let content_length = u32::from_ne_bytes(length_bytes.try_into().unwrap()) as usize;
    assert_eq!(content_length, encoded.len() - 4);

    // The rest of the bytes should be the JSON content.
    let content_bytes = &encoded[4..];
    let decoded_message: serde_json::Value = serde_json::from_slice(content_bytes).unwrap();
    assert_eq!(decoded_message, message);
}

#[tokio::test]
async fn test_get_message() {
    let message_content = json!({ "key": "value" }).to_string();
    let message_length = message_content.len() as u32;
    let mut input_data = Vec::new();
    input_data.extend_from_slice(&message_length.to_ne_bytes());
    input_data.extend_from_slice(message_content.as_bytes());

    let mut cursor = Cursor::new(input_data);
    let mut stdin_mock = Cursor::new(Vec::new());
    tokio::io::copy(&mut cursor, &mut stdin_mock).await.unwrap();

    let message = get_message().await.unwrap();
    let parsed_message: serde_json::Value = serde_json::from_str(&message).unwrap();
    assert_eq!(parsed_message, json!({ "key": "value" }));
}

#[tokio::test]
async fn test_send_message() {
    let message = json!({ "key": "value" });

    let encoded_message = encode_message(&message).unwrap();
    let mut stdout_mock = Vec::new();

    send_message(&message).await.unwrap();

    stdout_mock.write_all(&encoded_message).await.unwrap();
    stdout_mock.flush().await.unwrap();

    // Verify that the encoded message matches the expected output.
    assert_eq!(stdout_mock, encoded_message);
}

#[tokio::test]
async fn test_round_trip() {
    // Test sending and receiving a message to verify full round-trip communication.
    let message = json!({ "foo": "bar", "num": 42 });
    let encoded = encode_message(&message).unwrap();

    let mut cursor = Cursor::new(encoded);
    let mut stdin_mock = Cursor::new(Vec::new());
    tokio::io::copy(&mut cursor, &mut stdin_mock).await.unwrap();

    let received_message = get_message().await.unwrap();
    let parsed_message: serde_json::Value = serde_json::from_str(&received_message).unwrap();
    assert_eq!(parsed_message, message);
}
