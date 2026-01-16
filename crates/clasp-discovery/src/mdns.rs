//! mDNS/Bonjour discovery

use crate::{Device, DeviceInfo, DiscoveryError, DiscoveryEvent, Result};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// mDNS service type for SignalFlow
const SERVICE_TYPE: &str = "_clasp._tcp.local.";

/// Discover SignalFlow devices via mDNS
pub async fn discover(tx: mpsc::Sender<DiscoveryEvent>) -> Result<()> {
    // Create mDNS daemon
    let mdns = ServiceDaemon::new().map_err(|e| DiscoveryError::Mdns(e.to_string()))?;

    // Browse for SignalFlow services
    let receiver = mdns
        .browse(SERVICE_TYPE)
        .map_err(|e| DiscoveryError::Mdns(e.to_string()))?;

    info!("Starting mDNS discovery for {}", SERVICE_TYPE);

    // Process discovery events
    loop {
        match receiver.recv() {
            Ok(event) => match event {
                ServiceEvent::ServiceResolved(info) => {
                    debug!("mDNS resolved: {:?}", info);

                    // Extract device info from TXT records
                    let mut device = Device::new(
                        info.get_fullname().to_string(),
                        info.get_hostname().trim_end_matches('.').to_string(),
                    );

                    // Parse TXT records
                    let properties = info.get_properties();
                    let mut features = Vec::new();

                    if let Some(name) = properties.get("name") {
                        if let Some(val) = name.val() {
                            device.name = String::from_utf8_lossy(val).to_string();
                        }
                    }

                    if let Some(feat) = properties.get("features") {
                        if let Some(val) = feat.val() {
                            let feat_str = String::from_utf8_lossy(val);
                            // Parse feature string (e.g., "psetg" -> ["param", "stream", "event", "timeline", "gesture"])
                            for c in feat_str.chars() {
                                match c {
                                    'p' => features.push("param".to_string()),
                                    's' => features.push("stream".to_string()),
                                    'e' => features.push("event".to_string()),
                                    't' => features.push("timeline".to_string()),
                                    'g' => features.push("gesture".to_string()),
                                    _ => {}
                                }
                            }
                        }
                    }

                    // Get WebSocket port
                    let ws_port = properties
                        .get("ws")
                        .and_then(|v| v.val())
                        .and_then(|val| String::from_utf8_lossy(val).parse().ok())
                        .unwrap_or(clasp_core::DEFAULT_WS_PORT);

                    // Build WebSocket URL
                    if let Some(addr) = info.get_addresses().iter().next() {
                        let ws_url = format!("ws://{}:{}/clasp", addr, ws_port);
                        device = device.with_ws_endpoint(&ws_url);
                    }

                    device.info = DeviceInfo::default().with_features(features);

                    info!("Discovered device: {} at {:?}", device.name, device.endpoints);

                    if tx.send(DiscoveryEvent::Found(device)).await.is_err() {
                        break;
                    }
                }
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    info!("Device lost: {}", fullname);
                    if tx.send(DiscoveryEvent::Lost(fullname)).await.is_err() {
                        break;
                    }
                }
                ServiceEvent::SearchStarted(_) => {
                    debug!("mDNS search started");
                }
                ServiceEvent::SearchStopped(_) => {
                    debug!("mDNS search stopped");
                    break;
                }
                _ => {}
            },
            Err(e) => {
                warn!("mDNS receive error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Advertise a SignalFlow service via mDNS
pub struct ServiceAdvertiser {
    mdns: ServiceDaemon,
    fullname: Option<String>,
}

impl ServiceAdvertiser {
    /// Create a new service advertiser
    pub fn new() -> Result<Self> {
        let mdns = ServiceDaemon::new().map_err(|e| DiscoveryError::Mdns(e.to_string()))?;
        Ok(Self {
            mdns,
            fullname: None,
        })
    }

    /// Advertise a SignalFlow service
    pub fn advertise(
        &mut self,
        name: &str,
        port: u16,
        features: &[&str],
    ) -> Result<()> {
        use mdns_sd::ServiceInfo;

        // Build feature string
        let feat_str: String = features
            .iter()
            .filter_map(|f| match *f {
                "param" => Some('p'),
                "stream" => Some('s'),
                "event" => Some('e'),
                "timeline" => Some('t'),
                "gesture" => Some('g'),
                _ => None,
            })
            .collect();

        // Create service info
        let port_str = port.to_string();
        let properties: &[(&str, &str)] = &[
            ("version", "2"),
            ("name", name),
            ("features", &feat_str),
            ("ws", &port_str),
        ];
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            name,
            &format!("{}.local.", hostname::get().unwrap().to_string_lossy()),
            "",
            port,
            properties,
        )
        .map_err(|e| DiscoveryError::Mdns(e.to_string()))?;

        self.fullname = Some(service_info.get_fullname().to_string());

        self.mdns
            .register(service_info)
            .map_err(|e| DiscoveryError::Mdns(e.to_string()))?;

        info!("Advertising SignalFlow service: {} on port {}", name, port);

        Ok(())
    }

    /// Stop advertising
    pub fn stop(&mut self) -> Result<()> {
        if let Some(fullname) = self.fullname.take() {
            self.mdns
                .unregister(&fullname)
                .map_err(|e| DiscoveryError::Mdns(e.to_string()))?;
        }
        Ok(())
    }
}

impl Default for ServiceAdvertiser {
    fn default() -> Self {
        Self::new().expect("Failed to create mDNS daemon")
    }
}

impl Drop for ServiceAdvertiser {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
