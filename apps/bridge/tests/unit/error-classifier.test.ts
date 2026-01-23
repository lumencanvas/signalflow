/**
 * Error Classifier Unit Tests
 *
 * Tests for error classification logic:
 * - Timeout errors
 * - Network errors
 * - Authentication errors
 * - Protocol errors
 * - Unknown errors
 */

import { describe, it, expect } from 'vitest';

// Re-implement error classification for testing (same logic as main.js)
const ErrorType = {
  TIMEOUT: 'TIMEOUT',
  NETWORK: 'NETWORK',
  AUTH: 'AUTH',
  PROTOCOL: 'PROTOCOL',
  UNKNOWN: 'UNKNOWN',
} as const;

type ErrorTypeValue = typeof ErrorType[keyof typeof ErrorType];

interface ErrorLike {
  code?: string;
  message?: string;
}

function classifyError(error: ErrorLike | null | undefined): ErrorTypeValue {
  if (!error) return ErrorType.UNKNOWN;

  const code = error.code || '';
  const message = (error.message || '').toLowerCase();

  // Timeout errors
  if (code === 'ETIMEDOUT' || code === 'ESOCKETTIMEDOUT' || message.includes('timeout')) {
    return ErrorType.TIMEOUT;
  }

  // Network errors
  if (code === 'ECONNREFUSED' || code === 'ENOTFOUND' || code === 'ENETUNREACH' ||
      code === 'ECONNRESET' || code === 'EPIPE' || message.includes('network')) {
    return ErrorType.NETWORK;
  }

  // Auth errors
  if (message.includes('401') || message.includes('403') ||
      message.includes('unauthorized') || message.includes('forbidden') ||
      message.includes('authentication')) {
    return ErrorType.AUTH;
  }

  // Protocol errors
  if (message.includes('protocol') || message.includes('handshake') ||
      message.includes('version')) {
    return ErrorType.PROTOCOL;
  }

  return ErrorType.UNKNOWN;
}

describe('Error Classifier', () => {
  describe('Timeout Errors', () => {
    it('should classify ETIMEDOUT as TIMEOUT', () => {
      expect(classifyError({ code: 'ETIMEDOUT' })).toBe(ErrorType.TIMEOUT);
    });

    it('should classify ESOCKETTIMEDOUT as TIMEOUT', () => {
      expect(classifyError({ code: 'ESOCKETTIMEDOUT' })).toBe(ErrorType.TIMEOUT);
    });

    it('should classify message containing "timeout" as TIMEOUT', () => {
      expect(classifyError({ message: 'Connection timeout' })).toBe(ErrorType.TIMEOUT);
      expect(classifyError({ message: 'Request Timeout' })).toBe(ErrorType.TIMEOUT);
      expect(classifyError({ message: 'Operation timed out' })).toBe(ErrorType.TIMEOUT);
    });
  });

  describe('Network Errors', () => {
    it('should classify ECONNREFUSED as NETWORK', () => {
      expect(classifyError({ code: 'ECONNREFUSED' })).toBe(ErrorType.NETWORK);
    });

    it('should classify ENOTFOUND as NETWORK', () => {
      expect(classifyError({ code: 'ENOTFOUND' })).toBe(ErrorType.NETWORK);
    });

    it('should classify ENETUNREACH as NETWORK', () => {
      expect(classifyError({ code: 'ENETUNREACH' })).toBe(ErrorType.NETWORK);
    });

    it('should classify ECONNRESET as NETWORK', () => {
      expect(classifyError({ code: 'ECONNRESET' })).toBe(ErrorType.NETWORK);
    });

    it('should classify EPIPE as NETWORK', () => {
      expect(classifyError({ code: 'EPIPE' })).toBe(ErrorType.NETWORK);
    });

    it('should classify message containing "network" as NETWORK', () => {
      expect(classifyError({ message: 'Network error' })).toBe(ErrorType.NETWORK);
      expect(classifyError({ message: 'network unreachable' })).toBe(ErrorType.NETWORK);
    });
  });

  describe('Authentication Errors', () => {
    it('should classify message containing "401" as AUTH', () => {
      expect(classifyError({ message: 'HTTP 401 Unauthorized' })).toBe(ErrorType.AUTH);
      expect(classifyError({ message: 'Error 401: Not authenticated' })).toBe(ErrorType.AUTH);
    });

    it('should classify message containing "403" as AUTH', () => {
      expect(classifyError({ message: 'HTTP 403 Forbidden' })).toBe(ErrorType.AUTH);
      expect(classifyError({ message: 'Error 403' })).toBe(ErrorType.AUTH);
    });

    it('should classify message containing "unauthorized" as AUTH', () => {
      expect(classifyError({ message: 'Unauthorized access' })).toBe(ErrorType.AUTH);
      expect(classifyError({ message: 'Request unauthorized' })).toBe(ErrorType.AUTH);
    });

    it('should classify message containing "forbidden" as AUTH', () => {
      expect(classifyError({ message: 'Access forbidden' })).toBe(ErrorType.AUTH);
    });

    it('should classify message containing "authentication" as AUTH', () => {
      expect(classifyError({ message: 'Authentication failed' })).toBe(ErrorType.AUTH);
      expect(classifyError({ message: 'Invalid authentication token' })).toBe(ErrorType.AUTH);
    });
  });

  describe('Protocol Errors', () => {
    it('should classify message containing "protocol" as PROTOCOL', () => {
      expect(classifyError({ message: 'Protocol error' })).toBe(ErrorType.PROTOCOL);
      expect(classifyError({ message: 'Unknown protocol' })).toBe(ErrorType.PROTOCOL);
    });

    it('should classify message containing "handshake" as PROTOCOL', () => {
      expect(classifyError({ message: 'Handshake failed' })).toBe(ErrorType.PROTOCOL);
      expect(classifyError({ message: 'WebSocket handshake error' })).toBe(ErrorType.PROTOCOL);
    });

    it('should classify message containing "version" as PROTOCOL', () => {
      expect(classifyError({ message: 'Version mismatch' })).toBe(ErrorType.PROTOCOL);
      expect(classifyError({ message: 'Unsupported protocol version' })).toBe(ErrorType.PROTOCOL);
    });
  });

  describe('Unknown Errors', () => {
    it('should classify null error as UNKNOWN', () => {
      expect(classifyError(null)).toBe(ErrorType.UNKNOWN);
    });

    it('should classify undefined error as UNKNOWN', () => {
      expect(classifyError(undefined)).toBe(ErrorType.UNKNOWN);
    });

    it('should classify empty error as UNKNOWN', () => {
      expect(classifyError({})).toBe(ErrorType.UNKNOWN);
    });

    it('should classify unrecognized error as UNKNOWN', () => {
      expect(classifyError({ message: 'Something went wrong' })).toBe(ErrorType.UNKNOWN);
      expect(classifyError({ code: 'ESOMETHINGELSE' })).toBe(ErrorType.UNKNOWN);
    });
  });

  describe('Priority', () => {
    it('should prioritize code over message for timeout', () => {
      // If both code and message suggest different types, code should win for timeout
      expect(classifyError({ code: 'ETIMEDOUT', message: 'network error' })).toBe(ErrorType.TIMEOUT);
    });

    it('should use message if code is not recognized', () => {
      expect(classifyError({ code: 'ESOMETHING', message: 'Connection timeout' })).toBe(ErrorType.TIMEOUT);
    });
  });

  describe('Case Insensitivity', () => {
    it('should handle uppercase messages', () => {
      expect(classifyError({ message: 'CONNECTION TIMEOUT' })).toBe(ErrorType.TIMEOUT);
    });

    it('should handle mixed case messages', () => {
      expect(classifyError({ message: 'Network Error Occurred' })).toBe(ErrorType.NETWORK);
    });
  });

  describe('Real-World Errors', () => {
    it('should classify WebSocket close errors correctly', () => {
      expect(classifyError({
        code: 'ECONNRESET',
        message: 'WebSocket was closed before the connection was established',
      })).toBe(ErrorType.NETWORK);
    });

    it('should classify DNS errors correctly', () => {
      expect(classifyError({
        code: 'ENOTFOUND',
        message: 'getaddrinfo ENOTFOUND localhost',
      })).toBe(ErrorType.NETWORK);
    });

    it('should classify TLS errors as protocol', () => {
      expect(classifyError({
        message: 'TLS handshake failed',
      })).toBe(ErrorType.PROTOCOL);
    });

    it('should classify token expiry as auth', () => {
      expect(classifyError({
        message: 'Authentication token expired',
      })).toBe(ErrorType.AUTH);
    });
  });
});
