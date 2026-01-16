/**
 * CLASP client implementation
 */

import { encode, decode, encodeFrame } from './codec';
import {
  Message,
  Value,
  ConnectOptions,
  SubscriptionCallback,
  Unsubscribe,
  QoS,
  PROTOCOL_VERSION,
  WS_SUBPROTOCOL,
  SetMessage,
  SubscribeMessage,
  HelloMessage,
  WelcomeMessage,
  SnapshotMessage,
  PublishMessage,
  ParamValue,
} from './types';
import { ClaspBuilder } from './builder';

/**
 * Pattern matching for subscriptions
 */
function matchPattern(pattern: string, address: string): boolean {
  const regex = pattern
    .replace(/\*\*/g, '§§')
    .replace(/\*/g, '[^/]+')
    .replace(/§§/g, '.*');
  return new RegExp(`^${regex}$`).test(address);
}

/**
 * CLASP client
 */
export class Clasp {
  private url: string;
  private options: ConnectOptions;
  private ws: WebSocket | null = null;
  private sessionId: string | null = null;
  private _connected = false;
  private params = new Map<string, Value>();
  private subscriptions = new Map<number, { pattern: string; callback: SubscriptionCallback }>();
  private nextSubId = 1;
  private serverTimeOffset = 0;
  private pendingGets = new Map<string, (value: Value) => void>();

  // Event callbacks
  private onConnectCallbacks: (() => void)[] = [];
  private onDisconnectCallbacks: ((reason?: string) => void)[] = [];
  private onErrorCallbacks: ((error: Error) => void)[] = [];

  constructor(url: string, options: ConnectOptions = {}) {
    this.url = url;
    this.options = {
      name: 'CLASP JS Client',
      features: ['param', 'event', 'stream'],
      reconnect: true,
      reconnectInterval: 5000,
      ...options,
    };
  }

  /**
   * Create a builder
   */
  static builder(url: string): ClaspBuilder {
    return new ClaspBuilder(url);
  }

  /**
   * Connect to server
   */
  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url, WS_SUBPROTOCOL);
        this.ws.binaryType = 'arraybuffer';

        this.ws.onopen = () => {
          this.sendHello();
        };

        this.ws.onmessage = (event) => {
          const data = new Uint8Array(event.data as ArrayBuffer);
          try {
            const message = decode(data);
            this.handleMessage(message);

            if (message.type === 'WELCOME') {
              this._connected = true;
              resolve();
              this.onConnectCallbacks.forEach((cb) => cb());
            }
          } catch (e) {
            console.warn('Decode error:', e);
          }
        };

        this.ws.onerror = (event) => {
          const error = new Error('WebSocket error');
          this.onErrorCallbacks.forEach((cb) => cb(error));
          reject(error);
        };

        this.ws.onclose = (event) => {
          this._connected = false;
          this.onDisconnectCallbacks.forEach((cb) => cb(event.reason));

          // Reconnect if enabled
          if (this.options.reconnect) {
            setTimeout(() => {
              this.connect().catch(() => {});
            }, this.options.reconnectInterval);
          }
        };
      } catch (e) {
        reject(e);
      }
    });
  }

  /**
   * Check if connected
   */
  get connected(): boolean {
    return this._connected;
  }

  /**
   * Get session ID
   */
  get session(): string | null {
    return this.sessionId;
  }

  /**
   * Get current server time (microseconds)
   */
  time(): number {
    return Date.now() * 1000 + this.serverTimeOffset;
  }

  /**
   * Subscribe to an address pattern
   */
  subscribe(pattern: string, callback: SubscriptionCallback, options?: { maxRate?: number; epsilon?: number }): Unsubscribe {
    const id = this.nextSubId++;

    this.subscriptions.set(id, { pattern, callback });

    const msg: SubscribeMessage = {
      type: 'SUBSCRIBE',
      id,
      pattern,
      options: options ? { maxRate: options.maxRate, epsilon: options.epsilon } : undefined,
    };

    this.send(msg);

    return () => {
      this.subscriptions.delete(id);
      this.send({ type: 'UNSUBSCRIBE', id });
    };
  }

  /**
   * Shorthand for subscribe
   */
  on(pattern: string, callback: SubscriptionCallback, options?: { maxRate?: number; epsilon?: number }): Unsubscribe {
    return this.subscribe(pattern, callback, options);
  }

  /**
   * Set a parameter value
   */
  set(address: string, value: Value): void {
    const msg: SetMessage = {
      type: 'SET',
      address,
      value,
    };
    this.send(msg, QoS.Confirm);
  }

  /**
   * Get current value (from cache or server)
   */
  async get(address: string): Promise<Value> {
    // Check cache first
    if (this.params.has(address)) {
      return this.params.get(address)!;
    }

    // Request from server
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingGets.delete(address);
        reject(new Error('Timeout'));
      }, 5000);

      this.pendingGets.set(address, (value) => {
        clearTimeout(timeout);
        resolve(value);
      });

      this.send({ type: 'GET', address });
    });
  }

  /**
   * Emit an event
   */
  emit(address: string, payload?: Value): void {
    const msg: PublishMessage = {
      type: 'PUBLISH',
      address,
      signal: 'event',
      payload: payload ?? null,
      timestamp: this.time(),
    };
    this.send(msg, QoS.Confirm);
  }

  /**
   * Send stream sample
   */
  stream(address: string, value: Value): void {
    const msg: PublishMessage = {
      type: 'PUBLISH',
      address,
      signal: 'stream',
      value,
      timestamp: this.time(),
    };
    this.send(msg, QoS.Fire);
  }

  /**
   * Send atomic bundle
   */
  bundle(messages: Array<{ set?: [string, Value]; emit?: [string, Value] }>, options?: { at?: number }): void {
    const formatted: Message[] = messages.map((m) => {
      if (m.set) {
        return { type: 'SET' as const, address: m.set[0], value: m.set[1] };
      }
      if (m.emit) {
        return { type: 'PUBLISH' as const, address: m.emit[0], signal: 'event' as const, payload: m.emit[1] };
      }
      throw new Error('Invalid bundle message');
    });

    this.send(
      { type: 'BUNDLE', timestamp: options?.at, messages: formatted },
      QoS.Commit
    );
  }

  /**
   * Get cached value
   */
  cached(address: string): Value | undefined {
    return this.params.get(address);
  }

  /**
   * Register connect callback
   */
  onConnect(callback: () => void): void {
    this.onConnectCallbacks.push(callback);
  }

  /**
   * Register disconnect callback
   */
  onDisconnect(callback: (reason?: string) => void): void {
    this.onDisconnectCallbacks.push(callback);
  }

  /**
   * Register error callback
   */
  onError(callback: (error: Error) => void): void {
    this.onErrorCallbacks.push(callback);
  }

  /**
   * Close connection
   */
  close(): void {
    this.options.reconnect = false;
    this.ws?.close();
    this.ws = null;
    this._connected = false;
  }

  // Private methods

  private sendHello(): void {
    const hello: HelloMessage = {
      type: 'HELLO',
      version: PROTOCOL_VERSION,
      name: this.options.name!,
      features: this.options.features!,
      token: this.options.token,
    };
    this.send(hello);
  }

  private send(message: Message, qos: QoS = QoS.Fire): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      const frame = encodeFrame(message, { qos });
      this.ws.send(frame);
    }
  }

  private handleMessage(message: Message): void {
    switch (message.type) {
      case 'WELCOME': {
        const welcome = message as WelcomeMessage;
        this.sessionId = welcome.session;
        this.serverTimeOffset = welcome.time - Date.now() * 1000;
        break;
      }

      case 'SET': {
        const set = message as SetMessage;
        this.params.set(set.address, set.value);
        this.notifySubscribers(set.address, set.value);
        break;
      }

      case 'SNAPSHOT': {
        const snapshot = message as SnapshotMessage;
        for (const param of snapshot.params) {
          this.params.set(param.address, param.value);

          // Resolve pending gets
          const resolver = this.pendingGets.get(param.address);
          if (resolver) {
            resolver(param.value);
            this.pendingGets.delete(param.address);
          }

          this.notifySubscribers(param.address, param.value, param);
        }
        break;
      }

      case 'PUBLISH': {
        const pub = message as PublishMessage;
        const value = pub.value ?? pub.payload ?? null;
        this.notifySubscribers(pub.address, value);
        break;
      }

      case 'PING':
        this.send({ type: 'PONG' });
        break;

      case 'ERROR':
        console.error('CLASP error:', message);
        break;
    }
  }

  private notifySubscribers(address: string, value: Value, meta?: ParamValue): void {
    for (const [, sub] of this.subscriptions) {
      if (matchPattern(sub.pattern, address)) {
        sub.callback(value, address, meta);
      }
    }
  }
}
