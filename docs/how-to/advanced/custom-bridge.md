# Custom Bridge

Build a custom protocol bridge to connect CLASP with any external system.

## Overview

Bridges translate between CLASP and external protocols. Build a custom bridge when:

- Connecting to proprietary hardware
- Integrating with non-standard protocols
- Adding custom data transformation

## Bridge Architecture

```
External System ←→ Bridge ←→ CLASP Router
                    │
              Translation
               Layer
```

## Basic Bridge Structure

### Rust

```rust
use clasp_bridge::{Bridge, BridgeConfig, Message};
use clasp_client::{Clasp, ClaspBuilder};

struct MyBridge {
    client: Clasp,
    external: ExternalConnection,
}

impl Bridge for MyBridge {
    async fn handle_clasp_message(&mut self, msg: Message) -> Result<()> {
        // Convert CLASP message to external format
        let external_msg = self.translate_to_external(&msg);
        self.external.send(external_msg).await?;
        Ok(())
    }

    async fn handle_external_message(&mut self, data: Vec<u8>) -> Result<()> {
        // Convert external message to CLASP format
        let (address, value) = self.translate_to_clasp(&data);
        self.client.set(&address, value).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = ClaspBuilder::new("ws://localhost:7330")
        .name("My Bridge")
        .connect()
        .await?;
    let external = ExternalConnection::connect("192.168.1.50:5000").await?;

    let mut bridge = MyBridge { client, external };
    bridge.run().await
}
```

### JavaScript

```javascript
const { Clasp } = require('@clasp-to/core');

class MyBridge {
  constructor(claspUrl, externalConfig) {
    this.claspUrl = claspUrl;
    this.externalConfig = externalConfig;
  }

  async start() {
    // Connect to CLASP
    this.clasp = await Clasp.connect(this.claspUrl);

    // Connect to external system
    this.external = await this.connectExternal();

    // Set up bidirectional routing
    this.setupClaspToExternal();
    this.setupExternalToClasp();

    console.log('Bridge running');
  }

  setupClaspToExternal() {
    // Subscribe to CLASP addresses and forward to external
    this.clasp.on('/bridge/out/**', async (value, address) => {
      const externalMsg = this.translateToExternal(address, value);
      await this.external.send(externalMsg);
    });
  }

  setupExternalToClasp() {
    // Receive from external and forward to CLASP
    this.external.on('message', async (data) => {
      const { address, value } = this.translateToClasp(data);
      await this.clasp.set(address, value);
    });
  }

  translateToExternal(address, value) {
    // Implement your translation logic
    return {
      command: address.split('/').pop(),
      payload: value
    };
  }

  translateToClasp(data) {
    // Implement your translation logic
    return {
      address: `/bridge/in/${data.type}`,
      value: data.payload
    };
  }
}

// Usage
const bridge = new MyBridge('ws://localhost:7330', { host: '192.168.1.50' });
bridge.start();
```

## Example: Serial Port Bridge

Connect serial devices to CLASP:

```javascript
const { Clasp } = require('@clasp-to/core');
const { SerialPort } = require('serialport');
const { ReadlineParser } = require('@serialport/parser-readline');

class SerialBridge {
  constructor(claspUrl, portPath, baudRate = 9600) {
    this.claspUrl = claspUrl;
    this.portPath = portPath;
    this.baudRate = baudRate;
  }

  async start() {
    // Connect to CLASP
    this.clasp = await Clasp.connect(this.claspUrl);

    // Open serial port
    this.port = new SerialPort({
      path: this.portPath,
      baudRate: this.baudRate
    });

    const parser = this.port.pipe(new ReadlineParser({ delimiter: '\n' }));

    // Serial → CLASP
    parser.on('data', async (line) => {
      const { address, value } = this.parseSerialMessage(line);
      if (address) {
        await this.clasp.set(address, value);
      }
    });

    // CLASP → Serial
    this.clasp.on('/serial/out/**', async (value, address) => {
      const serialMsg = this.formatSerialMessage(address, value);
      this.port.write(serialMsg + '\n');
    });

    console.log(`Serial bridge running on ${this.portPath}`);
  }

  parseSerialMessage(line) {
    // Format: "sensor:temperature:23.5"
    const parts = line.trim().split(':');
    if (parts.length >= 2) {
      const address = `/serial/in/${parts.slice(0, -1).join('/')}`;
      const value = parseFloat(parts[parts.length - 1]) || parts[parts.length - 1];
      return { address, value };
    }
    return {};
  }

  formatSerialMessage(address, value) {
    // Format: "led:brightness:128"
    const parts = address.replace('/serial/out/', '').split('/');
    return `${parts.join(':')}:${value}`;
  }
}

const bridge = new SerialBridge('ws://localhost:7330', '/dev/ttyUSB0');
bridge.start();
```

## Example: HTTP API Bridge

Expose CLASP as a REST API:

```javascript
const { Clasp } = require('@clasp-to/core');
const express = require('express');

class HttpBridge {
  constructor(claspUrl, httpPort) {
    this.claspUrl = claspUrl;
    this.httpPort = httpPort;
    this.app = express();
    this.app.use(express.json());
  }

  async start() {
    this.clasp = await Clasp.connect(this.claspUrl);

    // GET /api/state/:address - Read value
    this.app.get('/api/state/*', async (req, res) => {
      try {
        const address = '/' + req.params[0];
        const value = await this.clasp.get(address);
        res.json({ address, value });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    });

    // PUT /api/state/:address - Set value
    this.app.put('/api/state/*', async (req, res) => {
      try {
        const address = '/' + req.params[0];
        await this.clasp.set(address, req.body.value);
        res.json({ success: true });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    });

    // POST /api/event/:address - Emit event
    this.app.post('/api/event/*', async (req, res) => {
      try {
        const address = '/' + req.params[0];
        await this.clasp.emit(address, req.body);
        res.json({ success: true });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    });

    // SSE for subscriptions
    this.app.get('/api/subscribe/*', (req, res) => {
      const address = '/' + req.params[0];

      res.setHeader('Content-Type', 'text/event-stream');
      res.setHeader('Cache-Control', 'no-cache');
      res.setHeader('Connection', 'keep-alive');

      const handler = (value, addr) => {
        res.write(`data: ${JSON.stringify({ address: addr, value })}\n\n`);
      };

      this.clasp.on(address, handler);

      req.on('close', () => {
        this.clasp.off(address, handler);
      });
    });

    this.app.listen(this.httpPort, () => {
      console.log(`HTTP bridge running on port ${this.httpPort}`);
    });
  }
}

const bridge = new HttpBridge('ws://localhost:7330', 3000);
bridge.start();
```

## Address Mapping

Define how external addresses map to CLASP:

```javascript
class MappedBridge {
  constructor() {
    // External → CLASP mapping
    this.inboundMap = {
      'sensor.temp': '/environment/temperature',
      'sensor.humidity': '/environment/humidity',
      'button.1': '/input/button/1'
    };

    // CLASP → External mapping
    this.outboundMap = {
      '/output/led/1': 'led.1',
      '/output/relay/1': 'relay.1'
    };
  }

  translateToClasp(externalAddress, value) {
    const claspAddress = this.inboundMap[externalAddress];
    if (!claspAddress) {
      // Dynamic mapping for unknown addresses
      return `/external/${externalAddress.replace(/\./g, '/')}`;
    }
    return claspAddress;
  }

  translateToExternal(claspAddress) {
    return this.outboundMap[claspAddress];
  }
}
```

## Value Transformation

Transform values between systems:

```javascript
const transformers = {
  // Temperature: Fahrenheit to Celsius
  '/sensors/temp': {
    inbound: (f) => (f - 32) * 5/9,
    outbound: (c) => c * 9/5 + 32
  },

  // Percentage: 0-255 to 0-1
  '/lights/brightness': {
    inbound: (v) => v / 255,
    outbound: (v) => Math.round(v * 255)
  },

  // Boolean: string to boolean
  '/switches/*': {
    inbound: (v) => v === 'ON' || v === '1',
    outbound: (v) => v ? 'ON' : 'OFF'
  }
};

function transformValue(address, value, direction) {
  // Find matching transformer
  for (const [pattern, transformer] of Object.entries(transformers)) {
    if (matchAddress(address, pattern)) {
      return transformer[direction](value);
    }
  }
  return value;  // No transformation
}
```

## Error Handling

Robust bridge error handling:

```javascript
class RobustBridge {
  async handleExternalMessage(data) {
    try {
      const { address, value } = this.translate(data);
      await this.clasp.set(address, value);
    } catch (error) {
      console.error('Translation error:', error);
      // Log to CLASP for monitoring
      await this.clasp.emit('/bridge/errors', {
        source: 'external',
        data: data.toString(),
        error: error.message
      });
    }
  }

  async reconnect() {
    let attempts = 0;
    while (attempts < 10) {
      try {
        await this.connect();
        console.log('Reconnected');
        return;
      } catch (error) {
        attempts++;
        await sleep(1000 * Math.pow(2, attempts));
      }
    }
    throw new Error('Failed to reconnect after 10 attempts');
  }
}
```

## Testing Bridges

```javascript
const { MockClasp, MockExternal } = require('./test-utils');

describe('MyBridge', () => {
  it('translates external to CLASP', async () => {
    const clasp = new MockClasp();
    const external = new MockExternal();
    const bridge = new MyBridge(clasp, external);

    external.emit('message', { type: 'sensor', value: 42 });

    expect(clasp.lastSet).toEqual({
      address: '/bridge/in/sensor',
      value: 42
    });
  });

  it('translates CLASP to external', async () => {
    const clasp = new MockClasp();
    const external = new MockExternal();
    const bridge = new MyBridge(clasp, external);

    await clasp.triggerSubscription('/bridge/out/led', 255);

    expect(external.lastSent).toEqual({
      command: 'led',
      payload: 255
    });
  });
});
```

## Next Steps

- [Embed Router](embed-router.md)
- [Performance Tuning](performance-tuning.md)
- [Bridge Reference](../../reference/bridges/)
