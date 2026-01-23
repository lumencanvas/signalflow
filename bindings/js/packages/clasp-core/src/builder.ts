/**
 * CLASP client builder
 */

import { Clasp } from './client';
import { ConnectOptions } from './types';

/**
 * Builder for CLASP client
 */
export class ClaspBuilder {
  private _url: string;
  private _options: ConnectOptions = {};

  constructor(url: string) {
    this._url = url;
  }

  /**
   * Get the configured URL
   */
  getUrl(): string {
    return this._url;
  }

  /**
   * Get the configured name
   */
  getName(): string | undefined {
    return this._options.name;
  }

  /**
   * Get the configured features
   */
  getFeatures(): string[] | undefined {
    return this._options.features;
  }

  /**
   * Get the configured token
   */
  getToken(): string | undefined {
    return this._options.token;
  }

  /**
   * Get the reconnect setting
   */
  getReconnect(): boolean | undefined {
    return this._options.reconnect;
  }

  /**
   * Get the reconnect interval
   */
  getReconnectInterval(): number | undefined {
    return this._options.reconnectInterval;
  }

  /**
   * Get all options (for testing)
   */
  getOptions(): ConnectOptions {
    return { ...this._options };
  }

  /**
   * Set client name
   */
  name(name: string): this {
    this._options.name = name;
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
    this._options.features = features;
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
    this._options.token = token;
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
    this._options.reconnect = enabled;
    return this;
  }

  /**
   * Enable/disable auto-reconnect with optional interval
   */
  withReconnect(enabled: boolean, intervalMs?: number): this {
    this._options.reconnect = enabled;
    if (intervalMs !== undefined) {
      this._options.reconnectInterval = intervalMs;
    }
    return this;
  }

  /**
   * Set reconnect interval in milliseconds
   */
  reconnectInterval(ms: number): this {
    this._options.reconnectInterval = ms;
    return this;
  }

  /**
   * Build and connect
   */
  async connect(): Promise<Clasp> {
    const client = new Clasp(this._url, this._options);
    await client.connect();
    return client;
  }
}
