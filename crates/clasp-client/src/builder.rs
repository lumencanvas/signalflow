//! Client builder pattern

use crate::{Clasp, Result};

/// Builder for Clasp client
pub struct ClaspBuilder {
    url: String,
    name: String,
    features: Vec<String>,
    token: Option<String>,
    reconnect: bool,
    reconnect_interval_ms: u64,
}

impl ClaspBuilder {
    /// Create a new builder
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            name: "Clasp Client".to_string(),
            features: vec![
                "param".to_string(),
                "event".to_string(),
                "stream".to_string(),
            ],
            token: None,
            reconnect: true,
            reconnect_interval_ms: 5000,
        }
    }

    /// Set client name
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set supported features
    pub fn features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    /// Set authentication token
    pub fn token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    /// Enable/disable auto-reconnect
    pub fn reconnect(mut self, enabled: bool) -> Self {
        self.reconnect = enabled;
        self
    }

    /// Set reconnect interval in milliseconds
    pub fn reconnect_interval(mut self, ms: u64) -> Self {
        self.reconnect_interval_ms = ms;
        self
    }

    /// Build and connect
    pub async fn connect(self) -> Result<Clasp> {
        let mut client = Clasp::new(
            &self.url,
            self.name,
            self.features,
            self.token,
            self.reconnect,
            self.reconnect_interval_ms,
        );

        client.do_connect().await?;
        Ok(client)
    }
}
