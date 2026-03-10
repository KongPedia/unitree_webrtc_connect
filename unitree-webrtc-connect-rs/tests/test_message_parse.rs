use serde_json::json;
use unitree_webrtc_connect_rs::message::parse_array_buffer;

#[test]
fn test_parse_normal_message() {
    // Construct a normal message buffer
    // header_1 != 2 || header_2 != 0 -> so let's say bytes 0..2 are header_length
    // length of json is 13: `{"topic":"a"}`
    let json_str = r#"{"topic":"a"}"#;
    let json_bytes = json_str.as_bytes();
    let header_length = json_bytes.len() as u16;

    let mut buffer = Vec::new();
    // 0..2
    buffer.extend_from_slice(&header_length.to_le_bytes());
    // 2..4 (padding/reserved, skipped in normal parsing)
    buffer.extend_from_slice(&[0, 0]);
    // 4..4+length (json)
    buffer.extend_from_slice(json_bytes);
    // binary data
    buffer.extend_from_slice(&[10, 20, 30]);

    let parsed = parse_array_buffer(&buffer).expect("Should parse successfully");

    assert!(!parsed.is_lidar);
    assert_eq!(parsed.json_data, json!({"topic": "a"}));
    assert_eq!(parsed.binary_data, vec![10, 20, 30]);
}

#[test]
fn test_parse_lidar_message() {
    // Construct a lidar message buffer
    // header_1 == 2 and header_2 == 0
    // so bytes 0..2 is 2, bytes 2..4 is 0
    let mut buffer = Vec::new();

    // header_1 (2)
    buffer.extend_from_slice(&2u16.to_le_bytes());
    // header_2 (0)
    buffer.extend_from_slice(&0u16.to_le_bytes());

    // Following is the shifted buffer:
    let json_str = r#"{"type":"lidar"}"#;
    let json_bytes = json_str.as_bytes();
    let header_length = json_bytes.len() as u32;

    // header_length (bytes 0..4 of shifted buffer)
    buffer.extend_from_slice(&header_length.to_le_bytes());
    // reserved/padding (bytes 4..8 of shifted buffer)
    buffer.extend_from_slice(&[0, 0, 0, 0]);
    // json (bytes 8..8+length)
    buffer.extend_from_slice(json_bytes);
    // binary
    buffer.extend_from_slice(&[255, 128]);

    let parsed = parse_array_buffer(&buffer).expect("Should parse successfully");

    assert!(parsed.is_lidar);
    assert_eq!(parsed.json_data, json!({"type": "lidar"}));
    assert_eq!(parsed.binary_data, vec![255, 128]);
}
