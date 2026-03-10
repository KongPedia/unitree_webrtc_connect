use crate::msgs::future_resolver::FutureResolver;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub type TopicCallback = Box<dyn Fn(&Value) + Send + Sync + 'static>;

pub struct WebRTCDataChannelPubSub {
    future_resolver: FutureResolver,
    subscriptions: HashMap<String, TopicCallback>,
    last_process_time: HashMap<String, Instant>,
    throttle_limits: HashMap<String, Duration>,
}

impl WebRTCDataChannelPubSub {
    pub fn new(lidar_hz: f64, video_fps: f64) -> Self {
        let mut throttle_limits = HashMap::new();
        throttle_limits.insert(
            "rt/utlidar/voxel_map_compressed".to_string(),
            duration_from_hz(lidar_hz),
        );
        throttle_limits.insert("vid".to_string(), duration_from_hz(video_fps));

        Self {
            future_resolver: FutureResolver::new(),
            subscriptions: HashMap::new(),
            last_process_time: HashMap::new(),
            throttle_limits,
        }
    }

    pub fn should_process(&mut self, topic_or_type: &str, now: Instant) -> bool {
        let Some(limit) = self.throttle_limits.get(topic_or_type) else {
            return true;
        };

        let last_time = self.last_process_time.get(topic_or_type).copied();
        if let Some(last) = last_time {
            if now.saturating_duration_since(last) < *limit {
                return false;
            }
        }

        self.last_process_time
            .insert(topic_or_type.to_string(), now);
        true
    }

    pub fn save_resolve(&mut self, message_type: &str, topic: &str, identifier: Option<&str>) {
        self.future_resolver
            .save_resolve(message_type, topic, identifier);
    }

    pub fn run_resolve(&mut self, message: &mut Value) -> Result<Option<String>, String> {
        let resolved = self.future_resolver.run_resolve_for_topic(message)?;

        if let Some(topic) = message.get("topic").and_then(Value::as_str) {
            if let Some(callback) = self.subscriptions.get(topic) {
                callback(message);
            }
        }

        Ok(resolved)
    }

    pub fn take_resolved(&mut self, key: &str) -> Option<Value> {
        self.future_resolver.take_resolved(key)
    }

    pub fn build_publish_message(topic: &str, data: Option<Value>, msg_type: &str) -> Value {
        let mut payload = json!({
            "type": msg_type,
            "topic": topic,
        });

        if let Some(data_value) = data {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("data".to_string(), data_value);
            }
        }

        payload
    }

    pub fn subscribe(&mut self, topic: &str, callback: TopicCallback) {
        self.subscriptions.insert(topic.to_string(), callback);
    }

    pub fn unsubscribe(&mut self, topic: &str) {
        self.subscriptions.remove(topic);
    }
}

fn duration_from_hz(hz: f64) -> Duration {
    if hz <= 0.0 {
        Duration::from_secs(0)
    } else {
        Duration::from_secs_f64(1.0 / hz)
    }
}
