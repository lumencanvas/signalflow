//! Real Hardware Integration Tests
//!
//! These tests require actual hardware devices to be connected:
//! - MIDI controllers/interfaces
//! - Art-Net fixtures or nodes
//! - OSC-capable applications (TouchOSC, etc.)
//!
//! Run with: cargo run -p clasp-test-suite --bin hardware-tests
//!
//! Environment variables:
//! - CLASP_TEST_MIDI=1          Enable MIDI hardware tests
//! - CLASP_TEST_ARTNET=1        Enable Art-Net hardware tests
//! - CLASP_TEST_OSC=1           Enable OSC hardware tests
//! - CLASP_ARTNET_TARGET=IP     Art-Net node IP address
//! - CLASP_OSC_TARGET=IP:PORT   OSC target address

use std::env;
use std::net::UdpSocket;
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

// ============================================================================
// MIDI Hardware Tests
// ============================================================================

fn test_midi_list_devices() -> TestResult {
    let start = Instant::now();
    let name = "midi_list_devices";

    if !is_enabled("CLASP_TEST_MIDI") {
        return TestResult::skip(name, "Set CLASP_TEST_MIDI=1 to enable");
    }

    match midir::MidiInput::new("CLASP Test") {
        Ok(midi_in) => {
            let ports = midi_in.ports();
            let port_names: Vec<String> = ports
                .iter()
                .filter_map(|p| midi_in.port_name(p).ok())
                .collect();

            if ports.is_empty() {
                TestResult::fail(
                    name,
                    "No MIDI input devices found",
                    start.elapsed().as_millis(),
                )
            } else {
                TestResult::pass(
                    name,
                    format!("Found {} devices: {}", ports.len(), port_names.join(", ")),
                    start.elapsed().as_millis(),
                )
            }
        }
        Err(e) => TestResult::fail(
            name,
            format!("MIDI init failed: {}", e),
            start.elapsed().as_millis(),
        ),
    }
}

fn test_midi_receive_cc() -> TestResult {
    let start = Instant::now();
    let name = "midi_receive_cc";

    if !is_enabled("CLASP_TEST_MIDI") {
        return TestResult::skip(name, "Set CLASP_TEST_MIDI=1 to enable");
    }

    let midi_in = match midir::MidiInput::new("CLASP Test") {
        Ok(m) => m,
        Err(e) => {
            return TestResult::fail(
                name,
                format!("MIDI init failed: {}", e),
                start.elapsed().as_millis(),
            )
        }
    };

    let ports = midi_in.ports();
    if ports.is_empty() {
        return TestResult::fail(name, "No MIDI input devices", start.elapsed().as_millis());
    }

    println!("    → Move a knob/fader on your MIDI controller (5 second timeout)...");

    let received = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let received_clone = received.clone();
    let message_data = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let message_data_clone = message_data.clone();

    let _conn = midi_in.connect(
        &ports[0],
        "clasp-test",
        move |_stamp, message, _| {
            // CC messages start with 0xB0-0xBF
            if message.len() >= 3 && (message[0] & 0xF0) == 0xB0 {
                received_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                if let Ok(mut data) = message_data_clone.lock() {
                    *data = message.to_vec();
                }
            }
        },
        (),
    );

    // Wait for CC message
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if received.load(std::sync::atomic::Ordering::SeqCst) {
            let data = message_data.lock().unwrap();
            let channel = (data[0] & 0x0F) + 1;
            let cc_num = data[1];
            let value = data[2];
            return TestResult::pass(
                name,
                format!("Received CC{} = {} on channel {}", cc_num, value, channel),
                start.elapsed().as_millis(),
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    TestResult::fail(
        name,
        "Timeout waiting for CC message",
        start.elapsed().as_millis(),
    )
}

fn test_midi_receive_note() -> TestResult {
    let start = Instant::now();
    let name = "midi_receive_note";

    if !is_enabled("CLASP_TEST_MIDI") {
        return TestResult::skip(name, "Set CLASP_TEST_MIDI=1 to enable");
    }

    let midi_in = match midir::MidiInput::new("CLASP Test") {
        Ok(m) => m,
        Err(e) => {
            return TestResult::fail(
                name,
                format!("MIDI init failed: {}", e),
                start.elapsed().as_millis(),
            )
        }
    };

    let ports = midi_in.ports();
    if ports.is_empty() {
        return TestResult::fail(name, "No MIDI input devices", start.elapsed().as_millis());
    }

    println!("    → Press a key/pad on your MIDI controller (5 second timeout)...");

    let received = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let received_clone = received.clone();
    let message_data = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let message_data_clone = message_data.clone();

    let _conn = midi_in.connect(
        &ports[0],
        "clasp-test",
        move |_stamp, message, _| {
            // Note On messages start with 0x90-0x9F
            if message.len() >= 3 && (message[0] & 0xF0) == 0x90 && message[2] > 0 {
                received_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                if let Ok(mut data) = message_data_clone.lock() {
                    *data = message.to_vec();
                }
            }
        },
        (),
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if received.load(std::sync::atomic::Ordering::SeqCst) {
            let data = message_data.lock().unwrap();
            let channel = (data[0] & 0x0F) + 1;
            let note = data[1];
            let velocity = data[2];
            return TestResult::pass(
                name,
                format!(
                    "Received Note {} velocity {} on channel {}",
                    note, velocity, channel
                ),
                start.elapsed().as_millis(),
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    TestResult::fail(
        name,
        "Timeout waiting for Note On",
        start.elapsed().as_millis(),
    )
}

fn test_midi_send_cc() -> TestResult {
    let start = Instant::now();
    let name = "midi_send_cc";

    if !is_enabled("CLASP_TEST_MIDI") {
        return TestResult::skip(name, "Set CLASP_TEST_MIDI=1 to enable");
    }

    let midi_out = match midir::MidiOutput::new("CLASP Test") {
        Ok(m) => m,
        Err(e) => {
            return TestResult::fail(
                name,
                format!("MIDI out init failed: {}", e),
                start.elapsed().as_millis(),
            )
        }
    };

    let ports = midi_out.ports();
    if ports.is_empty() {
        return TestResult::fail(name, "No MIDI output devices", start.elapsed().as_millis());
    }

    match midi_out.connect(&ports[0], "clasp-test") {
        Ok(mut conn) => {
            // Send CC 1 (mod wheel) value 64 on channel 1
            let result = conn.send(&[0xB0, 0x01, 64]);
            match result {
                Ok(()) => TestResult::pass(
                    name,
                    "Sent CC1=64 on channel 1",
                    start.elapsed().as_millis(),
                ),
                Err(e) => TestResult::fail(
                    name,
                    format!("Send failed: {}", e),
                    start.elapsed().as_millis(),
                ),
            }
        }
        Err(e) => TestResult::fail(
            name,
            format!("Connect failed: {}", e),
            start.elapsed().as_millis(),
        ),
    }
}

// ============================================================================
// Art-Net Hardware Tests
// ============================================================================

fn test_artnet_discover_nodes() -> TestResult {
    let start = Instant::now();
    let name = "artnet_discover_nodes";

    if !is_enabled("CLASP_TEST_ARTNET") {
        return TestResult::skip(name, "Set CLASP_TEST_ARTNET=1 to enable");
    }

    let socket = match UdpSocket::bind("0.0.0.0:6454") {
        Ok(s) => s,
        Err(_) => {
            // Port in use, try ephemeral
            match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(e) => {
                    return TestResult::fail(
                        name,
                        format!("Bind failed: {}", e),
                        start.elapsed().as_millis(),
                    )
                }
            }
        }
    };

    socket.set_read_timeout(Some(Duration::from_secs(3))).ok();
    socket.set_broadcast(true).ok();

    // Art-Net ArtPoll packet
    let art_poll = [
        b'A', b'r', b't', b'-', b'N', b'e', b't', 0x00, // ID
        0x00, 0x20, // OpCode ArtPoll (little-endian)
        0x00, 0x0E, // Protocol version
        0x00, // TalkToMe
        0x00, // Priority
    ];

    if let Err(e) = socket.send_to(&art_poll, "255.255.255.255:6454") {
        return TestResult::fail(
            name,
            format!("Broadcast failed: {}", e),
            start.elapsed().as_millis(),
        );
    }

    println!("    → Waiting for Art-Net nodes (3 second timeout)...");

    let mut nodes = Vec::new();
    let mut buf = [0u8; 512];
    let deadline = Instant::now() + Duration::from_secs(3);

    while Instant::now() < deadline {
        match socket.recv_from(&mut buf) {
            Ok((len, addr)) => {
                if len >= 10 && &buf[0..8] == b"Art-Net\0" {
                    let opcode = u16::from_le_bytes([buf[8], buf[9]]);
                    if opcode == 0x2100 {
                        // ArtPollReply
                        nodes.push(addr.ip().to_string());
                    }
                }
            }
            Err(_) => break,
        }
    }

    if nodes.is_empty() {
        TestResult::fail(
            name,
            "No Art-Net nodes found on network",
            start.elapsed().as_millis(),
        )
    } else {
        TestResult::pass(
            name,
            format!("Found {} nodes: {}", nodes.len(), nodes.join(", ")),
            start.elapsed().as_millis(),
        )
    }
}

fn test_artnet_send_dmx() -> TestResult {
    let start = Instant::now();
    let name = "artnet_send_dmx";

    if !is_enabled("CLASP_TEST_ARTNET") {
        return TestResult::skip(name, "Set CLASP_TEST_ARTNET=1 to enable");
    }

    let target = env::var("CLASP_ARTNET_TARGET").unwrap_or_else(|_| "255.255.255.255".to_string());

    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            return TestResult::fail(
                name,
                format!("Bind failed: {}", e),
                start.elapsed().as_millis(),
            )
        }
    };

    socket.set_broadcast(true).ok();

    // Art-Net ArtDmx packet for Universe 0
    let mut art_dmx = vec![
        b'A', b'r', b't', b'-', b'N', b'e', b't', 0x00, // ID
        0x00, 0x50, // OpCode ArtDmx (little-endian)
        0x00, 0x0E, // Protocol version
        0x00, // Sequence
        0x00, // Physical
        0x00, 0x00, // SubUni, Net (Universe 0)
        0x00, 0x08, // Length high, low (8 channels)
    ];

    // DMX data: ramp channels 1-8
    art_dmx.extend_from_slice(&[255, 200, 150, 100, 75, 50, 25, 0]);

    let target_addr = format!("{}:6454", target);
    match socket.send_to(&art_dmx, &target_addr) {
        Ok(sent) => TestResult::pass(
            name,
            format!(
                "Sent {} bytes DMX to {} (Universe 0, 8 channels)",
                sent, target
            ),
            start.elapsed().as_millis(),
        ),
        Err(e) => TestResult::fail(
            name,
            format!("Send failed: {}", e),
            start.elapsed().as_millis(),
        ),
    }
}

fn test_artnet_chase_effect() -> TestResult {
    let start = Instant::now();
    let name = "artnet_chase_effect";

    if !is_enabled("CLASP_TEST_ARTNET") {
        return TestResult::skip(name, "Set CLASP_TEST_ARTNET=1 to enable");
    }

    let target = env::var("CLASP_ARTNET_TARGET").unwrap_or_else(|_| "255.255.255.255".to_string());

    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            return TestResult::fail(
                name,
                format!("Bind failed: {}", e),
                start.elapsed().as_millis(),
            )
        }
    };

    socket.set_broadcast(true).ok();
    let target_addr = format!("{}:6454", target);

    println!("    → Running 3-second chase effect on channels 1-8...");

    let frames = 90; // 30fps * 3 seconds
    let channels = 8;

    for frame in 0..frames {
        let mut art_dmx = vec![
            b'A',
            b'r',
            b't',
            b'-',
            b'N',
            b'e',
            b't',
            0x00,
            0x00,
            0x50,
            0x00,
            0x0E,
            (frame & 0xFF) as u8, // Sequence
            0x00,
            0x00,
            0x00,
            0x00,
            channels as u8,
        ];

        // Chase pattern
        for ch in 0..channels {
            let phase = (frame + ch * 10) % 60;
            let value = if phase < 30 {
                (phase * 8) as u8
            } else {
                ((60 - phase) * 8) as u8
            };
            art_dmx.push(value.min(255));
        }

        if let Err(e) = socket.send_to(&art_dmx, &target_addr) {
            return TestResult::fail(
                name,
                format!("Send failed at frame {}: {}", frame, e),
                start.elapsed().as_millis(),
            );
        }

        std::thread::sleep(Duration::from_millis(33)); // ~30fps
    }

    TestResult::pass(
        name,
        format!("Completed chase effect ({} frames)", frames),
        start.elapsed().as_millis(),
    )
}

// ============================================================================
// OSC Hardware Tests
// ============================================================================

fn test_osc_send_message() -> TestResult {
    let start = Instant::now();
    let name = "osc_send_message";

    if !is_enabled("CLASP_TEST_OSC") {
        return TestResult::skip(name, "Set CLASP_TEST_OSC=1 to enable");
    }

    let target = env::var("CLASP_OSC_TARGET").unwrap_or_else(|_| "127.0.0.1:8000".to_string());

    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            return TestResult::fail(
                name,
                format!("Bind failed: {}", e),
                start.elapsed().as_millis(),
            )
        }
    };

    // Build OSC message: /test/clasp ,f 0.5
    let msg = rosc::OscMessage {
        addr: "/test/clasp".to_string(),
        args: vec![rosc::OscType::Float(0.5)],
    };

    let packet = rosc::OscPacket::Message(msg);
    let buf = rosc::encoder::encode(&packet).unwrap();

    match socket.send_to(&buf, &target) {
        Ok(sent) => TestResult::pass(
            name,
            format!("Sent {} bytes to {} (/test/clasp = 0.5)", sent, target),
            start.elapsed().as_millis(),
        ),
        Err(e) => TestResult::fail(
            name,
            format!("Send failed: {}", e),
            start.elapsed().as_millis(),
        ),
    }
}

fn test_osc_receive_message() -> TestResult {
    let start = Instant::now();
    let name = "osc_receive_message";

    if !is_enabled("CLASP_TEST_OSC") {
        return TestResult::skip(name, "Set CLASP_TEST_OSC=1 to enable");
    }

    let socket = match UdpSocket::bind("0.0.0.0:9000") {
        Ok(s) => s,
        Err(_) => match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => {
                return TestResult::fail(
                    name,
                    format!("Bind failed: {}", e),
                    start.elapsed().as_millis(),
                )
            }
        },
    };

    socket.set_read_timeout(Some(Duration::from_secs(10))).ok();
    let local_addr = socket.local_addr().unwrap();

    println!(
        "    → Send an OSC message to {} (10 second timeout)...",
        local_addr
    );

    let mut buf = [0u8; 1024];
    match socket.recv_from(&mut buf) {
        Ok((len, from)) => match rosc::decoder::decode_udp(&buf[..len]) {
            Ok((_, packet)) => {
                let addr = match &packet {
                    rosc::OscPacket::Message(m) => m.addr.clone(),
                    rosc::OscPacket::Bundle(_) => "(bundle)".to_string(),
                };
                TestResult::pass(
                    name,
                    format!("Received {} from {}", addr, from),
                    start.elapsed().as_millis(),
                )
            }
            Err(e) => TestResult::fail(
                name,
                format!("Decode failed: {:?}", e),
                start.elapsed().as_millis(),
            ),
        },
        Err(e) => TestResult::fail(
            name,
            format!("Receive timeout: {}", e),
            start.elapsed().as_millis(),
        ),
    }
}

fn test_osc_bidirectional() -> TestResult {
    let start = Instant::now();
    let name = "osc_bidirectional";

    if !is_enabled("CLASP_TEST_OSC") {
        return TestResult::skip(name, "Set CLASP_TEST_OSC=1 to enable");
    }

    let target = env::var("CLASP_OSC_TARGET").unwrap_or_else(|_| "127.0.0.1:8000".to_string());

    let socket = match UdpSocket::bind("0.0.0.0:9001") {
        Ok(s) => s,
        Err(_) => match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => {
                return TestResult::fail(
                    name,
                    format!("Bind failed: {}", e),
                    start.elapsed().as_millis(),
                )
            }
        },
    };

    socket.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let local_addr = socket.local_addr().unwrap();

    // Send query message
    let msg = rosc::OscMessage {
        addr: "/ping".to_string(),
        args: vec![rosc::OscType::String(local_addr.to_string())],
    };
    let packet = rosc::OscPacket::Message(msg);
    let buf = rosc::encoder::encode(&packet).unwrap();

    println!("    → Sent /ping to {}, waiting for /pong reply...", target);

    if let Err(e) = socket.send_to(&buf, &target) {
        return TestResult::fail(
            name,
            format!("Send failed: {}", e),
            start.elapsed().as_millis(),
        );
    }

    let mut recv_buf = [0u8; 1024];
    match socket.recv_from(&mut recv_buf) {
        Ok((len, _from)) => match rosc::decoder::decode_udp(&recv_buf[..len]) {
            Ok((_, packet)) => {
                let addr = match &packet {
                    rosc::OscPacket::Message(m) => m.addr.clone(),
                    rosc::OscPacket::Bundle(_) => "(bundle)".to_string(),
                };
                TestResult::pass(
                    name,
                    format!("Received reply: {}", addr),
                    start.elapsed().as_millis(),
                )
            }
            Err(_) => TestResult::fail(name, "No valid OSC reply", start.elapsed().as_millis()),
        },
        Err(_) => TestResult::fail(
            name,
            "No reply received (target may not support /ping)",
            start.elapsed().as_millis(),
        ),
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              CLASP Real Hardware Tests                           ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("Environment variables:");
    println!(
        "  CLASP_TEST_MIDI={}",
        if is_enabled("CLASP_TEST_MIDI") {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  CLASP_TEST_ARTNET={}",
        if is_enabled("CLASP_TEST_ARTNET") {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  CLASP_TEST_OSC={}",
        if is_enabled("CLASP_TEST_OSC") {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  CLASP_ARTNET_TARGET={}",
        env::var("CLASP_ARTNET_TARGET").unwrap_or_else(|_| "(broadcast)".to_string())
    );
    println!(
        "  CLASP_OSC_TARGET={}",
        env::var("CLASP_OSC_TARGET").unwrap_or_else(|_| "127.0.0.1:8000".to_string())
    );
    println!();

    let tests = vec![
        // MIDI tests
        test_midi_list_devices(),
        test_midi_receive_cc(),
        test_midi_receive_note(),
        test_midi_send_cc(),
        // Art-Net tests
        test_artnet_discover_nodes(),
        test_artnet_send_dmx(),
        test_artnet_chase_effect(),
        // OSC tests
        test_osc_send_message(),
        test_osc_receive_message(),
        test_osc_bidirectional(),
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
            if !test.message.is_empty() && test.message != "OK" {
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
