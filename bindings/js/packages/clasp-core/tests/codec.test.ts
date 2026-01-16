import { describe, it, expect } from 'vitest';
import { encodeFrame, decodeFrame, MAGIC_BYTE, QoS } from '../src/codec';

describe('Frame Codec', () => {
  describe('encodeFrame', () => {
    it('should encode a basic frame', () => {
      const payload = new Uint8Array([1, 2, 3, 4]);
      const frame = encodeFrame(payload);

      expect(frame[0]).toBe(MAGIC_BYTE);
      expect(frame.length).toBeGreaterThan(4);
    });

    it('should encode frame with QoS', () => {
      const payload = new Uint8Array([1, 2, 3]);

      const frameFire = encodeFrame(payload, { qos: QoS.Fire });
      const frameConfirm = encodeFrame(payload, { qos: QoS.Confirm });
      const frameCommit = encodeFrame(payload, { qos: QoS.Commit });

      // QoS is in bits 6-7 of flags byte
      expect(frameFire[1] & 0xC0).toBe(0x00);
      expect(frameConfirm[1] & 0xC0).toBe(0x40);
      expect(frameCommit[1] & 0xC0).toBe(0x80);
    });

    it('should encode frame with timestamp', () => {
      const payload = new Uint8Array([1, 2, 3]);
      const timestamp = BigInt(1234567890);
      const frame = encodeFrame(payload, { timestamp });

      // Timestamp flag should be set
      expect(frame[1] & 0x20).toBe(0x20);
      // Frame should be longer due to timestamp
      expect(frame.length).toBeGreaterThan(4 + 3);
    });
  });

  describe('decodeFrame', () => {
    it('should decode a basic frame', () => {
      const payload = new Uint8Array([10, 20, 30, 40, 50]);
      const encoded = encodeFrame(payload);
      const decoded = decodeFrame(encoded);

      expect(decoded.payload).toEqual(payload);
      expect(decoded.qos).toBe(QoS.Fire);
    });

    it('should decode frame with QoS', () => {
      const payload = new Uint8Array([1, 2, 3]);

      for (const qos of [QoS.Fire, QoS.Confirm, QoS.Commit]) {
        const encoded = encodeFrame(payload, { qos });
        const decoded = decodeFrame(encoded);
        expect(decoded.qos).toBe(qos);
      }
    });

    it('should decode frame with timestamp', () => {
      const payload = new Uint8Array([1, 2, 3]);
      const timestamp = BigInt(9876543210);
      const encoded = encodeFrame(payload, { timestamp });
      const decoded = decodeFrame(encoded);

      expect(decoded.timestamp).toBe(timestamp);
      expect(decoded.payload).toEqual(payload);
    });

    it('should throw on invalid magic byte', () => {
      const invalid = new Uint8Array([0x00, 0x00, 0x00, 0x04, 1, 2, 3, 4]);
      expect(() => decodeFrame(invalid)).toThrow();
    });

    it('should throw on truncated frame', () => {
      const truncated = new Uint8Array([MAGIC_BYTE, 0x00]);
      expect(() => decodeFrame(truncated)).toThrow();
    });
  });

  describe('roundtrip', () => {
    it('should roundtrip various payload sizes', () => {
      const sizes = [0, 1, 10, 100, 1000, 10000];

      for (const size of sizes) {
        const payload = new Uint8Array(size);
        for (let i = 0; i < size; i++) {
          payload[i] = i % 256;
        }

        const encoded = encodeFrame(payload);
        const decoded = decodeFrame(encoded);

        expect(decoded.payload).toEqual(payload);
      }
    });

    it('should roundtrip with all options', () => {
      const payload = new Uint8Array([1, 2, 3, 4, 5]);
      const options = {
        qos: QoS.Commit,
        timestamp: BigInt(1234567890123456),
        sequence: 42,
      };

      const encoded = encodeFrame(payload, options);
      const decoded = decodeFrame(encoded);

      expect(decoded.payload).toEqual(payload);
      expect(decoded.qos).toBe(QoS.Commit);
      expect(decoded.timestamp).toBe(options.timestamp);
    });
  });
});
