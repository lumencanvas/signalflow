//! Bluetooth Low Energy (BLE) transport implementation
//!
//! This module provides BLE transport for CLASP using GATT services.
//! Designed for wireless controllers and battery-powered devices.
//!
//! CLASP BLE Service:
//! - Service UUID: 0x7330 (CLASP port as short UUID)
//! - TX Characteristic: For sending CLASP frames (Write/WriteWithoutResponse)
//! - RX Characteristic: For receiving CLASP frames (Notify)

use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{TransportEvent, TransportReceiver, TransportSender};

#[cfg(feature = "ble")]
use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
#[cfg(feature = "ble")]
use btleplug::platform::{Adapter, Manager, Peripheral};
#[cfg(feature = "ble")]
use uuid::Uuid;

/// CLASP BLE Service UUID (based on port 7330)
pub const CLASP_SERVICE_UUID: Uuid = Uuid::from_u128(0x00007330_0000_1000_8000_00805f9b34fb);

/// CLASP TX Characteristic UUID (for sending to peripheral)
pub const CLASP_TX_CHAR_UUID: Uuid = Uuid::from_u128(0x00007331_0000_1000_8000_00805f9b34fb);

/// CLASP RX Characteristic UUID (for receiving from peripheral, via notifications)
pub const CLASP_RX_CHAR_UUID: Uuid = Uuid::from_u128(0x00007332_0000_1000_8000_00805f9b34fb);

/// BLE transport configuration
#[derive(Debug, Clone)]
pub struct BleConfig {
    /// Device name filter for scanning
    pub device_name_filter: Option<String>,
    /// Scan duration in seconds
    pub scan_duration_secs: u64,
    /// MTU size (default: 512 for BLE 5.0)
    pub mtu: usize,
    /// Use WriteWithoutResponse for lower latency
    pub write_without_response: bool,
}

impl Default for BleConfig {
    fn default() -> Self {
        Self {
            device_name_filter: None,
            scan_duration_secs: 5,
            mtu: 512,
            write_without_response: true, // Lower latency for real-time control
        }
    }
}

/// BLE transport for CLASP
#[cfg(feature = "ble")]
pub struct BleTransport {
    config: BleConfig,
    adapter: Adapter,
}

#[cfg(feature = "ble")]
impl BleTransport {
    /// Create a new BLE transport
    pub async fn new() -> Result<Self> {
        Self::with_config(BleConfig::default()).await
    }

    /// Create with custom config
    pub async fn with_config(config: BleConfig) -> Result<Self> {
        let manager = Manager::new()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("BLE manager error: {}", e)))?;

        let adapters = manager
            .adapters()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("No BLE adapters: {}", e)))?;

        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| TransportError::ConnectionFailed("No BLE adapter found".into()))?;

        info!("BLE adapter initialized");

        Ok(Self { config, adapter })
    }

    /// Scan for CLASP-compatible BLE devices
    pub async fn scan(&self) -> Result<Vec<BleDevice>> {
        info!(
            "Starting BLE scan for {} seconds",
            self.config.scan_duration_secs
        );

        self.adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Scan failed: {}", e)))?;

        tokio::time::sleep(tokio::time::Duration::from_secs(
            self.config.scan_duration_secs,
        ))
        .await;

        self.adapter
            .stop_scan()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Stop scan failed: {}", e)))?;

        let peripherals = self.adapter.peripherals().await.map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to get peripherals: {}", e))
        })?;

        let mut devices = Vec::new();

        for peripheral in peripherals {
            if let Ok(Some(props)) = peripheral.properties().await {
                let name = props.local_name.clone();

                // Filter by name if configured
                if let Some(ref filter) = self.config.device_name_filter {
                    if let Some(ref n) = name {
                        if !n.contains(filter) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                // Check if device advertises CLASP service
                let has_clasp_service = props
                    .services
                    .iter()
                    .any(|uuid| *uuid == CLASP_SERVICE_UUID);

                devices.push(BleDevice {
                    name,
                    address: props.address.to_string(),
                    rssi: props.rssi,
                    has_clasp_service,
                    peripheral,
                });
            }
        }

        info!("Found {} BLE devices", devices.len());
        Ok(devices)
    }

    /// Connect to a specific BLE device
    pub async fn connect(&self, device: &BleDevice) -> Result<(BleSender, BleReceiver)> {
        info!("Connecting to BLE device: {:?}", device.name);

        device
            .peripheral
            .connect()
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Connect failed: {}", e)))?;

        device.peripheral.discover_services().await.map_err(|e| {
            TransportError::ConnectionFailed(format!("Service discovery failed: {}", e))
        })?;

        // Find CLASP characteristics
        let chars = device.peripheral.characteristics();

        let tx_char = chars
            .iter()
            .find(|c| c.uuid == CLASP_TX_CHAR_UUID)
            .cloned()
            .ok_or_else(|| {
                TransportError::ConnectionFailed("TX characteristic not found".into())
            })?;

        let rx_char = chars
            .iter()
            .find(|c| c.uuid == CLASP_RX_CHAR_UUID)
            .cloned()
            .ok_or_else(|| {
                TransportError::ConnectionFailed("RX characteristic not found".into())
            })?;

        // Subscribe to notifications on RX characteristic
        device
            .peripheral
            .subscribe(&rx_char)
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("Subscribe failed: {}", e)))?;

        let (tx, rx) = mpsc::channel(100);
        let peripheral = device.peripheral.clone();
        let connected = Arc::new(Mutex::new(true));
        let connected_clone = connected.clone();

        // Spawn notification receiver
        tokio::spawn(async move {
            let mut notifications = match peripheral.notifications().await {
                Ok(n) => n,
                Err(e) => {
                    error!("Failed to get notifications stream: {}", e);
                    return;
                }
            };

            use futures::StreamExt;
            while let Some(data) = notifications.next().await {
                if data.uuid == CLASP_RX_CHAR_UUID {
                    let bytes = Bytes::copy_from_slice(&data.value);
                    if tx.send(TransportEvent::Data(bytes)).await.is_err() {
                        break;
                    }
                }
            }

            *connected_clone.lock() = false;
            let _ = tx.send(TransportEvent::Disconnected { reason: None }).await;
        });

        let sender = BleSender {
            peripheral: device.peripheral.clone(),
            tx_char,
            connected: connected.clone(),
            write_type: if self.config.write_without_response {
                WriteType::WithoutResponse
            } else {
                WriteType::WithResponse
            },
        };

        let receiver = BleReceiver { rx };

        info!("BLE connected to {:?}", device.name);
        Ok((sender, receiver))
    }
}

/// Discovered BLE device
#[cfg(feature = "ble")]
pub struct BleDevice {
    /// Device name (if advertised)
    pub name: Option<String>,
    /// Device address
    pub address: String,
    /// Signal strength
    pub rssi: Option<i16>,
    /// Whether device advertises CLASP service
    pub has_clasp_service: bool,
    /// Internal peripheral handle
    peripheral: Peripheral,
}

/// BLE sender
#[cfg(feature = "ble")]
pub struct BleSender {
    peripheral: Peripheral,
    tx_char: Characteristic,
    connected: Arc<Mutex<bool>>,
    write_type: WriteType,
}

#[cfg(feature = "ble")]
#[async_trait]
impl TransportSender for BleSender {
    async fn send(&self, data: Bytes) -> Result<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected);
        }

        self.peripheral
            .write(&self.tx_char, &data, self.write_type)
            .await
            .map_err(|e| TransportError::SendFailed(format!("BLE write failed: {}", e)))?;

        debug!("BLE sent {} bytes", data.len());
        Ok(())
    }

    fn try_send(&self, data: Bytes) -> Result<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected);
        }

        // BLE doesn't have a sync write, spawn a task for async send
        let peripheral = self.peripheral.clone();
        let tx_char = self.tx_char.clone();
        let write_type = self.write_type;
        let connected = Arc::clone(&self.connected);
        tokio::spawn(async move {
            if let Err(e) = peripheral.write(&tx_char, &data, write_type).await {
                error!("BLE async send failed: {}", e);
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
        self.peripheral
            .disconnect()
            .await
            .map_err(|e| TransportError::SendFailed(format!("Disconnect failed: {}", e)))?;
        Ok(())
    }
}

/// BLE receiver
#[cfg(feature = "ble")]
pub struct BleReceiver {
    rx: mpsc::Receiver<TransportEvent>,
}

#[cfg(feature = "ble")]
#[async_trait]
impl TransportReceiver for BleReceiver {
    async fn recv(&mut self) -> Option<TransportEvent> {
        self.rx.recv().await
    }
}

// Stub implementations when BLE feature is disabled
#[cfg(not(feature = "ble"))]
pub struct BleTransport;

#[cfg(not(feature = "ble"))]
pub struct BleConfig;

#[cfg(not(feature = "ble"))]
impl Default for BleConfig {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(feature = "ble"))]
impl BleTransport {
    pub async fn new() -> Result<Self> {
        Err(TransportError::ConnectionFailed(
            "BLE feature not enabled. Compile with --features ble".into(),
        ))
    }

    pub async fn with_config(_config: BleConfig) -> Result<Self> {
        Err(TransportError::ConnectionFailed(
            "BLE feature not enabled. Compile with --features ble".into(),
        ))
    }
}
