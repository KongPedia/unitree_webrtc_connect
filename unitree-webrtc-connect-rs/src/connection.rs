use crate::audio::WebRTCAudioChannel;
use crate::constants::WebRTCConnectionMethod;
use crate::datachannel::WebRTCDataChannel;
use crate::msgs::pubsub::WebRTCDataChannelPubSub;
use crate::video::WebRTCVideoChannel;
use datachannel::{
    ConnectionState, DataChannelHandler, DataChannelInfo, GatheringState, IceCandidate, IceState,
    PeerConnectionHandler, RtcConfig, RtcDataChannel, RtcPeerConnection, SdpType,
    SessionDescription, SignalingState,
};
use rand::Rng;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};

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

    pub pc: Option<Box<RtcPeerConnection<PcHandler>>>,
    pub rtc_dc: Option<Box<RtcDataChannel<DcHandler>>>,
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
            pc: None,
            rtc_dc: None,
        }
    }

    pub async fn connect(&mut self, self_arc: Arc<Mutex<Self>>) -> Result<(), String> {
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
        let turn_info = None; // TODO: Remote

        let config_opts = self.create_webrtc_configuration(turn_info, true, false)?;
        let mut servers_str = Vec::new();
        for server in &config_opts.ice_servers {
            for url in &server.urls {
                servers_str.push(url.as_str());
            }
        }
        let rtc_config = RtcConfig::new(&servers_str);

        let (dc_tx, mut dc_rx) = mpsc::unbounded_channel();
        let (sdp_tx, sdp_rx) = oneshot::channel();
        let handler = PcHandler {
            sdp_tx: Some(sdp_tx),
            sdp: String::new(),
            evt_tx: dc_tx.clone(),
        };

        let mut pc = RtcPeerConnection::new(&rtc_config, handler)
            .map_err(|e| format!("Failed to create PC: {}", e))?;

        let dc_handler = DcHandler { tx: dc_tx };
        let _dc = pc
            .create_data_channel("data", dc_handler)
            .map_err(|e| format!("Failed to create DC: {}", e))?;

        pc.set_local_description(SdpType::Offer)
            .map_err(|e| format!("Failed to set local desc: {}", e))?;

        let local_sdp_str = sdp_rx
            .await
            .map_err(|_| "ICE gathering failed or timed out".to_string())?;

        println!("Generated local SDP offer");

        let offer_json = json!({
            "type": "offer",
            "sdp": local_sdp_str,
        });
        let offer_str = serde_json::to_string(&offer_json).unwrap();

        let peer_answer_json_str = match self.connection_method {
            WebRTCConnectionMethod::Remote => {
                return Err("Remote signaling not yet implemented in Rust".to_string());
            }
            WebRTCConnectionMethod::LocalSTA | WebRTCConnectionMethod::LocalAP => {
                let ip_str = self.ip.as_deref().unwrap_or("192.168.12.1");
                crate::signaling::send_sdp_to_local_peer(ip_str, &offer_str)
                    .await
                    .map_err(|e| format!("Signaling failed: {:?}", e))?
            }
        };

        // Parse peer answer
        let remote_desc: SessionDescription = serde_json::from_str(&peer_answer_json_str)
            .map_err(|e| format!("Failed to parse peer answer as SessionDescription: {}", e))?;

        pc.set_remote_description(&remote_desc)
            .map_err(|e| format!("Failed to set remote description: {}", e))?;

        println!("Successfully set remote description");

        self.init_webrtc();
        self.pc = Some(pc);
        self.rtc_dc = Some(_dc);
        self.is_connected = true;

        // Spawn datachannel background task
        tokio::spawn(async move {
            while let Some(evt) = dc_rx.recv().await {
                // To be implemented: acquire lock on self_arc and pass events to self.datachannel
                let mut inner = match self_arc.lock() {
                    Ok(guard) => guard,
                    Err(_) => break, // poison
                };
                if inner.datachannel.is_none() {
                    continue;
                }
                match evt {
                    DcEvent::Open => {
                        println!("Data channel opened");
                    }
                    DcEvent::Closed => {
                        println!("Data channel closed");
                    }
                    DcEvent::Error(e) => {
                        println!("Data channel error: {:?}", e);
                    }
                    DcEvent::Message(msg) => {
                        // Forward to datachannel handler
                        let now = std::time::Instant::now();
                        let is_text = msg.iter().all(|&b| {
                            (32..=126).contains(&b)
                                || b == b'\n'
                                || b == b'\r'
                                || b == b'{'
                                || b == b'}'
                        });

                        if is_text {
                            if let Ok(msg_str) = String::from_utf8(msg) {
                                let _ = inner
                                    .datachannel
                                    .as_mut()
                                    .unwrap()
                                    .handle_text_message(&msg_str, now);
                            }
                        } else {
                            let _ = inner
                                .datachannel
                                .as_mut()
                                .unwrap()
                                .handle_binary_message(&msg, now);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn disconnect(&mut self) {
        self.intentional_disconnect = true;
        self.is_connected = false;
        self.datachannel = None;
        self.audio = None;
        self.video = None;
    }

    pub async fn reconnect(&mut self, self_arc: Arc<Mutex<Self>>) -> Result<(), String> {
        self.disconnect().await;
        self.connect(self_arc).await
    }

    pub async fn auto_reconnect(&mut self, self_arc: Arc<Mutex<Self>>, max_retries: u32) -> bool {
        self.auto_reconnect_with_backoff(self_arc, max_retries, Duration::from_millis(200))
            .await
    }

    pub async fn auto_reconnect_with_backoff(
        &mut self,
        self_arc: Arc<Mutex<Self>>,
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

            if self.reconnect(Arc::clone(&self_arc)).await.is_ok() {
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

    pub fn publish_send_req(&mut self, topic: &str, payload: Value) -> Result<String, String> {
        let dc_open = self
            .datachannel
            .as_ref()
            .map(|dc| dc.is_open())
            .unwrap_or(false);

        if !self.is_connected || !dc_open {
            return Err("Connection or Data channel is not established".to_string());
        }

        let api_id = payload
            .get("api_id")
            .and_then(Value::as_i64)
            .unwrap_or(1001);

        let req_id = generate_request_id();
        let identity_id = payload.get("id").and_then(Value::as_i64).unwrap_or(req_id);

        let mut request_payload = payload.clone();
        if request_payload.get("api_id").is_none() {
            if let Some(obj) = request_payload.as_object_mut() {
                obj.insert("api_id".to_string(), Value::Number(api_id.into()));
            }
        }
        if request_payload.get("id").is_none() {
            if let Some(obj) = request_payload.as_object_mut() {
                obj.insert("id".to_string(), Value::Number(identity_id.into()));
            }
        }

        let message =
            WebRTCDataChannelPubSub::build_publish_message(topic, Some(request_payload), "req");

        let msg_str = serde_json::to_string(&message)
            .map_err(|e| format!("Failed to serialize request payload: {e}"))?;

        if let Some(dc) = &mut self.rtc_dc {
            dc.send(msg_str.as_bytes())
                .map_err(|e| format!("DataChannel send error: {e}"))?;
        } else {
            return Err("Data channel missing inner RtcDataChannel".to_string());
        }

        let id_str = identity_id.to_string();
        let key = crate::msgs::future_resolver::FutureResolver::generate_message_key(
            "req",
            topic,
            Some(&id_str),
        );

        if let Some(dc) = &mut self.datachannel {
            dc.save_resolve("req", topic, Some(&id_str));
        }

        Ok(key)
    }

    pub fn publish_without_callback(
        &mut self,
        topic: &str,
        data: Option<Value>,
        msg_type: &str,
    ) -> Result<(), String> {
        let dc_open = self
            .datachannel
            .as_ref()
            .map(|dc| dc.is_open())
            .unwrap_or(false);

        if !self.is_connected || !dc_open {
            return Err("Connection or Data channel is not established".to_string());
        }

        let message = WebRTCDataChannelPubSub::build_publish_message(topic, data, msg_type);

        let msg_str = serde_json::to_string(&message)
            .map_err(|e| format!("Failed to serialize payload: {e}"))?;

        if let Some(dc) = &mut self.rtc_dc {
            dc.send(msg_str.as_bytes())
                .map_err(|e| format!("DataChannel send error: {e}"))?;
        } else {
            return Err("Data channel missing inner RtcDataChannel".to_string());
        }

        Ok(())
    }

    pub fn subscribe(
        &mut self,
        topic: &str,
        callback: crate::msgs::pubsub::TopicCallback,
    ) -> Result<(), String> {
        if let Some(dc) = &mut self.datachannel {
            dc.subscribe(topic, callback);
        }
        self.publish_without_callback(topic, Some(Value::String("".to_string())), "sub")
    }

    pub fn unsubscribe(&mut self, topic: &str) -> Result<(), String> {
        if let Some(dc) = &mut self.datachannel {
            dc.unsubscribe(topic);
        }
        self.publish_without_callback(topic, Some(Value::String("".to_string())), "unsub")
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

pub struct PcHandler {
    pub sdp_tx: Option<oneshot::Sender<String>>,
    pub sdp: String,
    pub evt_tx: mpsc::UnboundedSender<DcEvent>,
}

impl PeerConnectionHandler for PcHandler {
    type DCH = DcHandler;

    fn data_channel_handler(&mut self, _info: DataChannelInfo) -> Self::DCH {
        DcHandler {
            tx: self.evt_tx.clone(),
        }
    }

    fn on_description(&mut self, sess_desc: SessionDescription) {
        self.sdp = sess_desc.sdp.to_string();
    }

    fn on_candidate(&mut self, _cand: IceCandidate) {}
    fn on_connection_state_change(&mut self, _state: ConnectionState) {}
    fn on_gathering_state_change(&mut self, state: GatheringState) {
        if state == GatheringState::Complete {
            if let Some(tx) = self.sdp_tx.take() {
                let _ = tx.send(self.sdp.clone());
            }
        }
    }
    fn on_signaling_state_change(&mut self, _state: SignalingState) {}
    fn on_ice_state_change(&mut self, _state: IceState) {}
    fn on_data_channel(&mut self, _data_channel: Box<RtcDataChannel<Self::DCH>>) {}
}

pub enum DcEvent {
    Open,
    Closed,
    Message(Vec<u8>),
    Error(String),
}

pub struct DcHandler {
    pub tx: mpsc::UnboundedSender<DcEvent>,
}

impl DataChannelHandler for DcHandler {
    fn on_open(&mut self) {
        let _ = self.tx.send(DcEvent::Open);
    }
    fn on_closed(&mut self) {
        let _ = self.tx.send(DcEvent::Closed);
    }
    fn on_error(&mut self, err: &str) {
        let _ = self.tx.send(DcEvent::Error(err.to_string()));
    }
    fn on_message(&mut self, msg: &[u8]) {
        let _ = self.tx.send(DcEvent::Message(msg.to_vec()));
    }
    fn on_buffered_amount_low(&mut self) {}
    fn on_available(&mut self) {}
}
