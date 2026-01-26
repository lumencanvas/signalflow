# CLASP Security Model

## Overview

CLASP implements a capability-based security model with:
- Capability Pre-Shared Keys (CPSK)
- Scope-based authorization
- Pattern-matched permissions
- Token expiration and revocation

## Security Modes

### Open Mode (Default)
```rust
SecurityMode::Open
```
- No authentication required
- All clients have full access
- Suitable for trusted networks

### Authenticated Mode
```rust
SecurityMode::Authenticated
```
- Token required in HELLO message
- Validated against TokenValidator chain
- Scope-based permission checking
- Unauthorized requests rejected

## Token System

### CPSK Tokens
Capability Pre-Shared Keys are lookup-based tokens with cryptographic randomness:

```
Format: cpsk_<32 hex characters from UUID v4>
Example: cpsk_a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6
```

**Generation:**
- UUID v4 via `uuid::Uuid::new_v4()`
- Uses `getrandom` crate for OS-level cryptographic entropy
- 32 hex characters (122 bits of randomness)
- Cryptographically secure random generation
- Suitable for capability tokens requiring strong randomness

**Note (v3.1.1):** Token generation was upgraded from a weak time-seeded LCG to UUID v4.
This change **only affects how new tokens are generated** - all capability/scope mechanics
(scoped permissions, pattern matching, token validation) remain unchanged. Existing
registered tokens continue to work.

### TokenInfo Structure
```rust
pub struct TokenInfo {
    pub token_id: String,
    pub subject: Option<String>,
    pub scopes: Vec<Scope>,
    pub expires_at: Option<SystemTime>,
    pub metadata: HashMap<String, String>,
}
```

### Token Lifecycle
```
1. Create token with scopes and expiration
2. Client includes token in HELLO
3. Router validates via TokenValidator
4. Session marked as authenticated
5. Scopes checked on each operation
6. Token can be revoked at any time
```

## Scope System

### Scope Format
```
action:pattern
```

**Actions:**
- `read` - Subscribe, Get operations
- `write` - Set, Publish operations
- `admin` - Full access (includes read + write)

**Patterns:**
- Exact: `/lights/room/1`
- Single wildcard: `/lights/*/opacity`
- Multi wildcard: `/lights/**`

### Scope Examples
```
read:/sensors/**          # Read all sensors
write:/controls/*         # Write to one level under /controls/
admin:/**                 # Full admin access everywhere
read:/status              # Read only /status exactly
write:/lights/room/**/dim # Write to any .../dim under /lights/room/
```

### Scope Hierarchy
```
Admin ─┬─► Write ─┬─► Read
       │          │
       └──────────┴──► Specific actions

Admin allows Write operations
Write allows Read operations
```

### Scope Parsing
```rust
pub fn parse_scopes(s: &str) -> Result<Vec<Scope>>

// Input: "read:/a, write:/b, admin:/c"
// Output: [Scope(Read, /a), Scope(Write, /b), Scope(Admin, /c)]
```

## Authentication Flow

### HELLO with Token
```
Client                          Router
   │                              │
   │───HELLO(token: "cpsk_...")──►│
   │                              │
   │                  Validate token
   │                  Extract scopes
   │                  Check expiration
   │                              │
   │◄───WELCOME(session, time)────│
   │                              │
```

### Validation Chain
```rust
pub struct ValidatorChain {
    validators: Vec<Box<dyn TokenValidator>>,
}

// Try validators in order
// First "Valid" wins
// "NotMyToken" continues chain
// "Invalid" or "Expired" stops with error
```

### Validation Results
```rust
pub enum ValidationResult {
    Valid(TokenInfo),     // Token accepted
    NotMyToken,           // Try next validator
    Invalid(String),      // Token rejected
    Expired,              // Token expired
}
```

## Authorization Checks

### Permission Checking
```rust
// In Session
pub fn has_scope(&self, action: Action, address: &str) -> bool

// Called for:
// - SUBSCRIBE: Action::Read
// - GET: Action::Read
// - SET: Action::Write
// - PUBLISH: Action::Write
```

### Pattern Matching
```rust
pub struct Scope {
    action: Action,
    pattern: Pattern,  // Compiled glob pattern
    raw: String,
}

impl Scope {
    pub fn allows(&self, action: Action, address: &str) -> bool {
        self.action.allows(action) && self.pattern.matches(address)
    }
}
```

### Error Codes
```rust
ErrorCode::Unauthorized = 300   // No token provided
ErrorCode::Forbidden = 301      // Insufficient scope
ErrorCode::TokenExpired = 302   // Token past expiration
```

## Token Management (CLI)

### Create Token
```bash
clasp token create \
    --scopes "read:/sensors/**, write:/controls/*" \
    --expires "7d" \
    --subject "sensor-client"
```

### List Tokens
```bash
clasp token list
```

### Revoke Token
```bash
clasp token revoke cpsk_a1b2c3d4e5f6...
```

### Prune Expired
```bash
clasp token prune
```

## Token Storage

### File-Based Storage
```
~/.config/clasp/tokens.json

{
  "tokens": [
    {
      "token": "cpsk_...",
      "subject": "sensor-client",
      "scopes": ["read:/sensors/**"],
      "expires_at": 1737900000,
      "created_at": 1737300000,
      "metadata": {}
    }
  ]
}
```

### TokenStore Methods
```rust
pub struct TokenStore { ... }

impl TokenStore {
    pub fn add(&mut self, record: TokenRecord);
    pub fn remove(&mut self, token: &str) -> bool;
    pub fn get(&self, token: &str) -> Option<&TokenRecord>;
    pub fn list(&self) -> Vec<&TokenRecord>;
    pub fn prune_expired(&mut self) -> usize;
    pub fn to_validator(&self) -> CpskValidator;
}
```

## CpskValidator

### Implementation
```rust
pub struct CpskValidator {
    tokens: RwLock<HashMap<String, TokenInfo>>,
}

pub const PREFIX: &str = "cpsk_";

impl TokenValidator for CpskValidator {
    fn validate(&self, token: &str) -> ValidationResult {
        if !token.starts_with(PREFIX) {
            return ValidationResult::NotMyToken;
        }

        match self.tokens.read().get(token) {
            Some(info) => {
                if info.is_expired() {
                    ValidationResult::Expired
                } else {
                    ValidationResult::Valid(info.clone())
                }
            }
            None => ValidationResult::Invalid("Unknown token".into()),
        }
    }
}
```

### Thread Safety
- `RwLock<HashMap<...>>` for concurrent access
- Read lock for validation (non-blocking reads)
- Write lock for registration/revocation

## Security Considerations

### Token Generation
- UUID v4 with cryptographic PRNG (`getrandom`)
- 122 bits of entropy from OS-provided randomness
- Cryptographically secure token generation
- Suitable for capability tokens and secure lookups

### Pattern Matching
- Uses `glob-match` crate
- Compiled once on Scope creation
- Potential ReDoS with complex patterns
- Mitigated by `regex-lite`

### Transport Security
- Flags reserved for encryption
- Actual encryption via transport (TLS/DTLS)
- QUIC provides built-in TLS 1.3

### Rate Limiting
```rust
// Per-session rate limiting
pub max_messages_per_second: u32

// Check before processing
pub fn check_rate_limit(&self, max: u32) -> bool {
    let now = unix_second();
    if now != self.last_rate_limit_second {
        self.reset_counter();
    } else if self.count() > max {
        return false;  // Rate limited
    }
    true
}
```

## Attack Surface

### Mitigated
1. **Unauthorized Access** - Token validation
2. **Scope Escalation** - Pattern matching per operation
3. **Token Replay** - Expiration and revocation
4. **Rate Abuse** - Per-session rate limiting
5. **Connection Flooding** - Max sessions limit

### Not Mitigated (Transport Layer)
1. **Eavesdropping** - Use TLS/QUIC
2. **Man-in-the-Middle** - Use TLS/QUIC
3. **DDoS** - External protection needed

## Best Practices

### Token Scopes
```bash
# Principle of least privilege
--scopes "read:/sensors/**"           # Read-only sensors
--scopes "write:/controls/lights/*"   # Write only lights
```

### Token Expiration
```bash
# Short-lived for high-security
--expires "1h"

# Medium for normal use
--expires "7d"

# Long-lived for static systems
--expires "365d"
```

### Security Mode Selection
```rust
// Trusted network (local development)
RouterConfig::default()  // Open mode

// Production deployment
RouterConfig::default()
    .with_security_mode(SecurityMode::Authenticated)
    .with_validator(validator)
```

## Integration Examples

### Router with Auth
```rust
let validator = CpskValidator::new();
validator.register("cpsk_client1".into(), TokenInfo {
    token_id: "cpsk_client1".into(),
    subject: Some("sensor-client".into()),
    scopes: vec![Scope::parse("read:/sensors/**")?],
    expires_at: Some(SystemTime::now() + Duration::from_secs(86400)),
    metadata: HashMap::new(),
});

let router = Router::new(RouterConfig {
    security_mode: SecurityMode::Authenticated,
    ..Default::default()
})
.with_validator(validator);
```

### Client with Token
```rust
let client = ClaspBuilder::new("ws://localhost:7330")
    .token("cpsk_client1")
    .connect()
    .await?;
```

### Scope-Limited Operations
```rust
// This works (read scope allows subscribe)
client.subscribe("/sensors/**", |v, a| {}).await?;

// This fails (no write scope for /controls/)
client.set("/controls/light", 1.0).await?;
// Error: Forbidden (301)
```
