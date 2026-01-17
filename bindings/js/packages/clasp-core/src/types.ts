/**
 * CLASP types
 */

/** Protocol version */
export const PROTOCOL_VERSION = 2;

/** Default WebSocket port */
export const DEFAULT_WS_PORT = 7330;

/** Default discovery port */
export const DEFAULT_DISCOVERY_PORT = 7331;

/** WebSocket subprotocol */
export const WS_SUBPROTOCOL = 'clasp.v2';

/** Message type codes */
export enum MessageType {
  Hello = 0x01,
  Welcome = 0x02,
  Announce = 0x03,
  Subscribe = 0x10,
  Unsubscribe = 0x11,
  Publish = 0x20,
  Set = 0x21,
  Get = 0x22,
  Snapshot = 0x23,
  Bundle = 0x30,
  Sync = 0x40,
  Ping = 0x41,
  Pong = 0x42,
  Ack = 0x50,
  Error = 0x51,
  Query = 0x60,
  Result = 0x61,
}

/** Quality of Service levels */
export enum QoS {
  /** Best effort, no confirmation */
  Fire = 0,
  /** At least once delivery */
  Confirm = 1,
  /** Exactly once, ordered delivery */
  Commit = 2,
}

/** Signal type values */
export const SignalType = {
  Param: 'param' as const,
  Event: 'event' as const,
  Stream: 'stream' as const,
  Gesture: 'gesture' as const,
  Timeline: 'timeline' as const,
};

/** Signal types */
export type SignalType = 'param' | 'event' | 'stream' | 'gesture' | 'timeline';

/** Value type */
export type Value =
  | null
  | boolean
  | number
  | string
  | Uint8Array
  | Value[]
  | { [key: string]: Value };

/** Frame flags */
export interface FrameFlags {
  qos: QoS;
  hasTimestamp: boolean;
  encrypted: boolean;
  compressed: boolean;
}

/** HELLO message */
export interface HelloMessage {
  type: 'HELLO';
  version: number;
  name: string;
  features: string[];
  capabilities?: {
    encryption?: boolean;
    compression?: string;
  };
  token?: string;
}

/** WELCOME message */
export interface WelcomeMessage {
  type: 'WELCOME';
  version: number;
  session: string;
  name: string;
  features: string[];
  time: number;
  token?: string;
}

/** SUBSCRIBE message */
export interface SubscribeMessage {
  type: 'SUBSCRIBE';
  id: number;
  pattern: string;
  types?: SignalType[];
  options?: SubscribeOptions;
}

/** Subscribe options */
export interface SubscribeOptions {
  maxRate?: number;
  epsilon?: number;
  history?: number;
  window?: number;
}

/** UNSUBSCRIBE message */
export interface UnsubscribeMessage {
  type: 'UNSUBSCRIBE';
  id: number;
}

/** SET message */
export interface SetMessage {
  type: 'SET';
  address: string;
  value: Value;
  revision?: number;
  lock?: boolean;
  unlock?: boolean;
}

/** GET message */
export interface GetMessage {
  type: 'GET';
  address: string;
}

/** PUBLISH message */
export interface PublishMessage {
  type: 'PUBLISH';
  address: string;
  signal?: SignalType;
  value?: Value;
  payload?: Value;
  samples?: number[];
  rate?: number;
  id?: number;
  phase?: 'start' | 'move' | 'end' | 'cancel';
  timestamp?: number;
}

/** SNAPSHOT message */
export interface SnapshotMessage {
  type: 'SNAPSHOT';
  params: ParamValue[];
}

/** Parameter value in snapshot */
export interface ParamValue {
  address: string;
  value: Value;
  revision: number;
  writer?: string;
  timestamp?: number;
}

/** BUNDLE message */
export interface BundleMessage {
  type: 'BUNDLE';
  timestamp?: number;
  messages: Message[];
}

/** ACK message */
export interface AckMessage {
  type: 'ACK';
  address?: string;
  revision?: number;
  locked?: boolean;
  holder?: string;
  correlationId?: number;
}

/** ERROR message */
export interface ErrorMessage {
  type: 'ERROR';
  code: number;
  message: string;
  address?: string;
  correlationId?: number;
}

/** SYNC message */
export interface SyncMessage {
  type: 'SYNC';
  t1: number;
  t2?: number;
  t3?: number;
}

/** QUERY message */
export interface QueryMessage {
  type: 'QUERY';
  pattern: string;
}

/** RESULT message */
export interface ResultMessage {
  type: 'RESULT';
  signals: SignalDefinition[];
}

/** Signal definition */
export interface SignalDefinition {
  address: string;
  type: SignalType;
  datatype?: string;
  access?: string;
  meta?: {
    unit?: string;
    range?: [number, number];
    default?: Value;
    description?: string;
  };
}

/** All message types */
export type Message =
  | HelloMessage
  | WelcomeMessage
  | SubscribeMessage
  | UnsubscribeMessage
  | SetMessage
  | GetMessage
  | PublishMessage
  | SnapshotMessage
  | BundleMessage
  | AckMessage
  | ErrorMessage
  | SyncMessage
  | QueryMessage
  | ResultMessage
  | { type: 'PING' }
  | { type: 'PONG' };

/** Connection options */
export interface ConnectOptions {
  name?: string;
  features?: string[];
  token?: string;
  reconnect?: boolean;
  reconnectInterval?: number;
}

/** Subscription callback */
export type SubscriptionCallback = (value: Value, address: string, meta?: ParamValue) => void;

/** Unsubscribe function */
export type Unsubscribe = () => void;

/** Client events */
export interface ClaspEvents {
  connect: () => void;
  disconnect: (reason?: string) => void;
  error: (error: Error) => void;
  message: (message: Message) => void;
}

// ============================================================================
// Type Guards
// ============================================================================

/** Type guard for HelloMessage */
export function isHelloMessage(msg: unknown): msg is HelloMessage {
  return typeof msg === 'object' && msg !== null && (msg as Message).type === 'HELLO';
}

/** Type guard for WelcomeMessage */
export function isWelcomeMessage(msg: unknown): msg is WelcomeMessage {
  return typeof msg === 'object' && msg !== null && (msg as Message).type === 'WELCOME';
}

/** Type guard for SetMessage */
export function isSetMessage(msg: unknown): msg is SetMessage {
  return typeof msg === 'object' && msg !== null && (msg as Message).type === 'SET';
}

/** Type guard for PublishMessage */
export function isPublishMessage(msg: unknown): msg is PublishMessage {
  return typeof msg === 'object' && msg !== null && (msg as Message).type === 'PUBLISH';
}

/** Type guard for SubscribeMessage */
export function isSubscribeMessage(msg: unknown): msg is SubscribeMessage {
  return typeof msg === 'object' && msg !== null && (msg as Message).type === 'SUBSCRIBE';
}

/** Type guard for ErrorMessage */
export function isErrorMessage(msg: unknown): msg is ErrorMessage {
  return typeof msg === 'object' && msg !== null && (msg as Message).type === 'ERROR';
}

/** Type guard for AckMessage */
export function isAckMessage(msg: unknown): msg is AckMessage {
  return typeof msg === 'object' && msg !== null && (msg as Message).type === 'ACK';
}
