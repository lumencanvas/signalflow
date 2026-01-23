/**
 * Circuit Breaker Unit Tests
 *
 * Tests for the CircuitBreaker class functionality:
 * - Opens after failure threshold
 * - Transitions to half-open after timeout
 * - Closes after successful request in half-open
 * - Respects max retries
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Re-implement CircuitBreaker for testing (same logic as main.js)
const CircuitState = {
  CLOSED: 'CLOSED',
  OPEN: 'OPEN',
  HALF_OPEN: 'HALF_OPEN',
} as const;

type CircuitStateType = typeof CircuitState[keyof typeof CircuitState];

interface CircuitBreakerOptions {
  failureThreshold?: number;
  resetTimeout?: number;
  maxRetries?: number;
  halfOpenMaxAttempts?: number;
}

class CircuitBreaker {
  private failureThreshold: number;
  private resetTimeout: number;
  private maxRetries: number;
  private halfOpenMaxAttempts: number;

  private state: CircuitStateType = CircuitState.CLOSED;
  private failures: number = 0;
  private retries: number = 0;
  private lastFailure: number | null = null;
  private halfOpenAttempts: number = 0;

  constructor(options: CircuitBreakerOptions = {}) {
    this.failureThreshold = options.failureThreshold || 3;
    this.resetTimeout = options.resetTimeout || 30000;
    this.maxRetries = options.maxRetries || 10;
    this.halfOpenMaxAttempts = options.halfOpenMaxAttempts || 1;
  }

  shouldRetry(): boolean {
    if (this.retries >= this.maxRetries) {
      return false;
    }

    switch (this.state) {
      case CircuitState.CLOSED:
        return true;

      case CircuitState.OPEN:
        if (this.lastFailure && Date.now() - this.lastFailure >= this.resetTimeout) {
          this.state = CircuitState.HALF_OPEN;
          this.halfOpenAttempts = 0;
          return true;
        }
        return false;

      case CircuitState.HALF_OPEN:
        return this.halfOpenAttempts < this.halfOpenMaxAttempts;

      default:
        return false;
    }
  }

  recordSuccess(): void {
    this.failures = 0;
    this.retries = 0;
    this.state = CircuitState.CLOSED;
    this.halfOpenAttempts = 0;
  }

  recordFailure(): void {
    this.failures++;
    this.retries++;
    this.lastFailure = Date.now();

    if (this.state === CircuitState.HALF_OPEN) {
      this.halfOpenAttempts++;
      if (this.halfOpenAttempts >= this.halfOpenMaxAttempts) {
        this.state = CircuitState.OPEN;
      }
    } else if (this.failures >= this.failureThreshold) {
      this.state = CircuitState.OPEN;
    }
  }

  getState(): CircuitStateType {
    return this.state;
  }

  getRetryCount(): number {
    return this.retries;
  }

  reset(): void {
    this.state = CircuitState.CLOSED;
    this.failures = 0;
    this.retries = 0;
    this.lastFailure = null;
    this.halfOpenAttempts = 0;
  }
}

describe('CircuitBreaker', () => {
  let breaker: CircuitBreaker;

  beforeEach(() => {
    breaker = new CircuitBreaker({
      failureThreshold: 3,
      resetTimeout: 1000, // 1 second for tests
      maxRetries: 10,
      halfOpenMaxAttempts: 1,
    });
  });

  describe('Initial State', () => {
    it('should start in CLOSED state', () => {
      expect(breaker.getState()).toBe(CircuitState.CLOSED);
    });

    it('should allow retries in CLOSED state', () => {
      expect(breaker.shouldRetry()).toBe(true);
    });

    it('should have zero retry count initially', () => {
      expect(breaker.getRetryCount()).toBe(0);
    });
  });

  describe('Failure Threshold', () => {
    it('should remain CLOSED under threshold', () => {
      breaker.recordFailure();
      breaker.recordFailure();
      expect(breaker.getState()).toBe(CircuitState.CLOSED);
      expect(breaker.getRetryCount()).toBe(2);
    });

    it('should open after reaching threshold', () => {
      breaker.recordFailure();
      breaker.recordFailure();
      breaker.recordFailure();
      expect(breaker.getState()).toBe(CircuitState.OPEN);
    });

    it('should not allow retries when OPEN', () => {
      breaker.recordFailure();
      breaker.recordFailure();
      breaker.recordFailure();
      expect(breaker.shouldRetry()).toBe(false);
    });
  });

  describe('Half-Open State', () => {
    it('should transition to HALF_OPEN after reset timeout', async () => {
      // Open the circuit
      breaker.recordFailure();
      breaker.recordFailure();
      breaker.recordFailure();
      expect(breaker.getState()).toBe(CircuitState.OPEN);

      // Wait for reset timeout
      await new Promise(resolve => setTimeout(resolve, 1100));

      // Should transition to HALF_OPEN when shouldRetry is called
      expect(breaker.shouldRetry()).toBe(true);
      expect(breaker.getState()).toBe(CircuitState.HALF_OPEN);
    });

    it('should close on success in HALF_OPEN', async () => {
      // Open the circuit
      breaker.recordFailure();
      breaker.recordFailure();
      breaker.recordFailure();

      // Wait for reset timeout
      await new Promise(resolve => setTimeout(resolve, 1100));
      breaker.shouldRetry(); // Transitions to HALF_OPEN

      // Record success
      breaker.recordSuccess();
      expect(breaker.getState()).toBe(CircuitState.CLOSED);
    });

    it('should re-open on failure in HALF_OPEN', async () => {
      // Open the circuit
      breaker.recordFailure();
      breaker.recordFailure();
      breaker.recordFailure();

      // Wait for reset timeout
      await new Promise(resolve => setTimeout(resolve, 1100));
      breaker.shouldRetry(); // Transitions to HALF_OPEN

      // Record failure
      breaker.recordFailure();
      expect(breaker.getState()).toBe(CircuitState.OPEN);
    });
  });

  describe('Max Retries', () => {
    it('should stop retrying after max retries', () => {
      const maxRetries = 10;

      for (let i = 0; i < maxRetries; i++) {
        breaker.recordFailure();
      }

      expect(breaker.getRetryCount()).toBe(maxRetries);
      expect(breaker.shouldRetry()).toBe(false);
    });

    it('should respect max retries even in CLOSED state', () => {
      const limitedBreaker = new CircuitBreaker({
        failureThreshold: 100, // High threshold
        maxRetries: 5,
      });

      for (let i = 0; i < 5; i++) {
        limitedBreaker.recordFailure();
      }

      expect(limitedBreaker.getState()).toBe(CircuitState.CLOSED);
      expect(limitedBreaker.shouldRetry()).toBe(false);
    });
  });

  describe('Success Recovery', () => {
    it('should reset failure count on success', () => {
      breaker.recordFailure();
      breaker.recordFailure();
      expect(breaker.getRetryCount()).toBe(2);

      breaker.recordSuccess();
      expect(breaker.getRetryCount()).toBe(0);
      expect(breaker.getState()).toBe(CircuitState.CLOSED);
    });

    it('should allow retries after success', () => {
      breaker.recordFailure();
      breaker.recordFailure();
      breaker.recordFailure();
      expect(breaker.getState()).toBe(CircuitState.OPEN);

      breaker.recordSuccess();
      expect(breaker.shouldRetry()).toBe(true);
    });
  });

  describe('Reset', () => {
    it('should fully reset the circuit breaker', () => {
      breaker.recordFailure();
      breaker.recordFailure();
      breaker.recordFailure();

      breaker.reset();

      expect(breaker.getState()).toBe(CircuitState.CLOSED);
      expect(breaker.getRetryCount()).toBe(0);
      expect(breaker.shouldRetry()).toBe(true);
    });
  });

  describe('Custom Configuration', () => {
    it('should respect custom failure threshold', () => {
      const customBreaker = new CircuitBreaker({ failureThreshold: 5 });

      for (let i = 0; i < 4; i++) {
        customBreaker.recordFailure();
      }
      expect(customBreaker.getState()).toBe(CircuitState.CLOSED);

      customBreaker.recordFailure();
      expect(customBreaker.getState()).toBe(CircuitState.OPEN);
    });

    it('should respect custom max retries', () => {
      const customBreaker = new CircuitBreaker({ maxRetries: 3 });

      customBreaker.recordFailure();
      customBreaker.recordFailure();
      customBreaker.recordFailure();

      expect(customBreaker.shouldRetry()).toBe(false);
    });
  });
});
