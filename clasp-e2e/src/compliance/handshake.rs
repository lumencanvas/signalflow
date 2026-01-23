//! Handshake Conformance Tests
//!
//! Tests for CLASP handshake protocol (CLASP Spec 4.1):
//! - HELLO must be first message
//! - WELCOME contains session ID
//! - Version negotiation
//! - Feature negotiation

use super::{ConformanceConfig, ConformanceReport, TestResult};
use anyhow;
use clasp_client::Clasp;
use clasp_core::{codec, HelloMessage, Message, PROTOCOL_VERSION};
use clasp_transport::{TransportEvent, WebSocketTransport, Transport, TransportSender, TransportReceiver};
use std::time::{Duration, Instant};
use tokio::time::timeout;

pub async fn run_tests(config: &ConformanceConfig, report: &mut ConformanceReport) {
    test_hello_must_be_first(config, report).await;
    test_welcome_contains_session_id(config, report).await;
    test_version_negotiation(config, report).await;
    test_feature_negotiation(config, report).await;
    test_duplicate_hello_rejected(config, report).await;
    test_handshake_timeout(config, report).await;
}

async fn test_hello_must_be_first(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "HELLO must be first message";

    let result = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&config.router_url).await?;

        // Send SET before HELLO (protocol violation)
        let set = Message::Set(clasp_core::SetMessage {
            address: "/test".to_string(),
            value: clasp_core::Value::Int(1),
            revision: None,
            lock: false,
            unlock: false,
        });
        sender.send(codec::encode(&set)?).await?;

        // Should get error or disconnect
        let response = timeout(config.timeout, async {
            loop {
                match receiver.recv().await {
                    Some(TransportEvent::Data(data)) => {
                        let (msg, _) = codec::decode(&data)?;
                        return Ok::<_, anyhow::Error>(msg);
                    }
                    Some(TransportEvent::Disconnected { .. }) => {
                        return Err(anyhow::anyhow!("Disconnected"));
                    }
                    Some(TransportEvent::Error(e)) => {
                        return Err(anyhow::anyhow!("Error: {:?}", e));
                    }
                    _ => continue,
                }
            }
        })
        .await;

        // We expect either an error message, a disconnect, or silence (timeout)
        // The server should NOT acknowledge or process the message
        match response {
            Ok(Ok(Message::Error(_))) => Ok(()), // Error response - good
            Ok(Ok(Message::Ack(_))) => Err(anyhow::anyhow!("Server ACKed message before HELLO")),
            Ok(Ok(Message::Welcome(_))) => Err(anyhow::anyhow!("Server sent WELCOME without HELLO")),
            Ok(Err(_)) => Ok(()), // Disconnect is acceptable
            Err(_) => Ok(()), // Timeout is acceptable - server ignores invalid messages
            _ => Ok(()),
        }
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Handshake", duration)
                .with_spec_reference("CLASP 4.1.1"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Handshake", duration, &e.to_string())
                .with_spec_reference("CLASP 4.1.1"),
        ),
    }
}

async fn test_welcome_contains_session_id(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "WELCOME contains session ID";

    let result = async {
        let client = Clasp::connect_to(&config.router_url).await?;

        // Check that we have a session ID
        let session_id = client.session_id();
        if session_id.is_none() || session_id.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            return Err(anyhow::anyhow!("Session ID is missing or empty"));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Handshake", duration)
                .with_spec_reference("CLASP 4.1.2"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Handshake", duration, &e.to_string())
                .with_spec_reference("CLASP 4.1.2"),
        ),
    }
}

async fn test_version_negotiation(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Version negotiation";

    let result = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&config.router_url).await?;

        // Send HELLO with current version
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: "Version Test".to_string(),
            features: vec![],
            capabilities: None,
            token: None,
        });
        sender.send(codec::encode(&hello)?).await?;

        // Should get WELCOME
        let response = timeout(config.timeout, async {
            loop {
                match receiver.recv().await {
                    Some(TransportEvent::Data(data)) => {
                        let (msg, _) = codec::decode(&data)?;
                        return Ok::<_, anyhow::Error>(msg);
                    }
                    Some(TransportEvent::Disconnected { .. }) => {
                        return Err(anyhow::anyhow!("Disconnected"));
                    }
                    _ => continue,
                }
            }
        })
        .await??;

        match response {
            Message::Welcome(w) => {
                // WELCOME should have session ID
                if w.session.is_empty() {
                    return Err(anyhow::anyhow!("WELCOME missing session ID"));
                }
                Ok(())
            }
            Message::Error(e) => Err(anyhow::anyhow!("Server rejected: {} - {}", e.code, e.message)),
            _ => Err(anyhow::anyhow!("Expected WELCOME, got different message")),
        }
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Handshake", duration)
                .with_spec_reference("CLASP 4.1.3"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Handshake", duration, &e.to_string())
                .with_spec_reference("CLASP 4.1.3"),
        ),
    }
}

async fn test_feature_negotiation(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Feature negotiation";

    let result = async {
        let client = Clasp::builder(&config.router_url)
            .features(vec!["param".to_string(), "event".to_string(), "stream".to_string()])
            .connect()
            .await?;

        // Connection should succeed
        if !client.is_connected() {
            return Err(anyhow::anyhow!("Client not connected after handshake"));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Handshake", duration)
                .with_spec_reference("CLASP 4.1.4"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Handshake", duration, &e.to_string())
                .with_spec_reference("CLASP 4.1.4"),
        ),
    }
}

async fn test_duplicate_hello_rejected(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Duplicate HELLO rejected";

    let result = async {
        let (sender, mut receiver) = WebSocketTransport::connect(&config.router_url).await?;

        // First HELLO
        let hello = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: "First".to_string(),
            features: vec![],
            capabilities: None,
            token: None,
        });
        sender.send(codec::encode(&hello)?).await?;

        // Wait for WELCOME
        let got_welcome = timeout(config.timeout, async {
            loop {
                match receiver.recv().await {
                    Some(TransportEvent::Data(data)) => {
                        let (msg, _) = codec::decode(&data)?;
                        if matches!(msg, Message::Welcome(_)) {
                            return Ok::<_, anyhow::Error>(true);
                        }
                    }
                    Some(TransportEvent::Disconnected { .. }) => {
                        return Err(anyhow::anyhow!("Disconnected"));
                    }
                    _ => continue,
                }
            }
        })
        .await??;

        if !got_welcome {
            return Err(anyhow::anyhow!("Did not receive WELCOME"));
        }

        // Second HELLO (should be rejected or ignored)
        let hello2 = Message::Hello(HelloMessage {
            version: PROTOCOL_VERSION,
            name: "Second".to_string(),
            features: vec![],
            capabilities: None,
            token: None,
        });
        sender.send(codec::encode(&hello2)?).await?;

        // Should NOT get another WELCOME
        let response = timeout(Duration::from_millis(500), async {
            loop {
                match receiver.recv().await {
                    Some(TransportEvent::Data(data)) => {
                        let (msg, _) = codec::decode(&data)?;
                        return Ok::<_, anyhow::Error>(msg);
                    }
                    _ => continue,
                }
            }
        })
        .await;

        match response {
            Ok(Ok(Message::Welcome(_))) => Err(anyhow::anyhow!("Server sent second WELCOME")),
            Ok(Ok(Message::Error(_))) => Ok(()), // Error is acceptable
            Ok(Err(_)) => Ok(()), // Disconnect is acceptable
            Err(_) => Ok(()), // Timeout (ignored) is acceptable
            _ => Ok(()),
        }
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Handshake", duration)
                .with_spec_reference("CLASP 4.1.5"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Handshake", duration, &e.to_string())
                .with_spec_reference("CLASP 4.1.5"),
        ),
    }
}

async fn test_handshake_timeout(config: &ConformanceConfig, report: &mut ConformanceReport) {
    let start = Instant::now();
    let test_name = "Handshake timeout";

    // This test verifies that the server handles slow clients appropriately
    // We just verify that we can complete a handshake within the timeout
    let result = async {
        let client = timeout(config.timeout, Clasp::connect_to(&config.router_url)).await??;

        if !client.is_connected() {
            return Err(anyhow::anyhow!("Client not connected"));
        }

        Ok(())
    }
    .await;

    let duration = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => report.add_result(
            TestResult::pass(test_name, "Handshake", duration)
                .with_spec_reference("CLASP 4.1.6"),
        ),
        Err(e) => report.add_result(
            TestResult::fail(test_name, "Handshake", duration, &e.to_string())
                .with_spec_reference("CLASP 4.1.6"),
        ),
    }
}
