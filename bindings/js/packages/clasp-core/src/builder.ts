/**
 * CLASP client builder
 */

import { Clasp } from './client';
import { ConnectOptions } from './types';

/**
 * Builder for CLASP client
 */
export class ClaspBuilder {
  private url: string;
  private options: ConnectOptions = {};

  constructor(url: string) {
    this.url = url;
  }

  /**
   * Set client name
   */
  name(name: string): this {
    this.options.name = name;
    return this;
  }

  /**
   * Set client name (alias)
   */
  withName(name: string): this {
    return this.name(name);
  }

  /**
   * Set supported features
   */
  features(features: string[]): this {
    this.options.features = features;
    return this;
  }

  /**
   * Set supported features (alias)
   */
  withFeatures(features: string[]): this {
    return this.features(features);
  }

  /**
   * Set authentication token
   */
  token(token: string): this {
    this.options.token = token;
    return this;
  }

  /**
   * Set authentication token (alias)
   */
  withToken(token: string): this {
    return this.token(token);
  }

  /**
   * Enable/disable auto-reconnect
   */
  reconnect(enabled: boolean): this {
    this.options.reconnect = enabled;
    return this;
  }

  /**
   * Enable/disable auto-reconnect with optional interval
   */
  withReconnect(enabled: boolean, intervalMs?: number): this {
    this.options.reconnect = enabled;
    if (intervalMs !== undefined) {
      this.options.reconnectInterval = intervalMs;
    }
    return this;
  }

  /**
   * Set reconnect interval in milliseconds
   */
  reconnectInterval(ms: number): this {
    this.options.reconnectInterval = ms;
    return this;
  }

  /**
   * Build and connect
   */
  async connect(): Promise<Clasp> {
    const client = new Clasp(this.url, this.options);
    await client.connect();
    return client;
  }
}
