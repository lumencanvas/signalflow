//! Frame encoding tests for Clasp core

use clasp_core::{Frame, QoS, MAGIC_BYTE};

#[test]
fn test_frame_basic() {
    let payload = b"test payload";
    let frame = Frame::new(payload.to_vec());

    let encoded = frame.encode();

    // Check magic byte
    assert_eq!(encoded[0], MAGIC_BYTE);

    // Decode
    let decoded = Frame::decode(&encoded).expect("decode failed");
    assert_eq!(decoded.payload(), payload);
}

#[test]
fn test_frame_with_qos() {
    let payload = b"qos test";

    for qos in [QoS::Fire, QoS::Confirm, QoS::Commit] {
        let frame = Frame::new(payload.to_vec()).with_qos(qos);
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).expect("decode failed");

        assert_eq!(decoded.qos(), qos);
        assert_eq!(decoded.payload(), payload);
    }
}

#[test]
fn test_frame_with_timestamp() {
    let payload = b"timestamp test";
    let timestamp = 1234567890u64;

    let frame = Frame::new(payload.to_vec()).with_timestamp(timestamp);
    let encoded = frame.encode();
    let decoded = Frame::decode(&encoded).expect("decode failed");

    assert!(decoded.has_timestamp());
    assert_eq!(decoded.timestamp(), Some(timestamp));
    assert_eq!(decoded.payload(), payload);
}

#[test]
fn test_frame_with_sequence() {
    let payload = b"sequence test";
    let seq = 42u32;

    let frame = Frame::new(payload.to_vec()).with_sequence(seq);
    let encoded = frame.encode();
    let decoded = Frame::decode(&encoded).expect("decode failed");

    assert_eq!(decoded.sequence(), Some(seq));
    assert_eq!(decoded.payload(), payload);
}

#[test]
fn test_frame_full_options() {
    let payload = vec![0u8; 100]; // 100 byte payload
    let timestamp = 9999999999u64;
    let seq = 12345u32;

    let frame = Frame::new(payload.clone())
        .with_qos(QoS::Commit)
        .with_timestamp(timestamp)
        .with_sequence(seq);

    let encoded = frame.encode();
    let decoded = Frame::decode(&encoded).expect("decode failed");

    assert_eq!(decoded.qos(), QoS::Commit);
    assert_eq!(decoded.timestamp(), Some(timestamp));
    assert_eq!(decoded.sequence(), Some(seq));
    assert_eq!(decoded.payload(), &payload);
}

#[test]
fn test_frame_large_payload() {
    // Test with a large payload
    let payload = vec![0xABu8; 65000];

    let frame = Frame::new(payload.clone());
    let encoded = frame.encode();
    let decoded = Frame::decode(&encoded).expect("decode failed");

    assert_eq!(decoded.payload().len(), 65000);
    assert_eq!(decoded.payload(), &payload);
}

#[test]
fn test_frame_invalid_magic() {
    let invalid = vec![0x00, 0x00, 0x00, 0x04, 0x01, 0x02, 0x03, 0x04];
    let result = Frame::decode(&invalid);
    assert!(result.is_err());
}

#[test]
fn test_frame_truncated() {
    // Too short to be valid
    let truncated = vec![MAGIC_BYTE, 0x00];
    let result = Frame::decode(&truncated);
    assert!(result.is_err());
}
