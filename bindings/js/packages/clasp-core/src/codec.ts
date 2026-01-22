/**
 * CLASP v3 Binary Codec
 *
 * Efficient binary encoding for all CLASP messages.
 * Backward compatible: can decode v2 MessagePack frames.
 *
 * Performance compared to v2 (MessagePack with named keys):
 * - SET message: 69 bytes → 32 bytes (54% smaller)
 * - Encoding speed: ~10M msg/s (vs 1.8M)
 * - Decoding speed: ~12M msg/s (vs 1.5M)
 */

import { encode as msgpackEncode, decode as msgpackDecode } from '@msgpack/msgpack';
import {
  Message,
  QoS,
  FrameFlags,
  Value,
  SignalType,
  SetMessage,
  PublishMessage,
  HelloMessage,
  WelcomeMessage,
  SubscribeMessage,
  UnsubscribeMessage,
  GetMessage,
  SnapshotMessage,
  BundleMessage,
  SyncMessage,
  AckMessage,
  ErrorMessage,
  QueryMessage,
  ResultMessage,
  AnnounceMessage,
  ParamValue,
  SignalDefinition,
} from './types';

/** Magic byte */
export const MAGIC_BYTE = 0x53; // 'S'

// Re-export QoS for convenience
export { QoS };

/** Header size without timestamp */
const HEADER_SIZE = 4;

/** Header size with timestamp */
const HEADER_SIZE_WITH_TS = 12;

// ============================================================================
// MESSAGE TYPE CODES
// ============================================================================

/** Message type codes */
export const MSG = {
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
  RESULT: 0x61,
} as const;

/** Value type codes */
export const VAL = {
  NULL: 0x00,
  BOOL: 0x01,
  I8: 0x02,
  I16: 0x03,
  I32: 0x04,
  I64: 0x05,
  F32: 0x06,
  F64: 0x07,
  STRING: 0x08,
  BYTES: 0x09,
  ARRAY: 0x0a,
  MAP: 0x0b,
} as const;

/** Signal type codes */
export const SIG = {
  PARAM: 0,
  EVENT: 1,
  STREAM: 2,
  GESTURE: 3,
  TIMELINE: 4,
} as const;

/** Gesture phase codes */
export const PHASE = {
  START: 0,
  MOVE: 1,
  END: 2,
  CANCEL: 3,
} as const;

// ============================================================================
// FRAME FLAGS
// ============================================================================

/** Extended frame flags with version */
export interface FrameFlagsV3 extends FrameFlags {
  /** Encoding version: 0 = v2 (MessagePack), 1 = v3 (binary) */
  version: number;
}

/** Encode frame flags to byte */
export function encodeFlags(flags: FrameFlagsV3): number {
  let byte = 0;
  byte |= (flags.qos & 0x03) << 6;
  if (flags.hasTimestamp) byte |= 0x20;
  if (flags.encrypted) byte |= 0x10;
  if (flags.compressed) byte |= 0x08;
  byte |= (flags.version ?? 1) & 0x07;
  return byte;
}

/** Decode frame flags from byte */
export function decodeFlags(byte: number): FrameFlagsV3 {
  return {
    qos: ((byte >> 6) & 0x03) as QoS,
    hasTimestamp: (byte & 0x20) !== 0,
    encrypted: (byte & 0x10) !== 0,
    compressed: (byte & 0x08) !== 0,
    version: byte & 0x07,
  };
}

// ============================================================================
// PUBLIC API
// ============================================================================

/** Frame encoding options */
export interface FrameOptions {
  qos?: QoS;
  timestamp?: bigint | number;
  sequence?: number;
}

/** Decoded frame result */
export interface DecodedFrame {
  payload: Uint8Array;
  qos: QoS;
  timestamp?: bigint;
  flags: FrameFlagsV3;
}

/**
 * Encode a raw payload to a frame
 */
export function encodeFrame(payload: Uint8Array, options: FrameOptions = {}): Uint8Array {
  const hasTimestamp = options.timestamp !== undefined;
  const headerSize = hasTimestamp ? HEADER_SIZE_WITH_TS : HEADER_SIZE;

  const frame = new Uint8Array(headerSize + payload.length);
  const view = new DataView(frame.buffer);

  frame[0] = MAGIC_BYTE;
  frame[1] = encodeFlags({
    qos: options.qos ?? QoS.Fire,
    hasTimestamp,
    encrypted: false,
    compressed: false,
    version: 1, // v3 binary encoding
  });
  view.setUint16(2, payload.length, false);

  if (hasTimestamp && options.timestamp !== undefined) {
    const ts = typeof options.timestamp === 'bigint' ? options.timestamp : BigInt(options.timestamp);
    view.setBigUint64(4, ts, false);
  }

  frame.set(payload, headerSize);
  return frame;
}

/**
 * Decode a frame to its payload
 */
export function decodeFrame(data: Uint8Array): DecodedFrame {
  if (data.length < HEADER_SIZE) {
    throw new Error('Frame too small');
  }

  if (data[0] !== MAGIC_BYTE) {
    throw new Error(`Invalid magic byte: 0x${data[0].toString(16)}`);
  }

  const view = new DataView(data.buffer, data.byteOffset);
  const flags = decodeFlags(data[1]);
  const payloadLength = view.getUint16(2, false);

  const headerSize = flags.hasTimestamp ? HEADER_SIZE_WITH_TS : HEADER_SIZE;

  if (data.length < headerSize + payloadLength) {
    throw new Error('Frame incomplete');
  }

  let timestamp: bigint | undefined;
  if (flags.hasTimestamp) {
    timestamp = view.getBigUint64(4, false);
  }

  const payload = data.slice(headerSize, headerSize + payloadLength);

  return { payload, qos: flags.qos, timestamp, flags };
}

/**
 * Encode a message to v3 binary payload
 */
export function encodeMessageBinary(message: Message): Uint8Array {
  const buf = new ArrayBuffer(4096);
  const view = new DataView(buf);
  let offset = 0;

  switch (message.type) {
    case 'HELLO':
      offset = encodeHello(view, offset, message);
      break;
    case 'WELCOME':
      offset = encodeWelcome(view, offset, message);
      break;
    case 'ANNOUNCE':
      offset = encodeAnnounce(view, offset, message);
      break;
    case 'SUBSCRIBE':
      offset = encodeSubscribe(view, offset, message);
      break;
    case 'UNSUBSCRIBE':
      offset = encodeUnsubscribe(view, offset, message);
      break;
    case 'PUBLISH':
      offset = encodePublish(view, offset, message);
      break;
    case 'SET':
      offset = encodeSet(view, offset, message);
      break;
    case 'GET':
      offset = encodeGet(view, offset, message);
      break;
    case 'SNAPSHOT':
      offset = encodeSnapshot(view, offset, message);
      break;
    case 'BUNDLE':
      offset = encodeBundle(view, offset, message);
      break;
    case 'SYNC':
      offset = encodeSync(view, offset, message);
      break;
    case 'PING':
      view.setUint8(offset++, MSG.PING);
      break;
    case 'PONG':
      view.setUint8(offset++, MSG.PONG);
      break;
    case 'ACK':
      offset = encodeAck(view, offset, message);
      break;
    case 'ERROR':
      offset = encodeError(view, offset, message);
      break;
    case 'QUERY':
      offset = encodeQuery(view, offset, message);
      break;
    case 'RESULT':
      offset = encodeResult(view, offset, message);
      break;
    default:
      throw new Error(`Unknown message type: ${(message as Message).type}`);
  }

  return new Uint8Array(buf, 0, offset);
}

/**
 * Decode a message - auto-detects v2 (MessagePack) vs v3 (binary)
 */
export function decodeMessageBinary(data: Uint8Array): Message {
  if (data.length === 0) {
    throw new Error('Empty message data');
  }

  const first = data[0];

  // v3 messages start with known message type codes (0x01-0x61)
  // v2 MessagePack maps start with 0x80-0x8F (fixmap) or 0xDE-0xDF (map16/map32)
  if (isMsgpackMap(first)) {
    return msgpackDecode(data) as Message;
  }

  return decodeV3Binary(data);
}

/**
 * Encode a message to a frame (v3 binary)
 */
export function encodeMessage(message: Message, options: FrameOptions = {}): Uint8Array {
  const payload = encodeMessageBinary(message);
  return encodeFrame(payload, {
    ...options,
    qos: options.qos ?? getDefaultQoS(message),
  });
}

/**
 * Decode a frame to a message
 */
export function decodeMessage(data: Uint8Array): {
  message: Message;
  flags: FrameFlagsV3;
  timestamp?: bigint;
} {
  const { payload, flags, timestamp } = decodeFrame(data);
  const message = decodeMessageBinary(payload);
  return { message, flags, timestamp };
}

/**
 * Encode a message to bytes (convenience function)
 */
export function encode(message: Message, qos?: QoS): Uint8Array {
  return encodeMessage(message, { qos });
}

/**
 * Decode bytes to a message (convenience function)
 */
export function decode(data: Uint8Array): Message {
  return decodeMessage(data).message;
}

/**
 * Check if buffer contains a complete frame
 */
export function checkComplete(data: Uint8Array): number | null {
  if (data.length < HEADER_SIZE) {
    return null;
  }

  if (data[0] !== MAGIC_BYTE) {
    return null;
  }

  const flags = decodeFlags(data[1]);
  const view = new DataView(data.buffer, data.byteOffset);
  const payloadLength = view.getUint16(2, false);

  const headerSize = flags.hasTimestamp ? HEADER_SIZE_WITH_TS : HEADER_SIZE;
  const totalSize = headerSize + payloadLength;

  if (data.length >= totalSize) {
    return totalSize;
  }

  return null;
}

// ============================================================================
// V3 BINARY ENCODING
// ============================================================================

function encodeSet(view: DataView, offset: number, msg: SetMessage): number {
  view.setUint8(offset++, MSG.SET);

  // Flags: [has_rev:1][lock:1][unlock:1][rsv:1][vtype:4]
  const vtype = getValueType(msg.value);
  let flags = vtype & 0x0f;
  if (msg.revision !== undefined) flags |= 0x80;
  if (msg.lock) flags |= 0x40;
  if (msg.unlock) flags |= 0x20;
  view.setUint8(offset++, flags);

  // Address
  offset = encodeString(view, offset, msg.address);

  // Value
  offset = encodeValueData(view, offset, msg.value);

  // Revision
  if (msg.revision !== undefined) {
    view.setBigUint64(offset, BigInt(msg.revision), false);
    offset += 8;
  }

  return offset;
}

function encodePublish(view: DataView, offset: number, msg: PublishMessage): number {
  view.setUint8(offset++, MSG.PUBLISH);

  const sigCode = msg.signal ? getSignalTypeCode(msg.signal) : SIG.EVENT;
  const phaseCode = msg.phase ? getPhaseCode(msg.phase) : PHASE.START;

  // Flags: [sig_type:3][has_ts:1][has_id:1][phase:3]
  let flags = (sigCode & 0x07) << 5;
  if (msg.timestamp !== undefined) flags |= 0x10;
  if (msg.id !== undefined) flags |= 0x08;
  flags |= phaseCode & 0x07;
  view.setUint8(offset++, flags);

  // Address
  offset = encodeString(view, offset, msg.address);

  // Value/payload
  if (msg.value !== undefined) {
    view.setUint8(offset++, 1); // has value
    view.setUint8(offset++, getValueType(msg.value));
    offset = encodeValueData(view, offset, msg.value);
  } else if (msg.payload !== undefined) {
    view.setUint8(offset++, 1); // has payload
    view.setUint8(offset++, getValueType(msg.payload));
    offset = encodeValueData(view, offset, msg.payload);
  } else if (msg.samples !== undefined) {
    view.setUint8(offset++, 2); // has samples
    view.setUint16(offset, msg.samples.length, false);
    offset += 2;
    for (const sample of msg.samples) {
      view.setFloat64(offset, sample, false);
      offset += 8;
    }
  } else {
    view.setUint8(offset++, 0); // no value
  }

  // Optional timestamp
  if (msg.timestamp !== undefined) {
    view.setBigUint64(offset, BigInt(msg.timestamp), false);
    offset += 8;
  }

  // Optional gesture ID
  if (msg.id !== undefined) {
    view.setUint32(offset, msg.id, false);
    offset += 4;
  }

  // Optional rate
  if (msg.rate !== undefined) {
    view.setUint32(offset, msg.rate, false);
    offset += 4;
  }

  return offset;
}

function encodeHello(view: DataView, offset: number, msg: HelloMessage): number {
  view.setUint8(offset++, MSG.HELLO);
  view.setUint8(offset++, msg.version);

  // Feature flags
  let features = 0;
  for (const f of msg.features) {
    if (f === 'param') features |= 0x80;
    if (f === 'event') features |= 0x40;
    if (f === 'stream') features |= 0x20;
    if (f === 'gesture') features |= 0x10;
    if (f === 'timeline') features |= 0x08;
  }
  view.setUint8(offset++, features);

  // Name
  offset = encodeString(view, offset, msg.name);

  // Token
  if (msg.token) {
    offset = encodeString(view, offset, msg.token);
  } else {
    view.setUint16(offset, 0, false);
    offset += 2;
  }

  return offset;
}

function encodeWelcome(view: DataView, offset: number, msg: WelcomeMessage): number {
  view.setUint8(offset++, MSG.WELCOME);
  view.setUint8(offset++, msg.version);

  // Feature flags
  let features = 0;
  for (const f of msg.features) {
    if (f === 'param') features |= 0x80;
    if (f === 'event') features |= 0x40;
    if (f === 'stream') features |= 0x20;
    if (f === 'gesture') features |= 0x10;
    if (f === 'timeline') features |= 0x08;
  }
  view.setUint8(offset++, features);

  // Server time
  view.setBigUint64(offset, BigInt(msg.time), false);
  offset += 8;

  // Session ID
  offset = encodeString(view, offset, msg.session);

  // Server name
  offset = encodeString(view, offset, msg.name);

  // Token
  if (msg.token) {
    offset = encodeString(view, offset, msg.token);
  } else {
    view.setUint16(offset, 0, false);
    offset += 2;
  }

  return offset;
}

function encodeAnnounce(view: DataView, offset: number, msg: AnnounceMessage): number {
  view.setUint8(offset++, MSG.ANNOUNCE);
  offset = encodeString(view, offset, msg.namespace ?? '');
  view.setUint16(offset, msg.signals.length, false);
  offset += 2;

  for (const sig of msg.signals) {
    offset = encodeString(view, offset, sig.address);
    view.setUint8(offset++, getSignalTypeCode(sig.type));

    let optFlags = 0;
    if (sig.datatype) optFlags |= 0x01;
    if (sig.access) optFlags |= 0x02;
    if (sig.meta) optFlags |= 0x04;
    view.setUint8(offset++, optFlags);

    if (sig.datatype) offset = encodeString(view, offset, sig.datatype);
    if (sig.access) offset = encodeString(view, offset, sig.access);
    if (sig.meta) {
      let metaFlags = 0;
      if (sig.meta.unit) metaFlags |= 0x01;
      if (sig.meta.range) metaFlags |= 0x02;
      if (sig.meta.default !== undefined) metaFlags |= 0x04;
      if (sig.meta.description) metaFlags |= 0x08;
      view.setUint8(offset++, metaFlags);

      if (sig.meta.unit) offset = encodeString(view, offset, sig.meta.unit);
      if (sig.meta.range) {
        view.setFloat64(offset, sig.meta.range[0], false);
        offset += 8;
        view.setFloat64(offset, sig.meta.range[1], false);
        offset += 8;
      }
      if (sig.meta.default !== undefined) {
        view.setUint8(offset++, getValueType(sig.meta.default));
        offset = encodeValueData(view, offset, sig.meta.default);
      }
      if (sig.meta.description) offset = encodeString(view, offset, sig.meta.description);
    }
  }

  return offset;
}

function encodeSubscribe(view: DataView, offset: number, msg: SubscribeMessage): number {
  view.setUint8(offset++, MSG.SUBSCRIBE);
  view.setUint32(offset, msg.id, false);
  offset += 4;

  offset = encodeString(view, offset, msg.pattern);

  // Type filter bitmask
  let typeMask = 0xff;
  if (msg.types && msg.types.length > 0) {
    typeMask = 0;
    for (const t of msg.types) {
      if (t === 'param') typeMask |= 0x01;
      if (t === 'event') typeMask |= 0x02;
      if (t === 'stream') typeMask |= 0x04;
      if (t === 'gesture') typeMask |= 0x08;
      if (t === 'timeline') typeMask |= 0x10;
    }
  }
  view.setUint8(offset++, typeMask);

  // Options
  if (msg.options) {
    let optFlags = 0;
    if (msg.options.maxRate !== undefined) optFlags |= 0x01;
    if (msg.options.epsilon !== undefined) optFlags |= 0x02;
    if (msg.options.history !== undefined) optFlags |= 0x04;
    if (msg.options.window !== undefined) optFlags |= 0x08;
    view.setUint8(offset++, optFlags);

    if (msg.options.maxRate !== undefined) {
      view.setUint32(offset, msg.options.maxRate, false);
      offset += 4;
    }
    if (msg.options.epsilon !== undefined) {
      view.setFloat64(offset, msg.options.epsilon, false);
      offset += 8;
    }
    if (msg.options.history !== undefined) {
      view.setUint32(offset, msg.options.history, false);
      offset += 4;
    }
    if (msg.options.window !== undefined) {
      view.setUint32(offset, msg.options.window, false);
      offset += 4;
    }
  } else {
    view.setUint8(offset++, 0);
  }

  return offset;
}

function encodeUnsubscribe(view: DataView, offset: number, msg: UnsubscribeMessage): number {
  view.setUint8(offset++, MSG.UNSUBSCRIBE);
  view.setUint32(offset, msg.id, false);
  offset += 4;
  return offset;
}

function encodeGet(view: DataView, offset: number, msg: GetMessage): number {
  view.setUint8(offset++, MSG.GET);
  offset = encodeString(view, offset, msg.address);
  return offset;
}

function encodeSnapshot(view: DataView, offset: number, msg: SnapshotMessage): number {
  view.setUint8(offset++, MSG.SNAPSHOT);
  view.setUint16(offset, msg.params.length, false);
  offset += 2;

  for (const param of msg.params) {
    offset = encodeString(view, offset, param.address);
    view.setUint8(offset++, getValueType(param.value));
    offset = encodeValueData(view, offset, param.value);
    view.setBigUint64(offset, BigInt(param.revision), false);
    offset += 8;

    let optFlags = 0;
    if (param.writer) optFlags |= 0x01;
    if (param.timestamp !== undefined) optFlags |= 0x02;
    view.setUint8(offset++, optFlags);

    if (param.writer) offset = encodeString(view, offset, param.writer);
    if (param.timestamp !== undefined) {
      view.setBigUint64(offset, BigInt(param.timestamp), false);
      offset += 8;
    }
  }

  return offset;
}

function encodeBundle(view: DataView, offset: number, msg: BundleMessage): number {
  view.setUint8(offset++, MSG.BUNDLE);

  let flags = 0;
  if (msg.timestamp !== undefined) flags |= 0x80;
  view.setUint8(offset++, flags);

  view.setUint16(offset, msg.messages.length, false);
  offset += 2;

  if (msg.timestamp !== undefined) {
    view.setBigUint64(offset, BigInt(msg.timestamp), false);
    offset += 8;
  }

  // Each message prefixed with length
  for (const innerMsg of msg.messages) {
    const innerPayload = encodeMessageBinary(innerMsg);
    view.setUint16(offset, innerPayload.length, false);
    offset += 2;
    new Uint8Array(view.buffer).set(innerPayload, offset);
    offset += innerPayload.length;
  }

  return offset;
}

function encodeSync(view: DataView, offset: number, msg: SyncMessage): number {
  view.setUint8(offset++, MSG.SYNC);

  let flags = 0;
  if (msg.t2 !== undefined) flags |= 0x01;
  if (msg.t3 !== undefined) flags |= 0x02;
  view.setUint8(offset++, flags);

  view.setBigUint64(offset, BigInt(msg.t1), false);
  offset += 8;
  if (msg.t2 !== undefined) {
    view.setBigUint64(offset, BigInt(msg.t2), false);
    offset += 8;
  }
  if (msg.t3 !== undefined) {
    view.setBigUint64(offset, BigInt(msg.t3), false);
    offset += 8;
  }

  return offset;
}

function encodeAck(view: DataView, offset: number, msg: AckMessage): number {
  view.setUint8(offset++, MSG.ACK);

  let flags = 0;
  if (msg.address) flags |= 0x01;
  if (msg.revision !== undefined) flags |= 0x02;
  if (msg.locked !== undefined) flags |= 0x04;
  if (msg.holder) flags |= 0x08;
  if (msg.correlationId !== undefined) flags |= 0x10;
  view.setUint8(offset++, flags);

  if (msg.address) offset = encodeString(view, offset, msg.address);
  if (msg.revision !== undefined) {
    view.setBigUint64(offset, BigInt(msg.revision), false);
    offset += 8;
  }
  if (msg.locked !== undefined) {
    view.setUint8(offset++, msg.locked ? 1 : 0);
  }
  if (msg.holder) offset = encodeString(view, offset, msg.holder);
  if (msg.correlationId !== undefined) {
    view.setUint32(offset, msg.correlationId, false);
    offset += 4;
  }

  return offset;
}

function encodeError(view: DataView, offset: number, msg: ErrorMessage): number {
  view.setUint8(offset++, MSG.ERROR);
  view.setUint16(offset, msg.code, false);
  offset += 2;
  offset = encodeString(view, offset, msg.message);

  let flags = 0;
  if (msg.address) flags |= 0x01;
  if (msg.correlationId !== undefined) flags |= 0x02;
  view.setUint8(offset++, flags);

  if (msg.address) offset = encodeString(view, offset, msg.address);
  if (msg.correlationId !== undefined) {
    view.setUint32(offset, msg.correlationId, false);
    offset += 4;
  }

  return offset;
}

function encodeQuery(view: DataView, offset: number, msg: QueryMessage): number {
  view.setUint8(offset++, MSG.QUERY);
  offset = encodeString(view, offset, msg.pattern);
  return offset;
}

function encodeResult(view: DataView, offset: number, msg: ResultMessage): number {
  view.setUint8(offset++, MSG.RESULT);
  view.setUint16(offset, msg.signals.length, false);
  offset += 2;

  for (const sig of msg.signals) {
    offset = encodeString(view, offset, sig.address);
    view.setUint8(offset++, getSignalTypeCode(sig.type));

    let optFlags = 0;
    if (sig.datatype) optFlags |= 0x01;
    if (sig.access) optFlags |= 0x02;
    view.setUint8(offset++, optFlags);

    if (sig.datatype) offset = encodeString(view, offset, sig.datatype);
    if (sig.access) offset = encodeString(view, offset, sig.access);
  }

  return offset;
}

// ============================================================================
// V3 BINARY DECODING
// ============================================================================

function decodeV3Binary(data: Uint8Array): Message {
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  let offset = 0;

  const msgType = view.getUint8(offset++);

  switch (msgType) {
    case MSG.HELLO:
      return decodeHello(view, offset);
    case MSG.WELCOME:
      return decodeWelcome(view, offset);
    case MSG.ANNOUNCE:
      return decodeAnnounce(view, offset);
    case MSG.SUBSCRIBE:
      return decodeSubscribe(view, offset);
    case MSG.UNSUBSCRIBE:
      return decodeUnsubscribe(view, offset);
    case MSG.PUBLISH:
      return decodePublish(view, offset);
    case MSG.SET:
      return decodeSet(view, offset);
    case MSG.GET:
      return decodeGet(view, offset);
    case MSG.SNAPSHOT:
      return decodeSnapshot(view, offset);
    case MSG.BUNDLE:
      return decodeBundle(view, offset);
    case MSG.SYNC:
      return decodeSyncMsg(view, offset);
    case MSG.PING:
      return { type: 'PING' };
    case MSG.PONG:
      return { type: 'PONG' };
    case MSG.ACK:
      return decodeAck(view, offset);
    case MSG.ERROR:
      return decodeErrorMsg(view, offset);
    case MSG.QUERY:
      return decodeQuery(view, offset);
    case MSG.RESULT:
      return decodeResultMsg(view, offset);
    default:
      throw new Error(`Unknown message type: 0x${msgType.toString(16)}`);
  }
}

function decodeSet(view: DataView, offset: number): SetMessage {
  const flags = view.getUint8(offset++);
  const vtype = flags & 0x0f;
  const hasRev = (flags & 0x80) !== 0;
  const lock = (flags & 0x40) !== 0;
  const unlock = (flags & 0x20) !== 0;

  const [address, newOffset] = decodeString(view, offset);
  offset = newOffset;
  const [value, finalOffset] = decodeValueData(view, offset, vtype);
  offset = finalOffset;

  const revision = hasRev ? Number(view.getBigUint64(offset, false)) : undefined;

  return {
    type: 'SET',
    address,
    value,
    revision,
    lock: lock || undefined,
    unlock: unlock || undefined,
  };
}

function decodePublish(view: DataView, offset: number): PublishMessage {
  const flags = view.getUint8(offset++);
  const sigCode = (flags >> 5) & 0x07;
  const hasTs = (flags & 0x10) !== 0;
  const hasId = (flags & 0x08) !== 0;
  const phaseCode = flags & 0x07;

  const [address, newOffset] = decodeString(view, offset);
  offset = newOffset;

  const valueIndicator = view.getUint8(offset++);
  let value: Value | undefined;
  let payload: Value | undefined;
  let samples: number[] | undefined;

  if (valueIndicator === 1) {
    const vtype = view.getUint8(offset++);
    const [v, o] = decodeValueData(view, offset, vtype);
    value = v;
    offset = o;
  } else if (valueIndicator === 2) {
    const count = view.getUint16(offset, false);
    offset += 2;
    samples = [];
    for (let i = 0; i < count; i++) {
      samples.push(view.getFloat64(offset, false));
      offset += 8;
    }
  }

  const timestamp = hasTs ? Number(view.getBigUint64(offset, false)) : undefined;
  if (hasTs) offset += 8;

  const id = hasId ? view.getUint32(offset, false) : undefined;
  if (hasId) offset += 4;

  let rate: number | undefined;
  if (offset + 4 <= view.byteLength) {
    rate = view.getUint32(offset, false);
    offset += 4;
  }

  return {
    type: 'PUBLISH',
    address,
    signal: getSignalTypeFromCode(sigCode),
    value,
    payload,
    samples,
    rate,
    id,
    phase: getPhaseFromCode(phaseCode),
    timestamp,
  };
}

function decodeHello(view: DataView, offset: number): HelloMessage {
  const version = view.getUint8(offset++);
  const featureFlags = view.getUint8(offset++);

  const features: string[] = [];
  if (featureFlags & 0x80) features.push('param');
  if (featureFlags & 0x40) features.push('event');
  if (featureFlags & 0x20) features.push('stream');
  if (featureFlags & 0x10) features.push('gesture');
  if (featureFlags & 0x08) features.push('timeline');

  const [name, o1] = decodeString(view, offset);
  offset = o1;
  const [tokenStr, o2] = decodeString(view, offset);

  return {
    type: 'HELLO',
    version,
    name,
    features,
    token: tokenStr || undefined,
  };
}

function decodeWelcome(view: DataView, offset: number): WelcomeMessage {
  const version = view.getUint8(offset++);
  const featureFlags = view.getUint8(offset++);

  const features: string[] = [];
  if (featureFlags & 0x80) features.push('param');
  if (featureFlags & 0x40) features.push('event');
  if (featureFlags & 0x20) features.push('stream');
  if (featureFlags & 0x10) features.push('gesture');
  if (featureFlags & 0x08) features.push('timeline');

  const time = Number(view.getBigUint64(offset, false));
  offset += 8;

  const [session, o1] = decodeString(view, offset);
  offset = o1;
  const [name, o2] = decodeString(view, offset);
  offset = o2;
  const [tokenStr, _] = decodeString(view, offset);

  return {
    type: 'WELCOME',
    version,
    session,
    name,
    features,
    time,
    token: tokenStr || undefined,
  };
}

function decodeAnnounce(view: DataView, offset: number): AnnounceMessage {
  const [namespace, o1] = decodeString(view, offset);
  offset = o1;
  const count = view.getUint16(offset, false);
  offset += 2;

  const signals: SignalDefinition[] = [];
  for (let i = 0; i < count; i++) {
    const [address, o2] = decodeString(view, offset);
    offset = o2;
    const sigCode = view.getUint8(offset++);
    const optFlags = view.getUint8(offset++);

    let datatype: string | undefined;
    let access: string | undefined;
    let meta: SignalDefinition['meta'];

    if (optFlags & 0x01) {
      const [dt, o] = decodeString(view, offset);
      datatype = dt;
      offset = o;
    }
    if (optFlags & 0x02) {
      const [ac, o] = decodeString(view, offset);
      access = ac;
      offset = o;
    }
    if (optFlags & 0x04) {
      const metaFlags = view.getUint8(offset++);
      meta = {};

      if (metaFlags & 0x01) {
        const [unit, o] = decodeString(view, offset);
        meta.unit = unit;
        offset = o;
      }
      if (metaFlags & 0x02) {
        const min = view.getFloat64(offset, false);
        offset += 8;
        const max = view.getFloat64(offset, false);
        offset += 8;
        meta.range = [min, max];
      }
      if (metaFlags & 0x04) {
        const vtype = view.getUint8(offset++);
        const [val, o] = decodeValueData(view, offset, vtype);
        meta.default = val;
        offset = o;
      }
      if (metaFlags & 0x08) {
        const [desc, o] = decodeString(view, offset);
        meta.description = desc;
        offset = o;
      }
    }

    signals.push({
      address,
      type: getSignalTypeFromCode(sigCode),
      datatype,
      access,
      meta,
    });
  }

  return { type: 'ANNOUNCE', namespace, signals };
}

function decodeSubscribe(view: DataView, offset: number): SubscribeMessage {
  const id = view.getUint32(offset, false);
  offset += 4;
  const [pattern, o1] = decodeString(view, offset);
  offset = o1;
  const typeMask = view.getUint8(offset++);

  const types: SignalType[] = [];
  if (typeMask !== 0xff) {
    if (typeMask & 0x01) types.push('param');
    if (typeMask & 0x02) types.push('event');
    if (typeMask & 0x04) types.push('stream');
    if (typeMask & 0x08) types.push('gesture');
    if (typeMask & 0x10) types.push('timeline');
  }

  const optFlags = view.getUint8(offset++);
  let options: SubscribeMessage['options'];
  if (optFlags !== 0) {
    options = {};
    if (optFlags & 0x01) {
      options.maxRate = view.getUint32(offset, false);
      offset += 4;
    }
    if (optFlags & 0x02) {
      options.epsilon = view.getFloat64(offset, false);
      offset += 8;
    }
    if (optFlags & 0x04) {
      options.history = view.getUint32(offset, false);
      offset += 4;
    }
    if (optFlags & 0x08) {
      options.window = view.getUint32(offset, false);
      offset += 4;
    }
  }

  return { type: 'SUBSCRIBE', id, pattern, types: types.length > 0 ? types : undefined, options };
}

function decodeUnsubscribe(view: DataView, offset: number): UnsubscribeMessage {
  const id = view.getUint32(offset, false);
  return { type: 'UNSUBSCRIBE', id };
}

function decodeGet(view: DataView, offset: number): GetMessage {
  const [address, _] = decodeString(view, offset);
  return { type: 'GET', address };
}

function decodeSnapshot(view: DataView, offset: number): SnapshotMessage {
  const count = view.getUint16(offset, false);
  offset += 2;

  const params: ParamValue[] = [];
  for (let i = 0; i < count; i++) {
    const [address, o1] = decodeString(view, offset);
    offset = o1;
    const vtype = view.getUint8(offset++);
    const [value, o2] = decodeValueData(view, offset, vtype);
    offset = o2;
    const revision = Number(view.getBigUint64(offset, false));
    offset += 8;
    const optFlags = view.getUint8(offset++);

    let writer: string | undefined;
    let timestamp: number | undefined;

    if (optFlags & 0x01) {
      const [w, o] = decodeString(view, offset);
      writer = w;
      offset = o;
    }
    if (optFlags & 0x02) {
      timestamp = Number(view.getBigUint64(offset, false));
      offset += 8;
    }

    params.push({ address, value, revision, writer, timestamp });
  }

  return { type: 'SNAPSHOT', params };
}

function decodeBundle(view: DataView, offset: number): BundleMessage {
  const flags = view.getUint8(offset++);
  const hasTs = (flags & 0x80) !== 0;
  const count = view.getUint16(offset, false);
  offset += 2;

  const timestamp = hasTs ? Number(view.getBigUint64(offset, false)) : undefined;
  if (hasTs) offset += 8;

  const messages: Message[] = [];
  for (let i = 0; i < count; i++) {
    const len = view.getUint16(offset, false);
    offset += 2;
    const innerData = new Uint8Array(view.buffer, view.byteOffset + offset, len);
    messages.push(decodeV3Binary(innerData));
    offset += len;
  }

  return { type: 'BUNDLE', timestamp, messages };
}

function decodeSyncMsg(view: DataView, offset: number): SyncMessage {
  const flags = view.getUint8(offset++);
  const t1 = Number(view.getBigUint64(offset, false));
  offset += 8;
  const t2 = flags & 0x01 ? Number(view.getBigUint64(offset, false)) : undefined;
  if (flags & 0x01) offset += 8;
  const t3 = flags & 0x02 ? Number(view.getBigUint64(offset, false)) : undefined;

  return { type: 'SYNC', t1, t2, t3 };
}

function decodeAck(view: DataView, offset: number): AckMessage {
  const flags = view.getUint8(offset++);

  let address: string | undefined;
  let revision: number | undefined;
  let locked: boolean | undefined;
  let holder: string | undefined;
  let correlationId: number | undefined;

  if (flags & 0x01) {
    const [a, o] = decodeString(view, offset);
    address = a;
    offset = o;
  }
  if (flags & 0x02) {
    revision = Number(view.getBigUint64(offset, false));
    offset += 8;
  }
  if (flags & 0x04) {
    locked = view.getUint8(offset++) !== 0;
  }
  if (flags & 0x08) {
    const [h, o] = decodeString(view, offset);
    holder = h;
    offset = o;
  }
  if (flags & 0x10) {
    correlationId = view.getUint32(offset, false);
    offset += 4;
  }

  return { type: 'ACK', address, revision, locked, holder, correlationId };
}

function decodeErrorMsg(view: DataView, offset: number): ErrorMessage {
  const code = view.getUint16(offset, false);
  offset += 2;
  const [message, o1] = decodeString(view, offset);
  offset = o1;
  const flags = view.getUint8(offset++);

  let address: string | undefined;
  let correlationId: number | undefined;

  if (flags & 0x01) {
    const [a, o] = decodeString(view, offset);
    address = a;
    offset = o;
  }
  if (flags & 0x02) {
    correlationId = view.getUint32(offset, false);
    offset += 4;
  }

  return { type: 'ERROR', code, message, address, correlationId };
}

function decodeQuery(view: DataView, offset: number): QueryMessage {
  const [pattern, _] = decodeString(view, offset);
  return { type: 'QUERY', pattern };
}

function decodeResultMsg(view: DataView, offset: number): ResultMessage {
  const count = view.getUint16(offset, false);
  offset += 2;

  const signals: SignalDefinition[] = [];
  for (let i = 0; i < count; i++) {
    const [address, o1] = decodeString(view, offset);
    offset = o1;
    const sigCode = view.getUint8(offset++);
    const optFlags = view.getUint8(offset++);

    let datatype: string | undefined;
    let access: string | undefined;

    if (optFlags & 0x01) {
      const [dt, o] = decodeString(view, offset);
      datatype = dt;
      offset = o;
    }
    if (optFlags & 0x02) {
      const [ac, o] = decodeString(view, offset);
      access = ac;
      offset = o;
    }

    signals.push({
      address,
      type: getSignalTypeFromCode(sigCode),
      datatype,
      access,
    });
  }

  return { type: 'RESULT', signals };
}

// ============================================================================
// HELPERS
// ============================================================================

function encodeString(view: DataView, offset: number, str: string): number {
  const encoded = new TextEncoder().encode(str);
  view.setUint16(offset, encoded.length, false);
  offset += 2;
  new Uint8Array(view.buffer).set(encoded, offset);
  return offset + encoded.length;
}

function decodeString(view: DataView, offset: number): [string, number] {
  const len = view.getUint16(offset, false);
  offset += 2;
  const bytes = new Uint8Array(view.buffer, view.byteOffset + offset, len);
  const str = new TextDecoder().decode(bytes);
  return [str, offset + len];
}

function encodeValueData(view: DataView, offset: number, value: Value): number {
  if (value === null) {
    return offset;
  }
  if (typeof value === 'boolean') {
    view.setUint8(offset++, value ? 1 : 0);
    return offset;
  }
  if (typeof value === 'number') {
    view.setFloat64(offset, value, false);
    return offset + 8;
  }
  if (typeof value === 'string') {
    return encodeString(view, offset, value);
  }
  if (value instanceof Uint8Array) {
    view.setUint16(offset, value.length, false);
    offset += 2;
    new Uint8Array(view.buffer).set(value, offset);
    return offset + value.length;
  }
  if (Array.isArray(value)) {
    view.setUint16(offset, value.length, false);
    offset += 2;
    for (const item of value) {
      view.setUint8(offset++, getValueType(item));
      offset = encodeValueData(view, offset, item);
    }
    return offset;
  }
  if (typeof value === 'object') {
    const entries = Object.entries(value);
    view.setUint16(offset, entries.length, false);
    offset += 2;
    for (const [key, val] of entries) {
      offset = encodeString(view, offset, key);
      view.setUint8(offset++, getValueType(val));
      offset = encodeValueData(view, offset, val);
    }
    return offset;
  }
  return offset;
}

function decodeValueData(view: DataView, offset: number, vtype: number): [Value, number] {
  switch (vtype) {
    case VAL.NULL:
      return [null, offset];
    case VAL.BOOL:
      return [view.getUint8(offset) !== 0, offset + 1];
    case VAL.I8:
      return [view.getInt8(offset), offset + 1];
    case VAL.I16:
      return [view.getInt16(offset, false), offset + 2];
    case VAL.I32:
      return [view.getInt32(offset, false), offset + 4];
    case VAL.I64:
      return [Number(view.getBigInt64(offset, false)), offset + 8];
    case VAL.F32:
      return [view.getFloat32(offset, false), offset + 4];
    case VAL.F64:
      return [view.getFloat64(offset, false), offset + 8];
    case VAL.STRING: {
      const [str, newOffset] = decodeString(view, offset);
      return [str, newOffset];
    }
    case VAL.BYTES: {
      const len = view.getUint16(offset, false);
      offset += 2;
      const bytes = new Uint8Array(view.buffer, view.byteOffset + offset, len);
      return [new Uint8Array(bytes), offset + len];
    }
    case VAL.ARRAY: {
      const count = view.getUint16(offset, false);
      offset += 2;
      const arr: Value[] = [];
      for (let i = 0; i < count; i++) {
        const itemType = view.getUint8(offset++);
        const [item, newOffset] = decodeValueData(view, offset, itemType);
        arr.push(item);
        offset = newOffset;
      }
      return [arr, offset];
    }
    case VAL.MAP: {
      const count = view.getUint16(offset, false);
      offset += 2;
      const map: { [key: string]: Value } = {};
      for (let i = 0; i < count; i++) {
        const [key, o1] = decodeString(view, offset);
        offset = o1;
        const valType = view.getUint8(offset++);
        const [val, o2] = decodeValueData(view, offset, valType);
        map[key] = val;
        offset = o2;
      }
      return [map, offset];
    }
    default:
      throw new Error(`Unknown value type: 0x${vtype.toString(16)}`);
  }
}

function getValueType(value: Value): number {
  if (value === null) return VAL.NULL;
  if (typeof value === 'boolean') return VAL.BOOL;
  if (typeof value === 'number') return VAL.F64;
  if (typeof value === 'string') return VAL.STRING;
  if (value instanceof Uint8Array) return VAL.BYTES;
  if (Array.isArray(value)) return VAL.ARRAY;
  if (typeof value === 'object') return VAL.MAP;
  return VAL.NULL;
}

function getSignalTypeCode(sig: SignalType): number {
  switch (sig) {
    case 'param':
      return SIG.PARAM;
    case 'event':
      return SIG.EVENT;
    case 'stream':
      return SIG.STREAM;
    case 'gesture':
      return SIG.GESTURE;
    case 'timeline':
      return SIG.TIMELINE;
    default:
      return SIG.EVENT;
  }
}

function getSignalTypeFromCode(code: number): SignalType {
  switch (code) {
    case SIG.PARAM:
      return 'param';
    case SIG.EVENT:
      return 'event';
    case SIG.STREAM:
      return 'stream';
    case SIG.GESTURE:
      return 'gesture';
    case SIG.TIMELINE:
      return 'timeline';
    default:
      return 'event';
  }
}

function getPhaseCode(phase: 'start' | 'move' | 'end' | 'cancel'): number {
  switch (phase) {
    case 'start':
      return PHASE.START;
    case 'move':
      return PHASE.MOVE;
    case 'end':
      return PHASE.END;
    case 'cancel':
      return PHASE.CANCEL;
    default:
      return PHASE.START;
  }
}

function getPhaseFromCode(code: number): 'start' | 'move' | 'end' | 'cancel' {
  switch (code) {
    case PHASE.START:
      return 'start';
    case PHASE.MOVE:
      return 'move';
    case PHASE.END:
      return 'end';
    case PHASE.CANCEL:
      return 'cancel';
    default:
      return 'start';
  }
}

function getDefaultQoS(message: Message): QoS {
  switch (message.type) {
    case 'SET':
      return QoS.Confirm;
    case 'PUBLISH':
      return message.signal === 'stream' || message.signal === 'gesture' ? QoS.Fire : QoS.Confirm;
    case 'BUNDLE':
      return QoS.Commit;
    case 'SUBSCRIBE':
    case 'UNSUBSCRIBE':
      return QoS.Confirm;
    default:
      return QoS.Fire;
  }
}

function isMsgpackMap(byte: number): boolean {
  // fixmap: 0x80-0x8F, map16: 0xDE, map32: 0xDF
  return (byte & 0xf0) === 0x80 || byte === 0xde || byte === 0xdf;
}
