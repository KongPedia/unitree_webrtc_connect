import asyncio
import logging
import json
import sys
import os

from .constants import DATA_CHANNEL_TYPE, WebRTCConnectionMethod, RTC_TOPIC, SPORT_CMD
from .util import fetch_public_key, fetch_token, fetch_turn_server_info, print_status
from .multicast_scanner import discover_ip_sn
from .unitree_auth import send_sdp_to_local_peer, send_sdp_to_remote_peer
from .webrtc_datachannel import WebRTCDataChannel
from .webrtc_audio import WebRTCAudioChannel
from .webrtc_video import WebRTCVideoChannel
from .lidar.lidar_decoder_unified import UnifiedLidarDecoder

try:
    from aiortc import RTCPeerConnection, RTCSessionDescription, RTCIceServer, RTCConfiguration
    from aiortc.contrib.media import MediaPlayer
    _AIORTC_AVAILABLE = True
except ImportError:
    _AIORTC_AVAILABLE = False

try:
    from unitree_webrtc_connect_rs import (
        UnitreeWebRTCConnection as _RustUnitreeWebRTCConnection,
        WebRTCConnectionMethod as _RustWebRTCConnectionMethod,
    )
    _RUST_AVAILABLE = True
except ImportError:
    _RustUnitreeWebRTCConnection = None
    _RustWebRTCConnectionMethod = None
    _RUST_AVAILABLE = False

# Enable logging for debugging
# logging.basicConfig(level=logging.INFO)

def _to_rust_connection_method(connection_method):
    if not _RUST_AVAILABLE:
        raise RuntimeError("Rust backend is not available")

    if isinstance(connection_method, _RustWebRTCConnectionMethod):
        return connection_method

    if isinstance(connection_method, WebRTCConnectionMethod):
        return getattr(_RustWebRTCConnectionMethod, connection_method.name)

    if isinstance(connection_method, str):
        return getattr(_RustWebRTCConnectionMethod, connection_method)

    raise TypeError(
        "connection_method must be WebRTCConnectionMethod (python/rust) or a valid enum name string"
    )

class _PythonWebRTCBackend:
    def __init__(self, connectionMethod: WebRTCConnectionMethod, serialNumber=None, ip=None, username=None, password=None) -> None:
        if not _AIORTC_AVAILABLE:
            raise RuntimeError("aiortc is not installed. Cannot use Python backend.")
        
        self.pc = None
        self.sn = serialNumber
        self.ip = ip
        self.connectionMethod = connectionMethod
        self.isConnected = False
        self.token = fetch_token(username, password) if username and password else ""
        self.datachannel = None
        self.audio = None
        self.video = None

    async def connect(self):
        self._intentional_disconnect = False
        print_status("WebRTC connection", "🟡 started")
        if self.connectionMethod == WebRTCConnectionMethod.Remote:
            self.public_key = fetch_public_key()
            turn_server_info = fetch_turn_server_info(self.sn, self.token, self.public_key)
            await self.init_webrtc(turn_server_info)
        elif self.connectionMethod == WebRTCConnectionMethod.LocalSTA:
            if not self.ip and self.sn:
                discovered_ip_sn_addresses = discover_ip_sn()
                
                if discovered_ip_sn_addresses:
                    if self.sn in discovered_ip_sn_addresses:
                        self.ip = discovered_ip_sn_addresses[self.sn]
                    else:
                        raise ValueError("The provided serial number wasn't found on the network. Provide an IP address instead.")
                else:
                    raise ValueError("No devices found on the network. Provide an IP address instead.")

            await self.init_webrtc(ip=self.ip)
        elif self.connectionMethod == WebRTCConnectionMethod.LocalAP:
            self.ip = "192.168.12.1"
            await self.init_webrtc(ip=self.ip)
    
    async def disconnect(self):
        self._intentional_disconnect = True
        if self.pc:
            await self.pc.close()
            self.pc = None
        self.isConnected = False
        print_status("WebRTC connection", "🔴 disconnected")

    async def reconnect(self):
        await self.disconnect()
        await self.connect()
        print_status("WebRTC connection", "🟢 reconnected")

    async def _auto_reconnect(self, max_retries=5):
        if getattr(self, "_intentional_disconnect", False):
            return False
        if getattr(self, "_is_reconnecting", False):
            return False
        
        self._is_reconnecting = True
        logging.warning("Initiating auto-reconnect sequence...")
        
        for attempt in range(max_retries):
            try:
                logging.info(f"Auto-reconnect attempt {attempt + 1}/{max_retries}")
                await asyncio.sleep(2 ** attempt)  # exponential backoff
                await self.reconnect()
                
                # Wait briefly after reconnect before sending commands
                await asyncio.sleep(1.0)
                
                # Automatically send RecoveryStand
                if hasattr(self, "datachannel") and self.datachannel and getattr(self.datachannel, "pub_sub", None):
                    logging.info("Auto-reconnect successful. Sending RecoveryStand...")
                    await self.datachannel.pub_sub.publish_request_new(
                        RTC_TOPIC["SPORT_MOD"],
                        {"api_id": SPORT_CMD["RecoveryStand"]}
                    )
                
                self._is_reconnecting = False
                return True
            except Exception as e:
                logging.error(f"Reconnect attempt {attempt+1} failed: {e}")
                
        logging.error("All auto-reconnect attempts failed.")
        self._is_reconnecting = False
        return False

    def create_webrtc_configuration(self, turn_server_info, stunEnable=True, turnEnable=True):
        ice_servers = []

        if turn_server_info:
            username = turn_server_info.get("user")
            credential = turn_server_info.get("passwd")
            turn_url = turn_server_info.get("realm")
            
            if username and credential and turn_url:
                if turnEnable:
                    ice_servers.append(
                        RTCIceServer(
                            urls=[turn_url],
                            username=username,
                            credential=credential
                        )
                    )
                if stunEnable:
                    # Use Google's public STUN server
                    stun_url = "stun:stun.l.google.com:19302"
                    ice_servers.append(
                        RTCIceServer(
                            urls=[stun_url]
                        )
                    )
            else:
                raise ValueError("Invalid TURN server information")
        
        configuration = RTCConfiguration(
            iceServers=ice_servers
        )
        
        return configuration

    async def init_webrtc(self, turn_server_info=None, ip=None):
        configuration = self.create_webrtc_configuration(turn_server_info)
        self.pc = RTCPeerConnection(configuration)

        self.datachannel = WebRTCDataChannel(self, self.pc)
        self.audio = WebRTCAudioChannel(self.pc, self.datachannel)
        self.video = WebRTCVideoChannel(self.pc, self.datachannel)

        @self.pc.on("icegatheringstatechange")
        async def on_ice_gathering_state_change():
            state = self.pc.iceGatheringState
            if state == "new":
                print_status("ICE Gathering State", "🔵 new")
            elif state == "gathering":
                print_status("ICE Gathering State", "🟡 gathering")
            elif state == "complete":
                print_status("ICE Gathering State", "🟢 complete")

        @self.pc.on("iceconnectionstatechange")
        async def on_ice_connection_state_change():
            state = self.pc.iceConnectionState
            if state == "checking":
                print_status("ICE Connection State", "🔵 checking")
            elif state == "completed":
                print_status("ICE Connection State", "🟢 completed")
            elif state == "failed":
                print_status("ICE Connection State", "🔴 failed")
            elif state == "closed":
                print_status("ICE Connection State", "⚫ closed")

        @self.pc.on("connectionstatechange")
        async def on_connection_state_change():
            state = self.pc.connectionState
            if state == "connecting":
                print_status("Peer Connection State", "🔵 connecting")
            elif state == "connected":
                self.isConnected= True
                print_status("Peer Connection State", "🟢 connected")
            elif state == "closed":
                self.isConnected= False
                print_status("Peer Connection State", "⚫ closed")
                asyncio.create_task(self._auto_reconnect())
            elif state == "failed":
                print_status("Peer Connection State", "🔴 failed")
                asyncio.create_task(self._auto_reconnect())
        
        @self.pc.on("signalingstatechange")
        async def on_signaling_state_change():
            state = self.pc.signalingState
            if state == "stable":
                print_status("Signaling State", "🟢 stable")
            elif state == "have-local-offer":
                print_status("Signaling State", "🟡 have-local-offer")
            elif state == "have-remote-offer":
                print_status("Signaling State", "🟡 have-remote-offer")
            elif state == "closed":
                print_status("Signaling State", "⚫ closed")
        
        @self.pc.on("track")
        async def on_track(track):
            logging.info("Track recieved: %s", track.kind)

            from aiortc.mediastreams import MediaStreamError
            try:
                if track.kind == "video":
                    #await for the first frame, #ToDo make the code more nicer
                    frame = await track.recv()
                    await self.video.track_handler(track)
                    
                if track.kind == "audio":
                    frame = await track.recv()
                    while True:
                        frame = await track.recv()
                        await self.audio.frame_handler(frame)
            except MediaStreamError:
                logging.info(f"Track {track.kind} ended or encountered a MediaStreamError. This usually happens when connection resets.")

        logging.info("Creating offer...")
        offer = await self.pc.createOffer()
        await self.pc.setLocalDescription(offer)

        if self.connectionMethod == WebRTCConnectionMethod.Remote:
            peer_answer_json = await self.get_answer_from_remote_peer(self.pc, turn_server_info)
        elif self.connectionMethod == WebRTCConnectionMethod.LocalSTA or self.connectionMethod == WebRTCConnectionMethod.LocalAP:
            peer_answer_json = await self.get_answer_from_local_peer(self.pc, self.ip)

        if peer_answer_json is not None:
            peer_answer = json.loads(peer_answer_json)
        else:
            print("Could not get SDP from the peer. Check if the Go2 is switched on")
            sys.exit(1)

        if peer_answer['sdp'] == "reject":
            print("Go2 is connected by another WebRTC client. Close your mobile APP and try again.")
            sys.exit(1)

        remote_sdp = RTCSessionDescription(sdp=peer_answer['sdp'], type=peer_answer['type']) 
        await self.pc.setRemoteDescription(remote_sdp)
   
        await self.datachannel.wait_datachannel_open()

    
    async def get_answer_from_remote_peer(self, pc, turn_server_info):
        sdp_offer = pc.localDescription

        sdp_offer_json = {
            "id": "",
            "turnserver": turn_server_info,
            "sdp": sdp_offer.sdp,
            "type": sdp_offer.type,
            "token": self.token
        }

        logging.debug("Local SDP created: %s", sdp_offer_json)

        peer_answer_json = send_sdp_to_remote_peer(self.sn, json.dumps(sdp_offer_json), self.token, self.public_key)

        return peer_answer_json

    async def get_answer_from_local_peer(self, pc, ip):
        sdp_offer = pc.localDescription

        sdp_offer_json = {
            "id": "STA_localNetwork" if self.connectionMethod == WebRTCConnectionMethod.LocalSTA else "",
            "sdp": sdp_offer.sdp,
            "type": sdp_offer.type,
            "token": self.token
        }

        peer_answer_json = send_sdp_to_local_peer(ip, json.dumps(sdp_offer_json))

        return peer_answer_json


class _RustPubSubBridge:
    def __init__(self, connection) -> None:
        self._connection = connection

    @property
    def _py_pub_sub(self):
        datachannel = self._connection._py_datachannel
        return getattr(datachannel, "pub_sub", None) if datachannel else None

    async def publish_request_new(self, topic, payload, timeout=10.0):
        if self._connection._use_python_transport:
            return await self._py_pub_sub.publish_request_new(topic, payload, timeout=timeout)

        payload_json = json.dumps(payload, ensure_ascii=True)
        try:
            response_json = await asyncio.to_thread(
                self._connection._inner.publish_request_new,
                topic,
                payload_json,
            )
            return json.loads(response_json)
        except Exception as exc:
            if self._py_pub_sub is not None:
                return await self._py_pub_sub.publish_request_new(topic, payload, timeout=timeout)
            raise exc

    def publish_without_callback(self, topic, data=None, msg_type=None):
        if self._py_pub_sub is None:
            raise RuntimeError("publish_without_callback is unavailable without Python transport")
        return self._py_pub_sub.publish_without_callback(topic, data=data, msg_type=msg_type)

    def subscribe(self, topic, callback=None):
        if self._py_pub_sub is None:
            return self._connection._inner.subscribe(topic, callback)
        return self._py_pub_sub.subscribe(topic, callback=callback)

    def unsubscribe(self, topic):
        if self._py_pub_sub is None:
            return self._connection._inner.unsubscribe(topic)
        return self._py_pub_sub.unsubscribe(topic)


class _RustDataChannelBridge:
    def __init__(self, connection) -> None:
        self._connection = connection
        self.pub_sub = _RustPubSubBridge(connection)

    @property
    def _py_datachannel(self):
        return self._connection._py_datachannel

    @property
    def channel(self):
        if self._py_datachannel is None:
            raise RuntimeError("channel is unavailable without Python transport")
        return self._py_datachannel.channel

    async def disableTrafficSaving(self, switch: bool):
        if self._py_datachannel is None:
            message = self._connection._inner.disable_traffic_saving(switch)
            return bool(message)
        return await self._py_datachannel.disableTrafficSaving(switch)

    def switchVideoChannel(self, switch: bool):
        if self._py_datachannel is not None:
            return self._py_datachannel.switchVideoChannel(switch)
        return self._connection._inner.switch_video_channel(switch)

    def switchAudioChannel(self, switch: bool):
        if self._py_datachannel is not None:
            return self._py_datachannel.switchAudioChannel(switch)
        return self._connection._inner.switch_audio_channel(switch)

    def set_decoder(self, decoder_type):
        if self._py_datachannel is None:
            return self._connection._inner.set_decoder(decoder_type)
        return self._py_datachannel.set_decoder(decoder_type)

    async def wait_datachannel_open(self, timeout=5):
        if self._py_datachannel is None:
            if self._connection._inner is not None:
                return await asyncio.to_thread(self._connection._inner.wait_datachannel_open, timeout)
            return
        return await self._py_datachannel.wait_datachannel_open(timeout=timeout)


class _RustVideoBridge:
    def __init__(self, connection):
        self._connection = connection

    def add_track_callback(self, callback):
        if self._connection._py_datachannel is not None:
            self._connection._python.video.add_track_callback(callback)
            return
        if self._connection._inner is not None:
            self._connection._inner.add_video_track_callback(callback)

    def switchVideoChannel(self, switch: bool):
        if self._connection._py_datachannel is not None:
            self._connection._python.video.switchVideoChannel(switch)
            return
        if self._connection._inner is not None:
            self._connection._inner.switch_video_channel(switch)


class _RustAudioBridge:
    def __init__(self, connection):
        self._connection = connection

    def add_track_callback(self, callback):
        if self._connection._py_datachannel is not None:
            self._connection._python.audio.add_track_callback(callback)
            return
        if self._connection._inner is not None:
            self._connection._inner.add_audio_track_callback(callback)

    def switchAudioChannel(self, switch: bool):
        if self._connection._py_datachannel is not None:
            self._connection._python.audio.switchAudioChannel(switch)
            return
        if self._connection._inner is not None:
            self._connection._inner.switch_audio_channel(switch)


class UnitreeWebRTCConnection:
    def __init__(
        self,
        connection_method: WebRTCConnectionMethod,
        serial_number=None,
        ip=None,
        username=None,
        password=None,
        backend=None,
        **kwargs,
    ) -> None:
        if "connectionMethod" in kwargs:
            connection_method = kwargs["connectionMethod"]
        if "serialNumber" in kwargs:
            serial_number = kwargs["serialNumber"]
            
        if backend is None:
            env_backend = os.getenv("UNITREE_WEBRTC_BACKEND", "").lower()
            if env_backend in ("rust", "python"):
                backend = env_backend
            else:
                backend = "rust" if _RUST_AVAILABLE else "python"
        
        if backend == "rust" and not _RUST_AVAILABLE:
            logging.warning("Rust backend was requested but unitree_webrtc_connect_rs is not installed. Falling back to python.")
            backend = "python"
            
        if backend == "python" and not _AIORTC_AVAILABLE:
            if _RUST_AVAILABLE:
                logging.warning("Python backend requested but aiortc is missing. Falling back to rust.")
                backend = "rust"
            else:
                raise RuntimeError("Neither aiortc nor unitree_webrtc_connect_rs are available.")

        self._use_python_transport = (backend == "python")
        self._backend_type = backend

        if self._use_python_transport:
            self._inner = None
            self._python = _PythonWebRTCBackend(
                connectionMethod=connection_method,
                serialNumber=serial_number,
                ip=ip,
                username=username,
                password=password,
            )
        else:
            self._python = None
            rust_connection_method = _to_rust_connection_method(connection_method)
            self._inner = _RustUnitreeWebRTCConnection(
                rust_connection_method,
                serial_number=serial_number,
                ip=ip,
                username=username,
                password=password,
            )

        self.connection_method = connection_method
        self.sn = serial_number
        self.ip = ip
        self.is_connected = False
        self.datachannel = _RustDataChannelBridge(self)
        self.audio = _RustAudioBridge(self)
        self.video = _RustVideoBridge(self)
        self.pc = None

    @property
    def _py_datachannel(self):
        return getattr(self._python, "datachannel", None)

    def _read_rust_bool(self, attr_name: str) -> bool:
        attr = getattr(self._inner, attr_name)
        return attr() if callable(attr) else bool(attr)

    def _read_rust_value(self, attr_name: str):
        attr = getattr(self._inner, attr_name)
        return attr() if callable(attr) else attr

    def _sync_from_python(self):
        self.is_connected = self._python.isConnected
        self.ip = self._python.ip
        self.audio = getattr(self._python, "audio", None)
        self.video = getattr(self._python, "video", None)
        self.pc = getattr(self._python, "pc", None)

    def _sync_from_rust(self):
        self.is_connected = self._read_rust_bool("is_connected")
        self.ip = self._read_rust_value("ip")

    async def connect(self):
        if self._use_python_transport:
            await self._python.connect()
            self._sync_from_python()
            return

        if self._inner is None:
            raise RuntimeError("Rust backend is not available")

        await asyncio.to_thread(self._inner.connect)
        self._sync_from_rust()

    async def disconnect(self):
        if self._use_python_transport:
            await self._python.disconnect()
            self._sync_from_python()
            return

        if self._inner is None:
            raise RuntimeError("Rust backend is not available")

        await asyncio.to_thread(self._inner.disconnect)
        self._sync_from_rust()

    async def reconnect(self):
        if self._use_python_transport:
            await self._python.reconnect()
            self._sync_from_python()
            return

        if self._inner is None:
            raise RuntimeError("Rust backend is not available")

        await asyncio.to_thread(self._inner.reconnect)
        self._sync_from_rust()

    async def _auto_reconnect(self, max_retries=5):
        if self._use_python_transport:
            ok = await self._python._auto_reconnect(max_retries=max_retries)
            self._sync_from_python()
            return ok

        if self._inner is None:
            raise RuntimeError("Rust backend is not available")

        ok = await asyncio.to_thread(self._inner.auto_reconnect, max_retries)
        self._sync_from_rust()
        return ok

    @property
    def connectionMethod(self):
        return self.connection_method

    @property
    def isConnected(self):
        return self.is_connected
