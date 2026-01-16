//! MQTT Bridge for CLASP
//!
//! Provides bidirectional bridging between MQTT and CLASP protocols.
//! Supports MQTT 3.1.1 and 5.0 via rumqttc.

use crate::{Bridge, BridgeConfig, BridgeError, BridgeEvent, Result};
use async_trait::async_trait;
use clasp_core::{Message, PublishMessage, SetMessage, SignalType, Value};
use parking_lot::Mutex;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS as MqttQoS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// MQTT Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttBridgeConfig {
    /// MQTT broker URL (e.g., "localhost:1883")
    pub broker_host: String,
    /// MQTT broker port
    pub broker_port: u16,
    /// Client ID for MQTT connection
    pub client_id: String,
    /// Optional username for authentication
    #[serde(default)]
    pub username: Option<String>,
    /// Optional password for authentication
    #[serde(default)]
    pub password: Option<String>,
    /// Topics to subscribe to
    #[serde(default)]
    pub subscribe_topics: Vec<String>,
    /// QoS level (0, 1, or 2)
    #[serde(default)]
    pub qos: u8,
    /// Keep alive interval in seconds
    #[serde(default = "default_keep_alive")]
    pub keep_alive_secs: u16,
    /// CLASP namespace prefix
    #[serde(default = "default_namespace")]
    pub namespace: String,
}

fn default_keep_alive() -> u16 {
    60
}

fn default_namespace() -> String {
    "/mqtt".to_string()
}

impl Default for MqttBridgeConfig {
    fn default() -> Self {
        Self {
            broker_host: "localhost".to_string(),
            broker_port: 1883,
            client_id: format!(
                "clasp-{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
            ),
            username: None,
            password: None,
            subscribe_topics: vec!["#".to_string()],
            qos: 0,
            keep_alive_secs: 60,
            namespace: "/mqtt".to_string(),
        }
    }
}

/// MQTT Bridge implementation
pub struct MqttBridge {
    config: BridgeConfig,
    mqtt_config: MqttBridgeConfig,
    client: Option<AsyncClient>,
    running: Arc<Mutex<bool>>,
}

impl MqttBridge {
    /// Create a new MQTT bridge
    pub fn new(mqtt_config: MqttBridgeConfig) -> Self {
        let config = BridgeConfig {
            name: "MQTT Bridge".to_string(),
            protocol: "mqtt".to_string(),
            bidirectional: true,
            ..Default::default()
        };

        Self {
            config,
            mqtt_config,
            client: None,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Convert MQTT topic to CLASP address
    fn topic_to_address(&self, topic: &str) -> String {
        format!("{}/{}", self.mqtt_config.namespace, topic.replace('/', "/"))
    }

    /// Convert CLASP address to MQTT topic
    fn address_to_topic(&self, address: &str) -> String {
        address
            .strip_prefix(&self.mqtt_config.namespace)
            .unwrap_or(address)
            .trim_start_matches('/')
            .to_string()
    }

    /// Parse MQTT QoS level
    fn parse_qos(qos: u8) -> MqttQoS {
        match qos {
            0 => MqttQoS::AtMostOnce,
            1 => MqttQoS::AtLeastOnce,
            _ => MqttQoS::ExactlyOnce,
        }
    }

    /// Parse incoming MQTT payload to CLASP Value
    fn parse_payload(payload: &[u8]) -> Value {
        if let Ok(text) = std::str::from_utf8(payload) {
            // Try JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                return Self::json_to_value(json);
            }
            // Try as number
            if let Ok(f) = text.parse::<f64>() {
                return Value::Float(f);
            }
            // Try as bool
            if text == "true" {
                return Value::Bool(true);
            }
            if text == "false" {
                return Value::Bool(false);
            }
            // Return as string
            return Value::String(text.to_string());
        }
        Value::Bytes(payload.to_vec())
    }

    /// Convert JSON value to CLASP Value
    fn json_to_value(json: serde_json::Value) -> Value {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Self::json_to_value).collect())
            }
            serde_json::Value::Object(obj) => {
                let map: HashMap<String, Value> = obj
                    .into_iter()
                    .map(|(k, v)| (k, Self::json_to_value(v)))
                    .collect();
                Value::Map(map)
            }
        }
    }

    /// Convert CLASP Value to MQTT payload
    fn value_to_payload(value: &Value) -> Vec<u8> {
        match value {
            Value::Null => b"null".to_vec(),
            Value::Bool(b) => (if *b { "true" } else { "false" }).as_bytes().to_vec(),
            Value::Int(i) => i.to_string().into_bytes(),
            Value::Float(f) => f.to_string().into_bytes(),
            Value::String(s) => s.as_bytes().to_vec(),
            Value::Bytes(b) => b.clone(),
            Value::Array(_) | Value::Map(_) => {
                serde_json::to_vec(value).unwrap_or_else(|_| b"null".to_vec())
            }
        }
    }
}

#[async_trait]
impl Bridge for MqttBridge {
    fn config(&self) -> &BridgeConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>> {
        if *self.running.lock() {
            return Err(BridgeError::Other("Bridge already running".to_string()));
        }

        // Create MQTT options
        let mut mqttoptions = MqttOptions::new(
            &self.mqtt_config.client_id,
            &self.mqtt_config.broker_host,
            self.mqtt_config.broker_port,
        );
        mqttoptions.set_keep_alive(Duration::from_secs(self.mqtt_config.keep_alive_secs as u64));

        if let (Some(user), Some(pass)) = (&self.mqtt_config.username, &self.mqtt_config.password) {
            mqttoptions.set_credentials(user, pass);
        }

        // Create client
        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 100);
        self.client = Some(client.clone());
        *self.running.lock() = true;

        // Subscribe to topics
        let qos = Self::parse_qos(self.mqtt_config.qos);
        for topic in &self.mqtt_config.subscribe_topics {
            client
                .subscribe(topic, qos)
                .await
                .map_err(|e| BridgeError::ConnectionFailed(format!("Subscribe failed: {}", e)))?;
            debug!("MQTT subscribed to: {}", topic);
        }

        let (tx, rx) = mpsc::channel(100);
        let running = self.running.clone();
        let namespace = self.mqtt_config.namespace.clone();

        info!(
            "MQTT bridge connecting to {}:{}",
            self.mqtt_config.broker_host, self.mqtt_config.broker_port
        );

        // Spawn event loop
        tokio::spawn(async move {
            loop {
                if !*running.lock() {
                    break;
                }

                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        let topic = publish.topic.clone();
                        let payload = publish.payload.to_vec();

                        debug!("MQTT received: {} ({} bytes)", topic, payload.len());

                        let address = format!("{}/{}", namespace, topic);
                        let value = MqttBridge::parse_payload(&payload);

                        let msg = Message::Set(SetMessage {
                            address,
                            value,
                            revision: None,
                            lock: false,
                            unlock: false,
                        });

                        if tx.send(BridgeEvent::ToClasp(msg)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        info!("MQTT connected to broker");
                        let _ = tx.send(BridgeEvent::Connected).await;
                    }
                    Ok(Event::Incoming(Packet::Disconnect)) => {
                        warn!("MQTT disconnected from broker");
                        let _ = tx
                            .send(BridgeEvent::Disconnected {
                                reason: Some("Broker disconnect".to_string()),
                            })
                            .await;
                    }
                    Err(e) => {
                        error!("MQTT error: {:?}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    _ => {}
                }
            }

            let _ = tx.send(BridgeEvent::Disconnected { reason: None }).await;
        });

        Ok(rx)
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.lock() = false;
        if let Some(client) = &self.client {
            let _ = client.disconnect().await;
        }
        self.client = None;
        info!("MQTT bridge stopped");
        Ok(())
    }

    async fn send(&self, msg: Message) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| BridgeError::Other("Not connected".to_string()))?;

        let (address, value) = match &msg {
            Message::Set(set) => (&set.address, &set.value),
            Message::Publish(pub_msg) => {
                if let Some(val) = &pub_msg.value {
                    (&pub_msg.address, val)
                } else {
                    return Ok(());
                }
            }
            _ => return Ok(()),
        };

        let topic = self.address_to_topic(address);
        let payload = Self::value_to_payload(value);
        let qos = Self::parse_qos(self.mqtt_config.qos);

        client
            .publish(&topic, qos, false, payload)
            .await
            .map_err(|e| BridgeError::Other(format!("MQTT publish failed: {}", e)))?;

        debug!("MQTT sent to topic: {}", topic);
        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.running.lock()
    }

    fn namespace(&self) -> &str {
        &self.mqtt_config.namespace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = MqttBridgeConfig::default();
        assert_eq!(config.broker_host, "localhost");
        assert_eq!(config.broker_port, 1883);
        assert_eq!(config.qos, 0);
    }

    #[test]
    fn test_topic_conversion() {
        let config = MqttBridgeConfig::default();
        let bridge = MqttBridge::new(config);

        let address = bridge.topic_to_address("home/sensors/temp");
        assert_eq!(address, "/mqtt/home/sensors/temp");

        let topic = bridge.address_to_topic("/mqtt/home/sensors/temp");
        assert_eq!(topic, "home/sensors/temp");
    }

    #[test]
    fn test_payload_parsing() {
        // JSON object
        let payload = b"{\"value\": 42}";
        let value = MqttBridge::parse_payload(payload);
        assert!(matches!(value, Value::Map(_)));

        // Number
        let payload = b"3.14159";
        let value = MqttBridge::parse_payload(payload);
        assert!(matches!(value, Value::Float(_)));

        // Boolean
        let payload = b"true";
        let value = MqttBridge::parse_payload(payload);
        assert!(matches!(value, Value::Bool(true)));
    }
}
