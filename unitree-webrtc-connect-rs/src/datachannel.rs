use crate::message::parse_array_buffer;
use crate::msgs::error_handler::handle_error;
use crate::msgs::heartbeat::WebRTCDataChannelHeartBeat;
use crate::msgs::pubsub::WebRTCDataChannelPubSub;
use crate::msgs::rtc_inner_req::WebRTCDataChannelNetworkStatus;
use crate::msgs::validation::WebRTCDataChannelValidation;
use serde_json::{json, Value};
use std::time::Instant;

pub struct WebRTCDataChannel {
    pub_sub: WebRTCDataChannelPubSub,
    heartbeat: WebRTCDataChannelHeartBeat,
    validation: WebRTCDataChannelValidation,
    network_status: WebRTCDataChannelNetworkStatus,
    data_channel_opened: bool,
    reconnect_requested: bool,
    decoder_type: String,
}

impl Default for WebRTCDataChannel {
    fn default() -> Self {
        Self::new(15.0, 15.0)
    }
}

impl WebRTCDataChannel {
    pub fn new(lidar_hz: f64, video_fps: f64) -> Self {
        Self {
            pub_sub: WebRTCDataChannelPubSub::new(lidar_hz, video_fps),
            heartbeat: WebRTCDataChannelHeartBeat::new(),
            validation: WebRTCDataChannelValidation::new(),
            network_status: WebRTCDataChannelNetworkStatus::new(),
            data_channel_opened: false,
            reconnect_requested: false,
            decoder_type: "libvoxel".to_string(),
        }
    }

    pub fn set_decoder(&mut self, decoder_type: &str) -> Result<(), String> {
        if decoder_type != "libvoxel" && decoder_type != "native" {
            return Err("Invalid decoder type. Choose 'libvoxel' or 'native'.".to_string());
        }
        self.decoder_type = decoder_type.to_string();
        Ok(())
    }

    pub fn decoder_type(&self) -> &str {
        &self.decoder_type
    }

    pub fn is_open(&self) -> bool {
        self.data_channel_opened
    }

    pub fn reconnect_requested(&self) -> bool {
        self.reconnect_requested
    }

    pub fn trigger_reconnect(&mut self) {
        self.reconnect_requested = true;
    }

    pub fn save_resolve(&mut self, message_type: &str, topic: &str, identifier: Option<&str>) {
        self.pub_sub.save_resolve(message_type, topic, identifier);
    }

    pub fn take_resolved(&mut self, key: &str) -> Option<Value> {
        self.pub_sub.take_resolved(key)
    }

    pub fn subscribe(&mut self, topic: &str, callback: crate::msgs::pubsub::TopicCallback) {
        self.pub_sub.subscribe(topic, callback);
    }

    pub fn unsubscribe(&mut self, topic: &str) {
        self.pub_sub.unsubscribe(topic);
    }

    pub fn handle_text_message(&mut self, message: &str, now: Instant) -> Result<Value, String> {
        let parsed: Value =
            serde_json::from_str(message).map_err(|e| format!("JSON decode error: {e}"))?;
        let _ = self
            .pub_sub
            .run_resolve(crate::msgs::pubsub::CallbackPayload::Json(parsed.clone()))?;
        self.handle_response(&parsed, now)?;
        Ok(parsed)
    }

    pub fn handle_binary_message(
        &mut self,
        buffer: &[u8],
        now: Instant,
    ) -> Result<Option<Value>, String> {
        let parsed = parse_array_buffer(buffer)?;
        let mut decoded_json = parsed.json_data;

        let topic_or_type = decoded_json
            .get("topic")
            .and_then(Value::as_str)
            .or_else(|| decoded_json.get("type").and_then(Value::as_str))
            .unwrap_or_default();

        if !self.pub_sub.should_process(topic_or_type, now) {
            return Ok(None);
        }

        if parsed.is_lidar {
            let decoded_data =
                if let Some(meta) = decoded_json.get("data").and_then(|d| d.as_object()) {
                    let origin_arr = meta.get("origin").and_then(|o| o.as_array()).unwrap();
                    let origin = [
                        origin_arr[0].as_f64().unwrap_or(0.0),
                        origin_arr[1].as_f64().unwrap_or(0.0),
                        origin_arr[2].as_f64().unwrap_or(0.0),
                    ];
                    let resolution = meta
                        .get("resolution")
                        .and_then(|r| r.as_f64())
                        .unwrap_or(0.05);

                    if self.decoder_type == "native" {
                        let points = crate::lidar::native::decode_native_core(
                            &parsed.binary_data,
                            1000000,
                            origin,
                            resolution,
                        )
                        .map_err(|e| format!("Native decode error: {}", e))?;
                        crate::msgs::pubsub::LidarDecodedData::Native(points)
                    } else {
                        let (point_count, face_count, positions, uvs, indices) =
                            crate::lidar::wasm::decode_wasm_core(
                                &parsed.binary_data,
                                origin,
                                resolution,
                            )
                            .map_err(|e| format!("Wasm decode error: {}", e))?;
                        crate::msgs::pubsub::LidarDecodedData::Wasm {
                            point_count,
                            face_count,
                            positions,
                            uvs,
                            indices,
                        }
                    }
                } else {
                    return Err("Missing metadata for LiDAR decoding".to_string());
                };

            let payload =
                crate::msgs::pubsub::CallbackPayload::Lidar(decoded_json.clone(), decoded_data);
            let _ = self.pub_sub.run_resolve(payload)?;
            return Ok(Some(decoded_json));
        }

        let bytes_as_json = Value::Array(
            parsed
                .binary_data
                .into_iter()
                .map(Value::from)
                .collect::<Vec<_>>(),
        );

        if let Some(data_obj) = decoded_json.get_mut("data").and_then(Value::as_object_mut) {
            data_obj.insert("data".to_string(), bytes_as_json);
        }

        if decoded_json.get("type").is_some() {
            let _ = self
                .pub_sub
                .run_resolve(crate::msgs::pubsub::CallbackPayload::Json(
                    decoded_json.clone(),
                ))?;
        }
        Ok(Some(decoded_json))
    }

    pub fn disable_traffic_saving(&self, switch: bool) -> Value {
        let data = json!({
            "req_type": "disable_traffic_saving",
            "instruction": if switch { "on" } else { "off" },
        });
        WebRTCDataChannelPubSub::build_publish_message("", Some(data), "rtc_inner_req")
    }

    pub fn switch_video_channel(&self, switch: bool) -> Value {
        WebRTCDataChannelPubSub::build_publish_message(
            "",
            Some(Value::String(if switch { "on" } else { "off" }.to_string())),
            "vid",
        )
    }

    pub fn switch_audio_channel(&self, switch: bool) -> Value {
        WebRTCDataChannelPubSub::build_publish_message(
            "",
            Some(Value::String(if switch { "on" } else { "off" }.to_string())),
            "aud",
        )
    }

    fn handle_response(&mut self, msg: &Value, now: Instant) -> Result<(), String> {
        let msg_type = msg
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing message type".to_string())?;

        match msg_type {
            "validation" => {
                if msg.get("data").and_then(Value::as_str) == Some("Validation Ok.") {
                    self.validation.mark_validated();
                    self.data_channel_opened = true;
                    self.heartbeat.start_heartbeat(now);
                } else if let Some(key) = msg.get("data").and_then(Value::as_str) {
                    self.validation.set_key(key);
                }
            }
            "rtc_inner_req" => {
                if let Some(info) = msg.get("info") {
                    let status = info
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let _ = self
                        .network_status
                        .handle_status(status, crate::constants::WebRTCConnectionMethod::LocalSTA);
                }
            }
            "heartbeat" => {
                self.heartbeat.handle_response(now);
            }
            "errors" | "add_error" | "rm_error" => {
                let _ = handle_error(msg);
            }
            "err" => {}
            _ => {}
        }

        Ok(())
    }
}
