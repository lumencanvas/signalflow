//! MessagePack encoding/decoding for SignalFlow messages

use crate::{Error, Frame, Message, QoS, Result};
use bytes::Bytes;

/// Encode a message to MessagePack bytes
pub fn encode_message(message: &Message) -> Result<Bytes> {
    let bytes = rmp_serde::to_vec_named(message)?;
    Ok(Bytes::from(bytes))
}

/// Decode a message from MessagePack bytes
pub fn decode_message(bytes: &[u8]) -> Result<Message> {
    let message = rmp_serde::from_slice(bytes)?;
    Ok(message)
}

/// Encode a message into a complete frame
pub fn encode(message: &Message) -> Result<Bytes> {
    let payload = encode_message(message)?;
    let frame = Frame::new(payload).with_qos(message.default_qos());
    frame.encode()
}

/// Encode a message with options
pub fn encode_with_options(
    message: &Message,
    qos: Option<QoS>,
    timestamp: Option<u64>,
) -> Result<Bytes> {
    let payload = encode_message(message)?;
    let mut frame = Frame::new(payload);

    if let Some(qos) = qos {
        frame = frame.with_qos(qos);
    } else {
        frame = frame.with_qos(message.default_qos());
    }

    if let Some(ts) = timestamp {
        frame = frame.with_timestamp(ts);
    }

    frame.encode()
}

/// Decode a frame and extract the message
pub fn decode(bytes: &[u8]) -> Result<(Message, Frame)> {
    let frame = Frame::decode(bytes)?;
    let message = decode_message(&frame.payload)?;
    Ok((message, frame))
}

/// Helper to encode just the message payload (without frame)
pub fn encode_payload(message: &Message) -> Result<Vec<u8>> {
    let bytes = rmp_serde::to_vec_named(message)?;
    Ok(bytes)
}

/// Helper to decode just a message payload (without frame)
pub fn decode_payload(bytes: &[u8]) -> Result<Message> {
    decode_message(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_hello_roundtrip() {
        let msg = Message::Hello(HelloMessage {
            version: 2,
            name: "Test Client".to_string(),
            features: vec!["param".to_string(), "event".to_string()],
            capabilities: None,
            token: None,
        });

        let encoded = encode(&msg).unwrap();
        let (decoded, frame) = decode(&encoded).unwrap();

        match decoded {
            Message::Hello(hello) => {
                assert_eq!(hello.version, 2);
                assert_eq!(hello.name, "Test Client");
                assert_eq!(hello.features.len(), 2);
            }
            _ => panic!("Expected Hello message"),
        }

        assert_eq!(frame.flags.qos, QoS::Fire);
    }

    #[test]
    fn test_set_roundtrip() {
        let msg = Message::Set(SetMessage {
            address: "/test/value".to_string(),
            value: Value::Float(0.75),
            revision: Some(42),
            lock: false,
            unlock: false,
        });

        let encoded = encode(&msg).unwrap();
        let (decoded, frame) = decode(&encoded).unwrap();

        match decoded {
            Message::Set(set) => {
                assert_eq!(set.address, "/test/value");
                assert_eq!(set.value.as_f64(), Some(0.75));
                assert_eq!(set.revision, Some(42));
            }
            _ => panic!("Expected Set message"),
        }

        assert_eq!(frame.flags.qos, QoS::Confirm);
    }

    #[test]
    fn test_bundle_roundtrip() {
        let msg = Message::Bundle(BundleMessage {
            timestamp: Some(1000000),
            messages: vec![
                Message::Set(SetMessage {
                    address: "/light/1".to_string(),
                    value: Value::Float(1.0),
                    revision: None,
                    lock: false,
                    unlock: false,
                }),
                Message::Set(SetMessage {
                    address: "/light/2".to_string(),
                    value: Value::Float(0.0),
                    revision: None,
                    lock: false,
                    unlock: false,
                }),
            ],
        });

        let encoded = encode(&msg).unwrap();
        let (decoded, _) = decode(&encoded).unwrap();

        match decoded {
            Message::Bundle(bundle) => {
                assert_eq!(bundle.timestamp, Some(1000000));
                assert_eq!(bundle.messages.len(), 2);
            }
            _ => panic!("Expected Bundle message"),
        }
    }

    #[test]
    fn test_value_types() {
        // Test various value types
        let values = vec![
            Value::Null,
            Value::Bool(true),
            Value::Int(42),
            Value::Float(3.14),
            Value::String("hello".to_string()),
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        ];

        for value in values {
            let msg = Message::Set(SetMessage {
                address: "/test".to_string(),
                value: value.clone(),
                revision: None,
                lock: false,
                unlock: false,
            });

            let encoded = encode(&msg).unwrap();
            let (decoded, _) = decode(&encoded).unwrap();

            match decoded {
                Message::Set(set) => {
                    assert_eq!(set.value, value);
                }
                _ => panic!("Expected Set message"),
            }
        }
    }
}
