//! Time synchronization tests

use clasp_core::time::{ClockSync, JitterBuffer, SessionTime};
use std::time::Duration;

#[test]
fn test_clock_sync_new() {
    let sync = ClockSync::new();
    assert_eq!(sync.rtt(), 0);
    assert_eq!(sync.offset(), 0);
}

#[test]
fn test_clock_sync_process() {
    let mut sync = ClockSync::new();

    // Simulate a sync exchange with 100ms RTT
    // t1: client sends at time 1000
    // t2: server receives at 1050 (50ms network delay)
    // t3: server sends at 1060 (10ms processing)
    // t4: client receives at 1110 (50ms network delay back)
    let t1: u64 = 1000;
    let t2: u64 = 1050;
    let t3: u64 = 1060;
    let t4: u64 = 1110;

    sync.process_sync(t1, t2, t3, t4);

    // RTT = (t4 - t1) - (t3 - t2) = 110 - 10 = 100
    assert!(sync.rtt() > 0);
    assert!(sync.quality() > 0.0);
}

#[test]
fn test_clock_sync_multiple_samples() {
    let mut sync = ClockSync::new();

    // Add multiple samples
    let base: u64 = 1000000;
    for i in 0..5 {
        let offset = (i as u64) * 1000;
        sync.process_sync(
            base + offset,
            base + offset + 100,
            base + offset + 150,
            base + offset + 250,
        );
    }

    // Should have reasonable values after multiple samples
    assert!(sync.rtt() > 0);
    assert!(sync.quality() > 0.0);
}

#[test]
fn test_session_time() {
    let session = SessionTime::new();
    std::thread::sleep(Duration::from_millis(10));

    let elapsed = session.elapsed();
    assert!(elapsed >= 10_000); // At least 10ms in microseconds
}

#[test]
fn test_session_time_relative() {
    let session = SessionTime::new();

    let t1 = session.elapsed();
    std::thread::sleep(Duration::from_millis(10));
    let t2 = session.elapsed();

    assert!(t2 > t1);
}

#[test]
fn test_jitter_buffer_empty() {
    let mut buffer: JitterBuffer<i32> = JitterBuffer::new(100, 5000); // 5 second window
    let now = clasp_core::time::now();
    // Empty buffer returns None for any playback time
    assert!(buffer.pop(now + 1000).is_none());
}

#[test]
fn test_jitter_buffer_ordered() {
    let mut buffer: JitterBuffer<i32> = JitterBuffer::new(100, 5000); // 5 second window
    let now = clasp_core::time::now();

    // Add items in order with timestamps relative to now
    buffer.push(now + 100, 1);
    buffer.push(now + 200, 2);
    buffer.push(now + 300, 3);

    // Should come out in order when playback_time is >= timestamp
    assert_eq!(buffer.pop(now + 100), Some(1));
    assert_eq!(buffer.pop(now + 200), Some(2));
    assert_eq!(buffer.pop(now + 300), Some(3));
    assert_eq!(buffer.pop(now + 400), None);
}

#[test]
fn test_jitter_buffer_reorder() {
    let mut buffer: JitterBuffer<i32> = JitterBuffer::new(100, 5000); // 5 second window
    let now = clasp_core::time::now();

    // Add items out of order
    buffer.push(now + 300, 3);
    buffer.push(now + 100, 1);
    buffer.push(now + 200, 2);

    // Should come out in timestamp order
    assert_eq!(buffer.pop(now + 100), Some(1));
    assert_eq!(buffer.pop(now + 200), Some(2));
    assert_eq!(buffer.pop(now + 300), Some(3));
}

#[test]
fn test_jitter_buffer_held_until_ready() {
    let mut buffer: JitterBuffer<i32> = JitterBuffer::new(100, 5000); // 5 second window
    let now = clasp_core::time::now();

    buffer.push(now + 100, 1);

    // Should not be available when playback_time < timestamp
    assert_eq!(buffer.pop(now + 50), None);

    // Available when playback_time >= timestamp
    assert_eq!(buffer.pop(now + 100), Some(1));
}
