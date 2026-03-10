use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use unitree_webrtc_connect_rs::constants::WebRTCConnectionMethod;
use unitree_webrtc_connect_rs::msgs::error_handler::{
    get_error_code_text, get_error_source_text, handle_error, integer_to_hex_string,
};
use unitree_webrtc_connect_rs::msgs::future_resolver::FutureResolver;
use unitree_webrtc_connect_rs::msgs::heartbeat::WebRTCDataChannelHeartBeat;
use unitree_webrtc_connect_rs::msgs::pubsub::WebRTCDataChannelPubSub;
use unitree_webrtc_connect_rs::msgs::rtc_inner_req::{
    WebRTCDataChannelFileUploader, WebRTCDataChannelNetworkStatus,
};
use unitree_webrtc_connect_rs::msgs::validation::WebRTCDataChannelValidation;

#[test]
fn test_future_resolver_generates_key() {
    let key = FutureResolver::generate_message_key("req", "topic/a", None);
    assert_eq!(key, "req $ topic/a");

    let key_with_id = FutureResolver::generate_message_key("req", "topic/a", Some("abc"));
    assert_eq!(key_with_id, "abc");
}

#[test]
fn test_future_resolver_merges_chunks() {
    let mut resolver = FutureResolver::new();
    resolver.save_resolve("res", "topic/chunk", Some("u1"));

    let mut first = json!({
        "type": "res",
        "topic": "topic/chunk",
        "data": {
            "uuid": "u1",
            "content_info": {
                "enable_chunking": true,
                "chunk_index": 1,
                "total_chunk_num": 2
            },
            "data": [65, 66]
        }
    });

    let mut last = json!({
        "type": "res",
        "topic": "topic/chunk",
        "data": {
            "uuid": "u1",
            "content_info": {
                "enable_chunking": true,
                "chunk_index": 2,
                "total_chunk_num": 2
            },
            "data": [67]
        }
    });

    let first_result = resolver.run_resolve_for_topic(&mut first).unwrap();
    assert!(first_result.is_none());

    let resolved_key = resolver.run_resolve_for_topic(&mut last).unwrap();
    assert_eq!(resolved_key, Some("u1".to_string()));

    let resolved = resolver.take_resolved("u1").unwrap();
    let merged = resolved
        .get("data")
        .and_then(|v| v.get("data"))
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(merged, &vec![json!(65), json!(66), json!(67)]);
}

#[test]
fn test_pubsub_throttle_and_subscription_callback() {
    let mut pubsub = WebRTCDataChannelPubSub::new(10.0, 10.0);

    let now = Instant::now();
    assert!(pubsub.should_process("vid", now));
    assert!(!pubsub.should_process("vid", now + Duration::from_millis(20)));
    assert!(pubsub.should_process("vid", now + Duration::from_millis(120)));

    let called = Arc::new(Mutex::new(false));
    let called_clone = Arc::clone(&called);
    pubsub.subscribe(
        "topic/cb",
        Box::new(move |_| {
            if let Ok(mut guard) = called_clone.lock() {
                *guard = true;
            }
        }),
    );

    pubsub.save_resolve("res", "topic/cb", Some("id-cb"));
    let message = json!({
        "type": "res",
        "topic": "topic/cb",
        "data": {"uuid": "id-cb", "data": "ok"}
    });

    let resolved = pubsub
        .run_resolve(unitree_webrtc_connect_rs::msgs::pubsub::CallbackPayload::Json(message))
        .unwrap();
    assert_eq!(resolved, Some("id-cb".to_string()));
    assert!(*called.lock().unwrap());
}

#[test]
fn test_heartbeat_timeout() {
    let mut heartbeat = WebRTCDataChannelHeartBeat::new();
    let start = Instant::now();
    heartbeat.start_heartbeat(start);

    assert!(!heartbeat.is_timed_out(start + Duration::from_secs(5)));
    assert!(heartbeat.is_timed_out(start + Duration::from_secs(7)));
}

#[test]
fn test_validation_encrypt_key() {
    let encrypted = WebRTCDataChannelValidation::encrypt_key("abc").unwrap();
    assert!(!encrypted.is_empty());
}

#[test]
fn test_network_status_mapping() {
    let mut status = WebRTCDataChannelNetworkStatus::new();

    let local = status.handle_status(
        "NetworkStatus.ON_WIFI_CONNECTED",
        WebRTCConnectionMethod::LocalSTA,
    );
    assert_eq!(local, Some("STA-L".to_string()));

    let remote = status.handle_status(
        "NetworkStatus.ON_WIFI_CONNECTED",
        WebRTCConnectionMethod::Remote,
    );
    assert_eq!(remote, Some("STA-T".to_string()));

    let on_4g = status.handle_status(
        "NetworkStatus.ON_4G_CONNECTED",
        WebRTCConnectionMethod::Remote,
    );
    assert_eq!(on_4g, Some("4G".to_string()));
}

#[test]
fn test_file_uploader_chunk_slice() {
    let chunks = WebRTCDataChannelFileUploader::slice_base64_into_chunks("ABCDEFGHIJ", 4);
    assert_eq!(chunks, vec!["ABCD", "EFGH", "IJ"]);
}

#[test]
fn test_error_handler_mappings() {
    assert_eq!(integer_to_hex_string(16).unwrap(), "10");
    assert_eq!(
        get_error_code_text(100, "10"),
        "Battery communication error"
    );
    assert_eq!(get_error_source_text(300), "Motor malfunction");

    let lines = handle_error(&json!({
        "data": [[1700000000, 100, 16]]
    }));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("Battery communication error"));
}
