//! Time and synchronization tests

use clasp_core::time::{Timestamp, ClockSync, SessionTime, JitterBuffer};
use std::time::Duration;

#[test]
fn test_timestamp_now() {
    let ts = Timestamp::now();
    assert!(ts.as_micros() > 0);
}

#[test]
fn test_timestamp_from_micros() {
    let ts = Timestamp::from_micros(1234567890);
    assert_eq!(ts.as_micros(), 1234567890);
}

#[test]
fn test_timestamp_arithmetic() {
    let ts1 = Timestamp::from_micros(1000);
    let ts2 = Timestamp::from_micros(500);

    assert_eq!(ts1.duration_since(&ts2), Duration::from_micros(500));
}

#[test]
fn test_clock_sync_initial() {
    let sync = ClockSync::new();
    assert_eq!(sync.offset(), 0);
    assert!(sync.rtt().is_none());
}

#[test]
fn test_clock_sync_sample() {
    let mut sync = ClockSync::new();

    // Simulate a sync exchange
    let t1 = 1000u64; // Client send time
    let t2 = 1100u64; // Server receive time
    let t3 = 1150u64; // Server send time
    let t4 = 1250u64; // Client receive time

    sync.add_sample(t1, t2, t3, t4);

    // RTT should be (t4 - t1) - (t3 - t2) = 250 - 50 = 200
    assert!(sync.rtt().is_some());

    // Offset should be calculated
    // offset = ((t2 - t1) + (t3 - t4)) / 2 = (100 + (-100)) / 2 = 0
    // In this case, clocks are synchronized
}

#[test]
fn test_clock_sync_multiple_samples() {
    let mut sync = ClockSync::new();

    // Add multiple samples
    for i in 0..10 {
        let base = i * 1000;
        sync.add_sample(
            base,
            base + 100,
            base + 150,
            base + 250,
        );
    }

    // Should have averaged offset
    assert!(sync.rtt().is_some());
}

#[test]
fn test_session_time() {
    let session = SessionTime::new();
    let t1 = session.elapsed();

    std::thread::sleep(Duration::from_millis(10));

    let t2 = session.elapsed();
    assert!(t2 > t1);
}

#[test]
fn test_jitter_buffer_empty() {
    let mut buffer: JitterBuffer<i32> = JitterBuffer::new(Duration::from_millis(50));
    assert!(buffer.pop().is_none());
}

#[test]
fn test_jitter_buffer_ordered() {
    let mut buffer: JitterBuffer<i32> = JitterBuffer::new(Duration::from_millis(50));

    // Add items in order
    buffer.push(100, 1);
    buffer.push(200, 2);
    buffer.push(300, 3);

    // Should come out in order after buffer time
    std::thread::sleep(Duration::from_millis(60));

    assert_eq!(buffer.pop(), Some(1));
    assert_eq!(buffer.pop(), Some(2));
    assert_eq!(buffer.pop(), Some(3));
    assert_eq!(buffer.pop(), None);
}

#[test]
fn test_jitter_buffer_reorder() {
    let mut buffer: JitterBuffer<i32> = JitterBuffer::new(Duration::from_millis(50));

    // Add items out of order
    buffer.push(300, 3);
    buffer.push(100, 1);
    buffer.push(200, 2);

    // Should come out in timestamp order after buffer time
    std::thread::sleep(Duration::from_millis(60));

    assert_eq!(buffer.pop(), Some(1));
    assert_eq!(buffer.pop(), Some(2));
    assert_eq!(buffer.pop(), Some(3));
}

#[test]
fn test_jitter_buffer_held_until_ready() {
    let mut buffer: JitterBuffer<i32> = JitterBuffer::new(Duration::from_millis(100));

    buffer.push(100, 1);

    // Should not be available immediately
    assert_eq!(buffer.pop(), None);

    // Wait for buffer time
    std::thread::sleep(Duration::from_millis(110));

    assert_eq!(buffer.pop(), Some(1));
}
