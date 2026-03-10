use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub struct WebRTCDataChannelHeartBeat {
    heartbeat_response: Option<Instant>,
    timeout_threshold: Duration,
    heartbeat_interval: Duration,
}

impl Default for WebRTCDataChannelHeartBeat {
    fn default() -> Self {
        Self {
            heartbeat_response: None,
            timeout_threshold: Duration::from_secs(6),
            heartbeat_interval: Duration::from_secs(2),
        }
    }
}

impl WebRTCDataChannelHeartBeat {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    pub fn timeout_threshold(&self) -> Duration {
        self.timeout_threshold
    }

    pub fn start_heartbeat(&mut self, now: Instant) {
        self.heartbeat_response = Some(now);
    }

    pub fn handle_response(&mut self, now: Instant) {
        self.heartbeat_response = Some(now);
    }

    pub fn is_timed_out(&self, now: Instant) -> bool {
        self.heartbeat_response
            .map(|last| now.saturating_duration_since(last) > self.timeout_threshold)
            .unwrap_or(false)
    }

    pub fn build_heartbeat_payload(time_in_str: &str, time_in_num: i64) -> Value {
        json!({
            "timeInStr": time_in_str,
            "timeInNum": time_in_num,
        })
    }
}
