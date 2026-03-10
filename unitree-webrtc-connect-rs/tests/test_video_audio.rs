use serde_json::json;
use std::sync::{Arc, Mutex};
use unitree_webrtc_connect_rs::audio::WebRTCAudioChannel;
use unitree_webrtc_connect_rs::video::WebRTCVideoChannel;

#[test]
fn test_video_channel_callbacks_and_switch() {
    let mut video = WebRTCVideoChannel::new();
    assert!(video.is_enabled());

    let called = Arc::new(Mutex::new(false));
    let called_clone = Arc::clone(&called);
    video.add_track_callback(Box::new(move |_| {
        if let Ok(mut guard) = called_clone.lock() {
            *guard = true;
        }
    }));

    assert_eq!(video.callback_count(), 1);
    video.track_handler(&json!({"kind":"video"}));
    assert!(*called.lock().unwrap());

    video.switch_video_channel(false);
    assert!(!video.is_enabled());
}

#[test]
fn test_audio_channel_callbacks_and_switch() {
    let mut audio = WebRTCAudioChannel::new();
    assert!(audio.is_enabled());

    let called = Arc::new(Mutex::new(false));
    let called_clone = Arc::clone(&called);
    audio.add_track_callback(Box::new(move |_| {
        if let Ok(mut guard) = called_clone.lock() {
            *guard = true;
        }
    }));

    assert_eq!(audio.callback_count(), 1);
    audio.frame_handler(&json!({"kind":"audio"}));
    assert!(*called.lock().unwrap());

    audio.switch_audio_channel(false);
    assert!(!audio.is_enabled());
}
