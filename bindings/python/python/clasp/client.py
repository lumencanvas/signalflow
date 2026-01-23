"""
CLASP Python client

Supports binary encoding for efficient wire format.
Backward compatible: can decode MessagePack frames (legacy).
"""

import asyncio
import struct
import time
import fnmatch
from typing import Any, Callable, Dict, List, Optional, Union, Tuple
from dataclasses import dataclass, field

try:
    import websockets
    HAS_WEBSOCKETS = True
except ImportError:
    HAS_WEBSOCKETS = False

try:
    import msgpack
    HAS_MSGPACK = True
except ImportError:
    HAS_MSGPACK = False

from .types import (
    Value,
    SignalType,
    QoS,
    PROTOCOL_VERSION,
    WS_SUBPROTOCOL,
    SubscriptionCallback,
)


# Message type codes (binary format)
MSG_HELLO = 0x01
MSG_WELCOME = 0x02
MSG_ANNOUNCE = 0x03
MSG_SUBSCRIBE = 0x10
MSG_UNSUBSCRIBE = 0x11
MSG_PUBLISH = 0x20
MSG_SET = 0x21
MSG_GET = 0x22
MSG_SNAPSHOT = 0x23
MSG_BUNDLE = 0x30
MSG_SYNC = 0x40
MSG_PING = 0x41
MSG_PONG = 0x42
MSG_ACK = 0x50
MSG_ERROR = 0x51
MSG_QUERY = 0x60
MSG_RESULT = 0x61

# Value type codes (binary format)
VAL_NULL = 0x00
VAL_BOOL = 0x01
VAL_I8 = 0x02
VAL_I16 = 0x03
VAL_I32 = 0x04
VAL_I64 = 0x05
VAL_F32 = 0x06
VAL_F64 = 0x07
VAL_STRING = 0x08
VAL_BYTES = 0x09
VAL_ARRAY = 0x0A
VAL_MAP = 0x0B

# Signal type codes
SIG_PARAM = 0
SIG_EVENT = 1
SIG_STREAM = 2
SIG_GESTURE = 3
SIG_TIMELINE = 4


class ClaspError(Exception):
    """CLASP client error"""
    pass


@dataclass
class ClaspBuilder:
    """Builder for CLASP client"""
    url: str
    name: str = "CLASP Python Client"
    features: List[str] = field(default_factory=lambda: ["param", "event", "stream"])
    token: Optional[str] = None
    reconnect: bool = True
    reconnect_interval: float = 5.0

    def with_name(self, name: str) -> "ClaspBuilder":
        """Set client name"""
        self.name = name
        return self

    def with_features(self, features: List[str]) -> "ClaspBuilder":
        """Set supported features"""
        self.features = features
        return self

    def with_token(self, token: str) -> "ClaspBuilder":
        """Set authentication token"""
        self.token = token
        return self

    def with_reconnect(self, enabled: bool, interval: float = 5.0) -> "ClaspBuilder":
        """Configure reconnection"""
        self.reconnect = enabled
        self.reconnect_interval = interval
        return self

    async def connect(self) -> "Clasp":
        """Build and connect"""
        client = Clasp(
            url=self.url,
            name=self.name,
            features=self.features,
            token=self.token,
            reconnect=self.reconnect,
            reconnect_interval=self.reconnect_interval,
        )
        await client.connect()
        return client


class Clasp:
    """
    CLASP client for Python

    Example:
        >>> sf = Clasp('ws://localhost:7330')
        >>> await sf.connect()
        >>>
        >>> @sf.on('/lumen/layer/*/opacity')
        >>> def handle_opacity(value, address):
        ...     print(f'{address} = {value}')
        >>>
        >>> await sf.set('/lumen/layer/0/opacity', 0.75)
    """

    def __init__(
        self,
        url: str,
        name: str = "CLASP Python Client",
        features: Optional[List[str]] = None,
        token: Optional[str] = None,
        reconnect: bool = True,
        reconnect_interval: float = 5.0,
    ):
        if not HAS_WEBSOCKETS:
            raise ImportError("websockets package required: pip install websockets")
        if not HAS_MSGPACK:
            raise ImportError("msgpack package required: pip install msgpack")

        self.url = url
        self.name = name
        self.features = features or ["param", "event", "stream"]
        self.token = token
        self.reconnect = reconnect
        self.reconnect_interval = reconnect_interval

        self._ws: Optional[websockets.WebSocketClientProtocol] = None
        self._session_id: Optional[str] = None
        self._connected = False
        self._params: Dict[str, Value] = {}
        self._subscriptions: Dict[int, tuple] = {}  # id -> (pattern, callback)
        self._next_sub_id = 1
        self._server_time_offset = 0
        self._pending_gets: Dict[str, asyncio.Future] = {}
        self._receive_task: Optional[asyncio.Task] = None

        # Callbacks
        self._on_connect: List[Callable] = []
        self._on_disconnect: List[Callable] = []
        self._on_error: List[Callable] = []

    @classmethod
    def builder(cls, url: str) -> ClaspBuilder:
        """Create a builder"""
        return ClaspBuilder(url=url)

    @property
    def connected(self) -> bool:
        """Check if connected"""
        return self._connected

    @property
    def session_id(self) -> Optional[str]:
        """Get session ID"""
        return self._session_id

    def time(self) -> int:
        """Get current server time (microseconds)"""
        return int(time.time() * 1_000_000) + self._server_time_offset

    async def connect(self) -> None:
        """Connect to server"""
        if self._connected:
            raise ClaspError("Already connected")

        try:
            self._ws = await websockets.connect(
                self.url,
                subprotocols=[WS_SUBPROTOCOL],
            )

            # Send HELLO
            await self._send({
                "type": "HELLO",
                "version": PROTOCOL_VERSION,
                "name": self.name,
                "features": self.features,
                "token": self.token,
            })

            # Wait for WELCOME
            while True:
                data = await self._ws.recv()
                msg = self._decode(data)

                if msg.get("type") == "WELCOME":
                    self._session_id = msg["session"]
                    self._server_time_offset = msg["time"] - int(time.time() * 1_000_000)
                    self._connected = True
                    break

            # Start receive loop
            self._receive_task = asyncio.create_task(self._receive_loop())

            # Notify callbacks
            for cb in self._on_connect:
                cb()

        except Exception as e:
            raise ClaspError(f"Connection failed: {e}") from e

    async def close(self) -> None:
        """Close connection"""
        self.reconnect = False
        self._connected = False

        if self._receive_task:
            self._receive_task.cancel()
            try:
                await self._receive_task
            except asyncio.CancelledError:
                pass

        if self._ws:
            await self._ws.close()
            self._ws = None

    def subscribe(
        self,
        pattern: str,
        callback: SubscriptionCallback,
        **options,
    ) -> Callable[[], None]:
        """
        Subscribe to address pattern

        Args:
            pattern: Address pattern (e.g., '/lumen/layer/*/opacity')
            callback: Function called with (value, address)
            **options: maxRate, epsilon, history

        Returns:
            Unsubscribe function
        """
        sub_id = self._next_sub_id
        self._next_sub_id += 1

        self._subscriptions[sub_id] = (pattern, callback)

        # Send subscribe message
        asyncio.create_task(self._send({
            "type": "SUBSCRIBE",
            "id": sub_id,
            "pattern": pattern,
            "options": options if options else None,
        }))

        def unsubscribe():
            if sub_id in self._subscriptions:
                del self._subscriptions[sub_id]
                asyncio.create_task(self._send({
                    "type": "UNSUBSCRIBE",
                    "id": sub_id,
                }))

        return unsubscribe

    def on(self, pattern: str, **options) -> Callable[[SubscriptionCallback], SubscriptionCallback]:
        """
        Decorator for subscribing to address pattern

        Example:
            @sf.on('/lumen/layer/*/opacity')
            def handle_opacity(value, address):
                print(f'{address} = {value}')
        """
        def decorator(func: SubscriptionCallback) -> SubscriptionCallback:
            self.subscribe(pattern, func, **options)
            return func
        return decorator

    async def set(self, address: str, value: Value) -> None:
        """Set parameter value"""
        await self._send({
            "type": "SET",
            "address": address,
            "value": value,
        })

    async def get(self, address: str, timeout: float = 5.0) -> Value:
        """Get current value"""
        # Check cache first
        if address in self._params:
            return self._params[address]

        # Request from server
        future = asyncio.get_event_loop().create_future()
        self._pending_gets[address] = future

        await self._send({"type": "GET", "address": address})

        try:
            return await asyncio.wait_for(future, timeout=timeout)
        except asyncio.TimeoutError:
            del self._pending_gets[address]
            raise ClaspError("Get timeout")

    async def emit(self, address: str, payload: Value = None) -> None:
        """Emit event"""
        await self._send({
            "type": "PUBLISH",
            "address": address,
            "signal": "event",
            "payload": payload,
            "timestamp": self.time(),
        })

    async def stream(self, address: str, value: Value) -> None:
        """Send stream sample"""
        await self._send({
            "type": "PUBLISH",
            "address": address,
            "signal": "stream",
            "value": value,
            "timestamp": self.time(),
        })

    async def bundle(
        self,
        messages: List[Dict[str, Any]],
        at: Optional[int] = None,
    ) -> None:
        """Send atomic bundle"""
        formatted = []
        for m in messages:
            if "set" in m:
                formatted.append({
                    "type": "SET",
                    "address": m["set"][0],
                    "value": m["set"][1],
                })
            elif "emit" in m:
                formatted.append({
                    "type": "PUBLISH",
                    "address": m["emit"][0],
                    "signal": "event",
                    "payload": m["emit"][1],
                })

        await self._send({
            "type": "BUNDLE",
            "timestamp": at,
            "messages": formatted,
        })

    def cached(self, address: str) -> Optional[Value]:
        """Get cached value"""
        return self._params.get(address)

    def on_connect(self, callback: Callable[[], None]) -> None:
        """Register connect callback"""
        self._on_connect.append(callback)

    def on_disconnect(self, callback: Callable[[Optional[str]], None]) -> None:
        """Register disconnect callback"""
        self._on_disconnect.append(callback)

    def on_error(self, callback: Callable[[Exception], None]) -> None:
        """Register error callback"""
        self._on_error.append(callback)

    def run(self) -> None:
        """Run event loop (blocking)"""
        asyncio.get_event_loop().run_forever()

    # Private methods

    async def _send(self, msg: Dict[str, Any]) -> None:
        """Send message"""
        if not self._ws:
            raise ClaspError("Not connected")

        data = self._encode(msg)
        await self._ws.send(data)

    def _encode(self, msg: Dict[str, Any]) -> bytes:
        """Encode message to binary frame"""
        payload = self._encode_message_v3(msg)

        # Build frame header with encoding version bit
        flags = 0x01  # Encoding = 1 (binary), 0 = MessagePack (legacy)
        header = bytes([
            0x53,  # Magic
            flags,
            (len(payload) >> 8) & 0xFF,
            len(payload) & 0xFF,
        ])

        return header + payload

    def _decode(self, data: bytes) -> Dict[str, Any]:
        """Decode frame to message - auto-detects MessagePack vs binary encoding"""
        if len(data) < 4 or data[0] != 0x53:
            raise ClaspError("Invalid frame")

        flags = data[1]
        payload_len = (data[2] << 8) | data[3]
        has_timestamp = (flags & 0x20) != 0
        version = flags & 0x07

        offset = 12 if has_timestamp else 4
        payload = data[offset:offset + payload_len]

        # Check if v2 MessagePack (first byte is fixmap 0x80-0x8F or map)
        if len(payload) > 0:
            first = payload[0]
            if (first & 0xF0) == 0x80 or first in (0xDE, 0xDF):
                # v2 MessagePack
                return msgpack.unpackb(payload, raw=False)

        # Binary encoding format
        return self._decode_message_v3(payload)

    def _encode_message_v3(self, msg: Dict[str, Any]) -> bytes:
        """Encode message to binary format"""
        msg_type = msg.get("type")
        parts = []

        if msg_type == "SET":
            parts.append(struct.pack('B', MSG_SET))
            # Flags: [has_rev:1][lock:1][unlock:1][rsv:1][vtype:4]
            vtype = self._value_type(msg["value"])
            flags = vtype & 0x0F
            if msg.get("revision") is not None:
                flags |= 0x80
            if msg.get("lock"):
                flags |= 0x40
            if msg.get("unlock"):
                flags |= 0x20
            parts.append(struct.pack('B', flags))
            parts.append(self._encode_string(msg["address"]))
            parts.append(self._encode_value_data(msg["value"]))
            if msg.get("revision") is not None:
                parts.append(struct.pack('>Q', msg["revision"]))

        elif msg_type == "PUBLISH":
            parts.append(struct.pack('B', MSG_PUBLISH))
            sig_code = self._signal_type_code(msg.get("signal", "event"))
            phase_code = self._phase_code(msg.get("phase", "start"))
            flags = (sig_code & 0x07) << 5
            if msg.get("timestamp") is not None:
                flags |= 0x10
            if msg.get("id") is not None:
                flags |= 0x08
            flags |= phase_code & 0x07
            parts.append(struct.pack('B', flags))
            parts.append(self._encode_string(msg["address"]))

            if msg.get("value") is not None:
                parts.append(struct.pack('B', 1))
                parts.append(struct.pack('B', self._value_type(msg["value"])))
                parts.append(self._encode_value_data(msg["value"]))
            elif msg.get("payload") is not None:
                parts.append(struct.pack('B', 1))
                parts.append(struct.pack('B', self._value_type(msg["payload"])))
                parts.append(self._encode_value_data(msg["payload"]))
            else:
                parts.append(struct.pack('B', 0))

            if msg.get("timestamp") is not None:
                parts.append(struct.pack('>Q', msg["timestamp"]))
            if msg.get("id") is not None:
                parts.append(struct.pack('>I', msg["id"]))

        elif msg_type == "HELLO":
            parts.append(struct.pack('B', MSG_HELLO))
            parts.append(struct.pack('B', msg.get("version", PROTOCOL_VERSION)))
            features = 0
            for f in msg.get("features", []):
                if f == "param": features |= 0x80
                if f == "event": features |= 0x40
                if f == "stream": features |= 0x20
                if f == "gesture": features |= 0x10
                if f == "timeline": features |= 0x08
            parts.append(struct.pack('B', features))
            parts.append(self._encode_string(msg.get("name", "")))
            parts.append(self._encode_string(msg.get("token") or ""))

        elif msg_type == "SUBSCRIBE":
            parts.append(struct.pack('B', MSG_SUBSCRIBE))
            parts.append(struct.pack('>I', msg["id"]))
            parts.append(self._encode_string(msg["pattern"]))
            type_mask = 0xFF
            types = msg.get("types", [])
            if types:
                type_mask = 0
                for t in types:
                    if t == "param": type_mask |= 0x01
                    if t == "event": type_mask |= 0x02
                    if t == "stream": type_mask |= 0x04
            parts.append(struct.pack('B', type_mask))
            opts = msg.get("options") or {}
            opt_flags = 0
            opt_parts = []
            if opts.get("maxRate") is not None:
                opt_flags |= 0x01
                opt_parts.append(struct.pack('>I', opts["maxRate"]))
            if opts.get("epsilon") is not None:
                opt_flags |= 0x02
                opt_parts.append(struct.pack('>d', opts["epsilon"]))
            if opts.get("history") is not None:
                opt_flags |= 0x04
                opt_parts.append(struct.pack('>I', opts["history"]))
            parts.append(struct.pack('B', opt_flags))
            parts.extend(opt_parts)

        elif msg_type == "UNSUBSCRIBE":
            parts.append(struct.pack('B', MSG_UNSUBSCRIBE))
            parts.append(struct.pack('>I', msg["id"]))

        elif msg_type == "GET":
            parts.append(struct.pack('B', MSG_GET))
            parts.append(self._encode_string(msg["address"]))

        elif msg_type == "PING":
            parts.append(struct.pack('B', MSG_PING))

        elif msg_type == "PONG":
            parts.append(struct.pack('B', MSG_PONG))

        else:
            # Fall back to MessagePack for unsupported types
            return msgpack.packb(msg)

        return b''.join(parts)

    def _decode_message_v3(self, data: bytes) -> Dict[str, Any]:
        """Decode binary encoded message"""
        if not data:
            raise ClaspError("Empty message")

        msg_type = data[0]
        offset = 1

        if msg_type == MSG_SET:
            flags = data[offset]
            offset += 1
            vtype = flags & 0x0F
            has_rev = (flags & 0x80) != 0
            lock = (flags & 0x40) != 0
            unlock = (flags & 0x20) != 0

            address, offset = self._decode_string(data, offset)
            value, offset = self._decode_value_data(data, offset, vtype)
            revision = None
            if has_rev:
                revision = struct.unpack_from('>Q', data, offset)[0]

            return {
                "type": "SET",
                "address": address,
                "value": value,
                "revision": revision,
                "lock": lock,
                "unlock": unlock,
            }

        elif msg_type == MSG_PUBLISH:
            flags = data[offset]
            offset += 1
            sig_code = (flags >> 5) & 0x07
            has_ts = (flags & 0x10) != 0
            has_id = (flags & 0x08) != 0
            phase_code = flags & 0x07

            address, offset = self._decode_string(data, offset)
            value_indicator = data[offset]
            offset += 1
            value = None
            if value_indicator == 1:
                vtype = data[offset]
                offset += 1
                value, offset = self._decode_value_data(data, offset, vtype)

            timestamp = None
            if has_ts:
                timestamp = struct.unpack_from('>Q', data, offset)[0]
                offset += 8

            gesture_id = None
            if has_id:
                gesture_id = struct.unpack_from('>I', data, offset)[0]
                offset += 4

            return {
                "type": "PUBLISH",
                "address": address,
                "signal": self._signal_type_from_code(sig_code),
                "value": value,
                "timestamp": timestamp,
                "id": gesture_id,
                "phase": self._phase_from_code(phase_code),
            }

        elif msg_type == MSG_WELCOME:
            version = data[offset]
            offset += 1
            feature_flags = data[offset]
            offset += 1
            features = []
            if feature_flags & 0x80: features.append("param")
            if feature_flags & 0x40: features.append("event")
            if feature_flags & 0x20: features.append("stream")
            if feature_flags & 0x10: features.append("gesture")
            if feature_flags & 0x08: features.append("timeline")

            time_val = struct.unpack_from('>Q', data, offset)[0]
            offset += 8
            session, offset = self._decode_string(data, offset)
            name, offset = self._decode_string(data, offset)
            token, offset = self._decode_string(data, offset)

            return {
                "type": "WELCOME",
                "version": version,
                "session": session,
                "name": name,
                "features": features,
                "time": time_val,
                "token": token if token else None,
            }

        elif msg_type == MSG_SNAPSHOT:
            count = struct.unpack_from('>H', data, offset)[0]
            offset += 2
            params = []
            for _ in range(count):
                address, offset = self._decode_string(data, offset)
                vtype = data[offset]
                offset += 1
                value, offset = self._decode_value_data(data, offset, vtype)
                revision = struct.unpack_from('>Q', data, offset)[0]
                offset += 8
                opt_flags = data[offset]
                offset += 1
                writer = None
                timestamp = None
                if opt_flags & 0x01:
                    writer, offset = self._decode_string(data, offset)
                if opt_flags & 0x02:
                    timestamp = struct.unpack_from('>Q', data, offset)[0]
                    offset += 8
                params.append({
                    "address": address,
                    "value": value,
                    "revision": revision,
                    "writer": writer,
                    "timestamp": timestamp,
                })

            return {"type": "SNAPSHOT", "params": params}

        elif msg_type == MSG_PING:
            return {"type": "PING"}

        elif msg_type == MSG_PONG:
            return {"type": "PONG"}

        elif msg_type == MSG_ERROR:
            code = struct.unpack_from('>H', data, offset)[0]
            offset += 2
            message, offset = self._decode_string(data, offset)
            flags = data[offset]
            offset += 1
            address = None
            if flags & 0x01:
                address, offset = self._decode_string(data, offset)
            return {"type": "ERROR", "code": code, "message": message, "address": address}

        else:
            raise ClaspError(f"Unknown message type: 0x{msg_type:02x}")

    def _encode_string(self, s: str) -> bytes:
        """Encode string with length prefix"""
        encoded = s.encode('utf-8')
        return struct.pack('>H', len(encoded)) + encoded

    def _decode_string(self, data: bytes, offset: int) -> Tuple[str, int]:
        """Decode length-prefixed string"""
        length = struct.unpack_from('>H', data, offset)[0]
        offset += 2
        s = data[offset:offset + length].decode('utf-8')
        return s, offset + length

    def _encode_value_data(self, value: Value) -> bytes:
        """Encode value data (without type code)"""
        if value is None:
            return b''
        if isinstance(value, bool):
            return struct.pack('B', 1 if value else 0)
        if isinstance(value, int):
            return struct.pack('>q', value)
        if isinstance(value, float):
            return struct.pack('>d', value)
        if isinstance(value, str):
            return self._encode_string(value)
        if isinstance(value, bytes):
            return struct.pack('>H', len(value)) + value
        if isinstance(value, list):
            parts = [struct.pack('>H', len(value))]
            for item in value:
                parts.append(struct.pack('B', self._value_type(item)))
                parts.append(self._encode_value_data(item))
            return b''.join(parts)
        if isinstance(value, dict):
            parts = [struct.pack('>H', len(value))]
            for k, v in value.items():
                parts.append(self._encode_string(k))
                parts.append(struct.pack('B', self._value_type(v)))
                parts.append(self._encode_value_data(v))
            return b''.join(parts)
        return b''

    def _decode_value_data(self, data: bytes, offset: int, vtype: int) -> Tuple[Value, int]:
        """Decode value data based on type code"""
        if vtype == VAL_NULL:
            return None, offset
        if vtype == VAL_BOOL:
            return data[offset] != 0, offset + 1
        if vtype == VAL_I8:
            return struct.unpack_from('b', data, offset)[0], offset + 1
        if vtype == VAL_I16:
            return struct.unpack_from('>h', data, offset)[0], offset + 2
        if vtype == VAL_I32:
            return struct.unpack_from('>i', data, offset)[0], offset + 4
        if vtype == VAL_I64:
            return struct.unpack_from('>q', data, offset)[0], offset + 8
        if vtype == VAL_F32:
            return struct.unpack_from('>f', data, offset)[0], offset + 4
        if vtype == VAL_F64:
            return struct.unpack_from('>d', data, offset)[0], offset + 8
        if vtype == VAL_STRING:
            return self._decode_string(data, offset)
        if vtype == VAL_BYTES:
            length = struct.unpack_from('>H', data, offset)[0]
            offset += 2
            return data[offset:offset + length], offset + length
        if vtype == VAL_ARRAY:
            count = struct.unpack_from('>H', data, offset)[0]
            offset += 2
            arr = []
            for _ in range(count):
                item_type = data[offset]
                offset += 1
                item, offset = self._decode_value_data(data, offset, item_type)
                arr.append(item)
            return arr, offset
        if vtype == VAL_MAP:
            count = struct.unpack_from('>H', data, offset)[0]
            offset += 2
            m = {}
            for _ in range(count):
                key, offset = self._decode_string(data, offset)
                val_type = data[offset]
                offset += 1
                val, offset = self._decode_value_data(data, offset, val_type)
                m[key] = val
            return m, offset
        return None, offset

    def _value_type(self, value: Value) -> int:
        """Get value type code"""
        if value is None:
            return VAL_NULL
        if isinstance(value, bool):
            return VAL_BOOL
        if isinstance(value, int):
            return VAL_I64
        if isinstance(value, float):
            return VAL_F64
        if isinstance(value, str):
            return VAL_STRING
        if isinstance(value, bytes):
            return VAL_BYTES
        if isinstance(value, list):
            return VAL_ARRAY
        if isinstance(value, dict):
            return VAL_MAP
        return VAL_NULL

    def _signal_type_code(self, sig: str) -> int:
        """Get signal type code"""
        return {
            "param": SIG_PARAM,
            "event": SIG_EVENT,
            "stream": SIG_STREAM,
            "gesture": SIG_GESTURE,
            "timeline": SIG_TIMELINE,
        }.get(sig, SIG_EVENT)

    def _signal_type_from_code(self, code: int) -> str:
        """Get signal type from code"""
        return {
            SIG_PARAM: "param",
            SIG_EVENT: "event",
            SIG_STREAM: "stream",
            SIG_GESTURE: "gesture",
            SIG_TIMELINE: "timeline",
        }.get(code, "event")

    def _phase_code(self, phase: str) -> int:
        """Get gesture phase code"""
        return {"start": 0, "move": 1, "end": 2, "cancel": 3}.get(phase, 0)

    def _phase_from_code(self, code: int) -> str:
        """Get gesture phase from code"""
        return {0: "start", 1: "move", 2: "end", 3: "cancel"}.get(code, "start")

    async def _receive_loop(self) -> None:
        """Receive message loop"""
        try:
            while self._connected and self._ws:
                data = await self._ws.recv()
                msg = self._decode(data)
                self._handle_message(msg)

        except websockets.ConnectionClosed as e:
            self._connected = False
            for cb in self._on_disconnect:
                cb(str(e))

            # Reconnect if enabled
            if self.reconnect:
                await asyncio.sleep(self.reconnect_interval)
                try:
                    await self.connect()
                except Exception:
                    pass

        except Exception as e:
            for cb in self._on_error:
                cb(e)

    def _handle_message(self, msg: Dict[str, Any]) -> None:
        """Handle incoming message"""
        msg_type = msg.get("type")

        if msg_type == "SET":
            address = msg["address"]
            value = msg["value"]
            self._params[address] = value
            self._notify_subscribers(address, value)

        elif msg_type == "SNAPSHOT":
            for param in msg.get("params", []):
                address = param["address"]
                value = param["value"]
                self._params[address] = value

                # Resolve pending gets
                if address in self._pending_gets:
                    self._pending_gets[address].set_result(value)
                    del self._pending_gets[address]

                self._notify_subscribers(address, value)

        elif msg_type == "PUBLISH":
            address = msg["address"]
            value = msg.get("value") or msg.get("payload")
            self._notify_subscribers(address, value)

        elif msg_type == "PING":
            asyncio.create_task(self._send({"type": "PONG"}))

        elif msg_type == "ERROR":
            print(f"CLASP error: {msg.get('code')} - {msg.get('message')}")

    def _notify_subscribers(self, address: str, value: Value) -> None:
        """Notify matching subscribers"""
        for pattern, callback in self._subscriptions.values():
            if self._match_pattern(pattern, address):
                try:
                    callback(value, address)
                except Exception as e:
                    for cb in self._on_error:
                        cb(e)

    def _match_pattern(self, pattern: str, address: str) -> bool:
        """
        Match address against CLASP pattern.

        Patterns:
          /exact/path  - Exact match
          /path/*      - Matches single segment (no /)
          /path/**     - Matches zero or more segments

        Examples:
          /test/* matches /test/foo but not /test/foo/bar
          /test/** matches /test, /test/foo, /test/foo/bar
        """
        # Exact match
        if pattern == address:
            return True

        # Split into segments
        pattern_parts = pattern.split('/')
        address_parts = address.split('/')

        return self._match_segments(pattern_parts, address_parts)

    def _match_segments(self, pattern_parts: List[str], address_parts: List[str]) -> bool:
        """Recursively match pattern segments against address segments."""
        pi = 0  # pattern index
        ai = 0  # address index

        while pi < len(pattern_parts):
            p = pattern_parts[pi]

            if p == '**':
                # ** can match zero or more segments
                # Try matching 0, 1, 2, ... segments
                remaining_pattern = pattern_parts[pi + 1:]

                if not remaining_pattern:
                    # ** at end matches everything remaining
                    return True

                # Try matching from current position onwards
                for skip in range(len(address_parts) - ai + 1):
                    if self._match_segments(remaining_pattern, address_parts[ai + skip:]):
                        return True
                return False

            elif p == '*':
                # * matches exactly one non-empty segment
                if ai >= len(address_parts):
                    return False
                if not address_parts[ai]:  # Empty segment
                    return False
                pi += 1
                ai += 1

            else:
                # Exact segment match
                if ai >= len(address_parts):
                    return False
                if p != address_parts[ai]:
                    return False
                pi += 1
                ai += 1

        # Pattern consumed, check if address is also consumed
        return ai == len(address_parts)
