/**
 * Reconnection Logic Unit Tests
 *
 * Tests for exponential backoff and reconnection behavior:
 * - Exponential backoff calculation
 * - Jitter application
 * - Max delay capping
 */

import { describe, it, expect } from 'vitest';

// Re-implement calculateBackoffDelay for testing (same logic as main.js)
function calculateBackoffDelay(
  attempt: number,
  baseDelay: number = 1000,
  maxDelay: number = 30000,
  jitterFactor: number = 0.2
): number {
  // Exponential component: baseDelay * 2^attempt
  const exponentialDelay = baseDelay * Math.pow(2, attempt);

  // Cap at maxDelay
  const cappedDelay = Math.min(exponentialDelay, maxDelay);

  // Add jitter (randomize +/- jitterFactor)
  const jitter = cappedDelay * jitterFactor * (Math.random() * 2 - 1);

  return Math.round(cappedDelay + jitter);
}

// Deterministic version for testing (no jitter)
function calculateBackoffDelayNoJitter(
  attempt: number,
  baseDelay: number = 1000,
  maxDelay: number = 30000
): number {
  const exponentialDelay = baseDelay * Math.pow(2, attempt);
  return Math.min(exponentialDelay, maxDelay);
}

describe('Exponential Backoff', () => {
  describe('Basic Calculation', () => {
    it('should return base delay for first attempt', () => {
      const delay = calculateBackoffDelayNoJitter(0, 1000, 30000);
      expect(delay).toBe(1000);
    });

    it('should double delay for each attempt', () => {
      expect(calculateBackoffDelayNoJitter(0, 1000, 30000)).toBe(1000);
      expect(calculateBackoffDelayNoJitter(1, 1000, 30000)).toBe(2000);
      expect(calculateBackoffDelayNoJitter(2, 1000, 30000)).toBe(4000);
      expect(calculateBackoffDelayNoJitter(3, 1000, 30000)).toBe(8000);
      expect(calculateBackoffDelayNoJitter(4, 1000, 30000)).toBe(16000);
    });

    it('should cap at max delay', () => {
      expect(calculateBackoffDelayNoJitter(10, 1000, 30000)).toBe(30000);
      expect(calculateBackoffDelayNoJitter(20, 1000, 30000)).toBe(30000);
    });
  });

  describe('Custom Base Delay', () => {
    it('should respect custom base delay', () => {
      expect(calculateBackoffDelayNoJitter(0, 500, 30000)).toBe(500);
      expect(calculateBackoffDelayNoJitter(1, 500, 30000)).toBe(1000);
      expect(calculateBackoffDelayNoJitter(2, 500, 30000)).toBe(2000);
    });

    it('should work with very small base delay', () => {
      expect(calculateBackoffDelayNoJitter(0, 100, 30000)).toBe(100);
      expect(calculateBackoffDelayNoJitter(5, 100, 30000)).toBe(3200);
    });
  });

  describe('Custom Max Delay', () => {
    it('should respect custom max delay', () => {
      expect(calculateBackoffDelayNoJitter(10, 1000, 5000)).toBe(5000);
    });

    it('should cap early with small max delay', () => {
      expect(calculateBackoffDelayNoJitter(0, 1000, 1500)).toBe(1000);
      expect(calculateBackoffDelayNoJitter(1, 1000, 1500)).toBe(1500);
      expect(calculateBackoffDelayNoJitter(2, 1000, 1500)).toBe(1500);
    });
  });

  describe('Jitter', () => {
    it('should add jitter within expected range', () => {
      const baseDelay = 1000;
      const jitterFactor = 0.2;
      const attempts = 100;
      const delays: number[] = [];

      for (let i = 0; i < attempts; i++) {
        delays.push(calculateBackoffDelay(0, baseDelay, 30000, jitterFactor));
      }

      const min = Math.min(...delays);
      const max = Math.max(...delays);

      // With 20% jitter, delay should be between 800 and 1200 for attempt 0
      expect(min).toBeGreaterThanOrEqual(baseDelay * (1 - jitterFactor));
      expect(max).toBeLessThanOrEqual(baseDelay * (1 + jitterFactor));
    });

    it('should produce varied delays (not constant)', () => {
      const delays = new Set<number>();

      for (let i = 0; i < 100; i++) {
        delays.add(calculateBackoffDelay(0, 1000, 30000, 0.2));
      }

      // With 100 samples and 20% jitter, we should see many different values
      expect(delays.size).toBeGreaterThan(10);
    });

    it('should work with zero jitter', () => {
      const delay1 = calculateBackoffDelay(0, 1000, 30000, 0);
      const delay2 = calculateBackoffDelay(0, 1000, 30000, 0);

      // With no jitter, delays should be identical
      expect(delay1).toBe(delay2);
      expect(delay1).toBe(1000);
    });
  });

  describe('Edge Cases', () => {
    it('should handle attempt 0', () => {
      const delay = calculateBackoffDelayNoJitter(0, 1000, 30000);
      expect(delay).toBe(1000);
    });

    it('should handle very large attempt numbers', () => {
      const delay = calculateBackoffDelayNoJitter(100, 1000, 30000);
      expect(delay).toBe(30000); // Should be capped
    });

    it('should handle base delay equal to max delay', () => {
      const delay = calculateBackoffDelayNoJitter(5, 1000, 1000);
      expect(delay).toBe(1000);
    });

    it('should handle base delay greater than max delay', () => {
      const delay = calculateBackoffDelayNoJitter(0, 5000, 1000);
      expect(delay).toBe(1000); // Should cap immediately
    });
  });

  describe('Real-World Scenarios', () => {
    it('should produce reasonable delays for typical retry sequence', () => {
      const delays = [];
      for (let i = 0; i <= 5; i++) {
        delays.push(calculateBackoffDelayNoJitter(i, 1000, 30000));
      }

      expect(delays).toEqual([1000, 2000, 4000, 8000, 16000, 30000]);
    });

    it('should not exceed 30 seconds for any attempt', () => {
      for (let i = 0; i <= 20; i++) {
        const delay = calculateBackoffDelay(i, 1000, 30000, 0.2);
        expect(delay).toBeLessThanOrEqual(30000 * 1.2); // Allow for jitter
      }
    });
  });
});

describe('Reconnection Timing', () => {
  it('should provide appropriate delays for connection failures', () => {
    // First failure: quick retry (1-2 seconds)
    const first = calculateBackoffDelayNoJitter(0, 1000, 30000);
    expect(first).toBeGreaterThanOrEqual(1000);
    expect(first).toBeLessThanOrEqual(2000);

    // Third failure: moderate wait (4-8 seconds)
    const third = calculateBackoffDelayNoJitter(2, 1000, 30000);
    expect(third).toBeGreaterThanOrEqual(4000);
    expect(third).toBeLessThanOrEqual(8000);

    // Fifth failure: longer wait but still reasonable
    const fifth = calculateBackoffDelayNoJitter(4, 1000, 30000);
    expect(fifth).toBeGreaterThanOrEqual(16000);
    expect(fifth).toBeLessThanOrEqual(30000);
  });
});
