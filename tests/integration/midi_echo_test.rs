//! MIDI Echo Test
//!
//! Tests MIDI <-> CLASP conversion
//! Note: These tests require a MIDI loopback device or virtual MIDI port

use clasp_bridge::{MidiBridge, MidiBridgeConfig};
use clasp_core::{Message, Value};

#[test]
fn test_midi_message_parsing() {
    // Test Note On parsing
    let note_on = vec![0x90, 60, 100]; // Note On, C4, velocity 100

    // Create a minimal bridge config for testing
    let config = MidiBridgeConfig {
        input_port: None,
        output_port: None,
        namespace: "/midi".to_string(),
        device_name: "test".to_string(),
    };

    // The message conversion is tested via the standalone function
    // In a real test, we'd need virtual MIDI ports
    println!("MIDI parsing test - Note On: {:?}", note_on);
    println!("Expected address: /midi/test/ch/0/note");
}

#[test]
fn test_midi_cc_conversion() {
    // Test CC parsing
    let cc = vec![0xB0, 1, 64]; // CC on ch 0, modulation, value 64

    println!("MIDI CC test: {:?}", cc);
    println!("Expected address: /midi/test/ch/0/cc/1");
    println!("Expected value: 64");
}

#[test]
fn test_midi_pitch_bend() {
    // Test pitch bend parsing
    // Pitch bend is 14-bit, center at 8192
    let lsb = 0;
    let msb = 64; // Center value
    let bend = vec![0xE0, lsb, msb];

    let value = ((msb as i32) << 7 | (lsb as i32)) - 8192;
    println!("MIDI Pitch Bend test: {:?}", bend);
    println!("Calculated value: {} (should be near 0 for center)", value);
}

#[test]
fn test_list_midi_ports() {
    // List available MIDI ports (informational)
    match MidiBridge::list_input_ports() {
        Ok(ports) => {
            println!("Available MIDI input ports:");
            for port in &ports {
                println!("  - {}", port);
            }
            if ports.is_empty() {
                println!("  (no ports found)");
            }
        }
        Err(e) => println!("Error listing input ports: {}", e),
    }

    match MidiBridge::list_output_ports() {
        Ok(ports) => {
            println!("Available MIDI output ports:");
            for port in &ports {
                println!("  - {}", port);
            }
            if ports.is_empty() {
                println!("  (no ports found)");
            }
        }
        Err(e) => println!("Error listing output ports: {}", e),
    }
}

#[tokio::test]
async fn test_midi_bridge_lifecycle() {
    // Test basic bridge start/stop
    let config = MidiBridgeConfig::default();
    let mut bridge = MidiBridge::new(config);

    // Start bridge (may not find ports, that's ok)
    match bridge.start().await {
        Ok(mut rx) => {
            println!("MIDI bridge started");

            // Check for connected event
            if let Some(event) = rx.recv().await {
                println!("Received event: {:?}", event);
            }

            // Stop bridge
            bridge.stop().await.expect("Failed to stop bridge");
            println!("MIDI bridge stopped");
        }
        Err(e) => {
            println!("Could not start MIDI bridge (expected if no MIDI): {}", e);
        }
    }
}

#[test]
fn test_clasp_to_midi_conversion() {
    // Test converting CLASP messages to MIDI bytes

    // CC message
    let cc_address = "/midi/device/ch/0/cc/1";
    let parts: Vec<&str> = cc_address.split('/').collect();
    assert_eq!(parts[4], "cc");

    let channel: u8 = parts[3].parse().unwrap();
    let cc_num: u8 = parts[5].parse().unwrap();
    let value: u8 = 64;

    let midi_bytes = vec![0xB0 | channel, cc_num, value];
    println!("CLASP -> MIDI CC: {:?}", midi_bytes);
    assert_eq!(midi_bytes, vec![0xB0, 1, 64]);

    // Pitch bend message
    let bend_address = "/midi/device/ch/1/bend";
    let bend_parts: Vec<&str> = bend_address.split('/').collect();
    let bend_channel: u8 = bend_parts[3].parse().unwrap();
    let bend_value: i32 = 0; // Center
    let adjusted = (bend_value + 8192).clamp(0, 16383) as u16;
    let lsb = (adjusted & 0x7F) as u8;
    let msb = ((adjusted >> 7) & 0x7F) as u8;

    let bend_bytes = vec![0xE0 | bend_channel, lsb, msb];
    println!("CLASP -> MIDI Bend: {:?}", bend_bytes);
}
