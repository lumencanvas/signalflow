# Capability Tokens

Control access to CLASP addresses using JWT-based capability tokens.

## Overview

Capability tokens allow fine-grained access control:
- Restrict which addresses a client can read/write
- Set time-limited access
- Revoke access without changing router configuration

## Token Structure

CLASP uses JWT (JSON Web Tokens) with custom claims:

```json
{
  "sub": "client-id",
  "iss": "clasp-router",
  "exp": 1735689600,
  "iat": 1704067200,
  "clasp": {
    "read": ["/sensors/**", "/status"],
    "write": ["/control/lights/*"],
    "emit": ["/events/button"],
    "subscribe": ["/sensors/**"]
  }
}
```

## Router Configuration

### Enable Token Validation

```yaml
# clasp.yaml
server:
  port: 7330
  security:
    require_auth: true
    token_secret: "your-256-bit-secret-key"
    # Or use public key for RS256
    # token_public_key: /path/to/public.pem
```

```bash
# Or via CLI
clasp server --require-auth --token-secret "your-secret"
```

### Generate Tokens

```bash
# Generate a token with specific permissions
clasp token create \
  --read "/sensors/**" \
  --write "/control/*" \
  --expires 24h \
  --subject "mobile-app"
```

Output:
```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Programmatic Token Generation

```javascript
const jwt = require('jsonwebtoken');

function generateClaspToken(permissions, options = {}) {
  const payload = {
    sub: options.subject || 'client',
    clasp: {
      read: permissions.read || [],
      write: permissions.write || [],
      emit: permissions.emit || [],
      subscribe: permissions.subscribe || []
    }
  };

  return jwt.sign(payload, process.env.TOKEN_SECRET, {
    expiresIn: options.expiresIn || '24h',
    issuer: 'clasp-router'
  });
}

// Example: Limited sensor reader
const sensorToken = generateClaspToken({
  read: ['/sensors/**'],
  subscribe: ['/sensors/**']
}, {
  subject: 'sensor-dashboard',
  expiresIn: '7d'
});

// Example: Full control
const adminToken = generateClaspToken({
  read: ['/**'],
  write: ['/**'],
  emit: ['/**'],
  subscribe: ['/**']
}, {
  subject: 'admin',
  expiresIn: '1h'
});
```

## Client Authentication

### JavaScript

```javascript
const { Clasp } = require('@clasp-to/core');

const client = await Clasp.builder('ws://localhost:7330')
  .withToken('eyJhbGciOi...')
  .connect();
```

### Python

```python
from clasp import Clasp

client = await Clasp.connect(
    'ws://localhost:7330',
    token='eyJhbGciOi...'
)
```

### Rust

```rust
use clasp_client::ClaspBuilder;

let client = ClaspBuilder::new("ws://localhost:7330")
    .token("eyJhbGciOi...")
    .connect()
    .await?;
```

## Permission Patterns

### Address Wildcards

```json
{
  "clasp": {
    "read": [
      "/sensors/**",      // All under /sensors
      "/status",          // Exact match
      "/control/*/level"  // Single segment wildcard
    ]
  }
}
```

### Read-Only Client

```json
{
  "clasp": {
    "read": ["/**"],
    "subscribe": ["/**"]
  }
}
```

### Write-Only Client

```json
{
  "clasp": {
    "write": ["/ingest/**"]
  }
}
```

### Scoped Control

```json
{
  "clasp": {
    "read": ["/lights/**"],
    "write": ["/lights/zone1/*"],
    "subscribe": ["/lights/**"]
  }
}
```

## Token Refresh

Handle token expiration:

```javascript
const client = await Clasp.builder('ws://localhost:7330')
  .withToken(currentToken)
  .withTokenRefresh(async () => {
    // Called when token is about to expire
    const newToken = await fetchNewToken();
    return newToken;
  })
  .connect();
```

## Token Revocation

### Deny List

Configure router to reject specific tokens:

```yaml
security:
  revoked_tokens:
    - "token-id-1"
    - "token-id-2"
```

### Short Expiry + Refresh

Use short-lived tokens (15 min) with refresh:
- Compromised tokens expire quickly
- Refresh can be denied server-side

### Token ID Tracking

Include `jti` (JWT ID) claim for tracking:

```javascript
const payload = {
  jti: crypto.randomUUID(),
  sub: 'client',
  clasp: { /* permissions */ }
};
```

## Example: Multi-Tenant System

Different tokens for different users:

```javascript
// Admin token - full access
const adminToken = generateClaspToken({
  read: ['/**'],
  write: ['/**'],
  emit: ['/**'],
  subscribe: ['/**']
});

// User token - limited to their namespace
function userToken(userId) {
  return generateClaspToken({
    read: [`/users/${userId}/**`, '/public/**'],
    write: [`/users/${userId}/**`],
    subscribe: [`/users/${userId}/**`, '/public/**']
  });
}

// Device token - single device access
function deviceToken(deviceId) {
  return generateClaspToken({
    write: [`/devices/${deviceId}/telemetry`],
    read: [`/devices/${deviceId}/config`]
  });
}
```

## Troubleshooting

### "Token required"

Client didn't provide a token and router requires auth.

### "Token expired"

Token's `exp` claim is in the past. Generate a new token.

### "Permission denied"

Token doesn't have permission for the requested operation:
- Check address patterns match
- Verify operation type (read/write/emit/subscribe)

### "Invalid signature"

Token was signed with wrong secret or was tampered with.

## Security Best Practices

1. **Use HTTPS/WSS** - Tokens are bearer credentials
2. **Short expiration** - Use short-lived tokens with refresh
3. **Minimal permissions** - Grant only required access
4. **Secure storage** - Never store tokens in code
5. **Rotate secrets** - Change signing key periodically
6. **Audit logging** - Log token usage for security review

## Next Steps

- [Enable TLS](enable-tls.md)
- [Pairing](pairing.md)
- [Security Model](../../explanation/security-model.md)
