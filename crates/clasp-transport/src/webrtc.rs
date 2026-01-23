//! WebRTC DataChannel transport implementation
//!
//! This module provides WebRTC transport for CLASP, enabling:
//! - P2P connections with NAT traversal
//! - Low-latency data channels
//! - Configurable reliability (ordered/unordered, retransmits)
//!
//! CLASP uses two DataChannels:
//! - "clasp" - Unreliable, unordered (for streams, QoS Fire)
//! - "clasp-reliable" - Reliable, ordered (for params/events, QoS Confirm/Commit)

use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{TransportEvent, TransportReceiver, TransportSender};

#[cfg(feature = "webrtc")]
use webrtc_rs::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    data_channel::{
        data_channel_init::RTCDataChannelInit, data_channel_message::DataChannelMessage,
        RTCDataChannel,
    },
    ice_transport::{
        ice_candidate::RTCIceCandidate, ice_connection_state::RTCIceConnectionState,
        ice_server::RTCIceServer,
    },
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
};

/// WebRTC transport configuration
#[derive(Debug, Clone)]
pub struct WebRtcConfig {
    /// ICE servers for NAT traversal
    pub ice_servers: Vec<String>,
    /// Create unreliable channel for streams
    pub unreliable_channel: bool,
    /// Create reliable channel for params/events
    pub reliable_channel: bool,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            ice_servers: vec![
                "stun:stun.l.google.com:19302".into(),
                "stun:stun1.l.google.com:19302".into(),
            ],
            unreliable_channel: true,
            reliable_channel: true,
        }
    }
}

/// WebRTC transport for CLASP
#[cfg(feature = "webrtc")]
pub struct WebRtcTransport {
    config: WebRtcConfig,
    peer_connection: Arc<RTCPeerConnection>,
    unreliable_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    reliable_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    connection_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
    ice_candidate_callback: Arc<Mutex<Option<Box<dyn Fn(String) + Send + Sync>>>>,
}

#[cfg(feature = "webrtc")]
impl WebRtcTransport {
    /// Create a new WebRTC transport as the offerer (initiator)
    pub async fn new_offerer() -> Result<(Self, String)> {
        Self::new_offerer_with_config(WebRtcConfig::default()).await
    }

    /// Create offerer with custom config, returns (transport, SDP offer)
    pub async fn new_offerer_with_config(config: WebRtcConfig) -> Result<(Self, String)> {
        let peer_connection = Self::create_peer_connection(&config).await?;

        // Create data channels (offerer creates them)
        let unreliable_channel = if config.unreliable_channel {
            Some(Self::create_unreliable_channel(&peer_connection).await?)
        } else {
            None
        };

        let reliable_channel = if config.reliable_channel {
            Some(Self::create_reliable_channel(&peer_connection).await?)
        } else {
            None
        };

        // Create offer
        let offer = peer_connection
            .create_offer(None)
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Create offer failed: {}", e)))?;

        peer_connection
            .set_local_description(offer.clone())
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("Set local description failed: {}", e))
            })?;

        let sdp = offer.sdp;

        // Set up connection callback for offerer channels
        let connection_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>> =
            Arc::new(Mutex::new(None));
        let ice_candidate_callback: Arc<Mutex<Option<Box<dyn Fn(String) + Send + Sync>>>> =
            Arc::new(Mutex::new(None));

        // Set up on_open handlers for created channels
        if let Some(ref reliable) = reliable_channel {
            let callback = connection_callback.clone();
            // Check if channel is already open
            use webrtc_rs::data_channel::data_channel_state::RTCDataChannelState;
            let is_already_open = reliable.ready_state() == RTCDataChannelState::Open;

            if is_already_open {
                // Channel already open, call callback immediately if set
                if let Some(ref cb) = *callback.lock() {
                    cb();
                }
            } else {
                // Set up handler for when channel opens
                reliable.on_open(Box::new(move || {
                    if let Some(ref cb) = *callback.lock() {
                        cb();
                    }
                    Box::pin(async {})
                }));
            }
        }

        let transport = Self {
            config,
            peer_connection: peer_connection.clone(),
            unreliable_channel: Arc::new(Mutex::new(unreliable_channel)),
            reliable_channel: Arc::new(Mutex::new(reliable_channel)),
            connection_callback,
            ice_candidate_callback: ice_candidate_callback.clone(),
        };

        // Set up ICE candidate handler
        Self::setup_ice_candidate_handler(&peer_connection, ice_candidate_callback);

        Ok((transport, sdp))
    }

    /// Create a new WebRTC transport as the answerer, returns (transport, SDP answer)
    pub async fn new_answerer(remote_offer: &str) -> Result<(Self, String)> {
        Self::new_answerer_with_config(remote_offer, WebRtcConfig::default()).await
    }

    /// Create answerer with custom config
    pub async fn new_answerer_with_config(
        remote_offer: &str,
        config: WebRtcConfig,
    ) -> Result<(Self, String)> {
        let peer_connection = Self::create_peer_connection(&config).await?;

        // Set remote offer
        let offer = RTCSessionDescription::offer(remote_offer.to_string())
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid offer: {}", e)))?;

        peer_connection
            .set_remote_description(offer)
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("Set remote description failed: {}", e))
            })?;

        // Create answer
        let answer = peer_connection.create_answer(None).await.map_err(|e| {
            TransportError::ConnectionFailed(format!("Create answer failed: {}", e))
        })?;

        peer_connection
            .set_local_description(answer.clone())
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("Set local description failed: {}", e))
            })?;

        let sdp = answer.sdp;

        // Set up handler for incoming data channels (from offerer)
        let unreliable_channel_ref = Arc::new(Mutex::new(None::<Arc<RTCDataChannel>>));
        let reliable_channel_ref = Arc::new(Mutex::new(None::<Arc<RTCDataChannel>>));
        let connection_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>> =
            Arc::new(Mutex::new(None));
        let ice_candidate_callback: Arc<Mutex<Option<Box<dyn Fn(String) + Send + Sync>>>> =
            Arc::new(Mutex::new(None));

        let unreliable_clone = unreliable_channel_ref.clone();
        let reliable_clone = reliable_channel_ref.clone();
        let callback_clone = connection_callback.clone();

        peer_connection.on_data_channel(Box::new(move |channel: Arc<RTCDataChannel>| {
            let label: String = channel.label().to_string();
            info!("Received data channel from offerer: {}", label);

            // Set up channel handlers
            let (tx, _rx) = mpsc::channel(100);
            let tx_clone = tx.clone();

            let channel_for_message = channel.clone();
            channel_for_message.on_message(Box::new(move |msg: DataChannelMessage| {
                let data = Bytes::copy_from_slice(&msg.data);
                let tx = tx_clone.clone();
                Box::pin(async move {
                    let _ = tx.send(TransportEvent::Data(data)).await;
                })
            }));

            // Store channel first
            if label == "clasp-reliable" {
                *reliable_clone.lock() = Some(channel.clone());
            } else if label == "clasp" {
                *unreliable_clone.lock() = Some(channel.clone());
            }

            // Set up channel handlers (message, open, close)
            let tx_open = tx.clone();
            let is_reliable = label == "clasp-reliable";
            let callback = callback_clone.clone();
            let label_for_open = label.clone();

            // Check if channel is already open
            use webrtc_rs::data_channel::data_channel_state::RTCDataChannelState;
            let channel_for_check = channel.clone();
            let is_already_open = channel_for_check.ready_state() == RTCDataChannelState::Open;

            if is_already_open && is_reliable {
                // Channel already open, call callback immediately if set
                if let Some(ref cb) = *callback.lock() {
                    cb();
                }
            }

            let channel_for_open = channel.clone();
            channel_for_open.on_open(Box::new(move || {
                let tx = tx_open.clone();
                let callback = callback.clone();
                let is_reliable = is_reliable;
                let label = label_for_open.clone();
                Box::pin(async move {
                    info!("DataChannel '{}' opened (answerer)", label);
                    let _ = tx.send(TransportEvent::Connected).await;
                    // Also call connection callback for reliable channel
                    if is_reliable {
                        info!("Reliable channel opened (answerer), calling connection callback");
                        if let Some(ref cb) = *callback.lock() {
                            cb();
                            info!("Connection callback called (answerer)");
                        } else {
                            warn!("Connection callback not set when reliable channel opened (answerer)");
                        }
                    }
                })
            }));

            let tx_close = tx.clone();
            let channel_for_close = channel.clone();
            channel_for_close.on_close(Box::new(move || {
                let tx = tx_close.clone();
                Box::pin(async move {
                    let _ = tx.send(TransportEvent::Disconnected { reason: None }).await;
                })
            }));

            Box::pin(async {})
        }));

        let transport = Self {
            config,
            peer_connection: peer_connection.clone(),
            unreliable_channel: unreliable_channel_ref,
            reliable_channel: reliable_channel_ref,
            connection_callback,
            ice_candidate_callback: ice_candidate_callback.clone(),
        };

        // Set up ICE candidate handler
        Self::setup_ice_candidate_handler(&peer_connection, ice_candidate_callback);

        Ok((transport, sdp))
    }

    /// Set the remote SDP answer (for offerer after receiving answer)
    pub async fn set_remote_answer(&self, remote_answer: &str) -> Result<()> {
        let answer = RTCSessionDescription::answer(remote_answer.to_string())
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid answer: {}", e)))?;

        self.peer_connection
            .set_remote_description(answer)
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("Set remote description failed: {}", e))
            })?;

        Ok(())
    }

    /// Add ICE candidate from remote peer
    pub async fn add_ice_candidate(&self, candidate: &str) -> Result<()> {
        use webrtc_rs::ice_transport::ice_candidate::RTCIceCandidateInit;

        let candidate = serde_json::from_str::<RTCIceCandidateInit>(candidate)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid candidate: {}", e)))?;

        self.peer_connection
            .add_ice_candidate(candidate)
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("Add ICE candidate failed: {}", e))
            })?;

        Ok(())
    }

    /// Set callback to be called when connection is ready (reliable channel opens)
    pub fn on_connection_ready<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.connection_callback.lock() = Some(Box::new(callback));

        // Check if reliable channel is already open and call callback immediately
        use webrtc_rs::data_channel::data_channel_state::RTCDataChannelState;
        if let Some(ref channel) = *self.reliable_channel.lock() {
            if channel.ready_state() == RTCDataChannelState::Open {
                if let Some(ref cb) = *self.connection_callback.lock() {
                    cb();
                }
            }
        }
    }

    /// Set callback to be called when ICE candidates are generated
    pub fn on_ice_candidate<F>(&self, callback: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        *self.ice_candidate_callback.lock() = Some(Box::new(callback));
    }

    /// Get sender/receiver pair for the unreliable channel (streams)
    pub fn unreliable_channel(&self) -> Option<(WebRtcSender, WebRtcReceiver)> {
        self.unreliable_channel.lock().as_ref().map(|dc| {
            let (tx, rx) = Self::setup_channel_handlers(dc.clone());
            (
                WebRtcSender {
                    channel: dc.clone(),
                    connected: Arc::new(Mutex::new(true)),
                },
                WebRtcReceiver { rx },
            )
        })
    }

    /// Get sender/receiver pair for the reliable channel (params/events)
    pub fn reliable_channel(&self) -> Option<(WebRtcSender, WebRtcReceiver)> {
        self.reliable_channel.lock().as_ref().map(|dc| {
            let (tx, rx) = Self::setup_channel_handlers(dc.clone());
            (
                WebRtcSender {
                    channel: dc.clone(),
                    connected: Arc::new(Mutex::new(true)),
                },
                WebRtcReceiver { rx },
            )
        })
    }

    /// Send data via the reliable channel (for params/events that need guaranteed delivery)
    pub async fn send_reliable(&self, data: Bytes) -> Result<()> {
        let channel = self.reliable_channel.lock();
        if let Some(ref dc) = *channel {
            dc.send(&data)
                .await
                .map_err(|e| TransportError::SendFailed(format!("Reliable send failed: {}", e)))?;
            Ok(())
        } else {
            Err(TransportError::NotConnected)
        }
    }

    /// Send data via the unreliable channel (for streams that prioritize latency)
    pub async fn send_unreliable(&self, data: Bytes) -> Result<()> {
        let channel = self.unreliable_channel.lock();
        if let Some(ref dc) = *channel {
            dc.send(&data).await.map_err(|e| {
                TransportError::SendFailed(format!("Unreliable send failed: {}", e))
            })?;
            Ok(())
        } else {
            Err(TransportError::NotConnected)
        }
    }

    async fn create_peer_connection(config: &WebRtcConfig) -> Result<Arc<RTCPeerConnection>> {
        let mut m = MediaEngine::default();
        m.register_default_codecs().map_err(|e| {
            TransportError::ConnectionFailed(format!("Codec registration failed: {}", e))
        })?;

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m).map_err(|e| {
            TransportError::ConnectionFailed(format!("Interceptor registration failed: {}", e))
        })?;

        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();

        let ice_servers: Vec<RTCIceServer> = config
            .ice_servers
            .iter()
            .map(|url| RTCIceServer {
                urls: vec![url.clone()],
                ..Default::default()
            })
            .collect();

        let rtc_config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        let peer_connection = api.new_peer_connection(rtc_config).await.map_err(|e| {
            TransportError::ConnectionFailed(format!("PeerConnection creation failed: {}", e))
        })?;

        // Set up connection state handler
        peer_connection.on_peer_connection_state_change(Box::new(move |state| {
            info!("WebRTC connection state: {:?}", state);
            Box::pin(async {})
        }));

        Ok(Arc::new(peer_connection))
    }

    /// Set up ICE candidate handler on peer connection
    fn setup_ice_candidate_handler(
        pc: &Arc<RTCPeerConnection>,
        callback: Arc<Mutex<Option<Box<dyn Fn(String) + Send + Sync>>>>,
    ) {
        use webrtc_rs::ice_transport::ice_candidate::RTCIceCandidate;

        pc.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
            if let Some(candidate) = candidate {
                // Convert candidate to JSON format
                match candidate.to_json() {
                    Ok(candidate_init) => {
                        // Serialize RTCIceCandidateInit to JSON string
                        if let Ok(candidate_json) = serde_json::to_string(&candidate_init) {
                            if let Some(ref cb) = *callback.lock() {
                                cb(candidate_json);
                            }
                        } else {
                            warn!("Failed to serialize ICE candidate to JSON");
                        }
                    }
                    Err(e) => {
                        warn!("Failed to convert ICE candidate to JSON: {}", e);
                    }
                }
            }
            Box::pin(async {})
        }));
    }

    async fn create_unreliable_channel(pc: &Arc<RTCPeerConnection>) -> Result<Arc<RTCDataChannel>> {
        let options = RTCDataChannelInit {
            ordered: Some(false),
            max_retransmits: Some(0),
            ..Default::default()
        };

        let channel = pc
            .create_data_channel("clasp", Some(options))
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("DataChannel creation failed: {}", e))
            })?;

        info!("Created unreliable DataChannel 'clasp'");
        Ok(channel)
    }

    async fn create_reliable_channel(pc: &Arc<RTCPeerConnection>) -> Result<Arc<RTCDataChannel>> {
        let options = RTCDataChannelInit {
            ordered: Some(true),
            ..Default::default()
        };

        let channel = pc
            .create_data_channel("clasp-reliable", Some(options))
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("DataChannel creation failed: {}", e))
            })?;

        info!("Created reliable DataChannel 'clasp-reliable'");
        Ok(channel)
    }

    fn setup_channel_handlers(
        channel: Arc<RTCDataChannel>,
    ) -> (mpsc::Sender<TransportEvent>, mpsc::Receiver<TransportEvent>) {
        let (tx, rx) = mpsc::channel(100);
        let tx_clone = tx.clone();

        channel.on_message(Box::new(move |msg: DataChannelMessage| {
            let data = Bytes::copy_from_slice(&msg.data);
            let tx = tx_clone.clone();
            Box::pin(async move {
                let _ = tx.send(TransportEvent::Data(data)).await;
            })
        }));

        let tx_open = tx.clone();
        channel.on_open(Box::new(move || {
            let tx = tx_open.clone();
            Box::pin(async move {
                let _ = tx.send(TransportEvent::Connected).await;
            })
        }));

        let tx_close = tx.clone();
        channel.on_close(Box::new(move || {
            let tx = tx_close.clone();
            Box::pin(async move {
                let _ = tx.send(TransportEvent::Disconnected { reason: None }).await;
            })
        }));

        (tx, rx)
    }
}

/// WebRTC DataChannel sender
#[cfg(feature = "webrtc")]
pub struct WebRtcSender {
    channel: Arc<RTCDataChannel>,
    connected: Arc<Mutex<bool>>,
}

#[cfg(feature = "webrtc")]
#[async_trait]
impl TransportSender for WebRtcSender {
    async fn send(&self, data: Bytes) -> Result<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected);
        }

        self.channel
            .send(&data)
            .await
            .map_err(|e| TransportError::SendFailed(format!("DataChannel send failed: {}", e)))?;

        debug!("WebRTC sent {} bytes", data.len());
        Ok(())
    }

    fn is_connected(&self) -> bool {
        *self.connected.lock()
    }

    async fn close(&self) -> Result<()> {
        *self.connected.lock() = false;
        self.channel
            .close()
            .await
            .map_err(|e| TransportError::SendFailed(format!("DataChannel close failed: {}", e)))?;
        Ok(())
    }
}

/// WebRTC DataChannel receiver
#[cfg(feature = "webrtc")]
pub struct WebRtcReceiver {
    rx: mpsc::Receiver<TransportEvent>,
}

#[cfg(feature = "webrtc")]
#[async_trait]
impl TransportReceiver for WebRtcReceiver {
    async fn recv(&mut self) -> Option<TransportEvent> {
        self.rx.recv().await
    }
}

// Stub implementations when WebRTC feature is disabled
#[cfg(not(feature = "webrtc"))]
pub struct WebRtcTransport;

#[cfg(not(feature = "webrtc"))]
pub struct WebRtcConfig;

#[cfg(not(feature = "webrtc"))]
impl Default for WebRtcConfig {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(feature = "webrtc"))]
impl WebRtcTransport {
    pub async fn new_offerer() -> Result<(Self, String)> {
        Err(TransportError::ConnectionFailed(
            "WebRTC feature not enabled. Compile with --features webrtc".into(),
        ))
    }

    pub async fn new_answerer(_remote_offer: &str) -> Result<(Self, String)> {
        Err(TransportError::ConnectionFailed(
            "WebRTC feature not enabled. Compile with --features webrtc".into(),
        ))
    }
}
