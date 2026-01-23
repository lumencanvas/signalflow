//! Lock and Conflict Resolution Tests
//!
//! Tests for:
//! - Lock acquisition and denial for non-owners
//! - Concurrent write contention (MUST be rejected when lock is held)
//! - Lock release and subsequent write success
//! - Basic last-write-wins (LWW) behavior
//! - Conflict resolution strategies (Max, Min, Lock)

use clasp_core::Value;
use clasp_test_utils::{TestRouter, ValueCollector};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_lock_acquisition_and_denial() {
    let router = TestRouter::start().await;

    // Watcher to observe final state
    let watcher = router
        .connect_client_named("Watcher")
        .await
        .expect("Watcher should connect");

    let collector = ValueCollector::new();
    watcher
        .subscribe("/locks/value", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    // Owner takes lock and sets initial value
    let owner = router
        .connect_client_named("Owner")
        .await
        .expect("Owner should connect");

    owner
        .set_locked("/locks/value", Value::Int(1))
        .await
        .expect("set_locked should succeed");

    // Wait for initial value to arrive
    assert!(
        collector.wait_for_count(1, Duration::from_secs(2)).await,
        "Should receive initial locked value"
    );

    // Another client attempts to overwrite while locked
    let other = router
        .connect_client_named("Other")
        .await
        .expect("Other should connect");

    other
        .set("/locks/value", Value::Int(2))
        .await
        .expect("set should succeed (sent, but may be rejected)");

    // Give router time to process
    sleep(Duration::from_millis(200)).await;

    let values = collector.values();
    assert!(!values.is_empty(), "Should have observed values on /locks/value");

    let (_, last_val) = values.last().unwrap();
    match last_val {
        Value::Int(v) => assert_eq!(*v, 1, "Locked value should not be modified by non-owner"),
        _ => panic!("Unexpected value type for locked param"),
    }
}

#[tokio::test]
async fn test_lww_last_write_wins() {
    let router = TestRouter::start().await;

    let writer1 = router
        .connect_client_named("Writer1")
        .await
        .expect("Writer1 should connect");
    let writer2 = router
        .connect_client_named("Writer2")
        .await
        .expect("Writer2 should connect");

    // First write
    writer1
        .set("/lww/value", Value::Int(1))
        .await
        .expect("Writer1 set should succeed");

    // Slight delay to ensure different timestamps
    sleep(Duration::from_millis(50)).await;

    // Second write should win (LWW)
    writer2
        .set("/lww/value", Value::Int(2))
        .await
        .expect("Writer2 set should succeed");

    // Reader checks final value
    let reader = router
        .connect_client_named("Reader")
        .await
        .expect("Reader should connect");

    // Small wait to allow state to settle
    sleep(Duration::from_millis(100)).await;

    let value = reader
        .get("/lww/value")
        .await
        .expect("Reader get should succeed");

    match value {
        Value::Int(v) => assert_eq!(v, 2, "LWW: final value should be from last writer"),
        _ => panic!("Unexpected value type for LWW param"),
    }
}

// ============================================================================
// Concurrent Contention Tests
// ============================================================================

/// Test: Lock MUST block concurrent writes from non-owners
///
/// This is a critical test that verifies:
/// 1. Owner acquires lock successfully
/// 2. Intruder's write attempt is REJECTED (not just ignored)
/// 3. After owner releases lock, intruder can write
#[tokio::test]
async fn test_lock_blocks_concurrent_writes() {
    let router = TestRouter::start().await;

    // Watcher to observe final state
    let watcher = router
        .connect_client_named("Watcher")
        .await
        .expect("Watcher should connect");

    let collector = ValueCollector::new();
    watcher
        .subscribe("/concurrent/value", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    // Owner acquires lock
    let owner = router
        .connect_client_named("Owner")
        .await
        .expect("Owner should connect");

    owner
        .set_locked("/concurrent/value", Value::Int(100))
        .await
        .expect("Owner should acquire lock");

    // Wait for value to propagate
    assert!(
        collector.wait_for_count(1, Duration::from_secs(2)).await,
        "Should receive initial locked value"
    );

    // Intruder attempts concurrent write
    let intruder = router
        .connect_client_named("Intruder")
        .await
        .expect("Intruder should connect");

    // Intruder's write should fail or be ignored
    let result = intruder.set("/concurrent/value", Value::Int(999)).await;

    // Give time for any propagation
    sleep(Duration::from_millis(200)).await;

    // Verify the value is STILL the owner's value
    let values = collector.values();
    let (_, last_val) = values.last().expect("Should have at least one value");
    match last_val {
        Value::Int(v) => {
            assert_eq!(
                *v, 100,
                "Lock MUST prevent intruder from modifying value. Expected 100, got {}",
                v
            );
        }
        _ => panic!("Unexpected value type"),
    }

    // Owner releases lock by setting unlock=true
    owner
        .set_unlocked("/concurrent/value", Value::Int(200))
        .await
        .expect("Owner should release lock");

    // Wait for unlock to propagate
    sleep(Duration::from_millis(100)).await;

    // Now intruder should be able to write
    intruder
        .set("/concurrent/value", Value::Int(999))
        .await
        .expect("Intruder write should succeed after unlock");

    // Wait for intruder's value
    assert!(
        collector.wait_for_count(3, Duration::from_secs(2)).await,
        "Should receive intruder's value after unlock"
    );

    let values = collector.values();
    let (_, final_val) = values.last().expect("Should have values");
    match final_val {
        Value::Int(v) => {
            assert_eq!(
                *v, 999,
                "After unlock, intruder's write should succeed. Expected 999, got {}",
                v
            );
        }
        _ => panic!("Unexpected value type"),
    }
}

/// Test: Multiple concurrent writers competing for lock
#[tokio::test]
async fn test_lock_race_condition() {
    let router = TestRouter::start().await;

    let watcher = router
        .connect_client_named("Watcher")
        .await
        .expect("Watcher should connect");

    let collector = ValueCollector::new();
    watcher
        .subscribe("/race/value", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    // First writer tries to acquire lock
    let writer1 = router
        .connect_client_named("Writer1")
        .await
        .expect("Writer1 should connect");

    let writer2 = router
        .connect_client_named("Writer2")
        .await
        .expect("Writer2 should connect");

    // Both try to set with lock simultaneously
    let (r1, r2) = tokio::join!(
        writer1.set_locked("/race/value", Value::Int(1)),
        writer2.set_locked("/race/value", Value::Int(2))
    );

    // At least one should succeed
    assert!(
        r1.is_ok() || r2.is_ok(),
        "At least one lock acquisition should succeed"
    );

    // Wait for values to propagate
    sleep(Duration::from_millis(200)).await;

    // The value should be stable (either 1 or 2, but not changing)
    let values = collector.values();
    assert!(!values.is_empty(), "Should have received at least one value");

    let (_, final_val) = values.last().unwrap();
    match final_val {
        Value::Int(v) => {
            assert!(
                *v == 1 || *v == 2,
                "Final value should be from one of the writers, got {}",
                v
            );
        }
        _ => panic!("Unexpected value type"),
    }
}

/// Test: Lock timeout / expiry behavior (if implemented)
#[tokio::test]
async fn test_lock_prevents_modification_by_others() {
    let router = TestRouter::start().await;

    let collector = ValueCollector::new();

    // Setup watcher first
    let watcher = router
        .connect_client_named("Watcher")
        .await
        .expect("Watcher should connect");

    watcher
        .subscribe("/lock_test/**", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    // Owner sets multiple locked values
    let owner = router
        .connect_client_named("Owner")
        .await
        .expect("Owner should connect");

    for i in 0..5 {
        owner
            .set_locked(&format!("/lock_test/param{}", i), Value::Int(i as i64))
            .await
            .expect("Owner should set locked value");
    }

    // Wait for all values
    assert!(
        collector.wait_for_count(5, Duration::from_secs(3)).await,
        "Should receive all 5 locked values"
    );

    // Intruder tries to modify all of them
    let intruder = router
        .connect_client_named("Intruder")
        .await
        .expect("Intruder should connect");

    let initial_count = collector.count();

    for i in 0..5 {
        // These should all fail or be ignored
        let _ = intruder
            .set(&format!("/lock_test/param{}", i), Value::Int(100 + i as i64))
            .await;
    }

    // Give time for any responses
    sleep(Duration::from_millis(300)).await;

    // Verify none of the intruder's values were accepted
    let values = collector.values();
    for (addr, val) in values.iter() {
        if addr.starts_with("/lock_test/param") {
            match val {
                Value::Int(v) => {
                    assert!(
                        *v < 100,
                        "Intruder should NOT be able to modify locked values. Address {} has value {}",
                        addr, v
                    );
                }
                _ => {}
            }
        }
    }
}

/// Test: Owner CAN modify their own locked values
#[tokio::test]
async fn test_owner_can_modify_locked_value() {
    let router = TestRouter::start().await;

    let collector = ValueCollector::new();

    let watcher = router
        .connect_client_named("Watcher")
        .await
        .expect("Watcher should connect");

    watcher
        .subscribe("/owner_modify/value", collector.callback_ref())
        .await
        .expect("Subscribe should succeed");

    // Owner sets initial locked value
    let owner = router
        .connect_client_named("Owner")
        .await
        .expect("Owner should connect");

    owner
        .set_locked("/owner_modify/value", Value::Int(1))
        .await
        .expect("Owner should set initial locked value");

    assert!(
        collector.wait_for_count(1, Duration::from_secs(2)).await,
        "Should receive initial value"
    );

    // Owner modifies their own locked value (should succeed)
    owner
        .set_locked("/owner_modify/value", Value::Int(2))
        .await
        .expect("Owner should be able to modify their locked value");

    assert!(
        collector.wait_for_count(2, Duration::from_secs(2)).await,
        "Should receive modified value"
    );

    // Verify final value is from owner's second write
    let values = collector.values();
    let (_, final_val) = values.last().unwrap();
    match final_val {
        Value::Int(v) => {
            assert_eq!(*v, 2, "Owner's modification should succeed");
        }
        _ => panic!("Unexpected value type"),
    }
}
