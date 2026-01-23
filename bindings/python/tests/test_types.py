"""Tests for CLASP types."""

import pytest
from clasp.types import (
    QoS,
    SignalType,
    MessageType,
    Message,
    HelloMessage,
    WelcomeMessage,
    SetMessage,
    PublishMessage,
    SubscribeMessage,
    ParamValue,
    PROTOCOL_VERSION,
    DEFAULT_WS_PORT,
    DEFAULT_DISCOVERY_PORT,
)


class TestConstants:
    """Test protocol constants."""

    def test_protocol_version(self):
        assert PROTOCOL_VERSION == 1

    def test_default_ws_port(self):
        assert DEFAULT_WS_PORT == 7330

    def test_default_discovery_port(self):
        assert DEFAULT_DISCOVERY_PORT == 7331


class TestQoS:
    """Test QoS enum."""

    def test_qos_values(self):
        assert QoS.FIRE == 0
        assert QoS.CONFIRM == 1
        assert QoS.COMMIT == 2

    def test_qos_is_int(self):
        assert isinstance(QoS.FIRE, int)
        assert isinstance(QoS.CONFIRM, int)
        assert isinstance(QoS.COMMIT, int)


class TestSignalType:
    """Test SignalType enum."""

    def test_signal_type_values(self):
        assert SignalType.PARAM == "param"
        assert SignalType.EVENT == "event"
        assert SignalType.STREAM == "stream"
        assert SignalType.GESTURE == "gesture"
        assert SignalType.TIMELINE == "timeline"


class TestMessageType:
    """Test MessageType enum."""

    def test_message_type_values(self):
        assert MessageType.HELLO == 0x01
        assert MessageType.WELCOME == 0x02
        assert MessageType.SUBSCRIBE == 0x10
        assert MessageType.PUBLISH == 0x20
        assert MessageType.SET == 0x21


class TestMessages:
    """Test message dataclasses."""

    def test_hello_message(self):
        msg = HelloMessage(
            type="HELLO",
            version=2,
            name="Test Client",
            features=["param", "event"],
        )
        assert msg.type == "HELLO"
        assert msg.version == 2
        assert msg.name == "Test Client"
        assert "param" in msg.features
        assert msg.token is None

    def test_hello_message_with_token(self):
        msg = HelloMessage(
            type="HELLO",
            version=2,
            name="Test Client",
            features=["param"],
            token="secret-token",
        )
        assert msg.token == "secret-token"

    def test_welcome_message(self):
        msg = WelcomeMessage(
            type="WELCOME",
            version=2,
            session="sess-123",
            name="Test Server",
            features=["param", "event"],
            time=1234567890,
        )
        assert msg.type == "WELCOME"
        assert msg.session == "sess-123"
        assert msg.time == 1234567890

    def test_set_message(self):
        msg = SetMessage(
            type="SET",
            address="/test/path",
            value=42,
        )
        assert msg.type == "SET"
        assert msg.address == "/test/path"
        assert msg.value == 42
        assert msg.revision is None
        assert msg.lock is False

    def test_set_message_with_lock(self):
        msg = SetMessage(
            type="SET",
            address="/test/path",
            value="locked",
            lock=True,
        )
        assert msg.lock is True

    def test_publish_message(self):
        msg = PublishMessage(
            type="PUBLISH",
            address="/test/event",
            signal=SignalType.EVENT,
            payload={"key": "value"},
        )
        assert msg.type == "PUBLISH"
        assert msg.address == "/test/event"
        assert msg.signal == SignalType.EVENT
        assert msg.payload == {"key": "value"}

    def test_subscribe_message(self):
        msg = SubscribeMessage(
            type="SUBSCRIBE",
            id=42,
            pattern="/test/*",
        )
        assert msg.type == "SUBSCRIBE"
        assert msg.id == 42
        assert msg.pattern == "/test/*"
        assert msg.types is None

    def test_subscribe_message_with_types(self):
        msg = SubscribeMessage(
            type="SUBSCRIBE",
            id=1,
            pattern="/test/**",
            types=[SignalType.PARAM, SignalType.EVENT],
        )
        assert SignalType.PARAM in msg.types
        assert SignalType.EVENT in msg.types

    def test_param_value(self):
        pv = ParamValue(
            address="/test/param",
            value=3.14,
            revision=5,
        )
        assert pv.address == "/test/param"
        assert pv.value == 3.14
        assert pv.revision == 5
        assert pv.writer is None

    def test_param_value_with_writer(self):
        pv = ParamValue(
            address="/test/param",
            value=True,
            revision=1,
            writer="client-123",
            timestamp=9876543210,
        )
        assert pv.writer == "client-123"
        assert pv.timestamp == 9876543210
