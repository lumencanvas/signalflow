//! MQTT Bridge Integration Tests
//!
//! End-to-end tests for MQTT <-> CLASP bridge integration:
//! - MQTT -> CLASP message translation
//! - CLASP -> MQTT message translation
//! - Topic -> Address mapping
//! - QoS level mapping
//!
//! Note: These tests require an MQTT broker. They will skip if:
//! - No broker is available at localhost:1883
//! - CLASP_TEST_BROKERS environment variable is not set
//!
//! To run with a broker:
//!   docker run -d -p 1883:1883 eclipse-mosquitto:latest
//!   CLASP_TEST_BROKERS=1 cargo test --test mqtt_tests

use clasp_bridge::mqtt::{MqttBridge, MqttBridgeConfig};
use clasp_bridge::{Bridge, BridgeEvent};
use clasp_client::ClaspBuilder;
use clasp_core::Value;
use clasp_test_utils::TestRouter;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::env;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

/// Check if an MQTT broker is available for testing
fn is_broker_available() -> bool {
    env::var("CLASP_TEST_BROKERS")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
        || TcpStream::connect_timeout(&"127.0.0.1:1883".parse().unwrap(), Duration::from_secs(2))
            .is_ok()
}

/// Test environment that sets up router and MQTT bridge
struct TestEnv {
    router: TestRouter,
    bridge: MqttBridge,
}

impl TestEnv {
    async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Start router
        let router = TestRouter::start().await;

        // Start MQTT bridge
        let config = MqttBridgeConfig {
            broker_host: "localhost".to_string(),
            broker_port: 1883,
            client_id: format!(
                "clasp-test-{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
            ),
            username: None,
            password: None,
            subscribe_topics: vec!["test/#".to_string()],
            qos: 0,
            keep_alive_secs: 60,
            namespace: "/mqtt".to_string(),
        };

        let mut bridge = MqttBridge::new(config);
        let rx: mpsc::Receiver<BridgeEvent> = bridge.start().await?;

        // Drain bridge events in background
        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(_event) = rx.recv().await {
                // Events would be forwarded to router in real deployment
            }
        });

        // Wait for bridge to connect
        sleep(Duration::from_millis(500)).await;

        Ok(Self { router, bridge })
    }

    fn router_url(&self) -> String {
        self.router.url()
    }

    async fn stop(mut self) {
        let _ = self.bridge.stop().await;
        // TestRouter automatically cleans up on drop
    }
}

#[tokio::test]
async fn test_mqtt_topic_to_clasp_address() {
    if !is_broker_available() {
        eprintln!("Skipping test: MQTT broker not available (set CLASP_TEST_BROKERS=1 or start mosquitto)");
        return;
    }

    let env = TestEnv::start()
        .await
        .expect("Failed to start test environment");

    // Connect a CLASP client to subscribe to the translated address
    let client = ClaspBuilder::new(&env.router_url())
        .name("TestClient")
        .connect()
        .await
        .expect("Failed to connect CLASP client");

    let received = Arc::new(Mutex::new(None));
    let received_clone = received.clone();

    let _sub = client
        .subscribe("/mqtt/test/sensor/temp", move |value, _addr| {
            *received_clone.lock().unwrap() = Some(value);
        })
        .await
        .expect("Failed to subscribe");

    // Give subscription time to register
    sleep(Duration::from_millis(200)).await;

    // Publish to MQTT topic (using rumqttc directly for simplicity)
    let mut mqttoptions = MqttOptions::new("test-publisher", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    let (mqtt_client, mut _eventloop) = AsyncClient::new(mqttoptions, 10);

    // Publish a value
    mqtt_client
        .publish("test/sensor/temp", QoS::AtMostOnce, false, "25.5")
        .await
        .expect("MQTT publish failed");

    // Wait for CLASP client to receive it
    let mut received_value = false;
    for _ in 0..20 {
        if received.lock().unwrap().is_some() {
            let val = received.lock().unwrap().take().unwrap();
            if let Value::Float(f) = val {
                if (f - 25.5).abs() < 0.1 {
                    received_value = true;
                    break;
                }
            }
        }
        sleep(Duration::from_millis(100)).await;
    }

    env.stop().await;
    assert!(
        received_value,
        "Did not receive MQTT message in CLASP client"
    );
}

#[tokio::test]
async fn test_clasp_to_mqtt_translation() {
    if !is_broker_available() {
        eprintln!("Skipping test: MQTT broker not available (set CLASP_TEST_BROKERS=1 or start mosquitto)");
        return;
    }

    let env = TestEnv::start()
        .await
        .expect("Failed to start test environment");

    // Subscribe to MQTT topic to receive CLASP messages
    let mut mqttoptions = MqttOptions::new("test-subscriber", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    mqtt_client
        .subscribe("test/output/value", QoS::AtMostOnce)
        .await
        .expect("MQTT subscribe failed");

    // Spawn task to receive MQTT messages
    let received = Arc::new(Mutex::new(None));
    let received_clone = received.clone();
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    if publish.topic == "test/output/value" {
                        let payload = String::from_utf8_lossy(&publish.payload);
                        *received_clone.lock().unwrap() = Some(payload.to_string());
                        break;
                    }
                }
                _ => {}
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Send CLASP SET message
    let client = ClaspBuilder::new(&env.router_url())
        .name("TestPublisher")
        .connect()
        .await
        .expect("Failed to connect CLASP client");

    client
        .set("/mqtt/test/output/value", Value::Float(42.0))
        .await
        .expect("CLASP set failed");

    // Wait for MQTT message
    let mut received_value = false;
    for _ in 0..20 {
        if let Some(val_str) = received.lock().unwrap().take() {
            if let Ok(f) = val_str.parse::<f64>() {
                if (f - 42.0).abs() < 0.1 {
                    received_value = true;
                    break;
                }
            }
        }
        sleep(Duration::from_millis(100)).await;
    }

    env.stop().await;
    assert!(
        received_value,
        "Did not receive CLASP message in MQTT subscriber"
    );
}

#[tokio::test]
async fn test_mqtt_qos_mapping() {
    if !is_broker_available() {
        eprintln!("Skipping test: MQTT broker not available (set CLASP_TEST_BROKERS=1 or start mosquitto)");
        return;
    }

    // This test verifies that MQTT QoS levels are correctly configured
    // We test by creating bridges with different QoS levels and verifying they start
    let config_qos0 = MqttBridgeConfig {
        broker_host: "localhost".to_string(),
        broker_port: 1883,
        client_id: format!(
            "test-qos0-{}",
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        ),
        qos: 0,
        ..Default::default()
    };

    let config_qos1 = MqttBridgeConfig {
        broker_host: "localhost".to_string(),
        broker_port: 1883,
        client_id: format!(
            "test-qos1-{}",
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        ),
        qos: 1,
        ..Default::default()
    };

    // Both should start successfully
    let mut bridge0 = MqttBridge::new(config_qos0);
    let _rx0 = bridge0.start().await.expect("Failed to start QoS 0 bridge");
    bridge0.stop().await.expect("Failed to stop QoS 0 bridge");

    let mut bridge1 = MqttBridge::new(config_qos1);
    let _rx1 = bridge1.start().await.expect("Failed to start QoS 1 bridge");
    bridge1.stop().await.expect("Failed to stop QoS 1 bridge");
}
