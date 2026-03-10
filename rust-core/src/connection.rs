use crate::audio::WebRTCAudioChannel;
use crate::constants::WebRTCConnectionMethod;
use crate::datachannel::WebRTCDataChannel;
use crate::msgs::pubsub::WebRTCDataChannelPubSub;
use crate::video::WebRTCVideoChannel;
use rand::Rng;
use serde_json::{json, Value};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WebRtcConfiguration {
    pub ice_servers: Vec<IceServer>,
}

pub struct UnitreeWebRTCConnection {
    pub connection_method: WebRTCConnectionMethod,
    pub serial_number: Option<String>,
    pub ip: Option<String>,
    pub token: String,
    pub is_connected: bool,
    intentional_disconnect: bool,
    is_reconnecting: bool,
    pub reconnect_attempts: u32,
    pub datachannel: Option<WebRTCDataChannel>,
    pub audio: Option<WebRTCAudioChannel>,
    pub video: Option<WebRTCVideoChannel>,
}

impl UnitreeWebRTCConnection {
    pub fn new(
        connection_method: WebRTCConnectionMethod,
        serial_number: Option<String>,
        ip: Option<String>,
    ) -> Self {
        Self {
            connection_method,
            serial_number,
            ip,
            token: String::new(),
            is_connected: false,
            intentional_disconnect: false,
            is_reconnecting: false,
            reconnect_attempts: 0,
            datachannel: None,
            audio: None,
            video: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), String> {
        self.intentional_disconnect = false;

        match self.connection_method {
            WebRTCConnectionMethod::Remote => {
                if self.serial_number.is_none() {
                    return Err("Remote connection requires serial_number".to_string());
                }
            }
            WebRTCConnectionMethod::LocalSTA => {
                if self.ip.is_none() {
                    return Err("LocalSTA connection requires ip".to_string());
                }
            }
            WebRTCConnectionMethod::LocalAP => {
                self.ip = Some("192.168.12.1".to_string());
            }
        }

        self.init_webrtc();
        self.is_connected = true;
        Ok(())
    }

    pub async fn disconnect(&mut self) {
        self.intentional_disconnect = true;
        self.is_connected = false;
        self.datachannel = None;
        self.audio = None;
        self.video = None;
    }

    pub async fn reconnect(&mut self) -> Result<(), String> {
        self.disconnect().await;
        self.connect().await
    }

    pub async fn auto_reconnect(&mut self, max_retries: u32) -> bool {
        self.auto_reconnect_with_backoff(max_retries, Duration::from_millis(200))
            .await
    }

    pub async fn auto_reconnect_with_backoff(
        &mut self,
        max_retries: u32,
        base_delay: Duration,
    ) -> bool {
        if self.intentional_disconnect || self.is_reconnecting {
            return false;
        }

        self.is_reconnecting = true;
        self.reconnect_attempts = 0;

        for attempt in 0..max_retries {
            self.reconnect_attempts = attempt + 1;
            if attempt > 0 {
                let factor = 1u32 << attempt.min(6);
                let delay = base_delay.saturating_mul(factor);
                tokio::time::sleep(delay).await;
            }

            if self.reconnect().await.is_ok() {
                self.is_reconnecting = false;
                return true;
            }
        }

        self.is_reconnecting = false;
        false
    }

    pub fn create_webrtc_configuration(
        &self,
        turn_server_info: Option<(&str, &str, &str)>,
        stun_enable: bool,
        turn_enable: bool,
    ) -> Result<WebRtcConfiguration, String> {
        let mut ice_servers = Vec::new();

        if let Some((username, credential, turn_url)) = turn_server_info {
            if turn_enable {
                ice_servers.push(IceServer {
                    urls: vec![turn_url.to_string()],
                    username: Some(username.to_string()),
                    credential: Some(credential.to_string()),
                });
            }

            if stun_enable {
                ice_servers.push(IceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_string()],
                    username: None,
                    credential: None,
                });
            }
        } else if turn_enable {
            return Err("TURN enabled but turn_server_info is missing".to_string());
        }

        Ok(WebRtcConfiguration { ice_servers })
    }

    fn init_webrtc(&mut self) {
        self.datachannel = Some(WebRTCDataChannel::default());
        self.audio = Some(WebRTCAudioChannel::new());
        self.video = Some(WebRTCVideoChannel::new());
    }

    pub fn publish_request_new(&self, topic: &str, payload: Value) -> Result<Value, String> {
        if !self.is_connected {
            return Err("Connection is not established".to_string());
        }

        let api_id = payload
            .get("api_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| "Please provide api id".to_string())?;

        let generated_id = generate_request_id();
        let identity_id = payload
            .get("id")
            .and_then(Value::as_i64)
            .unwrap_or(generated_id);

        let mut request_payload = json!({
            "header": {
                "identity": {
                    "id": identity_id,
                    "api_id": api_id,
                }
            },
            "parameter": "",
        });

        if let Some(parameter) = payload.get("parameter") {
            let parameter_str = if let Some(s) = parameter.as_str() {
                s.to_string()
            } else {
                serde_json::to_string(parameter)
                    .map_err(|e| format!("Failed to serialize parameter: {e}"))?
            };

            if let Some(obj) = request_payload.as_object_mut() {
                obj.insert("parameter".to_string(), Value::String(parameter_str));
            }
        }

        if payload.get("priority").is_some() {
            if let Some(header) = request_payload.get_mut("header").and_then(Value::as_object_mut) {
                header.insert("policy".to_string(), json!({ "priority": 1 }));
            }
        }

        let message = WebRTCDataChannelPubSub::build_publish_message(topic, Some(request_payload), "req");

        Ok(json!({
            "type": "res",
            "topic": topic,
            "data": {
                "header": {
                    "status": {
                        "code": 0,
                        "message": "Rust transport stub response",
                    }
                },
                "data": serde_json::to_string(&message)
                    .map_err(|e| format!("Failed to serialize request echo payload: {e}"))?
            }
        }))
    }
}

fn generate_request_id() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let base = millis % 2_147_483_648i64;
    base + rand::thread_rng().gen::<u16>() as i64 % 1001
}
