//! UDP broadcast discovery

use crate::{Device, DeviceInfo, DiscoveryError, DiscoveryEvent, Result};
use clasp_core::{codec, Message, HelloMessage, PROTOCOL_VERSION};
use clasp_transport::UdpTransport;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

/// Discover Clasp devices via UDP broadcast
pub async fn discover(port: u16, tx: mpsc::Sender<DiscoveryEvent>) -> Result<()> {
    // Bind to any available port
    let transport = UdpTransport::bind("0.0.0.0:0")
        .await
        .map_err(|e| DiscoveryError::Network(e.to_string()))?;

    // Enable broadcast
    transport
        .set_broadcast(true)
        .map_err(|e| DiscoveryError::Network(e.to_string()))?;

    // Create broadcast address
    let broadcast_addr = format!("255.255.255.255:{}", port)
        .parse()
        .map_err(|e| DiscoveryError::Network(format!("Invalid address: {}", e)))?;

    // Send discovery request (HELLO message)
    let hello = Message::Hello(HelloMessage {
        version: PROTOCOL_VERSION,
        name: "Discovery".to_string(),
        features: vec![],
        capabilities: None,
        token: None,
    });

    let hello_bytes = codec::encode(&hello)
        .map_err(|e| DiscoveryError::Network(e.to_string()))?;

    info!("Broadcasting discovery request on port {}", port);

    transport
        .send_to(&hello_bytes, broadcast_addr)
        .await
        .map_err(|e| DiscoveryError::Broadcast(e.to_string()))?;

    // Start receiver
    let mut receiver = transport.start_receiver();

    // Listen for responses
    let discovery_timeout = Duration::from_secs(5);

    loop {
        match timeout(discovery_timeout, receiver.recv_from()).await {
            Ok(Some((event, from))) => {
                if let clasp_transport::TransportEvent::Data(data) = event {
                    debug!("Received {} bytes from {}", data.len(), from);

                    // Try to decode as Clasp message
                    match codec::decode(&data) {
                        Ok((msg, _)) => {
                            if let Message::Welcome(welcome) = msg {
                                let mut device = Device::new(
                                    welcome.session.clone(),
                                    welcome.name.clone(),
                                );

                                // Build WebSocket URL from source address
                                let ws_url = format!(
                                    "ws://{}:{}/clasp",
                                    from.ip(),
                                    clasp_core::DEFAULT_WS_PORT
                                );
                                device = device.with_ws_endpoint(&ws_url);
                                device = device.with_udp_endpoint(from);

                                device.info = DeviceInfo::default()
                                    .with_features(welcome.features);

                                info!("Discovered device via broadcast: {} at {}", device.name, from);

                                if tx.send(DiscoveryEvent::Found(device)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Failed to decode response from {}: {}", from, e);
                        }
                    }
                }
            }
            Ok(None) => {
                // Channel closed
                break;
            }
            Err(_) => {
                // Timeout - discovery complete
                debug!("Broadcast discovery timeout");
                break;
            }
        }
    }

    Ok(())
}

/// Respond to broadcast discovery requests
pub struct BroadcastResponder {
    transport: UdpTransport,
    name: String,
    features: Vec<String>,
}

impl BroadcastResponder {
    /// Create a new broadcast responder
    pub async fn bind(port: u16, name: String, features: Vec<String>) -> Result<Self> {
        let addr = format!("0.0.0.0:{}", port);
        let transport = UdpTransport::bind(&addr)
            .await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;

        info!("Broadcast responder listening on port {}", port);

        Ok(Self {
            transport,
            name,
            features,
        })
    }

    /// Start responding to discovery requests
    pub async fn run(&self) -> Result<()> {
        let mut receiver = self.transport.start_receiver();

        while let Some((event, from)) = receiver.recv_from().await {
            if let clasp_transport::TransportEvent::Data(data) = event {
                // Try to decode as HELLO
                if let Ok((Message::Hello(_), _)) = codec::decode(&data) {
                    debug!("Received discovery request from {}", from);

                    // Send WELCOME response
                    let welcome = Message::Welcome(clasp_core::WelcomeMessage {
                        version: PROTOCOL_VERSION,
                        session: uuid::Uuid::new_v4().to_string(),
                        name: self.name.clone(),
                        features: self.features.clone(),
                        time: clasp_core::time::now(),
                        token: None,
                    });

                    if let Ok(response) = codec::encode(&welcome) {
                        let _ = self.transport.send_to(&response, from).await;
                        debug!("Sent discovery response to {}", from);
                    }
                }
            }
        }

        Ok(())
    }
}
