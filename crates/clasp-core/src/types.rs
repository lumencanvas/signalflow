//! Protocol types and message definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message type codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MessageType {
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

impl MessageType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(MessageType::Hello),
            0x02 => Some(MessageType::Welcome),
            0x03 => Some(MessageType::Announce),
            0x10 => Some(MessageType::Subscribe),
            0x11 => Some(MessageType::Unsubscribe),
            0x20 => Some(MessageType::Publish),
            0x21 => Some(MessageType::Set),
            0x22 => Some(MessageType::Get),
            0x23 => Some(MessageType::Snapshot),
            0x30 => Some(MessageType::Bundle),
            0x40 => Some(MessageType::Sync),
            0x41 => Some(MessageType::Ping),
            0x42 => Some(MessageType::Pong),
            0x50 => Some(MessageType::Ack),
            0x51 => Some(MessageType::Error),
            0x60 => Some(MessageType::Query),
            0x61 => Some(MessageType::Result),
            _ => None,
        }
    }
}

/// Quality of Service levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum QoS {
    /// Best effort, no confirmation
    #[default]
    Fire = 0,
    /// At least once delivery
    Confirm = 1,
    /// Exactly once, ordered delivery
    Commit = 2,
}

impl QoS {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(QoS::Fire),
            1 => Some(QoS::Confirm),
            2 => Some(QoS::Commit),
            _ => None,
        }
    }
}

/// Signal types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalType {
    /// Stateful parameter with revision tracking
    Param,
    /// Ephemeral trigger event
    Event,
    /// High-rate continuous data
    Stream,
    /// Phased input (touch/pen/motion)
    Gesture,
    /// Time-indexed automation
    Timeline,
}

impl SignalType {
    pub fn default_qos(&self) -> QoS {
        match self {
            SignalType::Param => QoS::Confirm,
            SignalType::Event => QoS::Confirm,
            SignalType::Stream => QoS::Fire,
            SignalType::Gesture => QoS::Fire,
            SignalType::Timeline => QoS::Commit,
        }
    }
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConflictStrategy {
    /// Last write wins (by timestamp)
    #[default]
    Lww,
    /// Keep maximum value
    Max,
    /// Keep minimum value
    Min,
    /// First writer holds lock
    Lock,
    /// Application-defined merge
    Merge,
}

/// Gesture phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GesturePhase {
    Start,
    Move,
    End,
    Cancel,
}

/// Easing functions for timeline interpolation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EasingType {
    /// Linear interpolation (constant speed)
    #[default]
    Linear,
    /// Slow start, fast end
    EaseIn,
    /// Fast start, slow end
    EaseOut,
    /// Slow start and end, fast middle
    EaseInOut,
    /// Step function (instant change at keyframe)
    Step,
    /// Custom cubic bezier (control points in keyframe)
    CubicBezier,
}

/// A keyframe in a timeline
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineKeyframe {
    /// Time offset in microseconds from timeline start
    pub time: u64,
    /// Value at this keyframe
    pub value: Value,
    /// Easing function to next keyframe
    #[serde(default)]
    pub easing: EasingType,
    /// Optional cubic bezier control points [x1, y1, x2, y2] for CubicBezier easing
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bezier: Option<[f64; 4]>,
}

/// Timeline automation message
/// Immutable once published - to modify, publish a new timeline
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineData {
    /// Keyframes in chronological order
    pub keyframes: Vec<TimelineKeyframe>,
    /// Whether the timeline loops
    #[serde(default)]
    pub loop_: bool,
    /// When to start playback (absolute server time in µs)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<u64>,
    /// Duration override in µs (if None, derived from last keyframe)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

impl TimelineData {
    /// Create a new timeline with keyframes
    pub fn new(keyframes: Vec<TimelineKeyframe>) -> Self {
        Self {
            keyframes,
            loop_: false,
            start_time: None,
            duration: None,
        }
    }

    /// Set looping
    pub fn with_loop(mut self, loop_: bool) -> Self {
        self.loop_ = loop_;
        self
    }

    /// Set start time
    pub fn with_start_time(mut self, time: u64) -> Self {
        self.start_time = Some(time);
        self
    }

    /// Get the timeline duration
    pub fn duration(&self) -> u64 {
        self.duration.unwrap_or_else(|| {
            self.keyframes.last().map(|kf| kf.time).unwrap_or(0)
        })
    }
}

/// Value type that can be sent in messages
/// NOTE: Variant order matters for serde(untagged) - Array must come before Bytes
/// because MessagePack arrays of small integers can be misinterpreted as binary data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Bytes(Vec<u8>),
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v as f64)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v as i64)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

/// Protocol message enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "HELLO")]
    Hello(HelloMessage),

    #[serde(rename = "WELCOME")]
    Welcome(WelcomeMessage),

    #[serde(rename = "ANNOUNCE")]
    Announce(AnnounceMessage),

    #[serde(rename = "SUBSCRIBE")]
    Subscribe(SubscribeMessage),

    #[serde(rename = "UNSUBSCRIBE")]
    Unsubscribe(UnsubscribeMessage),

    #[serde(rename = "PUBLISH")]
    Publish(PublishMessage),

    #[serde(rename = "SET")]
    Set(SetMessage),

    #[serde(rename = "GET")]
    Get(GetMessage),

    #[serde(rename = "SNAPSHOT")]
    Snapshot(SnapshotMessage),

    #[serde(rename = "BUNDLE")]
    Bundle(BundleMessage),

    #[serde(rename = "SYNC")]
    Sync(SyncMessage),

    #[serde(rename = "PING")]
    Ping,

    #[serde(rename = "PONG")]
    Pong,

    #[serde(rename = "ACK")]
    Ack(AckMessage),

    #[serde(rename = "ERROR")]
    Error(ErrorMessage),

    #[serde(rename = "QUERY")]
    Query(QueryMessage),

    #[serde(rename = "RESULT")]
    Result(ResultMessage),
}

/// HELLO message - connection initiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloMessage {
    pub version: u8,
    pub name: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// WELCOME message - connection accepted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeMessage {
    pub version: u8,
    pub session: String,
    pub name: String,
    #[serde(default)]
    pub features: Vec<String>,
    pub time: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Client/server capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub encryption: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
}

/// ANNOUNCE message - capability advertisement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnounceMessage {
    pub namespace: String,
    #[serde(default)]
    pub signals: Vec<SignalDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

/// Signal definition for announcements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalDefinition {
    pub address: String,
    #[serde(rename = "type")]
    pub signal_type: SignalType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datatype: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<SignalMeta>,
}

/// Signal metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<(f64, f64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// SUBSCRIBE message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeMessage {
    pub id: u32,
    pub pattern: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<SignalType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<SubscribeOptions>,
}

/// Subscription options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscribeOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<u32>,
}

/// UNSUBSCRIBE message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeMessage {
    pub id: u32,
}

/// PUBLISH message - for events, streams, gestures, timelines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishMessage {
    pub address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<SignalType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    // For streams
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub samples: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<u32>,
    // For gestures
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<GesturePhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    // For timelines
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<TimelineData>,
}

/// SET message - set param value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetMessage {
    pub address: String,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    #[serde(default)]
    pub lock: bool,
    #[serde(default)]
    pub unlock: bool,
}

/// GET message - request current value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMessage {
    pub address: String,
}

/// SNAPSHOT message - current state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMessage {
    pub params: Vec<ParamValue>,
}

/// Parameter value in snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamValue {
    pub address: String,
    pub value: Value,
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

/// BUNDLE message - atomic group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMessage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    pub messages: Vec<Message>,
}

/// SYNC message - clock synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    pub t1: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t2: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t3: Option<u64>,
}

/// ACK message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckMessage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<u32>,
}

/// ERROR message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub code: u16,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<u32>,
}

/// QUERY message - introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMessage {
    pub pattern: String,
}

/// RESULT message - query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMessage {
    pub signals: Vec<SignalDefinition>,
}

impl Message {
    /// Get the message type code
    pub fn type_code(&self) -> MessageType {
        match self {
            Message::Hello(_) => MessageType::Hello,
            Message::Welcome(_) => MessageType::Welcome,
            Message::Announce(_) => MessageType::Announce,
            Message::Subscribe(_) => MessageType::Subscribe,
            Message::Unsubscribe(_) => MessageType::Unsubscribe,
            Message::Publish(_) => MessageType::Publish,
            Message::Set(_) => MessageType::Set,
            Message::Get(_) => MessageType::Get,
            Message::Snapshot(_) => MessageType::Snapshot,
            Message::Bundle(_) => MessageType::Bundle,
            Message::Sync(_) => MessageType::Sync,
            Message::Ping => MessageType::Ping,
            Message::Pong => MessageType::Pong,
            Message::Ack(_) => MessageType::Ack,
            Message::Error(_) => MessageType::Error,
            Message::Query(_) => MessageType::Query,
            Message::Result(_) => MessageType::Result,
        }
    }

    /// Get the default QoS for this message type
    pub fn default_qos(&self) -> QoS {
        match self {
            Message::Set(_) => QoS::Confirm,
            Message::Publish(p) => p.signal.map(|s| s.default_qos()).unwrap_or(QoS::Fire),
            Message::Bundle(_) => QoS::Commit,
            Message::Subscribe(_) | Message::Unsubscribe(_) => QoS::Confirm,
            _ => QoS::Fire,
        }
    }
}
