# Performance Tuning

Optimize CLASP for high throughput and low latency.

## Measuring Performance

### Built-in Metrics

```bash
# Enable metrics endpoint
clasp server --metrics --metrics-port 9090

# View metrics
curl http://localhost:9090/metrics
```

Key metrics:
- `clasp_messages_total` - Total messages processed
- `clasp_message_latency_seconds` - Message processing latency
- `clasp_connections_active` - Current connection count
- `clasp_subscriptions_active` - Current subscription count

### Benchmarking

```bash
# Run built-in benchmark
clasp benchmark --duration 30s --clients 10 --rate 1000
```

```javascript
// JavaScript benchmark
const start = Date.now();
let count = 0;

for (let i = 0; i < 10000; i++) {
  await client.set('/benchmark/value', i);
  count++;
}

const elapsed = Date.now() - start;
console.log(`${count / (elapsed / 1000)} msg/sec`);
```

## Router Optimization

### Resource Limits

```yaml
# clasp.yaml
server:
  port: 7330

  # Connection limits
  max_connections: 10000
  max_connections_per_ip: 100

  # Message limits
  max_message_size: 65536  # 64KB

  # Memory limits
  max_state_entries: 1000000
  max_subscriptions: 100000

  # Rate limiting (per client)
  rate_limiting:
    enabled: true
    max_messages_per_second: 1000

  # Gesture coalescing
  gesture:
    coalescing: true
    coalesce_interval_ms: 16  # 60fps
```

### Server-Side Rate Limiting

Rate limiting prevents individual clients from overwhelming the router:

```rust
use clasp_router::RouterConfig;

let config = RouterConfig {
    rate_limiting_enabled: true,
    max_messages_per_second: 500,  // Per client
    ..Default::default()
};
```

When a client exceeds the limit, excess messages are dropped and a warning is logged. Set `max_messages_per_second` to `0` for unlimited.

### Gesture Coalescing

High-frequency gesture streams (like touchpad moves) can be coalesced to reduce bandwidth:

```rust
let config = RouterConfig {
    gesture_coalescing: true,
    gesture_coalesce_interval_ms: 16,  // ~60fps
    ..Default::default()
};
```

With coalescing enabled, rapid gesture moves within the interval are combined into a single message.

### Thread Pool

```yaml
server:
  # Worker threads (default: CPU cores)
  worker_threads: 8

  # Async runtime threads
  async_threads: 4
```

### State Storage

```yaml
server:
  state:
    # In-memory (fastest, no persistence)
    backend: memory

    # Or disk-backed (persistent, slower)
    # backend: sqlite
    # path: /var/lib/clasp/state.db
```

## Client Optimization

### Connection Pooling

```javascript
// Reuse single connection
const client = await Clasp.connect('ws://localhost:7330');

// Don't create new connections per request
// BAD: for each request, create new connection
// GOOD: share client across requests
```

### Batching

```javascript
// BAD: Many individual operations
for (const value of values) {
  await client.set(`/data/${value.id}`, value);  // N round trips
}

// GOOD: Batch operations
const ops = values.map(v => ({ set: [`/data/${v.id}`, v] }));
await client.bundle(ops);  // 1 round trip
```

### Rate Limiting Subscriptions

```javascript
// Limit update rate for high-frequency data
client.on('/sensors/accelerometer', handler, {
  maxRate: 60  // Max 60 updates/sec
});

// Debounce for UI updates
client.on('/control/slider', handler, {
  debounce: 50  // Wait 50ms after last update
});
```

### Unsubscribe When Done

```javascript
// Clean up subscriptions
const unsubscribe = client.on('/temp/data', handler);

// When no longer needed
unsubscribe();
```

## Network Optimization

### Transport Selection

| Transport | Latency | Throughput | Use Case |
|-----------|---------|------------|----------|
| WebSocket | Low | High | Default choice |
| QUIC | Very Low | Very High | High-performance |
| UDP | Lowest | Highest | LAN, lossy OK |

```bash
# WebSocket (default)
clasp server --port 7330

# QUIC (requires TLS)
clasp server --port 7330 --quic --tls-cert cert.pem --tls-key key.pem

# UDP (unreliable, fastest)
clasp server --port 7330 --udp
```

### Compression

```yaml
server:
  compression:
    enabled: true
    threshold: 1024  # Compress messages > 1KB
    level: 6         # 1-9, higher = smaller but slower
```

### Keep-Alive

```yaml
server:
  keepalive:
    enabled: true
    interval: 30s
    timeout: 60s
```

## Message Optimization

### Use Binary Encoding

CLASP binary encoding is ~55% smaller than JSON:

```javascript
// Automatic - CLASP uses binary by default
await client.set('/data', { x: 1.5, y: 2.5, z: 3.5 });
```

### Minimize Payload Size

```javascript
// BAD: Large, verbose payload
await client.set('/sensor', {
  sensorIdentifier: 'temperature-sensor-001',
  currentTemperatureValueInCelsius: 23.456789,
  measurementTimestamp: new Date().toISOString()
});

// GOOD: Compact payload
await client.set('/s/t/001', {
  v: 23.46,  // Rounded
  t: Date.now()  // Unix timestamp
});
```

### Use Appropriate Signal Types

```javascript
// Param: Stateful, retained - for values that persist
await client.set('/config/brightness', 0.8);

// Event: Ephemeral - for one-time notifications
await client.emit('/events/button_pressed', { button: 1 });

// Stream: High-rate - for continuous data
client.stream('/audio/level', getAudioLevel);
```

## Subscription Optimization

### Use Specific Patterns

```javascript
// BAD: Too broad, receives everything
client.on('/**', handler);

// GOOD: Specific patterns
client.on('/sensors/temperature/*', tempHandler);
client.on('/sensors/humidity/*', humidityHandler);
```

### Server-Side Filtering

```javascript
// Filter on server to reduce traffic
client.on('/data/*', handler, {
  filter: {
    value: { $gt: 100 }  // Only values > 100
  }
});
```

## Memory Optimization

### Limit State History

```yaml
server:
  state:
    # Don't keep history (fastest)
    history_enabled: false

    # Or limit history
    # history_enabled: true
    # max_history_per_address: 100
```

### Clean Up Old State

```javascript
// Periodic cleanup
setInterval(async () => {
  const addresses = await client.list('/temp/**');
  const cutoff = Date.now() - 3600000;  // 1 hour

  for (const addr of addresses) {
    const meta = await client.getMeta(addr);
    if (meta.updatedAt < cutoff) {
      await client.delete(addr);
    }
  }
}, 60000);
```

### Use Weak References (JavaScript)

```javascript
// For caching received values
const cache = new WeakMap();

client.on('/data/*', (value, address) => {
  const obj = { address, value, receivedAt: Date.now() };
  cache.set(obj, true);  // Allows GC when not referenced
});
```

## Profiling

### Rust Profiling

```bash
# CPU profiling with flamegraph
cargo install flamegraph
flamegraph -- target/release/clasp-router

# Memory profiling
cargo install heaptrack
heaptrack target/release/clasp-router
```

### Node.js Profiling

```bash
# CPU profiling
node --prof your-app.js
node --prof-process isolate-*.log > profile.txt

# Memory profiling
node --inspect your-app.js
# Use Chrome DevTools Memory tab
```

### Network Analysis

```bash
# Monitor CLASP traffic
tcpdump -i lo port 7330 -w clasp.pcap

# Analyze with Wireshark
wireshark clasp.pcap
```

## Common Bottlenecks

### High Latency

1. Check network path (use local connection if possible)
2. Enable compression for large messages
3. Use QUIC transport for better performance
4. Batch operations to reduce round trips

### Low Throughput

1. Increase worker threads
2. Use connection pooling
3. Batch messages
4. Check for slow subscribers blocking delivery

### Memory Growth

1. Limit state entries
2. Clean up old data periodically
3. Unsubscribe unused subscriptions
4. Disable history if not needed

### CPU Usage

1. Profile to find hot spots
2. Reduce message parsing (use binary)
3. Limit subscription patterns
4. Use server-side filtering

## Next Steps

- [Embed Router](embed-router.md)
- [Cloud Deployment](../../use-cases/cloud-deployment.md)
- [Architecture](../../explanation/architecture.md)
