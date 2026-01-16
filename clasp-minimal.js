/**
 * CLASP Protocol v2 - Minimal Reference Implementation
 *
 * This demonstrates that CLASP can be implemented in ~300 lines.
 * Production implementations should add error handling, reconnection, etc.
 */

const msgpack = require('msgpack-lite');  // npm install msgpack-lite

// Message type codes
const MSG = {
  HELLO: 0x01,
  WELCOME: 0x02,
  ANNOUNCE: 0x03,
  SUBSCRIBE: 0x10,
  UNSUBSCRIBE: 0x11,
  PUBLISH: 0x20,
  SET: 0x21,
  GET: 0x22,
  SNAPSHOT: 0x23,
  BUNDLE: 0x30,
  SYNC: 0x40,
  PING: 0x41,
  PONG: 0x42,
  ACK: 0x50,
  ERROR: 0x51,
  QUERY: 0x60,
  RESULT: 0x61
};

// QoS levels
const QOS = {
  FIRE: 0,      // Best effort
  CONFIRM: 1,   // At least once
  COMMIT: 2     // Exactly once, ordered
};

/**
 * Encode a CLASP frame
 */
function encodeFrame(message, options = {}) {
  const payload = msgpack.encode(message);

  let flags = 0;
  flags |= (options.qos || QOS.FIRE) << 6;
  if (options.timestamp) flags |= 0x20;
  if (options.encrypted) flags |= 0x10;
  if (options.compressed) flags |= 0x08;

  const headerSize = options.timestamp ? 12 : 4;
  const frame = Buffer.alloc(headerSize + payload.length);

  frame[0] = 0x43;  // Magic 'C'
  frame[1] = flags;
  frame.writeUInt16BE(payload.length, 2);

  if (options.timestamp) {
    // Write 64-bit timestamp
    const ts = BigInt(options.timestamp);
    frame.writeBigUInt64BE(ts, 4);
    payload.copy(frame, 12);
  } else {
    payload.copy(frame, 4);
  }

  return frame;
}

/**
 * Decode a CLASP frame
 */
function decodeFrame(buffer) {
  if (buffer[0] !== 0x43) {
    throw new Error('Invalid magic byte');
  }

  const flags = buffer[1];
  const qos = (flags >> 6) & 0x03;
  const hasTimestamp = (flags & 0x20) !== 0;
  const payloadLength = buffer.readUInt16BE(2);

  let timestamp = null;
  let payloadOffset = 4;

  if (hasTimestamp) {
    timestamp = Number(buffer.readBigUInt64BE(4));
    payloadOffset = 12;
  }

  const payload = buffer.slice(payloadOffset, payloadOffset + payloadLength);
  const message = msgpack.decode(payload);

  return { message, qos, timestamp };
}

/**
 * CLASP Client
 */
class Clasp {
  constructor(url) {
    this.url = url;
    this.ws = null;
    this.session = null;
    this.subscriptions = new Map();
    this.params = new Map();
    this.callbacks = {
      connect: [],
      disconnect: [],
      error: []
    };
    this.subId = 0;
    this.serverTimeOffset = 0;
  }

  // Connect to server
  connect() {
    return new Promise((resolve, reject) => {
      // In browser: use native WebSocket
      // In Node: use 'ws' package
      const WebSocket = typeof window !== 'undefined'
        ? window.WebSocket
        : require('ws');

      this.ws = new WebSocket(this.url, 'clasp.v2');
      this.ws.binaryType = 'arraybuffer';

      this.ws.onopen = () => {
        this._sendHello();
      };

      this.ws.onmessage = (event) => {
        const buffer = Buffer.from(event.data);
        const { message, qos, timestamp } = decodeFrame(buffer);
        this._handleMessage(message, qos, timestamp);

        // Resolve on WELCOME
        if (message.type === MSG.WELCOME) {
          this.session = message.session;
          resolve(this);
          this.callbacks.connect.forEach(cb => cb());
        }
      };

      this.ws.onerror = (err) => {
        this.callbacks.error.forEach(cb => cb(err));
        reject(err);
      };

      this.ws.onclose = () => {
        this.callbacks.disconnect.forEach(cb => cb());
      };
    });
  }

  // Send HELLO
  _sendHello() {
    this._send({
      type: MSG.HELLO,
      version: 2,
      name: 'CLASP JS Client',
      features: ['param', 'event', 'stream']
    });
  }

  // Handle incoming message
  _handleMessage(msg, qos, timestamp) {
    switch (msg.type) {
      case MSG.WELCOME:
        this.serverTimeOffset = msg.time - Date.now() * 1000;
        break;

      case MSG.SET:
      case MSG.PUBLISH:
        // Update local state
        if (msg.address) {
          this.params.set(msg.address, {
            value: msg.value,
            revision: msg.revision,
            timestamp
          });
        }
        // Notify subscribers
        this._notifySubscribers(msg.address, msg.value, msg);
        break;

      case MSG.SNAPSHOT:
        if (msg.params) {
          msg.params.forEach(p => {
            this.params.set(p.address, p);
            this._notifySubscribers(p.address, p.value, p);
          });
        }
        break;

      case MSG.ERROR:
        console.error('CLASP error:', msg.code, msg.message);
        this.callbacks.error.forEach(cb => cb(msg));
        break;

      case MSG.PING:
        this._send({ type: MSG.PONG });
        break;
    }
  }

  // Notify matching subscribers
  _notifySubscribers(address, value, meta) {
    this.subscriptions.forEach((sub, pattern) => {
      if (this._matchPattern(pattern, address)) {
        sub.callback(value, address, meta);
      }
    });
  }

  // Pattern matching (simple glob)
  _matchPattern(pattern, address) {
    const regex = pattern
      .replace(/\*/g, '[^/]+')
      .replace(/\*\*/g, '.*');
    return new RegExp(`^${regex}$`).test(address);
  }

  // Send message
  _send(message, options = {}) {
    if (this.ws && this.ws.readyState === 1) {
      const frame = encodeFrame(message, options);
      this.ws.send(frame);
    }
  }

  // Public API: Subscribe
  subscribe(pattern, callback, options = {}) {
    const id = ++this.subId;

    this.subscriptions.set(pattern, { id, callback, options });

    this._send({
      type: MSG.SUBSCRIBE,
      id,
      pattern,
      types: options.types || ['param', 'event', 'stream'],
      options: {
        maxRate: options.maxRate,
        epsilon: options.epsilon,
        history: options.history
      }
    });

    // Return unsubscribe function
    return () => {
      this.subscriptions.delete(pattern);
      this._send({ type: MSG.UNSUBSCRIBE, id });
    };
  }

  // Alias for subscribe
  on(pattern, callback, options) {
    return this.subscribe(pattern, callback, options);
  }

  // Public API: Set param
  set(address, value, options = {}) {
    this._send({
      type: MSG.SET,
      address,
      value,
      lock: options.lock
    }, { qos: QOS.CONFIRM });
  }

  // Public API: Emit event
  emit(address, payload) {
    this._send({
      type: MSG.PUBLISH,
      address,
      signal: 'event',
      payload
    }, { qos: QOS.CONFIRM });
  }

  // Public API: Stream sample
  stream(address, value) {
    this._send({
      type: MSG.PUBLISH,
      address,
      signal: 'stream',
      value
    }, { qos: QOS.FIRE });
  }

  // Public API: Get current value
  async get(address) {
    // Return cached value if available
    if (this.params.has(address)) {
      return this.params.get(address).value;
    }

    // Request from server
    return new Promise((resolve) => {
      const unsub = this.on(address, (value) => {
        unsub();
        resolve(value);
      }, { history: 1 });
    });
  }

  // Public API: Bundle
  bundle(messages, options = {}) {
    const formatted = messages.map(m => {
      if (m.set) return { type: MSG.SET, address: m.set[0], value: m.set[1] };
      if (m.emit) return { type: MSG.PUBLISH, address: m.emit[0], payload: m.emit[1], signal: 'event' };
      return m;
    });

    this._send({
      type: MSG.BUNDLE,
      timestamp: options.at,
      messages: formatted
    }, {
      qos: QOS.COMMIT,
      timestamp: options.at
    });
  }

  // Public API: Current time (server-synced)
  time() {
    return Date.now() * 1000 + this.serverTimeOffset;
  }

  // Public API: Query signals
  async query(pattern) {
    return new Promise((resolve) => {
      const handler = (msg) => {
        if (msg.type === MSG.RESULT) {
          this.ws.removeEventListener('message', handler);
          resolve(msg.signals || []);
        }
      };
      this.ws.addEventListener('message', (event) => {
        const { message } = decodeFrame(Buffer.from(event.data));
        handler(message);
      });

      this._send({ type: MSG.QUERY, pattern });
    });
  }

  // Event handlers
  onConnect(callback) { this.callbacks.connect.push(callback); }
  onDisconnect(callback) { this.callbacks.disconnect.push(callback); }
  onError(callback) { this.callbacks.error.push(callback); }

  // Close connection
  close() {
    if (this.ws) {
      this.ws.close();
    }
  }
}

// Export
module.exports = { Clasp, encodeFrame, decodeFrame, MSG, QOS };

// Usage example (when run directly)
if (require.main === module) {
  console.log(`
CLASP v2 Minimal Implementation

Usage:
  const { Clasp } = require('./clasp-minimal');

  const clasp = new Clasp('wss://localhost:7330');
  await clasp.connect();

  // Subscribe
  clasp.on('/lumen/scene/*/layer/*/opacity', (value, address) => {
    console.log(\`\${address} = \${value}\`);
  });

  // Set value
  clasp.set('/lumen/scene/0/layer/0/opacity', 0.5);

  // Emit event
  clasp.emit('/lumen/cue/fire', { cue: 'intro' });

  // Bundle with timing
  clasp.bundle([
    { set: ['/light/1', 1.0] },
    { set: ['/light/2', 0.0] }
  ], { at: clasp.time() + 100000 });  // 100ms from now
`);
}
