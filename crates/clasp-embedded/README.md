# clasp-embedded

Embedded/no_std support for CLASP (Creative Low-Latency Application Streaming Protocol).

## Features

- **no_std Compatible** - Works without the standard library
- **Minimal Footprint** - Optimized for resource-constrained devices
- **ESP32/ARM Support** - Tested on common embedded platforms

## Usage

```rust
#![no_std]

use clasp_embedded::ClaspEmbedded;

fn main() {
    let clasp = ClaspEmbedded::new();

    // Encode a message
    let mut buffer = [0u8; 256];
    let len = clasp.encode_set("/sensor/temp", 25.5, &mut buffer);

    // Send buffer over your transport...
}
```

## Memory Requirements

- Minimum RAM: ~4KB
- Flash: ~20KB (with all features)

## Supported Platforms

- ESP32 (Xtensa)
- ARM Cortex-M (thumbv7em)
- RISC-V

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

Maintained by [LumenCanvas](https://lumencanvas.studio) | 2026
