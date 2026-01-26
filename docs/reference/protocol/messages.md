# Message Types

CLASP defines a complete set of message types for connection management, subscriptions, state synchronization, and timing.

## Message Catalog

| Message | Code | Direction | Description |
|---------|------|-----------|-------------|
| `HELLO` | 0x01 | Client→Router | Connection initiation |
| `WELCOME` | 0x02 | Router→Client | Connection accepted |
| `ANNOUNCE` | 0x03 | Both | Capability advertisement |
| `SUBSCRIBE` | 0x10 | Client→Router | Subscribe to pattern |
| `UNSUBSCRIBE` | 0x11 | Client→Router | Unsubscribe |
| `PUBLISH` | 0x20 | Both | Send signal (Event/Stream/Gesture) |
| `SET` | 0x21 | Both | Set Param value |
| `GET` | 0x22 | Client→Router | Request current value |
| `SNAPSHOT` | 0x23 | Router→Client | Current state dump |
| `BUNDLE` | 0x30 | Both | Atomic message group |
| `SYNC` | 0x40 | Both | Clock synchronization |
| `PING` | 0x41 | Both | Keepalive |
| `PONG` | 0x42 | Both | Keepalive response |
| `ACK` | 0x50 | Both | Acknowledgment |
| `ERROR` | 0x51 | Both | Error response |
| `QUERY` | 0x60 | Client→Router | Introspection |
| `RESULT` | 0x61 | Router→Client | Query response |

## Connection Messages

### HELLO (Client → Router)

Initiates a connection with capability negotiation.

```javascript
{
  type: "HELLO",
  version: 1,
  name: "My App",
  features: ["param", "event", "stream"],
  capabilities: {
    encryption: true,
    compression: "lz4"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | int | Yes | Protocol version (currently 1) |
| `name` | string | No | Human-readable client name |
| `features` | string[] | No | Requested signal types |
| `capabilities` | object | No | Optional capabilities |

### WELCOME (Router → Client)

Confirms connection and provides session information.

```javascript
{
  type: "WELCOME",
  version: 1,
  session: "abc123",
  name: "CLASP Router",
  features: ["param", "event", "stream", "timeline"],
  time: 1704067200000000,
  token: "bearer:xyz..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| `version` | int | Protocol version |
| `session` | string | Assigned session ID |
| `name` | string | Router name |
| `features` | string[] | Supported signal types |
| `time` | uint64 | Router time (microseconds) |
| `token` | string | Optional capability token |

### ANNOUNCE

Advertises available signals (namespace registration).

```javascript
{
  type: "ANNOUNCE",
  namespace: "/app",
  signals: [
    {
      address: "/app/scene/*/opacity",
      type: "param",
      datatype: "f64",
      access: "rw",
      meta: {
        unit: "normalized",
        range: [0, 1],
        default: 1.0
      }
    },
    {
      address: "/app/cue/*",
      type: "event",
      access: "w"
    }
  ]
}
```

## Subscription Messages

### SUBSCRIBE (Client → Router)

Subscribe to an address pattern.

```javascript
{
  type: "SUBSCRIBE",
  id: 1,
  pattern: "/app/scene/*/opacity",
  types: ["param"],
  options: {
    maxRate: 30,
    epsilon: 0.01,
    history: 1
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | int | Subscription ID (for unsubscribe) |
| `pattern` | string | Address pattern with wildcards |
| `types` | string[] | Filter by signal type |
| `options.maxRate` | int | Max updates per second |
| `options.epsilon` | float | Minimum change threshold |
| `options.history` | int | Request historical values |

### UNSUBSCRIBE (Client → Router)

Remove a subscription.

```javascript
{
  type: "UNSUBSCRIBE",
  id: 1
}
```

## Data Messages

### SET

Set a parameter value (stateful).

```javascript
{
  type: "SET",
  address: "/app/scene/0/opacity",
  value: 0.75,
  revision: 41,
  lock: false
}
```

| Field | Type | Description |
|-------|------|-------------|
| `address` | string | Target address |
| `value` | any | Value to set |
| `revision` | uint64 | Expected revision (optimistic lock) |
| `lock` | bool | Request exclusive lock |
| `unlock` | bool | Release lock |

### GET (Client → Router)

Request current value.

```javascript
{
  type: "GET",
  address: "/app/scene/0/opacity"
}
```

### SNAPSHOT (Router → Client)

Current state dump (response to GET or on connect).

```javascript
{
  type: "SNAPSHOT",
  params: [
    {
      address: "/app/scene/0/opacity",
      value: 0.75,
      revision: 42
    }
  ]
}
```

### PUBLISH

Send an ephemeral signal (Event, Stream, or Gesture).

```javascript
// Event
{
  type: "PUBLISH",
  address: "/app/cue/fire",
  payload: { cue: "intro", transition: "fade" },
  timestamp: 1704067200000000
}

// Stream sample
{
  type: "PUBLISH",
  address: "/controller/fader/1",
  samples: [0.50, 0.52, 0.55],
  rate: 60,
  timestamp: 1704067200000000
}

// Gesture
{
  type: "PUBLISH",
  address: "/input/touch",
  id: 1,
  phase: "move",
  payload: { position: [0.5, 0.3], pressure: 0.8 },
  timestamp: 1704067200000000
}
```

## Bundle Messages

### BUNDLE

Atomic group of messages with optional scheduled execution.

```javascript
{
  type: "BUNDLE",
  timestamp: 1704067300000000,
  messages: [
    { type: "SET", address: "/light/1/intensity", value: 1.0 },
    { type: "SET", address: "/light/2/intensity", value: 0.0 },
    { type: "PUBLISH", address: "/cue/fire", payload: { id: "intro" } }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | uint64 | Optional: execute at this time |
| `messages` | array | Array of messages to execute atomically |

## Timing Messages

### SYNC

Clock synchronization using NTP-like algorithm.

```
Client                              Router
  │                                    │
  │── SYNC { t1: T1 } ────────────────►│
  │                                    │ (receives at T2)
  │◄── SYNC { t1:T1, t2:T2, t3:T3 } ───│ (sends at T3)
  │                                    │
  │ (receives at T4)                   │
```

Offset calculation:
```
roundTrip = (T4 - T1) - (T3 - T2)
offset = ((T2 - T1) + (T3 - T4)) / 2
```

### PING / PONG

Keepalive messages.

```javascript
{ type: "PING", timestamp: 1704067200000000 }
{ type: "PONG", timestamp: 1704067200000000 }
```

## Response Messages

### ACK

Acknowledgment for QoS Confirm/Commit messages.

```javascript
{
  type: "ACK",
  correlationId: 42,
  revision: 43
}
```

### ERROR

Error response.

```javascript
{
  type: "ERROR",
  code: 403,
  message: "Permission denied",
  address: "/admin/config",
  correlationId: 42
}
```

**Error code ranges:**
- 100-199: Protocol errors
- 200-299: Address errors
- 300-399: Permission errors
- 400-499: State errors
- 500-599: Router errors

**Common error codes:**

| Code | Name | Description |
|------|------|-------------|
| 400 | Bad Request | Invalid message format or parameters |
| 403 | Forbidden | Permission denied for this operation |
| 404 | Not Found | Address or resource not found |
| 409 | Conflict | Revision conflict (optimistic locking) |
| 423 | Locked | Parameter is locked by another session |
| 503 | Buffer Overflow | Client buffer full, messages being dropped |

#### Buffer Overflow Notification (503)

When a client's receive buffer fills up and messages are being dropped, the router sends an ERROR 503 notification:

```javascript
{
  type: "ERROR",
  code: 503,
  message: "Buffer overflow: messages being dropped (100 drops in last 10 seconds)"
}
```

This notification is rate-limited to 1 per 10 seconds per session to avoid flooding slow clients. Clients receiving this error should:

1. Process incoming messages faster
2. Reduce subscription scope
3. Increase local buffer size if possible

## Introspection Messages

### QUERY (Client → Router)

Request information about available signals.

```javascript
{
  type: "QUERY",
  pattern: "/app/**"
}
```

### RESULT (Router → Client)

Query response.

```javascript
{
  type: "RESULT",
  signals: [
    { address: "/app/scene/0/opacity", type: "param", datatype: "f64" }
  ]
}
```
