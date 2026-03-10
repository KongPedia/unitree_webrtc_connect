use serde_json::Value;

pub type VideoTrackCallback = Box<dyn Fn(&Value) + Send + Sync + 'static>;

#[derive(Default)]
pub struct WebRTCVideoChannel {
    enabled: bool,
    track_callbacks: Vec<VideoTrackCallback>,
}

impl WebRTCVideoChannel {
    pub fn new() -> Self {
        Self {
            enabled: true,
            track_callbacks: Vec::new(),
        }
    }

    pub fn switch_video_channel(&mut self, switch: bool) {
        self.enabled = switch;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn add_track_callback(&mut self, callback: VideoTrackCallback) {
        self.track_callbacks.push(callback);
    }

    pub fn callback_count(&self) -> usize {
        self.track_callbacks.len()
    }

    pub fn track_handler(&self, track: &Value) {
        for callback in &self.track_callbacks {
            callback(track);
        }
    }
}
