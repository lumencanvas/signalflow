//! TCP transport implementation
//!
//! Raw TCP transport for CLASP. Uses length-prefixed framing for message boundaries.
//! Each message is preceded by a 4-byte big-endian length prefix.

use async_trait::async_trait;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{TransportEvent, TransportReceiver, TransportSender, TransportServer};

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

/// Maximum message size (64KB)
const MAX_MESSAGE_SIZE: usize = 64 * 1024;

/// Default channel buffer size for TCP connections
const DEFAULT_CHANNEL_BUFFER_SIZE: usize = 1000;

/// TCP configuration
#[derive(Debug, Clone)]
pub struct TcpConfig {
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Read buffer size
    pub read_buffer_size: usize,
    /// Keep-alive interval in seconds (0 = disabled)
    pub keepalive_secs: u64,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            max_message_size: MAX_MESSAGE_SIZE,
            read_buffer_size: 8192,
            keepalive_secs: 30,
        }
    }
}

/// TCP transport
pub struct TcpTransport {
    config: TcpConfig,
}

impl TcpTransport {
    pub fn new() -> Self {
        Self {
            config: TcpConfig::default(),
        }
    }

    pub fn with_config(config: TcpConfig) -> Self {
        Self { config }
    }

    /// Connect to a TCP server
    pub async fn connect(&self, addr: &str) -> Result<(TcpSender, TcpReceiver)> {
        info!("Connecting to TCP: {}", addr);

        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        // Enable TCP keepalive if configured
        if self.config.keepalive_secs > 0 {
            let socket = socket2::SockRef::from(&stream);
            let keepalive = socket2::TcpKeepalive::new()
                .with_time(std::time::Duration::from_secs(self.config.keepalive_secs));
            let _ = socket.set_tcp_keepalive(&keepalive);
        }

        let connected = Arc::new(Mutex::new(true));
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<Bytes>(DEFAULT_CHANNEL_BUFFER_SIZE);
        let (incoming_tx, incoming_rx) = mpsc::channel::<TransportEvent>(DEFAULT_CHANNEL_BUFFER_SIZE);

        let sender = TcpSender {
            tx: outgoing_tx,
            connected: connected.clone(),
        };

        let receiver = TcpReceiver { rx: incoming_rx };

        let max_size = self.config.max_message_size;
        let connected_clone = connected.clone();

        // Spawn reader/writer task
        tokio::spawn(async move {
            let (reader, writer) = stream.into_split();
            run_tcp_io_loop(
                reader,
                writer,
                outgoing_rx,
                incoming_tx,
                max_size,
                connected_clone,
            )
            .await;
        });

        info!("TCP connected to {}", addr);
        Ok((sender, receiver))
    }
}

impl Default for TcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared IO loop for TCP connections
async fn run_tcp_io_loop(
    mut reader: OwnedReadHalf,
    mut writer: OwnedWriteHalf,
    mut outgoing_rx: mpsc::Receiver<Bytes>,
    incoming_tx: mpsc::Sender<TransportEvent>,
    max_size: usize,
    connected: Arc<Mutex<bool>>,
) {
    let mut read_buf = BytesMut::with_capacity(8192);

    loop {
        tokio::select! {
            Some(data) = outgoing_rx.recv() => {
                let len = data.len() as u32;
                let mut frame = BytesMut::with_capacity(4 + data.len());
                frame.put_u32(len);
                frame.extend_from_slice(&data);

                if let Err(e) = writer.write_all(&frame).await {
                    error!("TCP write error: {}", e);
                    break;
                }
            }

            result = reader.read_buf(&mut read_buf) => {
                match result {
                    Ok(0) => {
                        debug!("TCP connection closed");
                        let _ = incoming_tx.send(TransportEvent::Disconnected { reason: None }).await;
                        break;
                    }
                    Ok(_) => {
                        while read_buf.len() >= 4 {
                            let len = (&read_buf[..4]).get_u32() as usize;

                            if len > max_size {
                                error!("Message too large: {} > {}", len, max_size);
                                let _ = incoming_tx.send(TransportEvent::Disconnected {
                                    reason: Some(format!("Message too large: {}", len))
                                }).await;
                                break;
                            }

                            if read_buf.len() >= 4 + len {
                                read_buf.advance(4);
                                let data = read_buf.split_to(len).freeze();
                                if incoming_tx.send(TransportEvent::Data(data)).await.is_err() {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("TCP read error: {}", e);
                        let _ = incoming_tx.send(TransportEvent::Error(e.to_string())).await;
                        break;
                    }
                }
            }
        }
    }

    *connected.lock() = false;
}

/// TCP sender for writing messages
pub struct TcpSender {
    tx: mpsc::Sender<Bytes>,
    connected: Arc<Mutex<bool>>,
}

#[async_trait]
impl TransportSender for TcpSender {
    async fn send(&self, data: Bytes) -> Result<()> {
        if !*self.connected.lock() {
            return Err(TransportError::NotConnected);
        }

        self.tx
            .send(data)
            .await
            .map_err(|_| TransportError::SendFailed("Channel closed".into()))
    }

    fn try_send(&self, data: Bytes) -> Result<()> {
        if !*self.connected.lock() {
            return Err(TransportError::NotConnected);
        }

        self.tx.try_send(data).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => TransportError::BufferFull,
            mpsc::error::TrySendError::Closed(_) => TransportError::ConnectionClosed,
        })
    }

    fn is_connected(&self) -> bool {
        *self.connected.lock()
    }

    async fn close(&self) -> Result<()> {
        *self.connected.lock() = false;
        Ok(())
    }
}

/// TCP receiver for reading messages
pub struct TcpReceiver {
    rx: mpsc::Receiver<TransportEvent>,
}

#[async_trait]
impl TransportReceiver for TcpReceiver {
    async fn recv(&mut self) -> Option<TransportEvent> {
        self.rx.recv().await
    }
}

/// TCP server for accepting connections
pub struct TcpServer {
    listener: TcpListener,
    config: TcpConfig,
}

impl TcpServer {
    /// Bind to an address and create a new TCP server
    pub async fn bind(addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        info!("TCP server listening on {}", addr);

        Ok(Self {
            listener,
            config: TcpConfig::default(),
        })
    }

    /// Bind with custom configuration
    pub async fn bind_with_config(addr: &str, config: TcpConfig) -> Result<Self> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| TransportError::BindFailed(e.to_string()))?;

        info!("TCP server listening on {}", addr);

        Ok(Self { listener, config })
    }
}

#[async_trait]
impl TransportServer for TcpServer {
    type Sender = TcpSender;
    type Receiver = TcpReceiver;

    async fn accept(&mut self) -> Result<(Self::Sender, Self::Receiver, SocketAddr)> {
        let (stream, peer_addr) = self
            .listener
            .accept()
            .await
            .map_err(|e| TransportError::AcceptFailed(e.to_string()))?;

        info!("TCP connection accepted from {}", peer_addr);

        let connected = Arc::new(Mutex::new(true));
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<Bytes>(DEFAULT_CHANNEL_BUFFER_SIZE);
        let (incoming_tx, incoming_rx) = mpsc::channel::<TransportEvent>(DEFAULT_CHANNEL_BUFFER_SIZE);

        let sender = TcpSender {
            tx: outgoing_tx,
            connected: connected.clone(),
        };

        let receiver = TcpReceiver { rx: incoming_rx };

        let max_size = self.config.max_message_size;
        let connected_clone = connected.clone();

        // Spawn reader/writer task
        let stream: TcpStream = stream; // Ensure type is known
        tokio::spawn(async move {
            let (reader, writer) = stream.into_split();
            run_tcp_io_loop(
                reader,
                writer,
                outgoing_rx,
                incoming_tx,
                max_size,
                connected_clone,
            )
            .await;
        });

        Ok((sender, receiver, peer_addr))
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        self.listener
            .local_addr()
            .map_err(|e| TransportError::Other(e.to_string()))
    }

    async fn close(&self) -> Result<()> {
        // TcpListener doesn't have a close method - it closes when dropped
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_tcp_config_default() {
        let config = TcpConfig::default();
        assert_eq!(config.max_message_size, 64 * 1024);
        assert_eq!(config.read_buffer_size, 8192);
        assert_eq!(config.keepalive_secs, 30);
    }

    #[tokio::test]
    async fn test_tcp_transport_creation() {
        let transport = TcpTransport::new();
        assert_eq!(transport.config.max_message_size, 64 * 1024);
    }

    #[tokio::test]
    async fn test_tcp_client_server_connection() {
        // Bind server
        let mut server = TcpServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();

        // Spawn accept task
        let accept_handle = tokio::spawn(async move {
            let (sender, mut receiver, peer) = server.accept().await.unwrap();
            info!("Server accepted connection from {}", peer);

            // Wait for data
            if let Some(TransportEvent::Data(data)) = receiver.recv().await {
                // Echo back
                sender.send(data).await.unwrap();
            }

            (sender, receiver)
        });

        // Give server time to start
        sleep(Duration::from_millis(50)).await;

        // Connect client
        let transport = TcpTransport::new();
        let (client_sender, mut client_receiver) =
            transport.connect(&addr.to_string()).await.unwrap();

        // Send data
        let test_data = Bytes::from("hello tcp");
        client_sender.send(test_data.clone()).await.unwrap();

        // Receive echo
        if let Some(TransportEvent::Data(received)) = client_receiver.recv().await {
            assert_eq!(received, test_data);
        } else {
            panic!("Expected Data event");
        }

        // Clean up
        client_sender.close().await.unwrap();
        let _ = accept_handle.await;
    }
}
