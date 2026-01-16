//! State management tests

use clasp_core::state::{ParamState, StateStore};
use clasp_core::{ConflictStrategy, Value};

#[test]
fn test_param_state_new() {
    let state = ParamState::new(Value::Int(42), "writer".to_string());

    assert_eq!(state.value, Value::Int(42));
    assert_eq!(state.revision, 1);
    assert_eq!(state.writer, "writer");
}

#[test]
fn test_param_state_update() {
    let mut state = ParamState::new(Value::Int(42), "writer-0".to_string());

    let result = state.try_update(
        Value::Int(100),
        "writer-1",
        None,  // no expected revision
        false, // don't request lock
        false, // don't release lock
    );

    assert!(result.is_ok());
    assert_eq!(state.value, Value::Int(100));
    assert_eq!(state.revision, 2);
    assert_eq!(state.writer, "writer-1");
}

#[test]
fn test_param_state_lww_strategy() {
    let state = ParamState::new(Value::Int(42), "initial".to_string())
        .with_strategy(ConflictStrategy::Lww);

    assert_eq!(state.strategy, ConflictStrategy::Lww);
    assert_eq!(state.value, Value::Int(42));
}

#[test]
fn test_param_state_max_strategy() {
    let mut state = ParamState::new(Value::Int(50), "writer".to_string())
        .with_strategy(ConflictStrategy::Max);

    // Higher value should be accepted
    let _ = state.try_update(Value::Int(100), "writer-1", None, false, false);
    assert_eq!(state.value, Value::Int(100));

    // Lower value should be rejected
    let result = state.try_update(Value::Int(30), "writer-2", None, false, false);
    assert!(result.is_err());
    assert_eq!(state.value, Value::Int(100)); // Still 100

    // Even higher value should be accepted
    let _ = state.try_update(Value::Int(150), "writer-3", None, false, false);
    assert_eq!(state.value, Value::Int(150));
}

#[test]
fn test_param_state_min_strategy() {
    let mut state = ParamState::new(Value::Int(50), "writer".to_string())
        .with_strategy(ConflictStrategy::Min);

    // Higher value should be rejected
    let result = state.try_update(Value::Int(100), "writer-1", None, false, false);
    assert!(result.is_err());
    assert_eq!(state.value, Value::Int(50)); // Still 50

    // Lower value should be accepted
    let _ = state.try_update(Value::Int(30), "writer-2", None, false, false);
    assert_eq!(state.value, Value::Int(30));
}

#[test]
fn test_param_state_lock() {
    let mut state = ParamState::new(Value::Int(42), "owner".to_string())
        .with_strategy(ConflictStrategy::Lock);

    // Request a lock
    let result = state.try_update(Value::Int(100), "owner-1", None, true, false);
    assert!(result.is_ok());
    assert!(state.lock_holder.is_some());
    assert_eq!(state.lock_holder, Some("owner-1".to_string()));

    // Non-owner can't update when locked
    let result = state.try_update(Value::Int(200), "owner-2", None, false, false);
    assert!(result.is_err());
    assert_eq!(state.value, Value::Int(100)); // Still 100

    // Owner can update
    let result = state.try_update(Value::Int(150), "owner-1", None, false, false);
    assert!(result.is_ok());
    assert_eq!(state.value, Value::Int(150));

    // Owner can release lock
    let result = state.try_update(Value::Int(200), "owner-1", None, false, true);
    assert!(result.is_ok());
    assert!(state.lock_holder.is_none());
}

#[test]
fn test_param_state_revision_conflict() {
    let mut state = ParamState::new(Value::Int(42), "writer".to_string());

    // Update expecting revision 1 (should succeed)
    let result = state.try_update(Value::Int(100), "writer-1", Some(1), false, false);
    assert!(result.is_ok());
    assert_eq!(state.revision, 2);

    // Update expecting wrong revision (should fail)
    let result = state.try_update(Value::Int(200), "writer-2", Some(1), false, false);
    assert!(result.is_err());
    assert_eq!(state.value, Value::Int(100)); // Unchanged
}

#[test]
fn test_state_store() {
    let mut store = StateStore::new();

    // Set values
    store.set("/test/a", Value::Int(1), "writer", None, false, false).unwrap();
    store.set("/test/b", Value::Int(2), "writer", None, false, false).unwrap();
    store.set("/test/c", Value::Int(3), "writer", None, false, false).unwrap();

    // Get values using get_value method
    assert_eq!(store.get_value("/test/a"), Some(&Value::Int(1)));
    assert_eq!(store.get_value("/test/b"), Some(&Value::Int(2)));
    assert_eq!(store.get_value("/test/c"), Some(&Value::Int(3)));
    assert_eq!(store.get_value("/test/d"), None);
}

#[test]
fn test_state_store_pattern_match() {
    let mut store = StateStore::new();

    store.set("/lumen/layer/0/opacity", Value::Float(0.5), "w", None, false, false).unwrap();
    store.set("/lumen/layer/0/enabled", Value::Bool(true), "w", None, false, false).unwrap();
    store.set("/lumen/layer/1/opacity", Value::Float(0.8), "w", None, false, false).unwrap();
    store.set("/lumen/layer/1/enabled", Value::Bool(false), "w", None, false, false).unwrap();
    store.set("/other/value", Value::Int(42), "w", None, false, false).unwrap();

    // Get all layer opacities
    let matches = store.get_matching("/lumen/layer/*/opacity");
    assert_eq!(matches.len(), 2);

    // Get all lumen values
    let matches = store.get_matching("/lumen/**");
    assert_eq!(matches.len(), 4);
}

#[test]
fn test_state_store_remove() {
    let mut store = StateStore::new();

    store.set("/test/value", Value::Int(42), "writer", None, false, false).unwrap();
    assert!(store.get_value("/test/value").is_some());

    store.remove("/test/value");
    assert!(store.get_value("/test/value").is_none());
}
