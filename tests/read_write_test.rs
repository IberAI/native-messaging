use native_messaging::host::encode_message;
use serde_json::json;

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
