/**
 * CLASP codec - frame encoding/decoding
 */

import { encode as msgpackEncode, decode as msgpackDecode } from '@msgpack/msgpack';
import { Message, QoS, FrameFlags } from './types';

/** Magic byte */
const MAGIC = 0x53; // 'S'

/** Header size without timestamp */
const HEADER_SIZE = 4;

/** Header size with timestamp */
const HEADER_SIZE_WITH_TS = 12;

/**
 * Encode frame flags to byte
 */
export function encodeFlags(flags: FrameFlags): number {
  let byte = 0;
  byte |= (flags.qos & 0x03) << 6;
  if (flags.hasTimestamp) byte |= 0x20;
  if (flags.encrypted) byte |= 0x10;
  if (flags.compressed) byte |= 0x08;
  return byte;
}

/**
 * Decode frame flags from byte
 */
export function decodeFlags(byte: number): FrameFlags {
  return {
    qos: ((byte >> 6) & 0x03) as QoS,
    hasTimestamp: (byte & 0x20) !== 0,
    encrypted: (byte & 0x10) !== 0,
    compressed: (byte & 0x08) !== 0,
  };
}

/**
 * Encode a message to a frame
 */
export function encodeFrame(
  message: Message,
  options: { qos?: QoS; timestamp?: number } = {}
): Uint8Array {
  // Encode payload with MessagePack
  const payload = msgpackEncode(message);

  // Determine header size
  const hasTimestamp = options.timestamp !== undefined;
  const headerSize = hasTimestamp ? HEADER_SIZE_WITH_TS : HEADER_SIZE;

  // Create frame buffer
  const frame = new Uint8Array(headerSize + payload.length);
  const view = new DataView(frame.buffer);

  // Write header
  frame[0] = MAGIC;
  frame[1] = encodeFlags({
    qos: options.qos ?? QoS.Fire,
    hasTimestamp,
    encrypted: false,
    compressed: false,
  });
  view.setUint16(2, payload.length, false); // Big-endian

  // Write timestamp if present
  if (hasTimestamp && options.timestamp !== undefined) {
    // Write as BigInt64
    const ts = BigInt(options.timestamp);
    view.setBigUint64(4, ts, false);
  }

  // Write payload
  frame.set(payload, headerSize);

  return frame;
}

/**
 * Decode a frame to a message
 */
export function decodeFrame(data: Uint8Array): {
  message: Message;
  flags: FrameFlags;
  timestamp?: number;
} {
  if (data.length < HEADER_SIZE) {
    throw new Error('Frame too small');
  }

  if (data[0] !== MAGIC) {
    throw new Error(`Invalid magic byte: 0x${data[0].toString(16)}`);
  }

  const view = new DataView(data.buffer, data.byteOffset);
  const flags = decodeFlags(data[1]);
  const payloadLength = view.getUint16(2, false);

  const headerSize = flags.hasTimestamp ? HEADER_SIZE_WITH_TS : HEADER_SIZE;

  if (data.length < headerSize + payloadLength) {
    throw new Error('Frame incomplete');
  }

  let timestamp: number | undefined;
  if (flags.hasTimestamp) {
    timestamp = Number(view.getBigUint64(4, false));
  }

  const payload = data.slice(headerSize, headerSize + payloadLength);
  const message = msgpackDecode(payload) as Message;

  return { message, flags, timestamp };
}

/**
 * Encode a message to bytes (convenience function)
 */
export function encode(message: Message, qos?: QoS): Uint8Array {
  return encodeFrame(message, { qos });
}

/**
 * Decode bytes to a message (convenience function)
 */
export function decode(data: Uint8Array): Message {
  return decodeFrame(data).message;
}

/**
 * Check if buffer contains a complete frame
 */
export function checkComplete(data: Uint8Array): number | null {
  if (data.length < HEADER_SIZE) {
    return null;
  }

  if (data[0] !== MAGIC) {
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
