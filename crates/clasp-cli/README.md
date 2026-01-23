# clasp-cli

Command-line interface for CLASP protocol routers and connections.

## Installation

```bash
cargo install clasp-cli
```

Or build from source:

```bash
git clone https://github.com/lumencanvas/clasp.git
cd clasp
cargo install --path crates/clasp-cli
```

## Commands

### Start CLASP Router

**Important:** You need a CLASP router running before protocol connections can work.

```bash
# Start a CLASP router (required - central message hub)
clasp server --port 7330

# Start router with specific transport
clasp server --protocol websocket --bind 0.0.0.0 --port 7330
```

### Start Protocol Connections

**Note:** These commands create **protocol connections** that connect to the CLASP router. Each connection translates bidirectionally between its protocol and CLASP.

```bash
# Start an OSC connection (listens for OSC, routes to CLASP router)
clasp osc --port 9000

# Start an MQTT connection (connects to broker, routes to CLASP router)
clasp mqtt --host localhost --port 1883 --topic "sensors/#"

# Start a WebSocket connection
clasp websocket --mode server --url 0.0.0.0:8080

# Start an HTTP REST API connection
clasp http --bind 0.0.0.0:3000
```

**How it works:**
```
External Protocol ←→ Protocol Connection ←→ CLASP Router ←→ Other Connections/Clients
```

For example, `clasp osc --port 9000`:
- Listens for OSC messages on UDP port 9000
- Connects to CLASP router (default: localhost:7330)
- Translates OSC ↔ CLASP bidirectionally
- Routes through CLASP router to other clients/connections

### Publish/Subscribe

```bash
# Publish a value
clasp pub /lights/brightness 0.75

# Subscribe to an address pattern
clasp sub "/lights/**"
```

### Create Bridges

```bash
# Bridge OSC to MQTT
clasp bridge --source osc:0.0.0.0:9000 --target mqtt:localhost:1883
```

### Configuration

```bash
# Show current configuration
clasp info

# Start with config file
clasp server --config clasp.toml
```

## Options

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Enable verbose logging |
| `--json` | Output in JSON format |
| `--config` | Path to configuration file |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

Maintained by [LumenCanvas](https://lumencanvas.studio) | 2026
