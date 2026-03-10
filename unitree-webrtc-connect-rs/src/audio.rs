use serde_json::Value;

pub type AudioFrameCallback = Box<dyn Fn(&Value) + Send + Sync + 'static>;

#[derive(Default)]
pub struct WebRTCAudioChannel {
    enabled: bool,
    frame_callbacks: Vec<AudioFrameCallback>,
}

impl WebRTCAudioChannel {
    pub fn new() -> Self {
        Self {
            enabled: true,
            frame_callbacks: Vec::new(),
        }
    }

    pub fn switch_audio_channel(&mut self, switch: bool) {
        self.enabled = switch;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn add_track_callback(&mut self, callback: AudioFrameCallback) {
        self.frame_callbacks.push(callback);
    }

    pub fn callback_count(&self) -> usize {
        self.frame_callbacks.len()
    }

    pub fn frame_handler(&self, frame: &Value) {
        for callback in &self.frame_callbacks {
            callback(frame);
        }
    }
}
