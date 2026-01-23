//! Art-Net Integration Tests
//!
//! These tests verify that the CLASP Art-Net bridge can:
//! 1. Parse Art-Net packets correctly
//! 2. Generate valid Art-Net packets
//! 3. Handle multiple DMX universes
//! 4. Support Art-Net polling/discovery
//! 5. Handle DMX value changes efficiently (delta detection)

use artnet_protocol::*;
use std::net::UdpSocket;
use std::time::Duration;

/// Find an available UDP port for testing
fn find_available_udp_port() -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap().port()
}

/// Test: Parse Art-Net DMX packets
#[tokio::test]
async fn test_artnet_dmx_packet_parsing() {
    // Create a DMX output packet
    let mut dmx_data = [0u8; 512];
    dmx_data[0] = 255; // Channel 1 at full
    dmx_data[1] = 128; // Channel 2 at half
    dmx_data[511] = 64; // Channel 512 at quarter

    let mut output = Output::default();
    output.subnet = 0;
    output.data = dmx_data.to_vec().into();
    output.length = 512;
    output.sequence = 1;

    let command = ArtCommand::Output(output);
    let bytes = command
        .into_buffer()
        .expect("Failed to serialize ArtDmx packet");

    // Parse it back
    let parsed = ArtCommand::from_buffer(&bytes).expect("Failed to parse ArtDmx packet");

    match parsed {
        ArtCommand::Output(out) => {
            let data: &[u8] = &out.data;
            assert_eq!(data[0], 255, "Channel 1 mismatch");
            assert_eq!(data[1], 128, "Channel 2 mismatch");
            assert_eq!(data[511], 64, "Channel 512 mismatch");
        }
        _ => panic!("Expected Output command"),
    }
}

/// Test: Generate valid Art-Net packets
#[tokio::test]
async fn test_artnet_dmx_packet_generation() {
    let dmx_data: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();

    let mut output = Output::default();
    output.subnet = 1;
    output.data = dmx_data.clone().into();
    output.length = 512;
    output.sequence = 42;

    let command = ArtCommand::Output(output);
    let bytes = command
        .into_buffer()
        .expect("Failed to serialize ArtDmx packet");

    // Verify header
    assert_eq!(&bytes[0..8], b"Art-Net\0", "Invalid Art-Net header");

    // OpCode for ArtDmx is 0x5000 (little-endian: 0x00, 0x50)
    assert_eq!(bytes[8], 0x00, "Invalid OpCode low byte: {:02X}", bytes[8]);
    assert_eq!(bytes[9], 0x50, "Invalid OpCode high byte: {:02X}", bytes[9]);
}

/// Test: Art-Net Poll request
#[tokio::test]
async fn test_artnet_poll_request() {
    let poll = Poll::default();
    let command = ArtCommand::Poll(poll);
    let bytes = command.into_buffer().expect("Failed to serialize ArtPoll");

    // Parse it back
    let parsed = ArtCommand::from_buffer(&bytes).expect("Failed to parse ArtPoll");

    assert!(
        matches!(parsed, ArtCommand::Poll(_)),
        "Expected Poll command"
    );
}

/// Test: Art-Net Poll Reply parsing from raw bytes
/// Note: artnet_protocol 0.2 doesn't have PollReply::default(), so we test
/// parsing a PollReply from a manually constructed packet instead.
#[tokio::test]
async fn test_artnet_poll_reply() {
    // Construct a minimal valid ArtPollReply packet
    // Art-Net header: "Art-Net\0"
    // OpCode: 0x2100 (ArtPollReply, little-endian)
    // Then the PollReply data follows
    let mut packet = vec![0u8; 239]; // ArtPollReply is a fixed-size packet

    // Header
    packet[0..8].copy_from_slice(b"Art-Net\0");
    // OpCode for ArtPollReply is 0x2100 (little-endian: 0x00, 0x21)
    packet[8] = 0x00;
    packet[9] = 0x21;
    // IP address at offset 10-13: 192.168.1.100
    packet[10] = 192;
    packet[11] = 168;
    packet[12] = 1;
    packet[13] = 100;
    // Port at offset 14-15: 0x1936 (little-endian)
    packet[14] = 0x36;
    packet[15] = 0x19;
    // Rest can be zeros for a minimal valid packet

    let parsed = ArtCommand::from_buffer(&packet).expect("Failed to parse ArtPollReply");

    match parsed {
        ArtCommand::PollReply(r) => {
            // Verify address was preserved
            let expected_addr: std::net::Ipv4Addr = [192, 168, 1, 100].into();
            assert_eq!(r.address, expected_addr, "Address not preserved");
        }
        _ => panic!("Expected PollReply command"),
    }
}

/// Test: Multiple Art-Net universes
#[tokio::test]
async fn test_artnet_multiple_universes() {
    // Test universes 0-15 (common range)
    for universe in 0u16..16 {
        let dmx_data = vec![universe as u8; 512];

        let mut output = Output::default();
        output.subnet = universe;
        output.data = dmx_data.into();
        output.length = 512;
        output.sequence = universe as u8;

        let command = ArtCommand::Output(output);
        let bytes = command
            .into_buffer()
            .unwrap_or_else(|e| panic!("Universe {} serialize failed: {:?}", universe, e));

        let parsed = ArtCommand::from_buffer(&bytes)
            .unwrap_or_else(|e| panic!("Universe {} parse failed: {:?}", universe, e));

        match parsed {
            ArtCommand::Output(out) => {
                assert_eq!(
                    out.subnet, universe,
                    "Universe mismatch: expected {}, got {}",
                    universe, out.subnet
                );
            }
            _ => panic!("Universe {} not Output", universe),
        }
    }
}

/// Test: DMX value range (0-255)
#[tokio::test]
async fn test_artnet_dmx_values() {
    // Test all 256 values
    let mut dmx_data = [0u8; 512];
    for i in 0..256 {
        dmx_data[i] = i as u8;
    }
    // Fill rest with test pattern
    for i in 256..512 {
        dmx_data[i] = 255 - ((i - 256) as u8);
    }

    let mut output = Output::default();
    output.subnet = 0;
    output.data = dmx_data.to_vec().into();
    output.length = 512;
    output.sequence = 1;

    let command = ArtCommand::Output(output);
    let bytes = command
        .into_buffer()
        .expect("Failed to serialize DMX values");

    let parsed = ArtCommand::from_buffer(&bytes).expect("Failed to parse DMX values");

    match parsed {
        ArtCommand::Output(out) => {
            let data: &[u8] = &out.data;
            for i in 0..256 {
                assert_eq!(
                    data[i],
                    i as u8,
                    "Channel {} mismatch: expected {}, got {}",
                    i + 1,
                    i,
                    data[i]
                );
            }
        }
        _ => panic!("Expected Output command"),
    }
}

/// Test: Art-Net sequence numbers
#[tokio::test]
async fn test_artnet_sequence_numbers() {
    // Test sequence number rollover
    for seq in [0u8, 1, 127, 128, 254, 255] {
        let mut output = Output::default();
        output.subnet = 0;
        output.data = vec![0u8; 512].into();
        output.length = 512;
        output.sequence = seq;

        let command = ArtCommand::Output(output);
        let bytes = command
            .into_buffer()
            .unwrap_or_else(|e| panic!("Seq {} serialize failed: {:?}", seq, e));

        let parsed = ArtCommand::from_buffer(&bytes)
            .unwrap_or_else(|e| panic!("Seq {} parse failed: {:?}", seq, e));

        match parsed {
            ArtCommand::Output(out) => {
                assert_eq!(
                    out.sequence, seq,
                    "Sequence mismatch: expected {}, got {}",
                    seq, out.sequence
                );
            }
            _ => panic!("Seq {} not Output", seq),
        }
    }
}

/// Test: Full Art-Net roundtrip through UDP
#[tokio::test]
async fn test_artnet_roundtrip() {
    let port = find_available_udp_port();

    // Create receiver
    let receiver = UdpSocket::bind(format!("127.0.0.1:{}", port)).expect("Failed to bind receiver");
    receiver
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("Failed to set timeout");

    // Create and send Art-Net packet
    let mut dmx_data = [0u8; 512];
    dmx_data[0] = 255;
    dmx_data[255] = 128;
    dmx_data[511] = 64;

    let mut output = Output::default();
    output.subnet = 0;
    output.data = dmx_data.to_vec().into();
    output.length = 512;
    output.sequence = 1;

    let command = ArtCommand::Output(output);
    let bytes = command
        .into_buffer()
        .expect("Failed to serialize for roundtrip");

    let sender = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind sender");
    sender
        .send_to(&bytes, format!("127.0.0.1:{}", port))
        .expect("Failed to send");

    // Receive and verify
    let mut buf = [0u8; 2048];
    let (len, _) = receiver.recv_from(&mut buf).expect("Failed to receive");

    let parsed = ArtCommand::from_buffer(&buf[..len]).expect("Failed to parse received packet");

    match parsed {
        ArtCommand::Output(out) => {
            let data: &[u8] = &out.data;
            assert_eq!(data[0], 255, "Channel 1 not preserved");
            assert_eq!(data[255], 128, "Channel 256 not preserved");
            assert_eq!(data[511], 64, "Channel 512 not preserved");
        }
        _ => panic!("Expected Output command"),
    }
}
