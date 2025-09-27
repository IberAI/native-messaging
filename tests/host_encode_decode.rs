use native_messaging::host::{decode_message, encode_message};
use serde_json::json;
use std::io::Cursor;

#[tokio::test]
async fn encode_then_decode_roundtrip() {
    let message = json!({ "key": "value", "n": 42, "unicode": "héllo 🌍" });
    let frame = encode_message(&message).expect("encode");
    // First 4 bytes = length
    let len = u32::from_ne_bytes(frame[0..4].try_into().unwrap()) as usize;
    assert_eq!(len, frame.len() - 4);

    // Decode back
    let mut cur = Cursor::new(frame);
    let decoded = decode_message(&mut cur, 64 * 1024 * 1024).expect("decode");
    let val: serde_json::Value = serde_json::from_str(&decoded).expect("json");
    assert_eq!(val, message);
}

#[tokio::test]
async fn encode_message_enforces_1mb_limit() {
    // Create >1MB payload
    let big = "x".repeat(1_200_000);
    let message = json!({ "blob": big });
    let err = encode_message(&message).expect_err("should exceed 1MB host->browser limit");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[tokio::test]
async fn decode_message_respects_max_size_cap() {
    // Craft a frame that claims length 1024 but provide zero bytes afterward.
    // Because we set max_size=8, decode should fail early before reading body.
    let mut frame = Vec::new();
    frame.extend_from_slice(&(1024u32).to_ne_bytes());
    let mut cur = Cursor::new(frame);
    let err = decode_message(&mut cur, 8).expect_err("should reject over cap");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn decode_message_invalid_utf8() {
    // Make a frame whose body is not valid UTF-8
    let mut frame = Vec::new();
    let body = vec![0xff, 0xfe, 0xfd]; // invalid UTF-8
    frame.extend_from_slice(&(body.len() as u32).to_ne_bytes());
    frame.extend_from_slice(&body);
    let mut cur = Cursor::new(frame);
    let err = decode_message(&mut cur, 1024).expect_err("invalid utf-8 should error");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}
