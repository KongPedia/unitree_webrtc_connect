import asyncio
import json
import os

from .constants import WebRTCConnectionMethod
from .webrtc_driver import UnitreeWebRTCConnection as _PythonUnitreeWebRTCConnection

try:
    from unitree_webrtc_core_rs import (
        UnitreeWebRTCConnection as _RustUnitreeWebRTCConnection,
        WebRTCConnectionMethod as _RustWebRTCConnectionMethod,
    )
except ImportError:  # pragma: no cover
    _RustUnitreeWebRTCConnection = None
    _RustWebRTCConnectionMethod = None


def _to_rust_connection_method(connection_method):
    if _RustWebRTCConnectionMethod is None:
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


class _RustPubSubBridge:
    def __init__(self, connection) -> None:
        self._connection = connection

    @property
    def _py_pub_sub(self):
        datachannel = self._connection._py_datachannel
        return getattr(datachannel, "pub_sub", None)

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
            raise RuntimeError("subscribe is unavailable without Python transport")
        return self._py_pub_sub.subscribe(topic, callback=callback)

    def unsubscribe(self, topic):
        if self._py_pub_sub is None:
            raise RuntimeError("unsubscribe is unavailable without Python transport")
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
            return
        return await self._py_datachannel.wait_datachannel_open(timeout=timeout)


class UnitreeWebRTCConnection:
    def __init__(
        self,
        connection_method: WebRTCConnectionMethod,
        serial_number=None,
        ip=None,
        username=None,
        password=None,
        **kwargs,
    ) -> None:
        if "connectionMethod" in kwargs:
            connection_method = kwargs["connectionMethod"]
        if "serialNumber" in kwargs:
            serial_number = kwargs["serialNumber"]

        self._strict_rust = os.getenv("UNITREE_WEBRTC_RS_STRICT", "0") == "1"
        self._rust_available = _RustUnitreeWebRTCConnection is not None

        if self._rust_available:
            rust_connection_method = _to_rust_connection_method(connection_method)
            self._inner = _RustUnitreeWebRTCConnection(
                rust_connection_method,
                serial_number=serial_number,
                ip=ip,
                username=username,
                password=password,
            )
        else:
            self._inner = None

        self._python = _PythonUnitreeWebRTCConnection(
            connectionMethod=connection_method,
            serialNumber=serial_number,
            ip=ip,
            username=username,
            password=password,
        )
        self._use_python_transport = not self._strict_rust

        self.connection_method = connection_method
        self.sn = serial_number
        self.ip = ip
        self.is_connected = False
        self.datachannel = _RustDataChannelBridge(self)
        self.audio = None
        self.video = None
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
