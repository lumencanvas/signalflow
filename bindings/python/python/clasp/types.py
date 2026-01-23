"""
CLASP type definitions
"""

from typing import Any, Dict, List, Optional, Union, Callable
from enum import Enum, IntEnum
from dataclasses import dataclass

# Protocol constants
PROTOCOL_VERSION = 1
DEFAULT_WS_PORT = 7330
DEFAULT_DISCOVERY_PORT = 7331
WS_SUBPROTOCOL = "clasp"

# Type aliases
Value = Union[None, bool, int, float, str, bytes, List[Any], Dict[str, Any]]
SubscriptionCallback = Callable[[Value, str], None]


class QoS(IntEnum):
    """Quality of Service levels"""
    FIRE = 0      # Best effort
    CONFIRM = 1   # At least once
    COMMIT = 2    # Exactly once, ordered


class SignalType(str, Enum):
    """Signal types"""
    PARAM = "param"
    EVENT = "event"
    STREAM = "stream"
    GESTURE = "gesture"
    TIMELINE = "timeline"


class MessageType(IntEnum):
    """Message type codes"""
    HELLO = 0x01
    WELCOME = 0x02
    ANNOUNCE = 0x03
    SUBSCRIBE = 0x10
    UNSUBSCRIBE = 0x11
    PUBLISH = 0x20
    SET = 0x21
    GET = 0x22
    SNAPSHOT = 0x23
    BUNDLE = 0x30
    SYNC = 0x40
    PING = 0x41
    PONG = 0x42
    ACK = 0x50
    ERROR = 0x51
    QUERY = 0x60
    RESULT = 0x61


@dataclass
class Message:
    """Base message class"""
    type: str


@dataclass
class HelloMessage(Message):
    """HELLO message"""
    version: int
    name: str
    features: List[str]
    capabilities: Optional[Dict[str, Any]] = None
    token: Optional[str] = None


@dataclass
class WelcomeMessage(Message):
    """WELCOME message"""
    version: int
    session: str
    name: str
    features: List[str]
    time: int
    token: Optional[str] = None


@dataclass
class SetMessage(Message):
    """SET message"""
    address: str
    value: Value
    revision: Optional[int] = None
    lock: bool = False
    unlock: bool = False


@dataclass
class PublishMessage(Message):
    """PUBLISH message"""
    address: str
    signal: Optional[SignalType] = None
    value: Optional[Value] = None
    payload: Optional[Value] = None
    timestamp: Optional[int] = None


@dataclass
class SubscribeMessage(Message):
    """SUBSCRIBE message"""
    id: int
    pattern: str
    types: Optional[List[SignalType]] = None
    options: Optional[Dict[str, Any]] = None


@dataclass
class ParamValue:
    """Parameter value"""
    address: str
    value: Value
    revision: int
    writer: Optional[str] = None
    timestamp: Optional[int] = None


@dataclass
class SnapshotMessage(Message):
    """SNAPSHOT message"""
    params: List[ParamValue]


@dataclass
class BundleMessage(Message):
    """BUNDLE message"""
    messages: List[Message]
    timestamp: Optional[int] = None


@dataclass
class ErrorMessage(Message):
    """ERROR message"""
    code: int
    message: str
    address: Optional[str] = None
    correlation_id: Optional[int] = None
