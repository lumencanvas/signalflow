"""
CLASP Python client
"""

import asyncio
import time
import fnmatch
from typing import Any, Callable, Dict, List, Optional, Union
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
        """Encode message to frame"""
        payload = msgpack.packb(msg)

        # Build frame header
        header = bytes([
            0x53,  # Magic
            0x00,  # Flags (QoS=0, no timestamp)
            (len(payload) >> 8) & 0xFF,
            len(payload) & 0xFF,
        ])

        return header + payload

    def _decode(self, data: bytes) -> Dict[str, Any]:
        """Decode frame to message"""
        if len(data) < 4 or data[0] != 0x53:
            raise ClaspError("Invalid frame")

        flags = data[1]
        payload_len = (data[2] << 8) | data[3]
        has_timestamp = (flags & 0x20) != 0

        offset = 12 if has_timestamp else 4
        payload = data[offset:offset + payload_len]

        return msgpack.unpackb(payload, raw=False)

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
        """Match address against pattern"""
        # Convert CLASP pattern to fnmatch pattern
        fn_pattern = pattern.replace("**", "§§").replace("*", "[^/]*").replace("§§", "*")
        return fnmatch.fnmatch(address, fn_pattern)
