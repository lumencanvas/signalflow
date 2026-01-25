# Enable TLS

Secure CLASP connections with TLS encryption.

## Overview

TLS encrypts all communication between clients and routers, protecting against eavesdropping and tampering. Use TLS in production and when routing over untrusted networks.

## Router Configuration

### Using Certificate Files

```bash
# Start router with TLS
clasp server --port 7330 \
  --tls-cert /path/to/cert.pem \
  --tls-key /path/to/key.pem
```

### Configuration File

```yaml
# clasp.yaml
server:
  port: 7330
  tls:
    enabled: true
    cert: /path/to/fullchain.pem
    key: /path/to/privkey.pem
```

### Environment Variables

```bash
export CLASP_TLS_CERT=/path/to/cert.pem
export CLASP_TLS_KEY=/path/to/key.pem
clasp server --port 7330
```

## Obtaining Certificates

### Let's Encrypt (Production)

For public-facing routers:

```bash
# Install certbot
sudo apt install certbot

# Obtain certificate
sudo certbot certonly --standalone -d clasp.example.com

# Certificates stored at:
# /etc/letsencrypt/live/clasp.example.com/fullchain.pem
# /etc/letsencrypt/live/clasp.example.com/privkey.pem
```

Configure auto-renewal:

```bash
sudo certbot renew --dry-run
```

### Self-Signed (Development)

For development and internal networks:

```bash
# Generate self-signed certificate
openssl req -x509 -newkey rsa:4096 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes \
  -subj "/CN=localhost"
```

For local network with IP address:

```bash
openssl req -x509 -newkey rsa:4096 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes \
  -subj "/CN=192.168.1.100" \
  -addext "subjectAltName=IP:192.168.1.100"
```

### mkcert (Development)

Easier local development certificates:

```bash
# Install mkcert
brew install mkcert  # macOS
# or: choco install mkcert  # Windows

# Install local CA
mkcert -install

# Generate certificates
mkcert localhost 192.168.1.100
# Creates localhost+1.pem and localhost+1-key.pem
```

## Client Connection

### JavaScript

```javascript
const { Clasp } = require('@clasp-to/core');

// Connect with TLS
const client = await Clasp.connect('wss://clasp.example.com:7330');

// Self-signed certificate (development only)
const client = await Clasp.builder('wss://192.168.1.100:7330')
  .withTlsConfig({
    rejectUnauthorized: false  // Accept self-signed
  })
  .connect();

// Custom CA certificate
const client = await Clasp.builder('wss://192.168.1.100:7330')
  .withTlsConfig({
    ca: fs.readFileSync('/path/to/ca.pem')
  })
  .connect();
```

### Python

```python
from clasp import Clasp
import ssl

# Connect with TLS
client = await Clasp.connect('wss://clasp.example.com:7330')

# Self-signed certificate
ssl_context = ssl.create_default_context()
ssl_context.check_hostname = False
ssl_context.verify_mode = ssl.CERT_NONE

client = await Clasp.connect(
    'wss://192.168.1.100:7330',
    ssl=ssl_context
)

# Custom CA
ssl_context = ssl.create_default_context(cafile='/path/to/ca.pem')
client = await Clasp.connect(
    'wss://192.168.1.100:7330',
    ssl=ssl_context
)
```

### Rust

```rust
use clasp_client::ClaspBuilder;

// TLS is automatic when using wss:// URLs
let client = ClaspBuilder::new("wss://clasp.example.com:7330")
    .name("My App")
    .connect()
    .await?;
```

## Docker Configuration

```yaml
# docker-compose.yaml
services:
  clasp:
    image: lumencanvas/clasp-router
    ports:
      - "7330:7330"
    volumes:
      - /etc/letsencrypt/live/clasp.example.com:/certs:ro
    environment:
      - CLASP_TLS_CERT=/certs/fullchain.pem
      - CLASP_TLS_KEY=/certs/privkey.pem
```

## Nginx Reverse Proxy

Terminate TLS at nginx:

```nginx
server {
    listen 443 ssl;
    server_name clasp.example.com;

    ssl_certificate /etc/letsencrypt/live/clasp.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/clasp.example.com/privkey.pem;

    location / {
        proxy_pass http://localhost:7330;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Certificate Verification

Check certificate details:

```bash
# View certificate
openssl x509 -in cert.pem -text -noout

# Test TLS connection
openssl s_client -connect clasp.example.com:7330

# Check expiry
openssl x509 -enddate -noout -in cert.pem
```

## Troubleshooting

### Certificate Errors

**"Certificate not trusted"**
- Use certificates from a trusted CA (Let's Encrypt)
- Or add self-signed cert to trust store
- Or configure client to accept self-signed (development only)

**"Certificate hostname mismatch"**
- Ensure certificate CN/SAN matches connection hostname/IP
- For IP connections, include IP in Subject Alternative Name

**"Certificate expired"**
- Renew certificate
- Set up auto-renewal for Let's Encrypt

### Connection Issues

**"Connection reset"**
- Verify TLS is enabled on router
- Check port is correct for TLS (same port, different scheme)
- Verify firewall allows TLS traffic

**"Protocol error"**
- Don't mix ws:// and wss:// schemes
- Ensure client and server TLS versions are compatible

## Security Best Practices

1. **Always use TLS in production**
2. **Use trusted CA certificates** (Let's Encrypt for public, mkcert for dev)
3. **Enable certificate validation** in production clients
4. **Keep certificates updated** (auto-renewal)
5. **Use strong cipher suites** (TLS 1.2+ only)
6. **Protect private keys** (restrictive file permissions)

## Next Steps

- [Capability Tokens](capability-tokens.md)
- [Pairing](pairing.md)
- [Cloud Deployment](../../use-cases/cloud-deployment.md)
