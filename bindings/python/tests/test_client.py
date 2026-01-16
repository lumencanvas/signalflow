"""Tests for CLASP client."""

import pytest
from clasp import Clasp, ClaspBuilder
from clasp.client import ClaspError


class TestClaspBuilder:
    """Test ClaspBuilder class."""

    def test_builder_creation(self):
        builder = ClaspBuilder(url="ws://localhost:7330")
        assert builder.url == "ws://localhost:7330"

    def test_builder_with_name(self):
        builder = ClaspBuilder(url="ws://localhost:7330")
        result = builder.with_name("Test Client")
        assert result is builder  # Returns self for chaining
        assert builder.name == "Test Client"

    def test_builder_with_features(self):
        builder = ClaspBuilder(url="ws://localhost:7330")
        result = builder.with_features(["param", "event"])
        assert result is builder
        assert builder.features == ["param", "event"]

    def test_builder_with_token(self):
        builder = ClaspBuilder(url="ws://localhost:7330")
        result = builder.with_token("secret-token")
        assert result is builder
        assert builder.token == "secret-token"

    def test_builder_with_reconnect(self):
        builder = ClaspBuilder(url="ws://localhost:7330")
        result = builder.with_reconnect(True, 10.0)
        assert result is builder
        assert builder.reconnect is True
        assert builder.reconnect_interval == 10.0

    def test_builder_chaining(self):
        builder = (
            ClaspBuilder(url="ws://localhost:7330")
            .with_name("Chained Client")
            .with_features(["param"])
            .with_token("token123")
            .with_reconnect(False)
        )
        assert builder.name == "Chained Client"
        assert builder.features == ["param"]
        assert builder.token == "token123"
        assert builder.reconnect is False


class TestClasp:
    """Test Clasp client class."""

    def test_client_creation(self):
        client = Clasp("ws://localhost:7330")
        assert client.url == "ws://localhost:7330"
        assert client.connected is False

    def test_client_with_options(self):
        client = Clasp(
            url="ws://localhost:7330",
            name="Test Client",
            features=["param", "event"],
            token="secret",
            reconnect=False,
        )
        assert client.name == "Test Client"
        assert client.features == ["param", "event"]
        assert client.token == "secret"
        assert client.reconnect is False

    def test_client_builder_method(self):
        builder = Clasp.builder("ws://localhost:7330")
        assert isinstance(builder, ClaspBuilder)
        assert builder.url == "ws://localhost:7330"

    def test_client_not_connected_initially(self):
        client = Clasp("ws://localhost:7330")
        assert client.connected is False
        assert client.session_id is None

    def test_client_cached_returns_none(self):
        client = Clasp("ws://localhost:7330")
        assert client.cached("/nonexistent/path") is None

    def test_client_time(self):
        client = Clasp("ws://localhost:7330")
        t = client.time()
        assert isinstance(t, int)
        assert t > 0


class TestPatternMatching:
    """Test address pattern matching."""

    def test_exact_match(self):
        client = Clasp("ws://localhost:7330")
        assert client._match_pattern("/test/path", "/test/path") is True
        assert client._match_pattern("/test/path", "/other/path") is False

    def test_single_wildcard(self):
        client = Clasp("ws://localhost:7330")
        pattern = "/test/*/value"
        assert client._match_pattern(pattern, "/test/foo/value") is True
        assert client._match_pattern(pattern, "/test/bar/value") is True
        assert client._match_pattern(pattern, "/test/value") is False
        assert client._match_pattern(pattern, "/test/foo/bar/value") is False

    def test_multi_wildcard(self):
        client = Clasp("ws://localhost:7330")
        pattern = "/test/**/value"
        assert client._match_pattern(pattern, "/test/value") is True
        assert client._match_pattern(pattern, "/test/foo/value") is True
        assert client._match_pattern(pattern, "/test/foo/bar/value") is True

    def test_trailing_wildcard(self):
        client = Clasp("ws://localhost:7330")
        pattern = "/test/**"
        assert client._match_pattern(pattern, "/test") is True
        assert client._match_pattern(pattern, "/test/foo") is True
        assert client._match_pattern(pattern, "/test/foo/bar") is True
        assert client._match_pattern(pattern, "/other/test") is False


class TestClaspError:
    """Test ClaspError exception."""

    def test_error_creation(self):
        error = ClaspError("Test error message")
        assert str(error) == "Test error message"

    def test_error_inheritance(self):
        error = ClaspError("Error")
        assert isinstance(error, Exception)
