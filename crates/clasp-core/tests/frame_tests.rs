//! Frame encoding tests for Clasp core

use bytes::Bytes;
use clasp_core::{Frame, QoS, MAGIC_BYTE};

#[test]
fn test_frame_basic() {
    let payload = Bytes::from_static(b"test payload");
    let frame = Frame::new(payload.clone());

    let encoded = frame.encode().expect("encode failed");

    // Check magic byte
    assert_eq!(encoded[0], MAGIC_BYTE);

    // Decode
    let decoded = Frame::decode(encoded).expect("decode failed");
    assert_eq!(decoded.payload, payload);
}

#[test]
fn test_frame_with_qos() {
    let payload = Bytes::from_static(b"qos test");

    for qos in [QoS::Fire, QoS::Confirm, QoS::Commit] {
        let frame = Frame::new(payload.clone()).with_qos(qos);
        let encoded = frame.encode().expect("encode failed");
        let decoded = Frame::decode(encoded).expect("decode failed");

        assert_eq!(decoded.flags.qos, qos);
        assert_eq!(decoded.payload, payload);
    }
}

#[test]
fn test_frame_with_timestamp() {
    let payload = Bytes::from_static(b"timestamp test");
    let timestamp = 1234567890u64;

    let frame = Frame::new(payload.clone()).with_timestamp(timestamp);
    let encoded = frame.encode().expect("encode failed");
    let decoded = Frame::decode(encoded).expect("decode failed");

    assert!(decoded.flags.has_timestamp);
    assert_eq!(decoded.timestamp, Some(timestamp));
    assert_eq!(decoded.payload, payload);
}

#[test]
fn test_frame_full_options() {
    let payload = Bytes::from(vec![0u8; 100]); // 100 byte payload
    let timestamp = 9999999999u64;

    let frame = Frame::new(payload.clone())
        .with_qos(QoS::Commit)
        .with_timestamp(timestamp);

    let encoded = frame.encode().expect("encode failed");
    let decoded = Frame::decode(encoded).expect("decode failed");

    assert_eq!(decoded.flags.qos, QoS::Commit);
    assert_eq!(decoded.timestamp, Some(timestamp));
    assert_eq!(decoded.payload, payload);
}

#[test]
fn test_frame_large_payload() {
    // Test with a large payload
    let payload = Bytes::from(vec![0xABu8; 65000]);

    let frame = Frame::new(payload.clone());
    let encoded = frame.encode().expect("encode failed");
    let decoded = Frame::decode(encoded).expect("decode failed");

    assert_eq!(decoded.payload.len(), 65000);
    assert_eq!(decoded.payload, payload);
}

#[test]
fn test_frame_invalid_magic() {
    let invalid = Bytes::from(vec![0x00u8, 0x00, 0x00, 0x04, 0x01, 0x02, 0x03, 0x04]);
    let result = Frame::decode(invalid);
    assert!(result.is_err());
}

#[test]
fn test_frame_truncated() {
    // Too short to be valid
    let truncated = Bytes::from(vec![MAGIC_BYTE, 0x00]);
    let result = Frame::decode(truncated);
    assert!(result.is_err());
}
