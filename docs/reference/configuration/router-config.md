# Router Configuration

Complete reference for CLASP router configuration options.

## Configuration File

Default location: `clasp.yaml` or `~/.config/clasp/config.yaml`

```yaml
server:
  # Network settings
  port: 7330
  bind: "0.0.0.0"

  # TLS settings
  tls:
    enabled: false
    cert: /path/to/cert.pem
    key: /path/to/key.pem

  # Additional transports
  quic:
    enabled: false
    port: 7330
  udp:
    enabled: false
    port: 7331

  # Discovery
  discovery:
    mdns:
      enabled: true
      name: "CLASP Router"
    udp:
      enabled: false
      port: 7331

  # Security
  security:
    require_auth: false
    token_secret: null
    token_public_key: null
    pairing:
      enabled: false
      timeout: 300

  # Limits
  limits:
    max_connections: 10000
    max_connections_per_ip: 100
    max_message_size: 65536
    max_subscriptions_per_client: 1000
    max_state_entries: 1000000

  # Rate Limiting
  rate_limiting:
    enabled: true
    max_messages_per_second: 1000

  # Gesture Coalescing
  gesture:
    coalescing: true
    coalesce_interval_ms: 16

  # Protocol Adapters
  mqtt:
    enabled: false
    port: 1883
    namespace: "/mqtt"
    require_auth: false
    max_clients: 100
    session_timeout: 300

  osc:
    enabled: false
    port: 8000
    namespace: "/osc"
    session_timeout: 30

  # Persistence
  persistence:
    enabled: false
    backend: sqlite
    path: ./clasp-state.db
    sync_interval: 5

  # Logging
  logging:
    level: info
    format: text
    file: null

  # Metrics
  metrics:
    enabled: false
    port: 9090
```

## Network Settings

### port

WebSocket listen port.

- Type: `integer`
- Default: `7330`
- Range: 1-65535

### bind

Address to bind to.

- Type: `string`
- Default: `"0.0.0.0"` (all interfaces)
- Examples: `"127.0.0.1"`, `"192.168.1.100"`

## TLS Settings

### tls.enabled

Enable TLS encryption.

- Type: `boolean`
- Default: `false`

### tls.cert

Path to TLS certificate file (PEM format).

- Type: `string`
- Required when `tls.enabled: true`

### tls.key

Path to TLS private key file (PEM format).

- Type: `string`
- Required when `tls.enabled: true`

## QUIC Transport

### quic.enabled

Enable QUIC transport (requires TLS).

- Type: `boolean`
- Default: `false`

### quic.port

QUIC listen port.

- Type: `integer`
- Default: Same as `port`

## UDP Transport

### udp.enabled

Enable UDP transport.

- Type: `boolean`
- Default: `false`

### udp.port

UDP listen port.

- Type: `integer`
- Default: `7331`

## Discovery

### discovery.mdns.enabled

Enable mDNS advertisement.

- Type: `boolean`
- Default: `true`

### discovery.mdns.name

Name to advertise.

- Type: `string`
- Default: System hostname

### discovery.udp.enabled

Enable UDP broadcast discovery responder.

- Type: `boolean`
- Default: `false`

## Security

### security.require_auth

Require authentication token for connections.

- Type: `boolean`
- Default: `false`

### security.token_secret

Secret for HS256 JWT validation.

- Type: `string`
- Should be 256+ bits

### security.token_public_key

Path to public key for RS256 JWT validation.

- Type: `string`

### security.pairing.enabled

Enable pairing mode for token-less connections.

- Type: `boolean`
- Default: `false`

### security.pairing.timeout

PIN code timeout in seconds.

- Type: `integer`
- Default: `300`

## Limits

### limits.max_connections

Maximum total client connections.

- Type: `integer`
- Default: `10000`

### limits.max_connections_per_ip

Maximum connections from single IP.

- Type: `integer`
- Default: `100`

### limits.max_message_size

Maximum message size in bytes.

- Type: `integer`
- Default: `65536` (64KB)

### limits.max_subscriptions_per_client

Maximum subscriptions per client.

- Type: `integer`
- Default: `1000`

### limits.max_state_entries

Maximum state entries in router.

- Type: `integer`
- Default: `1000000`

## Rate Limiting

### rate_limiting.enabled

Enable per-client rate limiting.

- Type: `boolean`
- Default: `true`

### rate_limiting.max_messages_per_second

Maximum messages per second per client. When exceeded, messages are dropped and a warning is logged.

- Type: `integer`
- Default: `1000`
- Set to `0` for unlimited

## Gesture Coalescing

### gesture.coalescing

Enable coalescing of high-frequency gesture move messages to reduce bandwidth.

- Type: `boolean`
- Default: `true`

### gesture.coalesce_interval_ms

Interval in milliseconds for coalescing gesture moves. 16ms equals approximately 60fps.

- Type: `integer`
- Default: `16`

## Protocol Adapters

Enable MQTT or OSC clients to connect directly to the router without external brokers.

### mqtt.enabled

Enable MQTT server adapter on the router.

- Type: `boolean`
- Default: `false`

### mqtt.port

Port for MQTT clients to connect.

- Type: `integer`
- Default: `1883`

### mqtt.namespace

Prefix for MQTT topics in CLASP address space.

- Type: `string`
- Default: `"/mqtt"`
- Example: MQTT topic `sensors/temp` becomes CLASP address `/mqtt/sensors/temp`

### mqtt.require_auth

Require MQTT clients to authenticate with username/password.

- Type: `boolean`
- Default: `false`

### mqtt.max_clients

Maximum concurrent MQTT client connections.

- Type: `integer`
- Default: `100`

### mqtt.session_timeout

MQTT session timeout in seconds.

- Type: `integer`
- Default: `300`

### osc.enabled

Enable OSC server adapter on the router.

- Type: `boolean`
- Default: `false`

### osc.port

UDP port for OSC messages.

- Type: `integer`
- Default: `8000`

### osc.namespace

Prefix for OSC addresses in CLASP address space.

- Type: `string`
- Default: `"/osc"`
- Example: OSC address `/synth/volume` becomes CLASP address `/osc/synth/volume`

### osc.session_timeout

OSC session timeout in seconds. Sessions are created per source IP:port and expire after inactivity.

- Type: `integer`
- Default: `30`

## Persistence

### persistence.enabled

Enable state persistence.

- Type: `boolean`
- Default: `false`

### persistence.backend

Storage backend.

- Type: `string`
- Options: `memory`, `sqlite`
- Default: `sqlite`

### persistence.path

Database file path.

- Type: `string`
- Default: `./clasp-state.db`

### persistence.sync_interval

Sync interval in seconds.

- Type: `integer`
- Default: `5`

## Logging

### logging.level

Log verbosity level.

- Type: `string`
- Options: `error`, `warn`, `info`, `debug`, `trace`
- Default: `info`

### logging.format

Log output format.

- Type: `string`
- Options: `text`, `json`
- Default: `text`

### logging.file

Log to file instead of stdout.

- Type: `string`
- Default: `null` (stdout)

## Metrics

### metrics.enabled

Enable Prometheus metrics endpoint.

- Type: `boolean`
- Default: `false`

### metrics.port

Metrics HTTP port.

- Type: `integer`
- Default: `9090`

## Environment Variables

All options can be set via environment variables:

```bash
CLASP_PORT=7330
CLASP_BIND=0.0.0.0
CLASP_TLS_ENABLED=true
CLASP_TLS_CERT=/path/to/cert.pem
CLASP_TLS_KEY=/path/to/key.pem
CLASP_SECURITY_REQUIRE_AUTH=true
CLASP_SECURITY_TOKEN_SECRET=your-secret
CLASP_LOG_LEVEL=debug
```

Environment variables override config file values.

## See Also

- [Start Router](../../how-to/connections/start-router.md)
- [clasp server CLI](../cli/clasp-server.md)
- [Bridge Configuration](bridge-config.md)
