# clasp-cli

Command-line interface for CLASP protocol servers and bridges.

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

### Start Protocol Servers

```bash
# Start an OSC server
clasp osc --port 9000

# Start an MQTT connection
clasp mqtt --host localhost --port 1883 --topic "sensors/#"

# Start a WebSocket server
clasp websocket --mode server --url 0.0.0.0:8080

# Start an HTTP REST API
clasp http --bind 0.0.0.0:3000
```

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
