//! Security primitives for CLASP authentication and authorization
//!
//! This module provides a hybrid token system that works across all platforms,
//! including embedded devices with limited resources.
//!
//! # Token Types
//!
//! ## Capability Pre-Shared Keys (CPSK) - Default
//! ```text
//! Format: cpsk_<base62-random-32-chars>
//! Example: cpsk_7kX9mP2nQ4rT6vW8xZ0aB3cD5eF1gH
//! ```
//! Simple lookup-based validation, works on any device.
//!
//! ## External Tokens (PASETO/JWT) - Optional
//! ```text
//! Format: ext_<paseto-or-jwt-token>
//! ```
//! Cryptographic validation for federated identity providers.
//!
//! # Scope Format
//! ```text
//! action:pattern
//!
//! Actions:
//!   read   - SUBSCRIBE, GET
//!   write  - SET, PUBLISH
//!   admin  - Full access
//!
//! Patterns:
//!   /path/to/addr   - Exact match
//!   /path/*         - Single segment wildcard
//!   /path/**        - Multi-segment wildcard
//!
//! Examples:
//!   read:/**                 - Read everything
//!   write:/lights/**         - Control lights namespace
//!   admin:/**                - Full access
//! ```

use crate::address::Pattern;
use crate::{Error, Result};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Actions that can be performed on addresses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Read operations: SUBSCRIBE, GET
    Read,
    /// Write operations: SET, PUBLISH
    Write,
    /// Full access: all operations
    Admin,
}

impl Action {
    /// Check if this action allows the given operation
    pub fn allows(&self, other: Action) -> bool {
        match self {
            Action::Admin => true, // Admin allows everything
            Action::Write => matches!(other, Action::Write | Action::Read),
            Action::Read => matches!(other, Action::Read),
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Read => write!(f, "read"),
            Action::Write => write!(f, "write"),
            Action::Admin => write!(f, "admin"),
        }
    }
}

impl FromStr for Action {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "read" | "r" => Ok(Action::Read),
            "write" | "w" => Ok(Action::Write),
            "admin" | "a" | "*" => Ok(Action::Admin),
            _ => Err(Error::InvalidPattern(format!("unknown action: {}", s))),
        }
    }
}

/// A scope defines what actions are allowed on which address patterns
#[derive(Debug, Clone)]
pub struct Scope {
    action: Action,
    pattern: Pattern,
    raw: String,
}

impl Scope {
    /// Create a new scope from an action and pattern string
    pub fn new(action: Action, pattern_str: &str) -> Result<Self> {
        let pattern = Pattern::compile(pattern_str)?;
        Ok(Self {
            action,
            pattern,
            raw: format!("{}:{}", action, pattern_str),
        })
    }

    /// Parse a scope from string format "action:pattern"
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(Error::InvalidPattern(format!(
                "scope must be in format 'action:pattern', got: {}",
                s
            )));
        }

        let action = Action::from_str(parts[0])?;
        let pattern = Pattern::compile(parts[1])?;

        Ok(Self {
            action,
            pattern,
            raw: s.to_string(),
        })
    }

    /// Check if this scope allows the given action on the given address
    pub fn allows(&self, action: Action, address: &str) -> bool {
        self.action.allows(action) && self.pattern.matches(address)
    }

    /// Get the action for this scope
    pub fn action(&self) -> Action {
        self.action
    }

    /// Get the pattern for this scope
    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    /// Get the raw scope string
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl FromStr for Scope {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Scope::parse(s)
    }
}

/// Information about a validated token
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// Token identifier (the token itself for CPSK, or extracted ID for JWT/PASETO)
    pub token_id: String,
    /// Subject identifier (user, device, or service)
    pub subject: Option<String>,
    /// Scopes granted by this token
    pub scopes: Vec<Scope>,
    /// When the token expires (if any)
    pub expires_at: Option<SystemTime>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl TokenInfo {
    /// Create a new TokenInfo with minimal fields
    pub fn new(token_id: String, scopes: Vec<Scope>) -> Self {
        Self {
            token_id,
            subject: None,
            scopes,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if this token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// Check if the token allows the given action on the given address
    pub fn has_scope(&self, action: Action, address: &str) -> bool {
        self.scopes.iter().any(|scope| scope.allows(action, address))
    }

    /// Set the subject
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set the expiration time
    pub fn with_expires_at(mut self, expires_at: SystemTime) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set the expiration from a duration
    pub fn with_expires_in(mut self, duration: Duration) -> Self {
        self.expires_at = Some(SystemTime::now() + duration);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Result of token validation
#[derive(Debug)]
pub enum ValidationResult {
    /// Token is valid
    Valid(TokenInfo),
    /// Token format not recognized by this validator
    NotMyToken,
    /// Token is invalid (wrong signature, malformed, etc.)
    Invalid(String),
    /// Token has expired
    Expired,
}

/// Trait for token validators
pub trait TokenValidator: Send + Sync + std::any::Any {
    /// Validate a token and return token information if valid
    fn validate(&self, token: &str) -> ValidationResult;

    /// Get the name of this validator (for logging)
    fn name(&self) -> &str;

    /// Returns self as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Capability Pre-Shared Key (CPSK) validator
///
/// Stores tokens in memory with their associated scopes.
/// Tokens have the format: `cpsk_<base62-random-32-chars>`
pub struct CpskValidator {
    tokens: RwLock<HashMap<String, TokenInfo>>,
}

impl CpskValidator {
    /// Token prefix for CPSK tokens
    pub const PREFIX: &'static str = "cpsk_";

    /// Create a new empty CPSK validator
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
        }
    }

    /// Register a token with the given scopes
    pub fn register(&self, token: String, info: TokenInfo) {
        self.tokens.write().unwrap().insert(token, info);
    }

    /// Revoke a token
    pub fn revoke(&self, token: &str) -> bool {
        self.tokens.write().unwrap().remove(token).is_some()
    }

    /// Check if a token exists (without full validation)
    pub fn exists(&self, token: &str) -> bool {
        self.tokens.read().unwrap().contains_key(token)
    }

    /// Get the number of registered tokens
    pub fn len(&self) -> usize {
        self.tokens.read().unwrap().len()
    }

    /// Check if the validator has no tokens
    pub fn is_empty(&self) -> bool {
        self.tokens.read().unwrap().is_empty()
    }

    /// List all token IDs (for admin purposes)
    pub fn list_tokens(&self) -> Vec<String> {
        self.tokens.read().unwrap().keys().cloned().collect()
    }

    /// Generate a new CPSK token string
    pub fn generate_token() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Use time-based seed for randomness
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        // Simple LCG-based random generator
        let mut state = seed as u64;
        let mut chars = String::with_capacity(32);
        const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

        for _ in 0..32 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let idx = ((state >> 33) as usize) % ALPHABET.len();
            chars.push(ALPHABET[idx] as char);
        }

        format!("{}{}", Self::PREFIX, chars)
    }
}

impl Default for CpskValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenValidator for CpskValidator {
    fn validate(&self, token: &str) -> ValidationResult {
        // Check prefix
        if !token.starts_with(Self::PREFIX) {
            return ValidationResult::NotMyToken;
        }

        // Look up token
        let tokens = self.tokens.read().unwrap();
        match tokens.get(token) {
            Some(info) => {
                if info.is_expired() {
                    ValidationResult::Expired
                } else {
                    ValidationResult::Valid(info.clone())
                }
            }
            None => ValidationResult::Invalid("token not found".to_string()),
        }
    }

    fn name(&self) -> &str {
        "CPSK"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// A chain of validators that tries each one in order
pub struct ValidatorChain {
    validators: Vec<Box<dyn TokenValidator>>,
}

impl ValidatorChain {
    /// Create a new empty validator chain
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// Add a validator to the chain
    pub fn add<V: TokenValidator + 'static>(&mut self, validator: V) {
        self.validators.push(Box::new(validator));
    }

    /// Add a validator and return self for chaining
    pub fn with<V: TokenValidator + 'static>(mut self, validator: V) -> Self {
        self.add(validator);
        self
    }

    /// Validate a token using all validators in order
    pub fn validate(&self, token: &str) -> ValidationResult {
        for validator in &self.validators {
            match validator.validate(token) {
                ValidationResult::NotMyToken => continue,
                result => return result,
            }
        }
        ValidationResult::Invalid("no validator accepted the token".to_string())
    }

    /// Get the number of validators
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// Check if chain is empty
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }
}

impl Default for ValidatorChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Security mode for the router
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecurityMode {
    /// No authentication required (default for local development)
    #[default]
    Open,
    /// Token authentication required
    Authenticated,
}

impl fmt::Display for SecurityMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityMode::Open => write!(f, "open"),
            SecurityMode::Authenticated => write!(f, "authenticated"),
        }
    }
}

impl FromStr for SecurityMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "open" | "none" | "off" => Ok(SecurityMode::Open),
            "authenticated" | "auth" | "token" => Ok(SecurityMode::Authenticated),
            _ => Err(Error::InvalidPattern(format!(
                "unknown security mode: {}",
                s
            ))),
        }
    }
}

/// Parse multiple scopes from a comma-separated string
pub fn parse_scopes(s: &str) -> Result<Vec<Scope>> {
    s.split(',')
        .map(|part| Scope::parse(part.trim()))
        .collect()
}

/// Parse a duration string like "7d", "24h", "30m", "60s"
pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return Err(Error::InvalidPattern("empty duration".to_string()));
    }

    let (num_str, unit) = if s.ends_with('d') {
        (&s[..s.len() - 1], "d")
    } else if s.ends_with('h') {
        (&s[..s.len() - 1], "h")
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], "m")
    } else if s.ends_with('s') {
        (&s[..s.len() - 1], "s")
    } else {
        // Default to seconds
        (s, "s")
    };

    let num: u64 = num_str
        .parse()
        .map_err(|_| Error::InvalidPattern(format!("invalid duration number: {}", num_str)))?;

    let secs = match unit {
        "d" => num * 86400,
        "h" => num * 3600,
        "m" => num * 60,
        "s" => num,
        _ => unreachable!(),
    };

    Ok(Duration::from_secs(secs))
}

/// Format a SystemTime as a Unix timestamp
pub fn to_unix_timestamp(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Parse a Unix timestamp to SystemTime
pub fn from_unix_timestamp(ts: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_allows() {
        assert!(Action::Admin.allows(Action::Read));
        assert!(Action::Admin.allows(Action::Write));
        assert!(Action::Admin.allows(Action::Admin));

        assert!(Action::Write.allows(Action::Read));
        assert!(Action::Write.allows(Action::Write));
        assert!(!Action::Write.allows(Action::Admin));

        assert!(Action::Read.allows(Action::Read));
        assert!(!Action::Read.allows(Action::Write));
        assert!(!Action::Read.allows(Action::Admin));
    }

    #[test]
    fn test_action_from_str() {
        assert_eq!(Action::from_str("read").unwrap(), Action::Read);
        assert_eq!(Action::from_str("write").unwrap(), Action::Write);
        assert_eq!(Action::from_str("admin").unwrap(), Action::Admin);
        assert_eq!(Action::from_str("r").unwrap(), Action::Read);
        assert_eq!(Action::from_str("w").unwrap(), Action::Write);
        assert_eq!(Action::from_str("a").unwrap(), Action::Admin);
        assert!(Action::from_str("invalid").is_err());
    }

    #[test]
    fn test_scope_parse() {
        let scope = Scope::parse("read:/**").unwrap();
        assert_eq!(scope.action(), Action::Read);
        assert!(scope.allows(Action::Read, "/any/path"));
        assert!(!scope.allows(Action::Write, "/any/path"));

        let scope = Scope::parse("write:/lights/**").unwrap();
        assert!(scope.allows(Action::Write, "/lights/room/1"));
        assert!(scope.allows(Action::Read, "/lights/room/1"));
        assert!(!scope.allows(Action::Write, "/sensors/temp"));
        assert!(!scope.allows(Action::Read, "/sensors/temp"));

        let scope = Scope::parse("admin:/**").unwrap();
        assert!(scope.allows(Action::Admin, "/any/path"));
        assert!(scope.allows(Action::Write, "/any/path"));
        assert!(scope.allows(Action::Read, "/any/path"));
    }

    #[test]
    fn test_scope_wildcards() {
        let scope = Scope::parse("read:/lumen/scene/*/layer/**").unwrap();
        assert!(scope.allows(Action::Read, "/lumen/scene/0/layer/1/opacity"));
        assert!(scope.allows(Action::Read, "/lumen/scene/main/layer/2"));
        assert!(!scope.allows(Action::Read, "/lumen/scene/0/effect"));
    }

    #[test]
    fn test_token_info() {
        let scopes = vec![
            Scope::parse("read:/**").unwrap(),
            Scope::parse("write:/lights/**").unwrap(),
        ];
        let info = TokenInfo::new("test_token".to_string(), scopes);

        assert!(info.has_scope(Action::Read, "/any/path"));
        assert!(info.has_scope(Action::Write, "/lights/room"));
        assert!(!info.has_scope(Action::Write, "/sensors/temp"));
        assert!(!info.is_expired());
    }

    #[test]
    fn test_token_expiry() {
        let scopes = vec![Scope::parse("read:/**").unwrap()];
        let info = TokenInfo::new("test_token".to_string(), scopes)
            .with_expires_at(SystemTime::now() - Duration::from_secs(1));
        assert!(info.is_expired());

        let scopes = vec![Scope::parse("read:/**").unwrap()];
        let info = TokenInfo::new("test_token".to_string(), scopes)
            .with_expires_in(Duration::from_secs(3600));
        assert!(!info.is_expired());
    }

    #[test]
    fn test_cpsk_validator() {
        let validator = CpskValidator::new();

        // Generate and register a token
        let token = CpskValidator::generate_token();
        assert!(token.starts_with("cpsk_"));
        assert_eq!(token.len(), 37); // "cpsk_" + 32 chars

        let scopes = vec![Scope::parse("read:/**").unwrap()];
        let info = TokenInfo::new(token.clone(), scopes);
        validator.register(token.clone(), info);

        // Validate
        match validator.validate(&token) {
            ValidationResult::Valid(info) => {
                assert!(info.has_scope(Action::Read, "/test"));
            }
            _ => panic!("expected valid token"),
        }

        // Unknown token
        match validator.validate("cpsk_unknown") {
            ValidationResult::Invalid(_) => {}
            _ => panic!("expected invalid token"),
        }

        // Wrong prefix
        match validator.validate("jwt_token") {
            ValidationResult::NotMyToken => {}
            _ => panic!("expected not my token"),
        }

        // Revoke
        assert!(validator.revoke(&token));
        match validator.validate(&token) {
            ValidationResult::Invalid(_) => {}
            _ => panic!("expected invalid after revoke"),
        }
    }

    #[test]
    fn test_validator_chain() {
        let mut chain = ValidatorChain::new();

        let cpsk = CpskValidator::new();
        let token = CpskValidator::generate_token();
        let scopes = vec![Scope::parse("admin:/**").unwrap()];
        cpsk.register(token.clone(), TokenInfo::new(token.clone(), scopes));
        chain.add(cpsk);

        match chain.validate(&token) {
            ValidationResult::Valid(_) => {}
            _ => panic!("expected valid token"),
        }

        match chain.validate("unknown_token") {
            ValidationResult::Invalid(_) => {}
            _ => panic!("expected invalid token"),
        }
    }

    #[test]
    fn test_parse_scopes() {
        let scopes = parse_scopes("read:/**, write:/lights/**").unwrap();
        assert_eq!(scopes.len(), 2);
        assert!(scopes[0].allows(Action::Read, "/any"));
        assert!(scopes[1].allows(Action::Write, "/lights/1"));
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("7d").unwrap(), Duration::from_secs(7 * 86400));
        assert_eq!(parse_duration("24h").unwrap(), Duration::from_secs(24 * 3600));
        assert_eq!(parse_duration("30m").unwrap(), Duration::from_secs(30 * 60));
        assert_eq!(parse_duration("60s").unwrap(), Duration::from_secs(60));
        assert_eq!(parse_duration("120").unwrap(), Duration::from_secs(120));
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn test_security_mode() {
        assert_eq!(SecurityMode::from_str("open").unwrap(), SecurityMode::Open);
        assert_eq!(
            SecurityMode::from_str("authenticated").unwrap(),
            SecurityMode::Authenticated
        );
        assert_eq!(SecurityMode::from_str("auth").unwrap(), SecurityMode::Authenticated);
    }
}
