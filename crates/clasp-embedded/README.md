# clasp-embedded

Minimal `no_std` implementation of the **standard CLASP v3 protocol** for embedded devices.

## Features

- **v3 Protocol Compatible** - Same wire format as desktop/cloud CLASP
- **no_std + no_alloc** - Works without heap allocation
- **Client + Server** - Both modes supported
- **~3KB RAM** - Suitable for ESP32, RP2040, etc.

## Usage

### Client Mode

```rust
#![no_std]

use clasp_embedded::{Client, Value, HEADER_SIZE};

fn main() {
    let mut client = Client::new();
    
    // Prepare HELLO frame to send
    let hello = client.prepare_hello("ESP32-Sensor");
    // send(hello) via your transport...
    
    // Prepare SET frame
    let set = client.prepare_set("/sensor/temp", Value::Float(25.5));
    // send(set)...
    
    // Process received data
    let received_bytes: &[u8] = /* from transport */;
    if let Some(msg) = client.process(received_bytes) {
        match msg {
            Message::Set { address, value } => {
                // Handle incoming SET
            }
            Message::Welcome { .. } => {
                // Connected!
            }
            _ => {}
        }
    }
    
    // Read cached values
    if let Some(temp) = client.get_cached("/sensor/temp") {
        // Use temp.as_float()
    }
}
```

### Server Mode (MiniRouter)

```rust
#![no_std]

use clasp_embedded::server::MiniRouter;
use clasp_embedded::Value;

fn main() {
    let mut router = MiniRouter::new();
    
    // Set local state
    router.set("/light/brightness", Value::Float(0.8));
    
    // Process client message, get optional response
    let client_data: &[u8] = /* from transport */;
    if let Some(response) = router.process(0, client_data) {
        // send(response) back to client
    }
    
    // Read state
    if let Some(v) = router.get("/light/brightness") {
        // ...
    }
}
```

## Memory Budget

| Component | Size |
|-----------|------|
| `Client` | ~3KB |
| `MiniRouter` | ~4KB |
| State cache (32 entries) | ~2KB |

**ESP32:** Uses <2% of available 320KB SRAM.

## Features (Cargo.toml)

```toml
[dependencies]
clasp-embedded = { version = "0.1", features = ["client"] }

# Or for server mode:
clasp-embedded = { version = "0.1", features = ["server"] }

# Or both:
clasp-embedded = { version = "0.1", features = ["client", "server"] }
```

## Protocol Compatibility

Messages are **100% compatible** with the full CLASP router. An ESP32 running `clasp-embedded` can:

1. Connect to a cloud/desktop CLASP router as a client
2. Act as a local hub (MiniRouter) that sensors connect to
3. Forward messages to a main router

## Supported Platforms

- ESP32 (Xtensa, RISC-V)
- RP2040 / Raspberry Pi Pico
- ARM Cortex-M
- Any platform with `no_std` Rust support

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

Maintained by [LumenCanvas](https://lumencanvas.studio)
