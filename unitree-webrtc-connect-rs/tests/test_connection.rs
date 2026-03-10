use std::sync::{Arc, Mutex};
use std::time::Duration;
use unitree_webrtc_connect_rs::connection::UnitreeWebRTCConnection;
use unitree_webrtc_connect_rs::constants::WebRTCConnectionMethod;

fn mock_arc(method: WebRTCConnectionMethod) -> Arc<Mutex<UnitreeWebRTCConnection>> {
    Arc::new(Mutex::new(UnitreeWebRTCConnection::new(method, None, None)))
}

#[tokio::test]
#[ignore = "Requires physical connection"]
async fn test_connect_local_ap_sets_default_ip() {
    let mut conn = UnitreeWebRTCConnection::new(WebRTCConnectionMethod::LocalAP, None, None);
    conn.connect(mock_arc(WebRTCConnectionMethod::LocalAP)).await.unwrap();

    assert!(conn.is_connected);
    assert_eq!(conn.ip.as_deref(), Some("192.168.12.1"));
    assert!(conn.datachannel.is_some());
    assert!(conn.audio.is_some());
    assert!(conn.video.is_some());
}

#[tokio::test]
async fn test_connect_local_sta_requires_ip() {
    let mut conn = UnitreeWebRTCConnection::new(WebRTCConnectionMethod::LocalSTA, None, None);
    let err = conn.connect(mock_arc(WebRTCConnectionMethod::LocalSTA)).await.unwrap_err();
    assert!(err.contains("requires ip"));
}

#[tokio::test]
#[ignore = "Requires network connection"]
async fn test_disconnect_and_reconnect() {
    let mut conn = UnitreeWebRTCConnection::new(
        WebRTCConnectionMethod::LocalSTA,
        None,
        Some("192.168.0.10".to_string()),
    );
    conn.connect(mock_arc(WebRTCConnectionMethod::LocalSTA)).await.unwrap();
    conn.disconnect().await;
    assert!(!conn.is_connected);

    conn.reconnect(mock_arc(WebRTCConnectionMethod::LocalSTA)).await.unwrap();
    assert!(conn.is_connected);
}

#[tokio::test]
async fn test_auto_reconnect_stops_when_intentional_disconnect() {
    let mut conn = UnitreeWebRTCConnection::new(
        WebRTCConnectionMethod::LocalSTA,
        None,
        Some("192.168.0.10".to_string()),
    );
    conn.disconnect().await;
    let ok = conn
        .auto_reconnect_with_backoff(mock_arc(WebRTCConnectionMethod::LocalSTA), 3, Duration::from_millis(0))
        .await;
    assert!(!ok);
}

#[tokio::test]
async fn test_auto_reconnect_failure_path() {
    let mut conn = UnitreeWebRTCConnection::new(WebRTCConnectionMethod::LocalSTA, None, None);
    let ok = conn
        .auto_reconnect_with_backoff(mock_arc(WebRTCConnectionMethod::LocalSTA), 2, Duration::from_millis(0))
        .await;
    assert!(!ok);
    assert_eq!(conn.reconnect_attempts, 2);
}
