//! Embedded/Lite Protocol Tests (clasp-embedded)
//!
//! Tests for the minimal embedded protocol including:
//! - Lite message encoding
//! - Lite message decoding
//! - Fixed-size frame handling

use clasp_embedded::{decode_lite_header, encode_lite_set, LiteMessageType};

// ============================================================================
// Test Framework
// ============================================================================

struct TestResult {
    name: &'static str,
    passed: bool,
    message: String,
    duration_ms: u128,
}

impl TestResult {
    fn pass(name: &'static str, duration_ms: u128) -> Self {
        Self {
            name,
            passed: true,
            message: "OK".to_string(),
            duration_ms,
        }
    }

    fn fail(name: &'static str, message: impl Into<String>, duration_ms: u128) -> Self {
        Self {
            name,
            passed: false,
            message: message.into(),
            duration_ms,
        }
    }
}

// ============================================================================
// Encoding Tests
// ============================================================================

fn test_encode_lite_set_basic() -> TestResult {
    let start = std::time::Instant::now();
    let name = "encode_lite_set_basic";

    let mut buf = [0u8; 16];
    let len = encode_lite_set(0x0001, 42, &mut buf);

    if len == 8
        && buf[0] == 0x53  // Magic
        && buf[1] == LiteMessageType::Set as u8
        && buf[2] == 0x00  // Address high
        && buf[3] == 0x01  // Address low
        && buf[4] == 0x00  // Value (big-endian)
        && buf[5] == 0x00
        && buf[6] == 0x00
        && buf[7] == 42
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Wrong encoding: {:?}", &buf[..len]),
            start.elapsed().as_millis(),
        )
    }
}

fn test_encode_lite_set_negative() -> TestResult {
    let start = std::time::Instant::now();
    let name = "encode_lite_set_negative";

    let mut buf = [0u8; 16];
    let len = encode_lite_set(0x0100, -1, &mut buf);

    // -1 in two's complement is 0xFFFFFFFF
    if len == 8 && buf[4] == 0xFF && buf[5] == 0xFF && buf[6] == 0xFF && buf[7] == 0xFF {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Wrong encoding: {:?}", &buf[..len]),
            start.elapsed().as_millis(),
        )
    }
}

fn test_encode_lite_set_large_value() -> TestResult {
    let start = std::time::Instant::now();
    let name = "encode_lite_set_large_value";

    let mut buf = [0u8; 16];
    let len = encode_lite_set(0xFFFF, 0x12345678, &mut buf);

    if len == 8
        && buf[2] == 0xFF  // Address high
        && buf[3] == 0xFF  // Address low
        && buf[4] == 0x12  // Value (big-endian)
        && buf[5] == 0x34
        && buf[6] == 0x56
        && buf[7] == 0x78
    {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Wrong encoding: {:?}", &buf[..len]),
            start.elapsed().as_millis(),
        )
    }
}

fn test_encode_lite_set_buffer_too_small() -> TestResult {
    let start = std::time::Instant::now();
    let name = "encode_lite_set_buffer_too_small";

    let mut buf = [0u8; 4]; // Too small for 8-byte message
    let len = encode_lite_set(0x0001, 42, &mut buf);

    if len == 0 {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            format!("Should return 0 for small buffer, got {}", len),
            start.elapsed().as_millis(),
        )
    }
}

// ============================================================================
// Decoding Tests
// ============================================================================

fn test_decode_lite_header_set() -> TestResult {
    let start = std::time::Instant::now();
    let name = "decode_lite_header_set";

    let buf = [0x53, LiteMessageType::Set as u8, 0x00, 0x42];
    let result = decode_lite_header(&buf);

    match result {
        Some((msg_type, address)) => {
            let is_set = matches!(msg_type, LiteMessageType::Set);
            if is_set && address == 0x0042 {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(
                    name,
                    format!("Wrong decode: addr={}", address),
                    start.elapsed().as_millis(),
                )
            }
        }
        None => TestResult::fail(name, "Decode returned None", start.elapsed().as_millis()),
    }
}

fn test_decode_lite_header_hello() -> TestResult {
    let start = std::time::Instant::now();
    let name = "decode_lite_header_hello";

    let buf = [0x53, LiteMessageType::Hello as u8, 0x12, 0x34];
    let result = decode_lite_header(&buf);

    match result {
        Some((msg_type, address)) => {
            let is_hello = matches!(msg_type, LiteMessageType::Hello);
            if is_hello && address == 0x1234 {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(
                    name,
                    format!("Wrong decode: addr={}", address),
                    start.elapsed().as_millis(),
                )
            }
        }
        None => TestResult::fail(name, "Decode returned None", start.elapsed().as_millis()),
    }
}

fn test_decode_lite_header_ping() -> TestResult {
    let start = std::time::Instant::now();
    let name = "decode_lite_header_ping";

    let buf = [0x53, LiteMessageType::Ping as u8, 0x00, 0x00];
    let result = decode_lite_header(&buf);

    match result {
        Some((msg_type, address)) => {
            let is_ping = matches!(msg_type, LiteMessageType::Ping);
            if is_ping && address == 0x0000 {
                TestResult::pass(name, start.elapsed().as_millis())
            } else {
                TestResult::fail(name, "Wrong message type", start.elapsed().as_millis())
            }
        }
        None => TestResult::fail(name, "Decode returned None", start.elapsed().as_millis()),
    }
}

fn test_decode_lite_header_invalid_magic() -> TestResult {
    let start = std::time::Instant::now();
    let name = "decode_lite_header_invalid_magic";

    let buf = [0x00, LiteMessageType::Set as u8, 0x00, 0x01]; // Wrong magic
    let result = decode_lite_header(&buf);

    if result.is_none() {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "Should return None for invalid magic",
            start.elapsed().as_millis(),
        )
    }
}

fn test_decode_lite_header_invalid_type() -> TestResult {
    let start = std::time::Instant::now();
    let name = "decode_lite_header_invalid_type";

    let buf = [0x53, 0xFF, 0x00, 0x01]; // Invalid message type
    let result = decode_lite_header(&buf);

    if result.is_none() {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "Should return None for invalid type",
            start.elapsed().as_millis(),
        )
    }
}

fn test_decode_lite_header_too_short() -> TestResult {
    let start = std::time::Instant::now();
    let name = "decode_lite_header_too_short";

    let buf = [0x53, LiteMessageType::Set as u8, 0x00]; // Only 3 bytes
    let result = decode_lite_header(&buf);

    if result.is_none() {
        TestResult::pass(name, start.elapsed().as_millis())
    } else {
        TestResult::fail(
            name,
            "Should return None for short buffer",
            start.elapsed().as_millis(),
        )
    }
}

// ============================================================================
// Round-trip Tests
// ============================================================================

fn test_encode_decode_roundtrip() -> TestResult {
    let start = std::time::Instant::now();
    let name = "encode_decode_roundtrip";

    let mut buf = [0u8; 16];
    let address: u16 = 0x1234;
    let value: i32 = 0xDEADBEEF_u32 as i32;

    let len = encode_lite_set(address, value, &mut buf);

    if len != 8 {
        return TestResult::fail(name, "Encoding failed", start.elapsed().as_millis());
    }

    let result = decode_lite_header(&buf);

    match result {
        Some((msg_type, decoded_addr)) => {
            let is_set = matches!(msg_type, LiteMessageType::Set);
            if is_set && decoded_addr == address {
                // Decode the value manually (big-endian)
                let decoded_value = ((buf[4] as i32) << 24)
                    | ((buf[5] as i32) << 16)
                    | ((buf[6] as i32) << 8)
                    | (buf[7] as i32);

                if decoded_value == value {
                    TestResult::pass(name, start.elapsed().as_millis())
                } else {
                    TestResult::fail(
                        name,
                        format!("Value mismatch: {} != {}", decoded_value, value),
                        start.elapsed().as_millis(),
                    )
                }
            } else {
                TestResult::fail(
                    name,
                    "Address mismatch or wrong type",
                    start.elapsed().as_millis(),
                )
            }
        }
        None => TestResult::fail(name, "Decode returned None", start.elapsed().as_millis()),
    }
}

// ============================================================================
// Message Type Tests
// ============================================================================

fn test_all_message_types() -> TestResult {
    let start = std::time::Instant::now();
    let name = "all_message_types";

    // Verify all message type values
    let checks = [
        (LiteMessageType::Hello as u8, 0x01, "Hello"),
        (LiteMessageType::Welcome as u8, 0x02, "Welcome"),
        (LiteMessageType::Set as u8, 0x21, "Set"),
        (LiteMessageType::Publish as u8, 0x20, "Publish"),
        (LiteMessageType::Ping as u8, 0x41, "Ping"),
        (LiteMessageType::Pong as u8, 0x42, "Pong"),
    ];

    for (actual, expected, type_name) in checks {
        if actual != expected {
            return TestResult::fail(
                name,
                format!("{} type: {} != {}", type_name, actual, expected),
                start.elapsed().as_millis(),
            );
        }
    }

    TestResult::pass(name, start.elapsed().as_millis())
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP Embedded/Lite Protocol Tests                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let tests = vec![
        // Encoding tests
        test_encode_lite_set_basic(),
        test_encode_lite_set_negative(),
        test_encode_lite_set_large_value(),
        test_encode_lite_set_buffer_too_small(),
        // Decoding tests
        test_decode_lite_header_set(),
        test_decode_lite_header_hello(),
        test_decode_lite_header_ping(),
        test_decode_lite_header_invalid_magic(),
        test_decode_lite_header_invalid_type(),
        test_decode_lite_header_too_short(),
        // Round-trip tests
        test_encode_decode_roundtrip(),
        // Message type tests
        test_all_message_types(),
    ];

    let mut passed = 0;
    let mut failed = 0;

    println!("┌──────────────────────────────────────┬────────┬──────────┐");
    println!("│ Test                                 │ Status │ Time     │");
    println!("├──────────────────────────────────────┼────────┼──────────┤");

    for test in &tests {
        let status = if test.passed { "✓ PASS" } else { "✗ FAIL" };
        let color = if test.passed { "\x1b[32m" } else { "\x1b[31m" };
        println!(
            "│ {:<36} │ {}{:<6}\x1b[0m │ {:>6}ms │",
            test.name, color, status, test.duration_ms
        );

        if test.passed {
            passed += 1;
        } else {
            failed += 1;
            println!(
                "│   └─ {:<56} │",
                &test.message[..test.message.len().min(56)]
            );
        }
    }

    println!("└──────────────────────────────────────┴────────┴──────────┘");
    println!("\nResults: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
