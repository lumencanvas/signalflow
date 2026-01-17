/**
 * CLASP codec - frame encoding/decoding
 */

import { encode as msgpackEncode, decode as msgpackDecode } from '@msgpack/msgpack';
import { Message, QoS, FrameFlags } from './types';

/** Magic byte */
export const MAGIC_BYTE = 0x53; // 'S'

// Re-export QoS for convenience
export { QoS };

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
  flags: FrameFlags;
}

/**
 * Encode a raw payload to a frame
 */
export function encodeFrame(
  payload: Uint8Array,
  options: FrameOptions = {}
): Uint8Array {
  // Determine header size
  const hasTimestamp = options.timestamp !== undefined;
  const headerSize = hasTimestamp ? HEADER_SIZE_WITH_TS : HEADER_SIZE;

  // Create frame buffer
  const frame = new Uint8Array(headerSize + payload.length);
  const view = new DataView(frame.buffer);

  // Write header
  frame[0] = MAGIC_BYTE;
  frame[1] = encodeFlags({
    qos: options.qos ?? QoS.Fire,
    hasTimestamp,
    encrypted: false,
    compressed: false,
  });
  view.setUint16(2, payload.length, false); // Big-endian

  // Write timestamp if present
  if (hasTimestamp && options.timestamp !== undefined) {
    const ts = typeof options.timestamp === 'bigint'
      ? options.timestamp
      : BigInt(options.timestamp);
    view.setBigUint64(4, ts, false);
  }

  // Write payload
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
 * Encode a message to a frame
 */
export function encodeMessage(
  message: Message,
  options: FrameOptions = {}
): Uint8Array {
  const payload = msgpackEncode(message);
  return encodeFrame(new Uint8Array(payload), options);
}

/**
 * Decode a frame to a message
 */
export function decodeMessage(data: Uint8Array): {
  message: Message;
  flags: FrameFlags;
  timestamp?: bigint;
} {
  const { payload, flags, timestamp } = decodeFrame(data);
  const message = msgpackDecode(payload) as Message;
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
