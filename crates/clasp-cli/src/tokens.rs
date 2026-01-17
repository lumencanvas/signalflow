//! Token management for CLASP CLI
//!
//! Provides file-based token storage and management for CPSK tokens.

use anyhow::{Context, Result};
use clasp_core::security::{parse_duration, parse_scopes, CpskValidator, Scope, TokenInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A stored token record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRecord {
    /// The token string (e.g., cpsk_xxx...)
    pub token: String,
    /// Token subject/description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Scope strings (e.g., ["read:/**", "write:/lights/**"])
    pub scopes: Vec<String>,
    /// Unix timestamp when token expires (None = never)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// Unix timestamp when token was created
    pub created_at: u64,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl TokenRecord {
    /// Create a new token record
    pub fn new(token: String, scopes: Vec<String>) -> Self {
        Self {
            token,
            subject: None,
            scopes,
            expires_at: None,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        }
    }

    /// Set the subject
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set expiration as Unix timestamp
    pub fn with_expires_at(mut self, expires_at: u64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set expiration from duration
    pub fn with_expires_in(mut self, duration: Duration) -> Self {
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + duration.as_secs();
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now > expires_at
        } else {
            false
        }
    }

    /// Convert to TokenInfo for use with CpskValidator
    pub fn to_token_info(&self) -> Result<TokenInfo> {
        let scopes: Vec<Scope> = self
            .scopes
            .iter()
            .map(|s| Scope::parse(s))
            .collect::<clasp_core::Result<Vec<_>>>()
            .context("Failed to parse scopes")?;

        let mut info = TokenInfo::new(self.token.clone(), scopes);

        if let Some(ref subject) = self.subject {
            info = info.with_subject(subject.clone());
        }

        if let Some(expires_at) = self.expires_at {
            info = info.with_expires_at(UNIX_EPOCH + Duration::from_secs(expires_at));
        }

        Ok(info)
    }
}

/// File-based token store
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenStore {
    /// Stored tokens by token string
    tokens: HashMap<String, TokenRecord>,
}

impl TokenStore {
    /// Create a new empty token store
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Load token store from a JSON file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read token file: {}", path.display()))?;

        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse token file: {}", path.display()))
    }

    /// Save token store to a JSON file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize token store")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write token file: {}", path.display()))
    }

    /// Add a token to the store
    pub fn add(&mut self, record: TokenRecord) {
        self.tokens.insert(record.token.clone(), record);
    }

    /// Remove a token from the store
    pub fn remove(&mut self, token: &str) -> Option<TokenRecord> {
        self.tokens.remove(token)
    }

    /// Get a token by its string
    pub fn get(&self, token: &str) -> Option<&TokenRecord> {
        self.tokens.get(token)
    }

    /// List all tokens
    pub fn list(&self) -> impl Iterator<Item = &TokenRecord> {
        self.tokens.values()
    }

    /// Get the number of tokens
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Create a CpskValidator populated with all valid (non-expired) tokens
    pub fn to_validator(&self) -> Result<CpskValidator> {
        let validator = CpskValidator::new();

        for record in self.tokens.values() {
            if !record.is_expired() {
                let info = record.to_token_info()?;
                validator.register(record.token.clone(), info);
            }
        }

        Ok(validator)
    }

    /// Remove all expired tokens
    pub fn prune_expired(&mut self) -> usize {
        let expired: Vec<String> = self
            .tokens
            .iter()
            .filter(|(_, r)| r.is_expired())
            .map(|(k, _)| k.clone())
            .collect();

        let count = expired.len();
        for token in expired {
            self.tokens.remove(&token);
        }
        count
    }
}

/// Create a new token with the given parameters
pub fn create_token(
    scopes_str: &str,
    expires_str: Option<&str>,
    subject: Option<&str>,
) -> Result<TokenRecord> {
    // Parse and validate scopes
    let scopes = parse_scopes(scopes_str).context("Failed to parse scopes")?;
    let scope_strings: Vec<String> = scopes.iter().map(|s| s.as_str().to_string()).collect();

    // Generate token
    let token = CpskValidator::generate_token();

    // Create record
    let mut record = TokenRecord::new(token, scope_strings);

    // Set expiration if provided
    if let Some(exp_str) = expires_str {
        let duration = parse_duration(exp_str).context("Failed to parse expiration")?;
        record = record.with_expires_in(duration);
    }

    // Set subject if provided
    if let Some(subj) = subject {
        record = record.with_subject(subj);
    }

    Ok(record)
}

/// Format a Unix timestamp as a human-readable string
pub fn format_timestamp(ts: u64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let time = UNIX_EPOCH + Duration::from_secs(ts);
    let now = SystemTime::now();

    match time.duration_since(now) {
        Ok(remaining) => {
            let secs = remaining.as_secs();
            if secs < 60 {
                format!("in {} seconds", secs)
            } else if secs < 3600 {
                format!("in {} minutes", secs / 60)
            } else if secs < 86400 {
                format!("in {} hours", secs / 3600)
            } else {
                format!("in {} days", secs / 86400)
            }
        }
        Err(_) => "expired".to_string(),
    }
}

/// Get the default token file path
pub fn default_token_file() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("clasp")
        .join("tokens.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_token_record_creation() {
        let record = TokenRecord::new(
            "cpsk_test".to_string(),
            vec!["read:/**".to_string()],
        );
        assert_eq!(record.token, "cpsk_test");
        assert!(!record.is_expired());
    }

    #[test]
    fn test_token_record_expiry() {
        let mut record = TokenRecord::new(
            "cpsk_test".to_string(),
            vec!["read:/**".to_string()],
        );

        // Set expiration to the past
        record.expires_at = Some(0);
        assert!(record.is_expired());

        // Set expiration to the future
        record = record.with_expires_in(Duration::from_secs(3600));
        assert!(!record.is_expired());
    }

    #[test]
    fn test_token_store_operations() {
        let mut store = TokenStore::new();
        assert!(store.is_empty());

        let record = TokenRecord::new(
            "cpsk_test".to_string(),
            vec!["read:/**".to_string()],
        );
        store.add(record);

        assert_eq!(store.len(), 1);
        assert!(store.get("cpsk_test").is_some());

        store.remove("cpsk_test");
        assert!(store.is_empty());
    }

    #[test]
    fn test_create_token() {
        let record = create_token("read:/**, write:/lights/**", Some("7d"), Some("test-device"))
            .expect("Failed to create token");

        assert!(record.token.starts_with("cpsk_"));
        assert_eq!(record.scopes.len(), 2);
        assert_eq!(record.subject, Some("test-device".to_string()));
        assert!(record.expires_at.is_some());
    }
}
