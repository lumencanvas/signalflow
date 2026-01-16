//! UDP transport implementation

use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::error::{Result, TransportError};
use crate::traits::{TransportEvent, TransportReceiver, TransportSender};

/// UDP configuration
#[derive(Debug, Clone)]
pub struct UdpConfig {
    /// Buffer size for receiving
    pub recv_buffer_size: usize,
    /// Maximum packet size
    pub max_packet_size: usize,
}

impl Default for UdpConfig {
    fn default() -> Self {
        Self {
            recv_buffer_size: 65536,
            max_packet_size: 65507, // Max UDP payload
        }
    }
}

/// UDP transport (connectionless)
pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    config: UdpConfig,
}

impl UdpTransport {
    /// Bind to a local address
    pub async fn bind(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        info!("UDP bound to {}", socket.local_addr().unwrap());

        Ok(Self {
            socket: Arc::new(socket),
            config: UdpConfig::default(),
        })
    }

    /// Bind with config
    pub async fn bind_with_config(addr: &str, config: UdpConfig) -> Result<Self> {
        let socket = UdpSocket::bind(addr)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            socket: Arc::new(socket),
            config,
        })
    }

    /// Get local address
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.socket.local_addr().map_err(TransportError::Io)
    }

    /// Create a sender for a specific remote address
    pub fn sender_to(&self, remote: SocketAddr) -> UdpSender {
        UdpSender {
            socket: self.socket.clone(),
            remote,
            connected: Arc::new(Mutex::new(true)),
        }
    }

    /// Start receiving packets
    pub fn start_receiver(&self) -> UdpReceiver {
        let (tx, rx) = mpsc::channel(100);
        let socket = self.socket.clone();
        let max_size = self.config.max_packet_size;

        tokio::spawn(async move {
            let mut buf = vec![0u8; max_size];

            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, from)) => {
                        debug!("UDP received {} bytes from {}", len, from);
                        let data = Bytes::copy_from_slice(&buf[..len]);
                        if tx.send((TransportEvent::Data(data), from)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("UDP receive error: {}", e);
                        if tx
                            .send((
                                TransportEvent::Error(e.to_string()),
                                SocketAddr::from(([0, 0, 0, 0], 0)),
                            ))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        });

        UdpReceiver { rx }
    }

    /// Send to a specific address
    pub async fn send_to(&self, data: &[u8], target: SocketAddr) -> Result<()> {
        self.socket
            .send_to(data, target)
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        Ok(())
    }

    /// Enable broadcast
    pub fn set_broadcast(&self, enable: bool) -> Result<()> {
        self.socket
            .set_broadcast(enable)
            .map_err(TransportError::Io)
    }
}

/// UDP sender (to a specific remote)
pub struct UdpSender {
    socket: Arc<UdpSocket>,
    remote: SocketAddr,
    connected: Arc<Mutex<bool>>,
}

#[async_trait]
impl TransportSender for UdpSender {
    async fn send(&self, data: Bytes) -> Result<()> {
        self.socket
            .send_to(&data, self.remote)
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        *self.connected.lock()
    }

    async fn close(&self) -> Result<()> {
        *self.connected.lock() = false;
        Ok(())
    }
}

/// UDP receiver
pub struct UdpReceiver {
    rx: mpsc::Receiver<(TransportEvent, SocketAddr)>,
}

impl UdpReceiver {
    /// Receive the next event with source address
    pub async fn recv_from(&mut self) -> Option<(TransportEvent, SocketAddr)> {
        self.rx.recv().await
    }
}

#[async_trait]
impl TransportReceiver for UdpReceiver {
    async fn recv(&mut self) -> Option<TransportEvent> {
        self.rx.recv().await.map(|(event, _)| event)
    }
}

/// UDP broadcast sender for discovery
pub struct UdpBroadcast {
    socket: Arc<UdpSocket>,
    broadcast_addr: SocketAddr,
}

impl UdpBroadcast {
    /// Create a broadcast sender
    pub async fn new(port: u16) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        socket
            .set_broadcast(true)
            .map_err(TransportError::Io)?;

        let broadcast_addr = SocketAddr::from(([255, 255, 255, 255], port));

        Ok(Self {
            socket: Arc::new(socket),
            broadcast_addr,
        })
    }

    /// Send broadcast
    pub async fn broadcast(&self, data: &[u8]) -> Result<()> {
        self.socket
            .send_to(data, self.broadcast_addr)
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_udp_bind() {
        let transport = UdpTransport::bind("127.0.0.1:0").await.unwrap();
        let addr = transport.local_addr().unwrap();
        assert!(addr.port() > 0);
    }

    #[tokio::test]
    async fn test_udp_send_recv() {
        // Bind two sockets
        let server = UdpTransport::bind("127.0.0.1:0").await.unwrap();
        let client = UdpTransport::bind("127.0.0.1:0").await.unwrap();

        let server_addr = server.local_addr().unwrap();
        let mut receiver = server.start_receiver();

        // Send from client to server
        client
            .send_to(b"hello", server_addr)
            .await
            .unwrap();

        // Receive on server
        let (event, from) = receiver.recv_from().await.unwrap();
        match event {
            TransportEvent::Data(data) => {
                assert_eq!(data.as_ref(), b"hello");
            }
            _ => panic!("Expected Data event"),
        }

        assert_eq!(from.port(), client.local_addr().unwrap().port());
    }
}
