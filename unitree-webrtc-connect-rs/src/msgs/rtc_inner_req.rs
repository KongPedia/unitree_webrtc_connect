use crate::constants::WebRTCConnectionMethod;
use serde_json::Value;

#[derive(Default)]
pub struct WebRTCDataChannelNetworkStatus {
    network_status: String,
}

impl WebRTCDataChannelNetworkStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn network_status(&self) -> &str {
        &self.network_status
    }

    pub fn handle_status(
        &mut self,
        status: &str,
        method: WebRTCConnectionMethod,
    ) -> Option<String> {
        match status {
            "NetworkStatus.ON_4G_CONNECTED" => {
                self.network_status = "4G".to_string();
                Some(self.network_status.clone())
            }
            "NetworkStatus.ON_WIFI_CONNECTED" => {
                self.network_status = if method == WebRTCConnectionMethod::Remote {
                    "STA-T".to_string()
                } else {
                    "STA-L".to_string()
                };
                Some(self.network_status.clone())
            }
            _ => None,
        }
    }
}
#[derive(Default)]
pub struct WebRTCDataChannelFileUploader {
    cancel_upload: bool,
}

impl WebRTCDataChannelFileUploader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn slice_base64_into_chunks(data: &str, chunk_size: usize) -> Vec<String> {
        if chunk_size == 0 {
            return vec![data.to_string()];
        }

        data.as_bytes()
            .chunks(chunk_size)
            .map(|chunk| String::from_utf8_lossy(chunk).to_string())
            .collect()
    }

    pub fn cancel(&mut self) {
        self.cancel_upload = true;
    }

    pub fn is_canceled(&self) -> bool {
        self.cancel_upload
    }
}

#[derive(Default)]
pub struct WebRTCDataChannelFileDownloader {
    cancel_download: bool,
}

impl WebRTCDataChannelFileDownloader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&mut self) {
        self.cancel_download = true;
    }

    pub fn is_canceled(&self) -> bool {
        self.cancel_download
    }
}

#[derive(Default)]
pub struct WebRTCDataChannelRTCInnerReq {
    pub network_status: WebRTCDataChannelNetworkStatus,
    pub file_uploader: WebRTCDataChannelFileUploader,
    pub file_downloader: WebRTCDataChannelFileDownloader,
}

impl WebRTCDataChannelRTCInnerReq {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_response(
        &mut self,
        msg: &Value,
        method: WebRTCConnectionMethod,
    ) -> Option<String> {
        let info = msg.get("info")?;
        let status = info.get("status")?.as_str()?;
        self.network_status.handle_status(status, method)
    }
}
