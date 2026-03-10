use std::time::Instant;
use unitree_webrtc_core_rs::datachannel::WebRTCDataChannel;

#[test]
fn test_set_decoder_validation() {
    let mut dc = WebRTCDataChannel::new(15.0, 15.0);
    assert_eq!(dc.decoder_type(), "libvoxel");

    dc.set_decoder("native").unwrap();
    assert_eq!(dc.decoder_type(), "native");

    let err = dc.set_decoder("invalid").unwrap_err();
    assert!(err.contains("Invalid decoder type"));
}

#[test]
fn test_handle_validation_opens_channel() {
    let mut dc = WebRTCDataChannel::new(15.0, 15.0);
    let now = Instant::now();
    dc.handle_text_message(r#"{"type":"validation","data":"Validation Ok."}"#, now)
        .unwrap();
    assert!(dc.is_open());
}

#[test]
fn test_switch_payload_builders() {
    let dc = WebRTCDataChannel::new(15.0, 15.0);

    let traffic = dc.disable_traffic_saving(true);
    assert_eq!(traffic["type"], "rtc_inner_req");
    assert_eq!(traffic["data"]["instruction"], "on");

    let vid = dc.switch_video_channel(false);
    assert_eq!(vid["type"], "vid");
    assert_eq!(vid["data"], "off");

    let aud = dc.switch_audio_channel(true);
    assert_eq!(aud["type"], "aud");
    assert_eq!(aud["data"], "on");
}

#[test]
fn test_reconnect_flag() {
    let mut dc = WebRTCDataChannel::new(15.0, 15.0);
    assert!(!dc.reconnect_requested());
    dc.trigger_reconnect();
    assert!(dc.reconnect_requested());
}

#[test]
fn test_handle_binary_message() {
    let mut dc = WebRTCDataChannel::new(15.0, 15.0);
    let now = Instant::now();

    let json_str = r#"{"topic":"a","data":{}}"#;
    let json_bytes = json_str.as_bytes();
    let header_length = json_bytes.len() as u16;

    let mut buffer = Vec::new();
    buffer.extend_from_slice(&header_length.to_le_bytes());
    buffer.extend_from_slice(&[0, 0]);
    buffer.extend_from_slice(json_bytes);
    buffer.extend_from_slice(&[10, 20, 30]);

    let out = dc.handle_binary_message(&buffer, now).unwrap().unwrap();
    assert_eq!(out["topic"], "a");
    assert_eq!(out["data"]["data"][0], 10);
}
