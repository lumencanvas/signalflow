//! Broker Integration Tests
//!
//! Tests CLASP bridges with real broker infrastructure.
//! Requires Docker services running (see test-suite/docker/docker-compose.yml)
//!
//! Run with:
//!   cd test-suite/docker && docker-compose up -d
//!   cargo run -p clasp-test-suite --bin broker-tests
//!
//! Environment variables:
//! - CLASP_MQTT_HOST=localhost:1883     MQTT broker address
//! - CLASP_REDIS_HOST=localhost:6379    Redis address (future)
//! - CLASP_TEST_BROKERS=1               Enable broker tests

use bytes::Bytes;
use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

// ============================================================================
// Test Framework
// ============================================================================

struct TestResult {
    name: &'static str,
    passed: bool,
    message: String,
    duration_ms: u128,
    skipped: bool,
}

impl TestResult {
    fn pass(name: &'static str, message: impl Into<String>, duration_ms: u128) -> Self {
        Self {
            name,
            passed: true,
            message: message.into(),
            duration_ms,
            skipped: false,
        }
    }

    fn fail(name: &'static str, message: impl Into<String>, duration_ms: u128) -> Self {
        Self {
            name,
            passed: false,
            message: message.into(),
            duration_ms,
            skipped: false,
        }
    }

    fn skip(name: &'static str, reason: impl Into<String>) -> Self {
        Self {
            name,
            passed: true,
            message: reason.into(),
            duration_ms: 0,
            skipped: true,
        }
    }
}

fn is_enabled(var: &str) -> bool {
    env::var(var)
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

fn get_mqtt_host() -> String {
    env::var("CLASP_MQTT_HOST").unwrap_or_else(|_| "localhost:1883".to_string())
}

fn check_mqtt_available() -> bool {
    let host = get_mqtt_host();
    TcpStream::connect_timeout(
        &host
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:1883".parse().unwrap()),
        Duration::from_secs(2),
    )
    .is_ok()
}

// ============================================================================
// MQTT Protocol Helpers
// ============================================================================

fn mqtt_connect(client_id: &str) -> Result<TcpStream, String> {
    let host = get_mqtt_host();
    let mut stream = TcpStream::connect(&host).map_err(|e| format!("Connect failed: {}", e))?;

    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();

    // Build CONNECT packet
    let mut packet = Vec::new();

    // Variable header
    let protocol_name = b"\x00\x04MQTT";
    let protocol_level = 4u8; // MQTT 3.1.1
    let connect_flags = 0x02u8; // Clean session
    let keep_alive = 60u16;

    let client_id_bytes = client_id.as_bytes();
    let remaining_length = 10 + 2 + client_id_bytes.len();

    // Fixed header
    packet.push(0x10); // CONNECT
    packet.push(remaining_length as u8);

    // Variable header
    packet.extend_from_slice(protocol_name);
    packet.push(protocol_level);
    packet.push(connect_flags);
    packet.push((keep_alive >> 8) as u8);
    packet.push((keep_alive & 0xFF) as u8);

    // Payload
    packet.push((client_id_bytes.len() >> 8) as u8);
    packet.push((client_id_bytes.len() & 0xFF) as u8);
    packet.extend_from_slice(client_id_bytes);

    stream
        .write_all(&packet)
        .map_err(|e| format!("Write failed: {}", e))?;

    // Read CONNACK
    let mut buf = [0u8; 4];
    stream
        .read_exact(&mut buf)
        .map_err(|e| format!("Read failed: {}", e))?;

    if buf[0] == 0x20 && buf[3] == 0x00 {
        Ok(stream)
    } else {
        Err(format!("CONNACK failed: {:02X} {:02X}", buf[0], buf[3]))
    }
}

fn mqtt_publish(stream: &mut TcpStream, topic: &str, payload: &[u8]) -> Result<(), String> {
    let topic_bytes = topic.as_bytes();
    let remaining_length = 2 + topic_bytes.len() + payload.len();

    let mut packet = Vec::new();

    // Fixed header
    packet.push(0x30); // PUBLISH, QoS 0
    packet.push(remaining_length as u8);

    // Variable header - topic
    packet.push((topic_bytes.len() >> 8) as u8);
    packet.push((topic_bytes.len() & 0xFF) as u8);
    packet.extend_from_slice(topic_bytes);

    // Payload
    packet.extend_from_slice(payload);

    stream
        .write_all(&packet)
        .map_err(|e| format!("Publish failed: {}", e))
}

fn mqtt_subscribe(stream: &mut TcpStream, topic: &str, packet_id: u16) -> Result<(), String> {
    let topic_bytes = topic.as_bytes();
    let remaining_length = 2 + 2 + topic_bytes.len() + 1;

    let mut packet = Vec::new();

    // Fixed header
    packet.push(0x82); // SUBSCRIBE
    packet.push(remaining_length as u8);

    // Variable header - packet ID
    packet.push((packet_id >> 8) as u8);
    packet.push((packet_id & 0xFF) as u8);

    // Payload - topic filter
    packet.push((topic_bytes.len() >> 8) as u8);
    packet.push((topic_bytes.len() & 0xFF) as u8);
    packet.extend_from_slice(topic_bytes);
    packet.push(0x00); // QoS 0

    stream
        .write_all(&packet)
        .map_err(|e| format!("Subscribe failed: {}", e))?;

    // Read SUBACK
    let mut buf = [0u8; 5];
    stream
        .read_exact(&mut buf)
        .map_err(|e| format!("SUBACK read failed: {}", e))?;

    if buf[0] == 0x90 {
        Ok(())
    } else {
        Err(format!("SUBACK failed: {:02X}", buf[0]))
    }
}

fn mqtt_disconnect(stream: &mut TcpStream) {
    let packet = [0xE0, 0x00]; // DISCONNECT
    let _ = stream.write_all(&packet);
}

// ============================================================================
// MQTT Tests
// ============================================================================

fn test_mqtt_connection() -> TestResult {
    let start = Instant::now();
    let name = "mqtt_connection";

    if !is_enabled("CLASP_TEST_BROKERS") && !check_mqtt_available() {
        return TestResult::skip(name, "MQTT not available (set CLASP_TEST_BROKERS=1)");
    }

    match mqtt_connect("clasp-test-conn") {
        Ok(mut stream) => {
            mqtt_disconnect(&mut stream);
            TestResult::pass(
                name,
                format!("Connected to {}", get_mqtt_host()),
                start.elapsed().as_millis(),
            )
        }
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

fn test_mqtt_publish() -> TestResult {
    let start = Instant::now();
    let name = "mqtt_publish";

    if !is_enabled("CLASP_TEST_BROKERS") && !check_mqtt_available() {
        return TestResult::skip(name, "MQTT not available");
    }

    match mqtt_connect("clasp-test-pub") {
        Ok(mut stream) => {
            let result = mqtt_publish(&mut stream, "clasp/test/value", b"42");
            mqtt_disconnect(&mut stream);

            match result {
                Ok(()) => TestResult::pass(
                    name,
                    "Published to clasp/test/value",
                    start.elapsed().as_millis(),
                ),
                Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
            }
        }
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

fn test_mqtt_subscribe() -> TestResult {
    let start = Instant::now();
    let name = "mqtt_subscribe";

    if !is_enabled("CLASP_TEST_BROKERS") && !check_mqtt_available() {
        return TestResult::skip(name, "MQTT not available");
    }

    match mqtt_connect("clasp-test-sub") {
        Ok(mut stream) => {
            let result = mqtt_subscribe(&mut stream, "clasp/test/#", 1);
            mqtt_disconnect(&mut stream);

            match result {
                Ok(()) => TestResult::pass(
                    name,
                    "Subscribed to clasp/test/#",
                    start.elapsed().as_millis(),
                ),
                Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
            }
        }
        Err(e) => TestResult::fail(name, e, start.elapsed().as_millis()),
    }
}

fn test_mqtt_pubsub_roundtrip() -> TestResult {
    let start = Instant::now();
    let name = "mqtt_pubsub_roundtrip";

    if !is_enabled("CLASP_TEST_BROKERS") && !check_mqtt_available() {
        return TestResult::skip(name, "MQTT not available");
    }

    // Subscriber
    let mut sub_stream = match mqtt_connect("clasp-test-roundtrip-sub") {
        Ok(s) => s,
        Err(e) => return TestResult::fail(name, e, start.elapsed().as_millis()),
    };

    if let Err(e) = mqtt_subscribe(&mut sub_stream, "clasp/roundtrip", 1) {
        return TestResult::fail(name, e, start.elapsed().as_millis());
    }

    // Publisher
    let mut pub_stream = match mqtt_connect("clasp-test-roundtrip-pub") {
        Ok(s) => s,
        Err(e) => return TestResult::fail(name, e, start.elapsed().as_millis()),
    };

    // Publish
    let test_payload = b"roundtrip-test-123";
    if let Err(e) = mqtt_publish(&mut pub_stream, "clasp/roundtrip", test_payload) {
        return TestResult::fail(name, e, start.elapsed().as_millis());
    }

    mqtt_disconnect(&mut pub_stream);

    // Try to receive
    sub_stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .ok();
    let mut buf = [0u8; 256];

    match sub_stream.read(&mut buf) {
        Ok(len) if len > 0 => {
            mqtt_disconnect(&mut sub_stream);
            // Check if it's a PUBLISH packet
            if buf[0] & 0xF0 == 0x30 {
                TestResult::pass(
                    name,
                    format!("Received {} bytes", len),
                    start.elapsed().as_millis(),
                )
            } else {
                TestResult::fail(
                    name,
                    format!("Unexpected packet type: {:02X}", buf[0]),
                    start.elapsed().as_millis(),
                )
            }
        }
        Ok(_) => {
            mqtt_disconnect(&mut sub_stream);
            TestResult::fail(name, "No data received", start.elapsed().as_millis())
        }
        Err(e) => {
            mqtt_disconnect(&mut sub_stream);
            TestResult::fail(
                name,
                format!("Read error: {}", e),
                start.elapsed().as_millis(),
            )
        }
    }
}

fn test_mqtt_multiple_topics() -> TestResult {
    let start = Instant::now();
    let name = "mqtt_multiple_topics";

    if !is_enabled("CLASP_TEST_BROKERS") && !check_mqtt_available() {
        return TestResult::skip(name, "MQTT not available");
    }

    let mut stream = match mqtt_connect("clasp-test-multi") {
        Ok(s) => s,
        Err(e) => return TestResult::fail(name, e, start.elapsed().as_millis()),
    };

    let topics = [
        "clasp/param/volume",
        "clasp/param/pan",
        "clasp/event/trigger",
        "clasp/state/active",
    ];

    let mut success = 0;
    for topic in &topics {
        if mqtt_publish(&mut stream, topic, b"test").is_ok() {
            success += 1;
        }
    }

    mqtt_disconnect(&mut stream);

    if success == topics.len() {
        TestResult::pass(
            name,
            format!("Published to {} topics", success),
            start.elapsed().as_millis(),
        )
    } else {
        TestResult::fail(
            name,
            format!("Only {}/{} topics", success, topics.len()),
            start.elapsed().as_millis(),
        )
    }
}

fn test_mqtt_rapid_publish() -> TestResult {
    let start = Instant::now();
    let name = "mqtt_rapid_publish";

    if !is_enabled("CLASP_TEST_BROKERS") && !check_mqtt_available() {
        return TestResult::skip(name, "MQTT not available");
    }

    let mut stream = match mqtt_connect("clasp-test-rapid") {
        Ok(s) => s,
        Err(e) => return TestResult::fail(name, e, start.elapsed().as_millis()),
    };

    let message_count = 1000;
    let mut success = 0;

    for i in 0..message_count {
        let payload = format!("{}", i);
        if mqtt_publish(&mut stream, "clasp/rapid/value", payload.as_bytes()).is_ok() {
            success += 1;
        }
    }

    mqtt_disconnect(&mut stream);

    if success == message_count {
        let elapsed = start.elapsed().as_millis();
        let rate = (message_count as f64 / elapsed as f64) * 1000.0;
        TestResult::pass(name, format!("{} msg/s", rate as u32), elapsed as u128)
    } else {
        TestResult::fail(
            name,
            format!("Only {}/{}", success, message_count),
            start.elapsed().as_millis(),
        )
    }
}

// ============================================================================
// HTTP Bridge Tests
// ============================================================================

fn test_http_bridge_get() -> TestResult {
    let start = Instant::now();
    let name = "http_bridge_get";

    // This would test the HTTP bridge endpoint
    // For now, we'll skip as it requires the bridge to be running
    TestResult::skip(name, "Requires HTTP bridge running")
}

fn test_http_bridge_post() -> TestResult {
    let start = Instant::now();
    let name = "http_bridge_post";

    TestResult::skip(name, "Requires HTTP bridge running")
}

// ============================================================================
// WebSocket Bridge Tests
// ============================================================================

fn test_ws_bridge_connect() -> TestResult {
    let start = Instant::now();
    let name = "ws_bridge_connect";

    TestResult::skip(name, "Requires WebSocket bridge running")
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP Broker Integration Tests                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let mqtt_status = if check_mqtt_available() {
        "connected"
    } else {
        "not available"
    };
    println!("MQTT Broker ({}): {}", get_mqtt_host(), mqtt_status);
    println!();

    if !check_mqtt_available() {
        println!("To enable broker tests:");
        println!("  cd test-suite/docker && docker-compose up -d");
        println!("  cargo run -p clasp-test-suite --bin broker-tests");
        println!();
    }

    let tests = vec![
        // MQTT tests
        test_mqtt_connection(),
        test_mqtt_publish(),
        test_mqtt_subscribe(),
        test_mqtt_pubsub_roundtrip(),
        test_mqtt_multiple_topics(),
        test_mqtt_rapid_publish(),
        // HTTP bridge tests
        test_http_bridge_get(),
        test_http_bridge_post(),
        // WebSocket bridge tests
        test_ws_bridge_connect(),
    ];

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    println!("┌──────────────────────────────────────┬────────┬──────────┐");
    println!("│ Test                                 │ Status │ Time     │");
    println!("├──────────────────────────────────────┼────────┼──────────┤");

    for test in &tests {
        let (status, color) = if test.skipped {
            ("⊘ SKIP", "\x1b[33m")
        } else if test.passed {
            ("✓ PASS", "\x1b[32m")
        } else {
            ("✗ FAIL", "\x1b[31m")
        };

        println!(
            "│ {:<36} │ {}{:<6}\x1b[0m │ {:>6}ms │",
            test.name, color, status, test.duration_ms
        );

        if test.skipped {
            skipped += 1;
        } else if test.passed {
            passed += 1;
            if !test.message.is_empty() {
                println!(
                    "│   └─ {:<56} │",
                    &test.message[..test.message.len().min(56)]
                );
            }
        } else {
            failed += 1;
            println!(
                "│   └─ {:<56} │",
                &test.message[..test.message.len().min(56)]
            );
        }
    }

    println!("└──────────────────────────────────────┴────────┴──────────┘");
    println!(
        "\nResults: {} passed, {} failed, {} skipped",
        passed, failed, skipped
    );

    if failed > 0 {
        std::process::exit(1);
    }
}
