use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use md5::{Digest, Md5};

#[derive(Default)]
pub struct WebRTCDataChannelValidation {
    key: String,
    validated: bool,
}

impl WebRTCDataChannelValidation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_key(&mut self, key: &str) {
        self.key = key.to_string();
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn is_validated(&self) -> bool {
        self.validated
    }

    pub fn mark_validated(&mut self) {
        self.validated = true;
    }

    pub fn hex_to_base64(hex_str: &str) -> Result<String, String> {
        let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {e}"))?;
        Ok(BASE64.encode(bytes))
    }

    pub fn encrypt_by_md5(input_str: &str) -> String {
        let mut hasher = Md5::new();
        hasher.update(input_str.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn encrypt_key(key: &str) -> Result<String, String> {
        let prefixed_key = format!("UnitreeGo2_{key}");
        let encrypted = Self::encrypt_by_md5(&prefixed_key);
        Self::hex_to_base64(&encrypted)
    }
}
