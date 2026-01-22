/**
 * CLASP Protocol v3 - Minimal Reference Implementation
 *
 * This demonstrates that CLASP can be implemented in ~400 lines.
 * Production implementations should add error handling, reconnection, etc.
 *
 * v3 uses efficient binary encoding (54% smaller than v2 MessagePack).
 * Backward compatible: can decode v2 MessagePack frames.
 */

const msgpack = require('msgpack-lite');  // npm install msgpack-lite (for v2 compat)

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

// Value type codes (v3 binary)
const VAL = {
  NULL: 0x00,
  BOOL: 0x01,
  I64: 0x05,
  F64: 0x07,
  STRING: 0x08,
  BYTES: 0x09,
  ARRAY: 0x0A,
  MAP: 0x0B
};

// QoS levels
const QOS = {
  FIRE: 0,      // Best effort
  CONFIRM: 1,   // At least once
  COMMIT: 2     // Exactly once, ordered
};

/**
 * Encode a CLASP v3 binary frame
 */
function encodeFrame(message, options = {}) {
  const payload = encodeMessageV3(message);

  let flags = 0;
  flags |= (options.qos || QOS.FIRE) << 6;
  if (options.timestamp) flags |= 0x20;
  if (options.encrypted) flags |= 0x10;
  if (options.compressed) flags |= 0x08;
  flags |= 0x01;  // Version = 1 (v3 binary)

  const headerSize = options.timestamp ? 12 : 4;
  const frame = Buffer.alloc(headerSize + payload.length);

  frame[0] = 0x53;  // Magic 'S' (for Streaming)
  frame[1] = flags;
  frame.writeUInt16BE(payload.length, 2);

  if (options.timestamp) {
    const ts = BigInt(options.timestamp);
    frame.writeBigUInt64BE(ts, 4);
    payload.copy(frame, 12);
  } else {
    payload.copy(frame, 4);
  }

  return frame;
}

/**
 * Decode a CLASP frame - auto-detects v2 vs v3
 */
function decodeFrame(buffer) {
  if (buffer[0] !== 0x53) {
    throw new Error('Invalid magic byte');
  }

  const flags = buffer[1];
  const qos = (flags >> 6) & 0x03;
  const hasTimestamp = (flags & 0x20) !== 0;
  const version = flags & 0x07;
  const payloadLength = buffer.readUInt16BE(2);

  let timestamp = null;
  let payloadOffset = 4;

  if (hasTimestamp) {
    timestamp = Number(buffer.readBigUInt64BE(4));
    payloadOffset = 12;
  }

  const payload = buffer.slice(payloadOffset, payloadOffset + payloadLength);

  // Check if v2 MessagePack (first byte is fixmap or map)
  const first = payload[0];
  if ((first & 0xF0) === 0x80 || first === 0xDE || first === 0xDF) {
    return { message: msgpack.decode(payload), qos, timestamp };
  }

  // v3 binary
  return { message: decodeMessageV3(payload), qos, timestamp };
}

/**
 * Encode message to v3 binary
 */
function encodeMessageV3(msg) {
  const parts = [];

  if (msg.type === MSG.SET || msg.type === 'SET') {
    const buf = Buffer.alloc(256);
    let offset = 0;
    buf[offset++] = MSG.SET;

    // Flags: [has_rev:1][lock:1][unlock:1][rsv:1][vtype:4]
    const vtype = getValueType(msg.value);
    let flags = vtype & 0x0F;
    if (msg.revision !== undefined) flags |= 0x80;
    if (msg.lock) flags |= 0x40;
    if (msg.unlock) flags |= 0x20;
    buf[offset++] = flags;

    offset = encodeString(buf, offset, msg.address);
    offset = encodeValue(buf, offset, msg.value);

    if (msg.revision !== undefined) {
      buf.writeBigUInt64BE(BigInt(msg.revision), offset);
      offset += 8;
    }

    return buf.slice(0, offset);
  }

  if (msg.type === MSG.PUBLISH || msg.type === 'PUBLISH') {
    const buf = Buffer.alloc(512);
    let offset = 0;
    buf[offset++] = MSG.PUBLISH;

    const sigCode = { param: 0, event: 1, stream: 2, gesture: 3, timeline: 4 }[msg.signal || 'event'] || 1;
    const phaseCode = { start: 0, move: 1, end: 2, cancel: 3 }[msg.phase || 'start'] || 0;

    let flags = (sigCode & 0x07) << 5;
    if (msg.timestamp !== undefined) flags |= 0x10;
    if (msg.id !== undefined) flags |= 0x08;
    flags |= phaseCode & 0x07;
    buf[offset++] = flags;

    offset = encodeString(buf, offset, msg.address);

    if (msg.value !== undefined) {
      buf[offset++] = 1;
      buf[offset++] = getValueType(msg.value);
      offset = encodeValue(buf, offset, msg.value);
    } else if (msg.payload !== undefined) {
      buf[offset++] = 1;
      buf[offset++] = getValueType(msg.payload);
      offset = encodeValue(buf, offset, msg.payload);
    } else {
      buf[offset++] = 0;
    }

    if (msg.timestamp !== undefined) {
      buf.writeBigUInt64BE(BigInt(msg.timestamp), offset);
      offset += 8;
    }
    if (msg.id !== undefined) {
      buf.writeUInt32BE(msg.id, offset);
      offset += 4;
    }

    return buf.slice(0, offset);
  }

  if (msg.type === MSG.HELLO || msg.type === 'HELLO') {
    const buf = Buffer.alloc(256);
    let offset = 0;
    buf[offset++] = MSG.HELLO;
    buf[offset++] = msg.version || 3;

    let features = 0;
    for (const f of msg.features || []) {
      if (f === 'param') features |= 0x80;
      if (f === 'event') features |= 0x40;
      if (f === 'stream') features |= 0x20;
      if (f === 'gesture') features |= 0x10;
      if (f === 'timeline') features |= 0x08;
    }
    buf[offset++] = features;

    offset = encodeString(buf, offset, msg.name || '');
    offset = encodeString(buf, offset, msg.token || '');

    return buf.slice(0, offset);
  }

  if (msg.type === MSG.SUBSCRIBE || msg.type === 'SUBSCRIBE') {
    const buf = Buffer.alloc(256);
    let offset = 0;
    buf[offset++] = MSG.SUBSCRIBE;
    buf.writeUInt32BE(msg.id, offset);
    offset += 4;
    offset = encodeString(buf, offset, msg.pattern);
    buf[offset++] = 0xFF;  // All types
    buf[offset++] = 0;     // No options
    return buf.slice(0, offset);
  }

  if (msg.type === MSG.UNSUBSCRIBE || msg.type === 'UNSUBSCRIBE') {
    const buf = Buffer.alloc(8);
    buf[0] = MSG.UNSUBSCRIBE;
    buf.writeUInt32BE(msg.id, 1);
    return buf.slice(0, 5);
  }

  if (msg.type === MSG.PING || msg.type === 'PING') {
    return Buffer.from([MSG.PING]);
  }

  if (msg.type === MSG.PONG || msg.type === 'PONG') {
    return Buffer.from([MSG.PONG]);
  }

  // Fall back to MessagePack for other types
  return msgpack.encode(msg);
}

/**
 * Decode v3 binary message
 */
function decodeMessageV3(buf) {
  const msgType = buf[0];
  let offset = 1;

  if (msgType === MSG.SET) {
    const flags = buf[offset++];
    const vtype = flags & 0x0F;
    const hasRev = (flags & 0x80) !== 0;
    const lock = (flags & 0x40) !== 0;
    const unlock = (flags & 0x20) !== 0;

    const [address, o1] = decodeString(buf, offset);
    offset = o1;
    const [value, o2] = decodeValue(buf, offset, vtype);
    offset = o2;

    const revision = hasRev ? Number(buf.readBigUInt64BE(offset)) : undefined;

    return { type: MSG.SET, address, value, revision, lock, unlock };
  }

  if (msgType === MSG.PUBLISH) {
    const flags = buf[offset++];
    const sigCode = (flags >> 5) & 0x07;
    const hasTs = (flags & 0x10) !== 0;
    const hasId = (flags & 0x08) !== 0;
    const phaseCode = flags & 0x07;

    const [address, o1] = decodeString(buf, offset);
    offset = o1;

    const valIndicator = buf[offset++];
    let value = undefined;
    if (valIndicator === 1) {
      const vtype = buf[offset++];
      const [v, o] = decodeValue(buf, offset, vtype);
      value = v;
      offset = o;
    }

    const timestamp = hasTs ? Number(buf.readBigUInt64BE(offset)) : undefined;
    if (hasTs) offset += 8;

    const id = hasId ? buf.readUInt32BE(offset) : undefined;

    const signals = ['param', 'event', 'stream', 'gesture', 'timeline'];
    const phases = ['start', 'move', 'end', 'cancel'];

    return { type: MSG.PUBLISH, address, signal: signals[sigCode], value, timestamp, id, phase: phases[phaseCode] };
  }

  if (msgType === MSG.WELCOME) {
    const version = buf[offset++];
    const featureFlags = buf[offset++];

    const features = [];
    if (featureFlags & 0x80) features.push('param');
    if (featureFlags & 0x40) features.push('event');
    if (featureFlags & 0x20) features.push('stream');
    if (featureFlags & 0x10) features.push('gesture');
    if (featureFlags & 0x08) features.push('timeline');

    const time = Number(buf.readBigUInt64BE(offset));
    offset += 8;

    const [session, o1] = decodeString(buf, offset);
    offset = o1;
    const [name, o2] = decodeString(buf, offset);
    offset = o2;
    const [token, _] = decodeString(buf, offset);

    return { type: MSG.WELCOME, version, session, name, features, time, token: token || undefined };
  }

  if (msgType === MSG.PING) return { type: MSG.PING };
  if (msgType === MSG.PONG) return { type: MSG.PONG };

  if (msgType === MSG.ERROR) {
    const code = buf.readUInt16BE(offset);
    offset += 2;
    const [message, o1] = decodeString(buf, offset);
    return { type: MSG.ERROR, code, message };
  }

  throw new Error(`Unknown message type: 0x${msgType.toString(16)}`);
}

function encodeString(buf, offset, str) {
  const encoded = Buffer.from(str, 'utf8');
  buf.writeUInt16BE(encoded.length, offset);
  offset += 2;
  encoded.copy(buf, offset);
  return offset + encoded.length;
}

function decodeString(buf, offset) {
  const len = buf.readUInt16BE(offset);
  offset += 2;
  const str = buf.toString('utf8', offset, offset + len);
  return [str, offset + len];
}

function encodeValue(buf, offset, value) {
  if (value === null) return offset;
  if (typeof value === 'boolean') {
    buf[offset++] = value ? 1 : 0;
    return offset;
  }
  if (typeof value === 'number') {
    buf.writeDoubleBE(value, offset);
    return offset + 8;
  }
  if (typeof value === 'string') {
    return encodeString(buf, offset, value);
  }
  if (Array.isArray(value)) {
    buf.writeUInt16BE(value.length, offset);
    offset += 2;
    for (const item of value) {
      buf[offset++] = getValueType(item);
      offset = encodeValue(buf, offset, item);
    }
    return offset;
  }
  if (typeof value === 'object') {
    const keys = Object.keys(value);
    buf.writeUInt16BE(keys.length, offset);
    offset += 2;
    for (const k of keys) {
      offset = encodeString(buf, offset, k);
      buf[offset++] = getValueType(value[k]);
      offset = encodeValue(buf, offset, value[k]);
    }
    return offset;
  }
  return offset;
}

function decodeValue(buf, offset, vtype) {
  if (vtype === VAL.NULL) return [null, offset];
  if (vtype === VAL.BOOL) return [buf[offset] !== 0, offset + 1];
  if (vtype === VAL.I64) return [Number(buf.readBigInt64BE(offset)), offset + 8];
  if (vtype === VAL.F64) return [buf.readDoubleBE(offset), offset + 8];
  if (vtype === VAL.STRING) return decodeString(buf, offset);
  if (vtype === VAL.BYTES) {
    const len = buf.readUInt16BE(offset);
    offset += 2;
    return [buf.slice(offset, offset + len), offset + len];
  }
  if (vtype === VAL.ARRAY) {
    const count = buf.readUInt16BE(offset);
    offset += 2;
    const arr = [];
    for (let i = 0; i < count; i++) {
      const itemType = buf[offset++];
      const [item, o] = decodeValue(buf, offset, itemType);
      arr.push(item);
      offset = o;
    }
    return [arr, offset];
  }
  if (vtype === VAL.MAP) {
    const count = buf.readUInt16BE(offset);
    offset += 2;
    const map = {};
    for (let i = 0; i < count; i++) {
      const [key, o1] = decodeString(buf, offset);
      offset = o1;
      const valType = buf[offset++];
      const [val, o2] = decodeValue(buf, offset, valType);
      map[key] = val;
      offset = o2;
    }
    return [map, offset];
  }
  return [null, offset];
}

function getValueType(value) {
  if (value === null) return VAL.NULL;
  if (typeof value === 'boolean') return VAL.BOOL;
  if (typeof value === 'number') return VAL.F64;
  if (typeof value === 'string') return VAL.STRING;
  if (Buffer.isBuffer(value)) return VAL.BYTES;
  if (Array.isArray(value)) return VAL.ARRAY;
  if (typeof value === 'object') return VAL.MAP;
  return VAL.NULL;
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

      this.ws = new WebSocket(this.url, 'clasp');
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
  async query(pattern, timeout = 5000) {
    return new Promise((resolve, reject) => {
      let timeoutId;

      const messageHandler = (event) => {
        try {
          const { message } = decodeFrame(Buffer.from(event.data));
          if (message.type === MSG.RESULT) {
            cleanup();
            resolve(message.signals || []);
          }
        } catch (e) {
          // Ignore decode errors for non-RESULT messages
        }
      };

      const cleanup = () => {
        if (timeoutId) clearTimeout(timeoutId);
        this.ws.removeEventListener('message', messageHandler);
      };

      // Set timeout to prevent hanging promises
      timeoutId = setTimeout(() => {
        cleanup();
        reject(new Error('Query timeout'));
      }, timeout);

      this.ws.addEventListener('message', messageHandler);
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
