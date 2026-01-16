# clasp-wasm

WebAssembly bindings for CLASP (Creative Low-Latency Application Streaming Protocol).

## Features

- **Browser Support** - Use CLASP directly in web browsers
- **wasm-bindgen** - Seamless JavaScript interop
- **Async/Await** - Native Promise support

## Installation

```bash
npm install @clasp-to/wasm
```

Or build from source:

```bash
wasm-pack build --target web
```

## Usage

```javascript
import init, { ClaspWasm } from '@clasp-to/wasm';

await init();

const client = new ClaspWasm('ws://localhost:7330');
await client.connect();

// Set a parameter
await client.set('/lights/brightness', 0.75);

// Get a parameter
const value = await client.get('/lights/brightness');

// Subscribe to changes
client.subscribe('/lights/*', (value, address) => {
  console.log(`${address} = ${value}`);
});

await client.close();
```

## Building

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
wasm-pack build --target web

# Build for Node.js
wasm-pack build --target nodejs
```

## Documentation

Visit **[clasp.to](https://clasp.to)** for full documentation.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

Maintained by [LumenCanvas](https://lumencanvas.studio) | 2026
