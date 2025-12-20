use native_messaging::host::{
    decode_message, decode_message_opt, encode_message, recv_json, send_frame, send_json, NmError,
    MAX_FROM_BROWSER, MAX_TO_BROWSER,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, Cursor, Write};

fn frame_bytes(body: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_ne_bytes());
    frame.extend_from_slice(body);
    frame
}

fn frame_len_only(len: u32) -> Vec<u8> {
    len.to_ne_bytes().to_vec()
}

#[tokio::test]
async fn encode_then_decode_roundtrip_string_json() {
    let message = json!({ "key": "value", "n": 42, "unicode": "héllo 🌍" });
    let frame = encode_message(&message).expect("encode");

    // First 4 bytes = native-endian length
    let len = u32::from_ne_bytes(frame[0..4].try_into().unwrap()) as usize;
    assert_eq!(len, frame.len() - 4);

    let mut cur = Cursor::new(frame);
    let decoded = decode_message(&mut cur, MAX_FROM_BROWSER).expect("decode");

    let val: serde_json::Value = serde_json::from_str(&decoded).expect("json parse");
    assert_eq!(val, message);
}

#[tokio::test]
async fn encode_message_enforces_1mb_limit_structured_error() {
    // comfortably > 1 MiB
    let big = "x".repeat(1_200_000);
    let message = json!({ "blob": big });

    let err = encode_message(&message).expect_err("should exceed 1MB host->browser limit");

    match err {
        NmError::OutgoingTooLarge { len, max } => {
            assert!(len > MAX_TO_BROWSER);
            assert_eq!(max, MAX_TO_BROWSER);
        }
        other => panic!("expected OutgoingTooLarge, got: {other:?}"),
    }
}

#[tokio::test]
async fn encode_message_allows_small_payload() {
    let message = json!({"ok": true});
    let frame = encode_message(&message).expect("encode");
    assert!(frame.len() >= 4);
}

#[tokio::test]
async fn decode_message_opt_returns_none_on_clean_eof() {
    let mut cur = Cursor::new(Vec::<u8>::new());
    let res = decode_message_opt(&mut cur, MAX_FROM_BROWSER).expect("should not error");
    assert!(res.is_none());
}

#[tokio::test]
async fn decode_message_returns_disconnected_on_clean_eof() {
    let mut cur = Cursor::new(Vec::<u8>::new());
    let err = decode_message(&mut cur, MAX_FROM_BROWSER).expect_err("EOF => Disconnected");
    assert!(matches!(err, NmError::Disconnected));
}

#[tokio::test]
async fn decode_message_opt_truncated_length_prefix_is_none() {
    // If only 1–3 bytes are available for length, read_exact returns UnexpectedEof.
    // Our decode_message_opt treats that as clean EOF (Ok(None)).
    for n in 1..=3 {
        let mut cur = Cursor::new(vec![0u8; n]);
        let res = decode_message_opt(&mut cur, MAX_FROM_BROWSER).expect("should not error");
        assert!(res.is_none(), "n={n}");
    }
}

#[tokio::test]
async fn decode_message_rejects_over_user_cap_before_reading_body() {
    // length=1024, but max_size=8 => should error before reading body (no body present).
    let frame = frame_len_only(1024);
    let mut cur = Cursor::new(frame);

    let err = decode_message(&mut cur, 8).expect_err("reject over cap");
    match err {
        NmError::IncomingTooLarge { len, max } => {
            assert_eq!(len, 1024);
            assert_eq!(max, 8);
        }
        other => panic!("expected IncomingTooLarge, got: {other:?}"),
    }
}

#[tokio::test]
async fn decode_message_rejects_over_global_cap_even_if_user_cap_higher() {
    // Set a claimed length slightly above global cap; we don't allocate body.
    // Use user cap bigger than global cap, still should cap at MAX_FROM_BROWSER.
    let too_big = (MAX_FROM_BROWSER as u32).saturating_add(1);
    let frame = frame_len_only(too_big);
    let mut cur = Cursor::new(frame);

    let err = decode_message(&mut cur, usize::MAX).expect_err("reject over global cap");
    match err {
        NmError::IncomingTooLarge { len, max } => {
            assert_eq!(len, too_big as usize);
            assert_eq!(max, MAX_FROM_BROWSER);
        }
        other => panic!("expected IncomingTooLarge, got: {other:?}"),
    }
}

#[tokio::test]
async fn decode_message_invalid_utf8_is_structured_error() {
    let body = vec![0xff, 0xfe, 0xfd];
    let frame = frame_bytes(&body);

    let mut cur = Cursor::new(frame);
    let err = decode_message(&mut cur, 1024).expect_err("invalid utf-8 should error");

    assert!(matches!(err, NmError::IncomingNotUtf8(_)));
}

#[tokio::test]
async fn decode_message_truncated_body_is_io_error() {
    // Claim length=10 but provide only 3 bytes.
    let mut frame = Vec::new();
    frame.extend_from_slice(&(10u32).to_ne_bytes());
    frame.extend_from_slice(&[1, 2, 3]);

    let mut cur = Cursor::new(frame);
    let err = decode_message(&mut cur, 1024).expect_err("truncated body should error");

    match err {
        NmError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
        other => panic!("expected Io(UnexpectedEof), got: {other:?}"),
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MyMsg {
    a: u32,
    b: String,
}

#[tokio::test]
async fn recv_json_typed_success() {
    let msg = MyMsg {
        a: 7,
        b: "ok".into(),
    };
    let body = serde_json::to_vec(&msg).unwrap();
    let frame = frame_bytes(&body);

    let mut cur = Cursor::new(frame);
    let decoded: MyMsg = recv_json(&mut cur, MAX_FROM_BROWSER).expect("typed recv_json");
    assert_eq!(decoded, msg);
}

#[tokio::test]
async fn recv_json_typed_deserialize_error() {
    // wrong types for fields
    let body = br#"{"a":"not-a-number","b":123}"#.to_vec();
    let frame = frame_bytes(&body);

    let mut cur = Cursor::new(frame);
    let err = recv_json::<MyMsg, _>(&mut cur, MAX_FROM_BROWSER).expect_err("should fail");
    assert!(matches!(err, NmError::DeserializeJson(_)));
}

/// A writer that can simulate write/flush behavior and capture bytes.
#[derive(Default)]
struct TestWriter {
    buf: Vec<u8>,
    fail_writes: bool,
    fail_flush: bool,
    flushed: bool,
}

impl Write for TestWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.fail_writes {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "simulated write error",
            ));
        }
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.fail_flush {
            return Err(io::Error::other("simulated flush error"));
        }
        self.flushed = true;
        Ok(())
    }
}

#[tokio::test]
async fn send_frame_writes_exact_bytes_and_flushes() {
    let frame = frame_bytes(br#"{"ok":true}"#);

    let mut w = TestWriter::default();
    send_frame(&mut w, &frame).expect("send_frame");

    assert_eq!(w.buf, frame);
    assert!(w.flushed);
}

#[tokio::test]
async fn send_json_encodes_writes_and_flushes_and_is_decodable() {
    let message = json!({"x": 1, "y": "z"});

    let mut w = TestWriter::default();
    send_json(&mut w, &message).expect("send_json");
    assert!(w.flushed);

    let mut cur = Cursor::new(w.buf);
    let decoded = decode_message(&mut cur, MAX_FROM_BROWSER).expect("decode");
    let val: serde_json::Value = serde_json::from_str(&decoded).expect("json");
    assert_eq!(val, message);
}

#[tokio::test]
async fn send_frame_surfaces_write_errors() {
    let frame = frame_bytes(br#"{"ok":true}"#);

    let mut w = TestWriter {
        fail_writes: true,
        ..Default::default()
    };

    let err = send_frame(&mut w, &frame).expect_err("should fail");
    match err {
        NmError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::BrokenPipe),
        other => panic!("expected Io(BrokenPipe), got: {other:?}"),
    }
}

#[tokio::test]
async fn send_frame_surfaces_flush_errors() {
    let frame = frame_bytes(br#"{"ok":true}"#);

    let mut w = TestWriter {
        fail_flush: true,
        ..Default::default()
    };

    let err = send_frame(&mut w, &frame).expect_err("should fail");
    match err {
        NmError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::Other),
        other => panic!("expected Io(Other), got: {other:?}"),
    }
}
