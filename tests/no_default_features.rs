#![cfg(not(feature = "install"))]

use native_messaging::host::{decode_message, encode_message, MAX_FROM_BROWSER};
use serde_json::json;
use std::io::Cursor;

#[test]
fn sync_framing_works_without_default_features() {
    let message = json!({"ok": true});
    let frame = encode_message(&message).expect("encode");
    let mut cur = Cursor::new(frame);

    let decoded = decode_message(&mut cur, MAX_FROM_BROWSER).expect("decode");
    let decoded_json: serde_json::Value = serde_json::from_str(&decoded).expect("json");

    assert_eq!(decoded_json, message);
}
