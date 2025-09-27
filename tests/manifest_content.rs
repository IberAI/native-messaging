use serde_json::json;

#[test]
fn manifest_shapes_match_expectations() {
    // What Chrome should look like (shape-wise)
    let chrome = json!({
        "name": "com.example.host",
        "description": "desc",
        "path": "/abs/path",
        "type": "stdio",
        "allowed_origins": ["chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/"]
    });
    assert!(chrome.get("allowed_origins").is_some());
    assert!(chrome.get("allowed_extensions").is_none());

    // What Firefox should look like (shape-wise)
    let firefox = json!({
        "name": "com.example.host",
        "description": "desc",
        "path": "/abs/path",
        "type": "stdio",
        "allowed_extensions": ["native-test@example.com"]
    });
    assert!(firefox.get("allowed_extensions").is_some());
    assert!(firefox.get("allowed_origins").is_none());
}
