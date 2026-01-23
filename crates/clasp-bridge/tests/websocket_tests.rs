//! WebSocket Bridge Integration Tests
//!
//! Tests cover:
//! - WebSocket -> CLASP message translation (JSON text)
//! - CLASP -> WebSocket JSON messages

use clasp_bridge::{
    Bridge, BridgeEvent, WebSocketBridge, WebSocketBridgeConfig, WsMessageFormat, WsMode,
};
use clasp_core::{Message, SetMessage, Value};
use clasp_test_utils::find_available_port;
use futures::{SinkExt, StreamExt};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};

#[tokio::test]
async fn test_websocket_text_to_clasp_set() {
    let port = find_available_port().await;
    let addr = format!("127.0.0.1:{}", port);

    let ws_config = WebSocketBridgeConfig {
        mode: WsMode::Server,
        url: addr.clone(),
        format: WsMessageFormat::Json,
        ping_interval_secs: 0,
        ..WebSocketBridgeConfig::default()
    };

    let mut bridge = WebSocketBridge::new(ws_config);
    let mut rx: mpsc::Receiver<BridgeEvent> = bridge.start().await.expect("Failed to start bridge");

    // Give server time to bind
    sleep(Duration::from_millis(100)).await;

    // Connect WebSocket client
    let url = format!("ws://{}", addr);
    let (mut ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect WebSocket client");

    // Send JSON message
    let json = serde_json::json!({
        "address": "/ws/test/value",
        "value": 42,
    });
    ws_stream
        .send(WsMessage::Text(json.to_string()))
        .await
        .expect("Failed to send WebSocket message");

    // Wait for bridge event
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut received_set: Option<SetMessage> = None;

    while Instant::now() < deadline {
        if let Some(event) = rx.recv().await {
            if let BridgeEvent::ToClasp(Message::Set(set)) = event {
                received_set = Some(set);
                break;
            }
        } else {
            break;
        }
    }

    bridge.stop().await.expect("Failed to stop bridge");

    let set = received_set.expect("Did not receive SET from WebSocket bridge");
    assert_eq!(
        set.address, "/ws/test/value",
        "Wrong address: {}",
        set.address
    );

    match set.value {
        Value::Int(v) => {
            assert_eq!(v, 42, "Wrong value: {}", v);
        }
        other => {
            panic!("Unexpected value type: {:?}", other);
        }
    }
}

#[tokio::test]
async fn test_clasp_set_to_websocket_json() {
    let port = find_available_port().await;
    let addr = format!("127.0.0.1:{}", port);

    let ws_config = WebSocketBridgeConfig {
        mode: WsMode::Server,
        url: addr.clone(),
        format: WsMessageFormat::Json,
        ping_interval_secs: 0,
        ..WebSocketBridgeConfig::default()
    };

    let mut bridge = WebSocketBridge::new(ws_config);
    let mut rx: mpsc::Receiver<BridgeEvent> = bridge.start().await.expect("Failed to start bridge");

    // Give server time to bind
    sleep(Duration::from_millis(100)).await;

    let url = format!("ws://{}", addr);
    let (mut ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect WebSocket client");

    // Drain initial Connected event from bridge
    if let Some(_ev) = rx.recv().await {
        // ignore
    }

    // Send CLASP message through bridge
    let msg = Message::Set(SetMessage {
        address: "/ws/out/value".to_string(),
        value: Value::Int(7),
        revision: None,
        lock: false,
        unlock: false,
    });

    bridge
        .send(msg)
        .await
        .expect("Failed to send CLASP message");

    // Wait for WebSocket message
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut received_text: Option<String> = None;

    while Instant::now() < deadline {
        if let Some(msg) = ws_stream.next().await {
            match msg.expect("WebSocket receive error") {
                WsMessage::Text(text) => {
                    received_text = Some(text);
                    break;
                }
                _ => {}
            }
        } else {
            break;
        }
    }

    bridge.stop().await.expect("Failed to stop bridge");

    let text = received_text.expect("Did not receive JSON from bridge");
    let json: serde_json::Value = serde_json::from_str(&text).expect("Failed to parse JSON");

    let addr = json
        .get("address")
        .and_then(|v| v.as_str())
        .expect("Missing address field");
    assert_eq!(addr, "/ws/out/value", "Wrong address in JSON: {}", addr);

    let value = json.get("value").expect("Missing value field");
    let int_val = value.as_i64().expect("Value is not an integer");
    assert_eq!(int_val, 7, "Wrong value in JSON: {}", int_val);
}
