//! QUIC transport implementation
//!
//! This module provides QUIC transport for CLASP using the quinn crate.
//! QUIC offers modern, secure, multiplexed connections with features like:
//! - Connection migration (seamless network changes)
//! - 0-RTT connection establishment
//! - Built-in encryption (TLS 1.3)
//! - Stream multiplexing
//!
//! Ideal for:
//! - Mobile applications (connection migration)
//! - High-performance native apps
//! - Scenarios requiring both reliable and unreliable streams

use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{TransportEvent, TransportReceiver, TransportSender};

#[cfg(feature = "quic")]
use quinn::{
    ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig, TransportConfig,
};
#[cfg(feature = "quic")]
use std::net::SocketAddr;

/// ALPN protocol identifier for CLASP over QUIC
pub const CLASP_ALPN: &[u8] = b"clasp/2";

/// Default channel buffer size for QUIC connections
const DEFAULT_CHANNEL_BUFFER_SIZE: usize = 1000;

/// Certificate verification mode
#[derive(Debug, Clone, Default)]
pub enum CertVerification {
    /// Skip certificate verification (INSECURE - development only)
    #[default]
    SkipVerification,
    /// Use system root certificates
    SystemRoots,
    /// Use custom root certificates (DER-encoded)
    CustomRoots(Vec<Vec<u8>>),
}

/// QUIC transport configuration
#[derive(Debug, Clone)]
pub struct QuicConfig {
    /// Enable 0-RTT for faster connection establishment
    pub enable_0rtt: bool,
    /// Keep-alive interval in milliseconds (0 to disable)
    pub keep_alive_ms: u64,
    /// Maximum idle timeout in milliseconds
    pub idle_timeout_ms: u64,
    /// Initial congestion window (packets)
    pub initial_window: u32,
    /// Certificate verification mode
    pub cert_verification: CertVerification,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            enable_0rtt: true,
            keep_alive_ms: 5000,
            idle_timeout_ms: 30000,
            initial_window: 10,
            cert_verification: CertVerification::default(),
        }
    }
}

impl QuicConfig {
    /// Create a config with system root certificate verification (recommended for production)
    pub fn with_system_roots() -> Self {
        Self {
            cert_verification: CertVerification::SystemRoots,
            ..Default::default()
        }
    }

    /// Create a config that skips certificate verification (development only)
    pub fn insecure() -> Self {
        Self {
            cert_verification: CertVerification::SkipVerification,
            ..Default::default()
        }
    }

    /// Create a config with custom root certificates
    pub fn with_custom_roots(certs: Vec<Vec<u8>>) -> Self {
        Self {
            cert_verification: CertVerification::CustomRoots(certs),
            ..Default::default()
        }
    }
}

/// QUIC transport for CLASP
#[cfg(feature = "quic")]
pub struct QuicTransport {
    config: QuicConfig,
    endpoint: Endpoint,
}

#[cfg(feature = "quic")]
impl QuicTransport {
    /// Create a client endpoint (for connecting to servers)
    pub fn new_client() -> Result<Self> {
        Self::new_client_with_config(QuicConfig::default())
    }

    /// Create a client with custom config
    pub fn new_client_with_config(config: QuicConfig) -> Result<Self> {
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).map_err(|e| {
            TransportError::ConnectionFailed(format!("Endpoint creation failed: {}", e))
        })?;

        // Configure client with dangerous (skip verification) for development
        // In production, proper certificate verification should be used
        let client_config = Self::build_client_config(&config)?;
        endpoint.set_default_client_config(client_config);

        info!("QUIC client endpoint created");
        Ok(Self { config, endpoint })
    }

    /// Create a server endpoint
    pub fn new_server(bind_addr: SocketAddr, cert_der: Vec<u8>, key_der: Vec<u8>) -> Result<Self> {
        Self::new_server_with_config(bind_addr, cert_der, key_der, QuicConfig::default())
    }

    /// Create a server with custom config
    pub fn new_server_with_config(
        bind_addr: SocketAddr,
        cert_der: Vec<u8>,
        key_der: Vec<u8>,
        config: QuicConfig,
    ) -> Result<Self> {
        let server_config = Self::build_server_config(&config, cert_der, key_der)?;

        let endpoint = Endpoint::server(server_config, bind_addr).map_err(|e| {
            TransportError::ConnectionFailed(format!("Server endpoint failed: {}", e))
        })?;

        info!("QUIC server listening on {}", bind_addr);
        Ok(Self { config, endpoint })
    }

    /// Connect to a QUIC server
    pub async fn connect(&self, addr: SocketAddr, server_name: &str) -> Result<QuicConnection> {
        let connection = self
            .endpoint
            .connect(addr, server_name)
            .map_err(|e| TransportError::ConnectionFailed(format!("Connect failed: {}", e)))?
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Connection failed: {}", e)))?;

        info!("QUIC connected to {} ({})", server_name, addr);
        Ok(QuicConnection::new(connection))
    }

    /// Accept incoming connections (server mode)
    pub async fn accept(&self) -> Result<QuicConnection> {
        let incoming = self
            .endpoint
            .accept()
            .await
            .ok_or_else(|| TransportError::ConnectionFailed("Endpoint closed".into()))?;

        let connection = incoming
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Accept failed: {}", e)))?;

        let remote = connection.remote_address();
        info!("QUIC accepted connection from {}", remote);
        Ok(QuicConnection::new(connection))
    }

    /// Get the local address
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.endpoint.local_addr().map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to get local addr: {}", e))
        })
    }

    fn build_client_config(config: &QuicConfig) -> Result<ClientConfig> {
        let crypto = match &config.cert_verification {
            CertVerification::SkipVerification => {
                // WARNING: Do not use in production - vulnerable to MITM attacks
                warn!("QUIC using insecure certificate verification - DO NOT USE IN PRODUCTION");
                let mut cfg = rustls::ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
                    .with_no_client_auth();
                cfg.alpn_protocols = vec![CLASP_ALPN.to_vec()];
                cfg
            }
            CertVerification::SystemRoots => {
                // Use system root certificates
                let mut root_store = rustls::RootCertStore::empty();

                // Load native certs - CertificateResult has certs and errors fields
                let cert_result = rustls_native_certs::load_native_certs();

                // Log any errors encountered during loading
                for err in &cert_result.errors {
                    debug!("Certificate loading error: {}", err);
                }

                // Add all successfully loaded certificates
                for cert in cert_result.certs {
                    if let Err(e) = root_store.add(cert) {
                        debug!("Failed to add system cert: {}", e);
                    }
                }

                info!("Loaded {} system root certificates", root_store.len());

                if root_store.is_empty() {
                    return Err(TransportError::ConnectionFailed(
                        "No root certificates available".to_string(),
                    ));
                }

                let mut cfg = rustls::ClientConfig::builder()
                    .with_root_certificates(root_store)
                    .with_no_client_auth();
                cfg.alpn_protocols = vec![CLASP_ALPN.to_vec()];
                cfg
            }
            CertVerification::CustomRoots(certs) => {
                // Use custom root certificates
                let mut root_store = rustls::RootCertStore::empty();

                for cert_der in certs {
                    let cert = rustls::pki_types::CertificateDer::from(cert_der.clone());
                    if let Err(e) = root_store.add(cert) {
                        warn!("Failed to add custom cert: {}", e);
                    }
                }

                if root_store.is_empty() {
                    return Err(TransportError::ConnectionFailed(
                        "No valid custom certificates provided".to_string(),
                    ));
                }

                info!("Using {} custom root certificates", root_store.len());
                let mut cfg = rustls::ClientConfig::builder()
                    .with_root_certificates(root_store)
                    .with_no_client_auth();
                cfg.alpn_protocols = vec![CLASP_ALPN.to_vec()];
                cfg
            }
        };

        let quic_crypto =
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto).map_err(|e| {
                TransportError::ConnectionFailed(format!("Crypto config failed: {}", e))
            })?;
        let mut client_config = ClientConfig::new(Arc::new(quic_crypto));

        let mut transport = TransportConfig::default();
        if config.keep_alive_ms > 0 {
            transport
                .keep_alive_interval(Some(std::time::Duration::from_millis(config.keep_alive_ms)));
        }
        transport.max_idle_timeout(Some(
            std::time::Duration::from_millis(config.idle_timeout_ms)
                .try_into()
                .unwrap(),
        ));
        client_config.transport_config(Arc::new(transport));

        Ok(client_config)
    }

    fn build_server_config(
        config: &QuicConfig,
        cert_der: Vec<u8>,
        key_der: Vec<u8>,
    ) -> Result<ServerConfig> {
        let cert = rustls::pki_types::CertificateDer::from(cert_der);
        let key = rustls::pki_types::PrivateKeyDer::try_from(key_der)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid private key: {}", e)))?;

        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .map_err(|e| TransportError::ConnectionFailed(format!("TLS config failed: {}", e)))?;

        server_crypto.alpn_protocols = vec![CLASP_ALPN.to_vec()];

        let quic_server_crypto = quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
            .map_err(|e| {
            TransportError::ConnectionFailed(format!("Crypto config failed: {}", e))
        })?;
        let mut server_config = ServerConfig::with_crypto(Arc::new(quic_server_crypto));

        let mut transport = TransportConfig::default();
        if config.keep_alive_ms > 0 {
            transport
                .keep_alive_interval(Some(std::time::Duration::from_millis(config.keep_alive_ms)));
        }
        transport.max_idle_timeout(Some(
            std::time::Duration::from_millis(config.idle_timeout_ms)
                .try_into()
                .unwrap(),
        ));
        server_config.transport_config(Arc::new(transport));

        Ok(server_config)
    }
}

/// Skip server certificate verification (for development only)
#[cfg(feature = "quic")]
#[derive(Debug)]
struct SkipServerVerification;

#[cfg(feature = "quic")]
impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

/// QUIC connection wrapper
#[cfg(feature = "quic")]
pub struct QuicConnection {
    connection: Connection,
}

#[cfg(feature = "quic")]
impl QuicConnection {
    fn new(connection: Connection) -> Self {
        Self { connection }
    }

    /// Open a bidirectional stream (reliable, ordered)
    pub async fn open_bi(&self) -> Result<(QuicSender, QuicReceiver)> {
        let (send, recv) =
            self.connection.open_bi().await.map_err(|e| {
                TransportError::ConnectionFailed(format!("Open stream failed: {}", e))
            })?;

        let (tx, rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER_SIZE);
        let connected = Arc::new(Mutex::new(true));
        let connected_clone = connected.clone();

        // Spawn receiver task
        tokio::spawn(async move {
            let mut recv = recv;
            let mut buf = vec![0u8; 65536];

            loop {
                match recv.read(&mut buf).await {
                    Ok(Some(n)) => {
                        let data = Bytes::copy_from_slice(&buf[..n]);
                        if tx.send(TransportEvent::Data(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        *connected_clone.lock() = false;
                        let _ = tx.send(TransportEvent::Disconnected { reason: None }).await;
                        break;
                    }
                    Err(e) => {
                        error!("QUIC read error: {}", e);
                        *connected_clone.lock() = false;
                        let _ = tx
                            .send(TransportEvent::Disconnected {
                                reason: Some(e.to_string()),
                            })
                            .await;
                        break;
                    }
                }
            }
        });

        Ok((
            QuicSender {
                send: Arc::new(tokio::sync::Mutex::new(send)),
                connected,
            },
            QuicReceiver { rx },
        ))
    }

    /// Accept an incoming bidirectional stream
    pub async fn accept_bi(&self) -> Result<(QuicSender, QuicReceiver)> {
        let (send, recv) = self.connection.accept_bi().await.map_err(|e| {
            TransportError::ConnectionFailed(format!("Accept stream failed: {}", e))
        })?;

        let (tx, rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER_SIZE);
        let connected = Arc::new(Mutex::new(true));
        let connected_clone = connected.clone();

        // Spawn receiver task
        tokio::spawn(async move {
            let mut recv = recv;
            let mut buf = vec![0u8; 65536];

            loop {
                match recv.read(&mut buf).await {
                    Ok(Some(n)) => {
                        let data = Bytes::copy_from_slice(&buf[..n]);
                        if tx.send(TransportEvent::Data(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        *connected_clone.lock() = false;
                        let _ = tx.send(TransportEvent::Disconnected { reason: None }).await;
                        break;
                    }
                    Err(e) => {
                        error!("QUIC read error: {}", e);
                        *connected_clone.lock() = false;
                        let _ = tx
                            .send(TransportEvent::Disconnected {
                                reason: Some(e.to_string()),
                            })
                            .await;
                        break;
                    }
                }
            }
        });

        Ok((
            QuicSender {
                send: Arc::new(tokio::sync::Mutex::new(send)),
                connected,
            },
            QuicReceiver { rx },
        ))
    }

    /// Open a unidirectional send stream
    pub async fn open_uni(&self) -> Result<QuicSender> {
        let send = self
            .connection
            .open_uni()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Open uni failed: {}", e)))?;

        Ok(QuicSender {
            send: Arc::new(tokio::sync::Mutex::new(send)),
            connected: Arc::new(Mutex::new(true)),
        })
    }

    /// Accept an incoming unidirectional stream
    pub async fn accept_uni(&self) -> Result<QuicReceiver> {
        let recv =
            self.connection.accept_uni().await.map_err(|e| {
                TransportError::ConnectionFailed(format!("Accept uni failed: {}", e))
            })?;

        let (tx, rx) = mpsc::channel(DEFAULT_CHANNEL_BUFFER_SIZE);
        let connected = Arc::new(Mutex::new(true));
        let connected_clone = connected.clone();

        tokio::spawn(async move {
            let mut recv = recv;
            let mut buf = vec![0u8; 65536];

            loop {
                match recv.read(&mut buf).await {
                    Ok(Some(n)) => {
                        let data = Bytes::copy_from_slice(&buf[..n]);
                        if tx.send(TransportEvent::Data(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        *connected_clone.lock() = false;
                        let _ = tx.send(TransportEvent::Disconnected { reason: None }).await;
                        break;
                    }
                    Err(e) => {
                        error!("QUIC read error: {}", e);
                        *connected_clone.lock() = false;
                        let _ = tx
                            .send(TransportEvent::Disconnected {
                                reason: Some(e.to_string()),
                            })
                            .await;
                        break;
                    }
                }
            }
        });

        Ok(QuicReceiver { rx })
    }

    /// Send unreliable datagram (if supported by configuration)
    pub fn send_datagram(&self, data: Bytes) -> Result<()> {
        self.connection
            .send_datagram(data)
            .map_err(|e| TransportError::SendFailed(format!("Datagram send failed: {}", e)))
    }

    /// Receive unreliable datagram
    pub async fn recv_datagram(&self) -> Result<Bytes> {
        self.connection
            .read_datagram()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Datagram recv failed: {}", e)))
    }

    /// Get remote address
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    /// Close the connection
    pub fn close(&self, code: u32, reason: &str) {
        self.connection
            .close(quinn::VarInt::from_u32(code), reason.as_bytes());
    }
}

/// QUIC stream sender
#[cfg(feature = "quic")]
pub struct QuicSender {
    send: Arc<tokio::sync::Mutex<SendStream>>,
    connected: Arc<Mutex<bool>>,
}

#[cfg(feature = "quic")]
#[async_trait]
impl TransportSender for QuicSender {
    async fn send(&self, data: Bytes) -> Result<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected);
        }

        let mut send = self.send.lock().await;
        send.write_all(&data)
            .await
            .map_err(|e| TransportError::SendFailed(format!("QUIC write failed: {}", e)))?;

        debug!("QUIC sent {} bytes", data.len());
        Ok(())
    }

    fn try_send(&self, data: Bytes) -> Result<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected);
        }

        // QUIC doesn't have a channel buffer - spawn a task to send asynchronously
        // This makes the call non-blocking from the caller's perspective
        let send = Arc::clone(&self.send);
        let connected = Arc::clone(&self.connected);
        tokio::spawn(async move {
            let mut stream = send.lock().await;
            if let Err(e) = stream.write_all(&data).await {
                error!("QUIC async send failed: {}", e);
                *connected.lock() = false;
            }
        });

        Ok(())
    }

    fn is_connected(&self) -> bool {
        *self.connected.lock()
    }

    async fn close(&self) -> Result<()> {
        *self.connected.lock() = false;
        let mut send = self.send.lock().await;
        send.finish()
            .map_err(|e| TransportError::SendFailed(format!("Stream finish failed: {}", e)))?;
        Ok(())
    }
}

/// QUIC stream receiver
#[cfg(feature = "quic")]
pub struct QuicReceiver {
    rx: mpsc::Receiver<TransportEvent>,
}

#[cfg(feature = "quic")]
#[async_trait]
impl TransportReceiver for QuicReceiver {
    async fn recv(&mut self) -> Option<TransportEvent> {
        self.rx.recv().await
    }
}

// Stub implementations when QUIC feature is disabled
#[cfg(not(feature = "quic"))]
pub struct QuicTransport;

#[cfg(not(feature = "quic"))]
pub struct QuicConfig;

#[cfg(not(feature = "quic"))]
impl Default for QuicConfig {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(feature = "quic"))]
impl QuicTransport {
    pub fn new_client() -> Result<Self> {
        Err(TransportError::ConnectionFailed(
            "QUIC feature not enabled. Compile with --features quic".into(),
        ))
    }

    pub fn new_server(
        _bind_addr: std::net::SocketAddr,
        _cert_der: Vec<u8>,
        _key_der: Vec<u8>,
    ) -> Result<Self> {
        Err(TransportError::ConnectionFailed(
            "QUIC feature not enabled. Compile with --features quic".into(),
        ))
    }
}

#[cfg(not(feature = "quic"))]
pub struct QuicConnection;

#[cfg(not(feature = "quic"))]
pub struct QuicSender;

#[cfg(not(feature = "quic"))]
pub struct QuicReceiver;
