//! Serial transport implementation
//!
//! This module provides serial port transport for CLASP.
//! Ideal for direct hardware integration with lowest latency.
//!
//! Typical uses:
//! - DMX controllers over USB serial
//! - Direct microcontroller communication
//! - Arduino/ESP32 CLASP bridges

use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::error::{Result, TransportError};
use crate::traits::{TransportEvent, TransportReceiver, TransportSender};

/// Serial transport configuration
#[derive(Debug, Clone)]
pub struct SerialConfig {
    /// Baud rate (default: 115200)
    pub baud_rate: u32,
    /// Data bits (default: 8)
    pub data_bits: u8,
    /// Stop bits (default: 1)
    pub stop_bits: u8,
    /// Parity (default: none)
    pub parity: SerialParity,
    /// Flow control (default: none)
    pub flow_control: SerialFlowControl,
}

/// Serial parity options
#[derive(Debug, Clone, Copy, Default)]
pub enum SerialParity {
    #[default]
    None,
    Odd,
    Even,
}

/// Serial flow control options
#[derive(Debug, Clone, Copy, Default)]
pub enum SerialFlowControl {
    #[default]
    None,
    Hardware,
    Software,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            parity: SerialParity::None,
            flow_control: SerialFlowControl::None,
        }
    }
}

/// Serial transport for CLASP
#[cfg(feature = "serial")]
pub struct SerialTransport {
    config: SerialConfig,
    port_name: String,
}

#[cfg(feature = "serial")]
impl SerialTransport {
    /// List available serial ports
    pub fn list_ports() -> Result<Vec<String>> {
        use tokio_serial::available_ports;
        let ports = available_ports().map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to list ports: {}", e))
        })?;
        Ok(ports.into_iter().map(|p| p.port_name).collect())
    }

    /// Connect to a serial port
    pub async fn connect(port_name: &str) -> Result<(SerialSender, SerialReceiver)> {
        Self::connect_with_config(port_name, SerialConfig::default()).await
    }

    /// Connect with custom config
    pub async fn connect_with_config(
        port_name: &str,
        config: SerialConfig,
    ) -> Result<(SerialSender, SerialReceiver)> {
        use tokio_serial::{SerialPortBuilderExt, SerialStream};

        let port = tokio_serial::new(port_name, config.baud_rate)
            .open_native_async()
            .map_err(|e| TransportError::ConnectionFailed(format!("Failed to open port: {}", e)))?;

        info!(
            "Serial port opened: {} @ {} baud",
            port_name, config.baud_rate
        );

        let port = Arc::new(tokio::sync::Mutex::new(port));
        let (tx, rx) = mpsc::channel(100);
        let connected = Arc::new(Mutex::new(true));
        let connected_clone = connected.clone();
        let port_recv = port.clone();

        // Spawn receiver task
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut buf = vec![0u8; 1024];

            loop {
                let mut port = port_recv.lock().await;
                match port.read(&mut buf).await {
                    Ok(0) => {
                        *connected_clone.lock() = false;
                        let _ = tx.send(TransportEvent::Disconnected { reason: None }).await;
                        break;
                    }
                    Ok(n) => {
                        let data = Bytes::copy_from_slice(&buf[..n]);
                        if tx.send(TransportEvent::Data(data)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Serial read error: {}", e);
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

        let sender = SerialSender {
            port,
            connected: connected.clone(),
        };

        let receiver = SerialReceiver { rx };

        Ok((sender, receiver))
    }
}

/// Serial sender
#[cfg(feature = "serial")]
pub struct SerialSender {
    port: Arc<tokio::sync::Mutex<tokio_serial::SerialStream>>,
    connected: Arc<Mutex<bool>>,
}

#[cfg(feature = "serial")]
#[async_trait]
impl TransportSender for SerialSender {
    async fn send(&self, data: Bytes) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        if !self.is_connected() {
            return Err(TransportError::NotConnected);
        }

        let mut port = self.port.lock().await;
        port.write_all(&data)
            .await
            .map_err(|e| TransportError::SendFailed(format!("Serial write failed: {}", e)))?;

        debug!("Serial sent {} bytes", data.len());
        Ok(())
    }

    fn try_send(&self, data: Bytes) -> Result<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected);
        }

        // Spawn a task to send asynchronously
        let port = Arc::clone(&self.port);
        let connected = Arc::clone(&self.connected);
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let mut port = port.lock().await;
            if let Err(e) = port.write_all(&data).await {
                error!("Serial async send failed: {}", e);
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
        Ok(())
    }
}

/// Serial receiver
#[cfg(feature = "serial")]
pub struct SerialReceiver {
    rx: mpsc::Receiver<TransportEvent>,
}

#[cfg(feature = "serial")]
#[async_trait]
impl TransportReceiver for SerialReceiver {
    async fn recv(&mut self) -> Option<TransportEvent> {
        self.rx.recv().await
    }
}

// Stub implementations when serial feature is disabled
#[cfg(not(feature = "serial"))]
pub struct SerialTransport;

#[cfg(not(feature = "serial"))]
impl SerialTransport {
    pub fn list_ports() -> Result<Vec<String>> {
        Err(TransportError::ConnectionFailed(
            "Serial feature not enabled. Compile with --features serial".into(),
        ))
    }

    pub async fn connect(_port_name: &str) -> Result<(SerialSender, SerialReceiver)> {
        Err(TransportError::ConnectionFailed(
            "Serial feature not enabled. Compile with --features serial".into(),
        ))
    }
}

#[cfg(not(feature = "serial"))]
pub struct SerialSender;

#[cfg(not(feature = "serial"))]
pub struct SerialReceiver;
