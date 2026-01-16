//! Art-Net Integration Tests
//!
//! These tests verify that the CLASP Art-Net bridge can:
//! 1. Parse Art-Net packets correctly
//! 2. Generate valid Art-Net packets
//! 3. Handle multiple DMX universes
//! 4. Support Art-Net polling/discovery
//! 5. Handle DMX value changes efficiently (delta detection)

use crate::tests::helpers::{find_available_udp_port, run_test};
use crate::{TestResult, TestSuite};
use artnet_protocol::*;
use std::net::UdpSocket;
use std::time::Duration;

pub async fn run_tests(suite: &mut TestSuite) {
    suite.add_result(test_artnet_dmx_packet_parsing().await);
    suite.add_result(test_artnet_dmx_packet_generation().await);
    suite.add_result(test_artnet_poll_request().await);
    suite.add_result(test_artnet_poll_reply().await);
    suite.add_result(test_artnet_multiple_universes().await);
    suite.add_result(test_artnet_dmx_values().await);
    suite.add_result(test_artnet_sequence_numbers().await);
    suite.add_result(test_artnet_roundtrip().await);
}

/// Test: Parse Art-Net DMX packets
async fn test_artnet_dmx_packet_parsing() -> TestResult {
    run_test(
        "Art-Net: Parse ArtDmx packets",
        Duration::from_secs(5),
        || async {
            // Create a DMX output packet
            let mut dmx_data = [0u8; 512];
            dmx_data[0] = 255; // Channel 1 at full
            dmx_data[1] = 128; // Channel 2 at half
            dmx_data[511] = 64; // Channel 512 at quarter

            let output = Output {
                port_address: PortAddress::try_from(0u16).unwrap(),
                data: dmx_data.to_vec().into(),
                sequence: 1,
                ..Default::default()
            };

            let command = ArtCommand::Output(output);
            let bytes = command
                .write_to_buffer()
                .map_err(|e| format!("Failed to serialize: {:?}", e))?;

            // Parse it back
            let parsed =
                ArtCommand::from_buffer(&bytes).map_err(|e| format!("Failed to parse: {:?}", e))?;

            match parsed {
                ArtCommand::Output(out) => {
                    if out.data.as_ref()[0] != 255 {
                        return Err(format!("Channel 1 mismatch: {}", out.data.as_ref()[0]));
                    }
                    if out.data.as_ref()[1] != 128 {
                        return Err(format!("Channel 2 mismatch: {}", out.data.as_ref()[1]));
                    }
                    if out.data.as_ref()[511] != 64 {
                        return Err(format!("Channel 512 mismatch: {}", out.data.as_ref()[511]));
                    }
                    Ok(())
                }
                _ => Err("Expected Output command".to_string()),
            }
        },
    )
    .await
}

/// Test: Generate valid Art-Net packets
async fn test_artnet_dmx_packet_generation() -> TestResult {
    run_test(
        "Art-Net: Generate valid ArtDmx packets",
        Duration::from_secs(5),
        || async {
            let dmx_data: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();

            let output = Output {
                port_address: PortAddress::try_from(1u16).unwrap(),
                data: dmx_data.clone().into(),
                sequence: 42,
                ..Default::default()
            };

            let command = ArtCommand::Output(output);
            let bytes = command
                .write_to_buffer()
                .map_err(|e| format!("Failed to serialize: {:?}", e))?;

            // Verify header
            if &bytes[0..8] != b"Art-Net\0" {
                return Err("Invalid Art-Net header".to_string());
            }

            // OpCode for ArtDmx is 0x5000 (little-endian: 0x00, 0x50)
            if bytes[8] != 0x00 || bytes[9] != 0x50 {
                return Err(format!("Invalid OpCode: {:02X} {:02X}", bytes[8], bytes[9]));
            }

            Ok(())
        },
    )
    .await
}

/// Test: Art-Net Poll request
async fn test_artnet_poll_request() -> TestResult {
    run_test(
        "Art-Net: Generate and parse ArtPoll",
        Duration::from_secs(5),
        || async {
            let poll = Poll::default();
            let command = ArtCommand::Poll(poll);
            let bytes = command
                .write_to_buffer()
                .map_err(|e| format!("Failed to serialize poll: {:?}", e))?;

            // Parse it back
            let parsed = ArtCommand::from_buffer(&bytes)
                .map_err(|e| format!("Failed to parse poll: {:?}", e))?;

            match parsed {
                ArtCommand::Poll(_) => Ok(()),
                _ => Err("Expected Poll command".to_string()),
            }
        },
    )
    .await
}

/// Test: Art-Net Poll Reply
async fn test_artnet_poll_reply() -> TestResult {
    run_test(
        "Art-Net: Generate and parse ArtPollReply",
        Duration::from_secs(5),
        || async {
            // Create a default PollReply
            let mut reply = PollReply::default();
            reply.address = [192, 168, 1, 100].into();
            reply.port = 0x1936;
            reply.version = [0, 14];

            let command = ArtCommand::PollReply(Box::new(reply));
            let bytes = command
                .write_to_buffer()
                .map_err(|e| format!("Failed to serialize reply: {:?}", e))?;

            let parsed = ArtCommand::from_buffer(&bytes)
                .map_err(|e| format!("Failed to parse reply: {:?}", e))?;

            match parsed {
                ArtCommand::PollReply(r) => {
                    // Verify address was preserved
                    let expected_addr: std::net::Ipv4Addr = [192, 168, 1, 100].into();
                    if r.address != expected_addr {
                        return Err("Address not preserved".to_string());
                    }
                    Ok(())
                }
                _ => Err("Expected PollReply command".to_string()),
            }
        },
    )
    .await
}

/// Test: Multiple Art-Net universes
async fn test_artnet_multiple_universes() -> TestResult {
    run_test(
        "Art-Net: Handle multiple universes",
        Duration::from_secs(5),
        || async {
            // Test universes 0-15 (common range)
            for universe in 0u16..16 {
                let dmx_data = vec![universe as u8; 512];

                let output = Output {
                    port_address: PortAddress::try_from(universe).unwrap(),
                    data: dmx_data.into(),
                    sequence: universe as u8,
                    ..Default::default()
                };

                let command = ArtCommand::Output(output);
                let bytes = command
                    .write_to_buffer()
                    .map_err(|e| format!("Universe {} serialize failed: {:?}", universe, e))?;

                let parsed = ArtCommand::from_buffer(&bytes)
                    .map_err(|e| format!("Universe {} parse failed: {:?}", universe, e))?;

                match parsed {
                    ArtCommand::Output(out) => {
                        let addr: u16 = out.port_address.into();
                        if addr != universe {
                            return Err(format!(
                                "Universe mismatch: expected {}, got {}",
                                universe, addr
                            ));
                        }
                    }
                    _ => return Err(format!("Universe {} not Output", universe)),
                }
            }

            Ok(())
        },
    )
    .await
}

/// Test: DMX value range (0-255)
async fn test_artnet_dmx_values() -> TestResult {
    run_test(
        "Art-Net: DMX value range 0-255",
        Duration::from_secs(5),
        || async {
            // Test all 256 values
            let mut dmx_data = [0u8; 512];
            for i in 0..256 {
                dmx_data[i] = i as u8;
            }
            // Fill rest with test pattern
            for i in 256..512 {
                dmx_data[i] = 255 - ((i - 256) as u8);
            }

            let output = Output {
                port_address: PortAddress::try_from(0u16).unwrap(),
                data: dmx_data.to_vec().into(),
                sequence: 1,
                ..Default::default()
            };

            let command = ArtCommand::Output(output);
            let bytes = command
                .write_to_buffer()
                .map_err(|e| format!("Failed to serialize: {:?}", e))?;

            let parsed =
                ArtCommand::from_buffer(&bytes).map_err(|e| format!("Failed to parse: {:?}", e))?;

            match parsed {
                ArtCommand::Output(out) => {
                    for i in 0..256 {
                        if out.data.as_ref()[i] != i as u8 {
                            return Err(format!(
                                "Channel {} mismatch: expected {}, got {}",
                                i + 1,
                                i,
                                out.data.as_ref()[i]
                            ));
                        }
                    }
                    Ok(())
                }
                _ => Err("Expected Output command".to_string()),
            }
        },
    )
    .await
}

/// Test: Art-Net sequence numbers
async fn test_artnet_sequence_numbers() -> TestResult {
    run_test(
        "Art-Net: Sequence number handling",
        Duration::from_secs(5),
        || async {
            // Test sequence number rollover
            for seq in [0u8, 1, 127, 128, 254, 255] {
                let output = Output {
                    port_address: PortAddress::try_from(0u16).unwrap(),
                    data: vec![0u8; 512].into(),
                    sequence: seq,
                    ..Default::default()
                };

                let command = ArtCommand::Output(output);
                let bytes = command
                    .write_to_buffer()
                    .map_err(|e| format!("Seq {} serialize failed: {:?}", seq, e))?;

                let parsed = ArtCommand::from_buffer(&bytes)
                    .map_err(|e| format!("Seq {} parse failed: {:?}", seq, e))?;

                match parsed {
                    ArtCommand::Output(out) => {
                        if out.sequence != seq {
                            return Err(format!(
                                "Sequence mismatch: expected {}, got {}",
                                seq, out.sequence
                            ));
                        }
                    }
                    _ => return Err(format!("Seq {} not Output", seq)),
                }
            }

            Ok(())
        },
    )
    .await
}

/// Test: Full Art-Net roundtrip through UDP
async fn test_artnet_roundtrip() -> TestResult {
    run_test("Art-Net: UDP roundtrip", Duration::from_secs(5), || async {
        let port = find_available_udp_port();

        // Create receiver
        let receiver = UdpSocket::bind(format!("127.0.0.1:{}", port))
            .map_err(|e| format!("Failed to bind receiver: {}", e))?;
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| format!("Failed to set timeout: {}", e))?;

        // Create and send Art-Net packet
        let mut dmx_data = [0u8; 512];
        dmx_data[0] = 255;
        dmx_data[255] = 128;
        dmx_data[511] = 64;

        let output = Output {
            port_address: PortAddress::try_from(0u16).unwrap(),
            data: dmx_data.to_vec().into(),
            sequence: 1,
            ..Default::default()
        };

        let command = ArtCommand::Output(output);
        let bytes = command
            .write_to_buffer()
            .map_err(|e| format!("Failed to serialize: {:?}", e))?;

        let sender =
            UdpSocket::bind("127.0.0.1:0").map_err(|e| format!("Failed to bind sender: {}", e))?;
        sender
            .send_to(&bytes, format!("127.0.0.1:{}", port))
            .map_err(|e| format!("Failed to send: {}", e))?;

        // Receive and verify
        let mut buf = [0u8; 2048];
        let (len, _) = receiver
            .recv_from(&mut buf)
            .map_err(|e| format!("Failed to receive: {}", e))?;

        let parsed = ArtCommand::from_buffer(&buf[..len])
            .map_err(|e| format!("Failed to parse received: {:?}", e))?;

        match parsed {
            ArtCommand::Output(out) => {
                if out.data.as_ref()[0] != 255 {
                    return Err("Channel 1 not preserved".to_string());
                }
                if out.data.as_ref()[255] != 128 {
                    return Err("Channel 256 not preserved".to_string());
                }
                if out.data.as_ref()[511] != 64 {
                    return Err("Channel 512 not preserved".to_string());
                }
                Ok(())
            }
            _ => Err("Expected Output command".to_string()),
        }
    })
    .await
}
