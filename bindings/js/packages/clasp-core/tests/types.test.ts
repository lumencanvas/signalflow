import { describe, it, expect } from 'vitest';
import {
  QoS,
  SignalType,
  MessageType,
  isHelloMessage,
  isWelcomeMessage,
  isSetMessage,
  isPublishMessage,
  isSubscribeMessage,
} from '../src/types';
import type {
  HelloMessage,
  WelcomeMessage,
  SetMessage,
  PublishMessage,
  SubscribeMessage,
} from '../src/types';

describe('Types', () => {
  describe('QoS enum', () => {
    it('should have correct values', () => {
      expect(QoS.Fire).toBe(0);
      expect(QoS.Confirm).toBe(1);
      expect(QoS.Commit).toBe(2);
    });
  });

  describe('SignalType enum', () => {
    it('should have correct values', () => {
      expect(SignalType.Param).toBe('param');
      expect(SignalType.Event).toBe('event');
      expect(SignalType.Stream).toBe('stream');
      expect(SignalType.Gesture).toBe('gesture');
      expect(SignalType.Timeline).toBe('timeline');
    });
  });

  describe('MessageType enum', () => {
    it('should have correct values', () => {
      expect(MessageType.Hello).toBe(0x01);
      expect(MessageType.Welcome).toBe(0x02);
      expect(MessageType.Subscribe).toBe(0x10);
      expect(MessageType.Publish).toBe(0x20);
      expect(MessageType.Set).toBe(0x21);
    });
  });

  describe('Type guards', () => {
    it('should identify HelloMessage', () => {
      const msg: HelloMessage = {
        type: 'HELLO',
        version: 2,
        name: 'Test',
        features: ['param'],
      };
      expect(isHelloMessage(msg)).toBe(true);
      expect(isWelcomeMessage(msg)).toBe(false);
    });

    it('should identify WelcomeMessage', () => {
      const msg: WelcomeMessage = {
        type: 'WELCOME',
        version: 2,
        session: 'sess-123',
        name: 'Server',
        features: ['param'],
        time: 1234567890,
      };
      expect(isWelcomeMessage(msg)).toBe(true);
      expect(isHelloMessage(msg)).toBe(false);
    });

    it('should identify SetMessage', () => {
      const msg: SetMessage = {
        type: 'SET',
        address: '/test/path',
        value: 42,
      };
      expect(isSetMessage(msg)).toBe(true);
      expect(isPublishMessage(msg)).toBe(false);
    });

    it('should identify PublishMessage', () => {
      const msg: PublishMessage = {
        type: 'PUBLISH',
        address: '/test/event',
        signal: SignalType.Event,
      };
      expect(isPublishMessage(msg)).toBe(true);
      expect(isSetMessage(msg)).toBe(false);
    });

    it('should identify SubscribeMessage', () => {
      const msg: SubscribeMessage = {
        type: 'SUBSCRIBE',
        id: 1,
        pattern: '/test/*',
      };
      expect(isSubscribeMessage(msg)).toBe(true);
      expect(isSetMessage(msg)).toBe(false);
    });
  });
});

describe('Value types', () => {
  it('should accept null', () => {
    const msg: SetMessage = {
      type: 'SET',
      address: '/test',
      value: null,
    };
    expect(msg.value).toBeNull();
  });

  it('should accept boolean', () => {
    const msg: SetMessage = {
      type: 'SET',
      address: '/test',
      value: true,
    };
    expect(msg.value).toBe(true);
  });

  it('should accept number', () => {
    const msg: SetMessage = {
      type: 'SET',
      address: '/test',
      value: 3.14159,
    };
    expect(msg.value).toBeCloseTo(3.14159);
  });

  it('should accept string', () => {
    const msg: SetMessage = {
      type: 'SET',
      address: '/test',
      value: 'hello world',
    };
    expect(msg.value).toBe('hello world');
  });

  it('should accept array', () => {
    const msg: SetMessage = {
      type: 'SET',
      address: '/test',
      value: [1, 2, 3],
    };
    expect(msg.value).toEqual([1, 2, 3]);
  });

  it('should accept object', () => {
    const msg: SetMessage = {
      type: 'SET',
      address: '/test',
      value: { key: 'value', num: 42 },
    };
    expect(msg.value).toEqual({ key: 'value', num: 42 });
  });

  it('should accept Uint8Array', () => {
    const bytes = new Uint8Array([1, 2, 3, 4]);
    const msg: SetMessage = {
      type: 'SET',
      address: '/test',
      value: bytes,
    };
    expect(msg.value).toEqual(bytes);
  });
});
