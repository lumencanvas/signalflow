//! State management tests

use clasp_core::state::{ConflictStrategy, ParamState, StateStore};
use clasp_core::Value;

#[test]
fn test_param_state_new() {
    let state = ParamState::new("/test/param".to_string(), Value::Int(42));

    assert_eq!(state.address(), "/test/param");
    assert_eq!(state.value(), &Value::Int(42));
    assert_eq!(state.revision(), 1);
}

#[test]
fn test_param_state_update() {
    let mut state = ParamState::new("/test/param".to_string(), Value::Int(42));

    state.update(Value::Int(100), "writer-1".to_string());

    assert_eq!(state.value(), &Value::Int(100));
    assert_eq!(state.revision(), 2);
    assert_eq!(state.writer(), Some(&"writer-1".to_string()));
}

#[test]
fn test_param_state_lww_strategy() {
    let mut state = ParamState::new("/test/param".to_string(), Value::Int(42));
    state.set_strategy(ConflictStrategy::LastWriterWins);

    // First update
    state.update_with_timestamp(Value::Int(100), "writer-1".to_string(), 1000);

    // Later timestamp wins
    state.update_with_timestamp(Value::Int(200), "writer-2".to_string(), 2000);
    assert_eq!(state.value(), &Value::Int(200));

    // Earlier timestamp loses
    state.update_with_timestamp(Value::Int(50), "writer-3".to_string(), 500);
    assert_eq!(state.value(), &Value::Int(200)); // Still 200
}

#[test]
fn test_param_state_max_strategy() {
    let mut state = ParamState::new("/test/param".to_string(), Value::Int(50));
    state.set_strategy(ConflictStrategy::Max);

    state.update(Value::Int(100), "writer-1".to_string());
    assert_eq!(state.value(), &Value::Int(100));

    state.update(Value::Int(30), "writer-2".to_string());
    assert_eq!(state.value(), &Value::Int(100)); // Max wins

    state.update(Value::Int(150), "writer-3".to_string());
    assert_eq!(state.value(), &Value::Int(150));
}

#[test]
fn test_param_state_min_strategy() {
    let mut state = ParamState::new("/test/param".to_string(), Value::Int(50));
    state.set_strategy(ConflictStrategy::Min);

    state.update(Value::Int(100), "writer-1".to_string());
    assert_eq!(state.value(), &Value::Int(50)); // Min wins

    state.update(Value::Int(30), "writer-2".to_string());
    assert_eq!(state.value(), &Value::Int(30));

    state.update(Value::Int(150), "writer-3".to_string());
    assert_eq!(state.value(), &Value::Int(30)); // Still min
}

#[test]
fn test_param_state_lock() {
    let mut state = ParamState::new("/test/param".to_string(), Value::Int(42));

    // Lock the state
    assert!(state.try_lock("owner-1".to_string()));
    assert!(state.is_locked());
    assert_eq!(state.lock_owner(), Some(&"owner-1".to_string()));

    // Another writer can't lock
    assert!(!state.try_lock("owner-2".to_string()));

    // Owner can update
    assert!(state.try_update(Value::Int(100), "owner-1".to_string()));
    assert_eq!(state.value(), &Value::Int(100));

    // Non-owner can't update
    assert!(!state.try_update(Value::Int(200), "owner-2".to_string()));
    assert_eq!(state.value(), &Value::Int(100));

    // Unlock
    assert!(state.unlock("owner-1".to_string()));
    assert!(!state.is_locked());

    // Now anyone can update
    assert!(state.try_update(Value::Int(200), "owner-2".to_string()));
    assert_eq!(state.value(), &Value::Int(200));
}

#[test]
fn test_state_store() {
    let store = StateStore::new();

    // Set values
    store.set("/test/a", Value::Int(1));
    store.set("/test/b", Value::Int(2));
    store.set("/test/c", Value::Int(3));

    // Get values
    assert_eq!(store.get("/test/a"), Some(Value::Int(1)));
    assert_eq!(store.get("/test/b"), Some(Value::Int(2)));
    assert_eq!(store.get("/test/c"), Some(Value::Int(3)));
    assert_eq!(store.get("/test/d"), None);
}

#[test]
fn test_state_store_pattern_match() {
    let store = StateStore::new();

    store.set("/lumen/layer/0/opacity", Value::Float(0.5));
    store.set("/lumen/layer/0/enabled", Value::Bool(true));
    store.set("/lumen/layer/1/opacity", Value::Float(0.8));
    store.set("/lumen/layer/1/enabled", Value::Bool(false));
    store.set("/other/value", Value::Int(42));

    // Get all layer opacities
    let matches = store.get_matching("/lumen/layer/*/opacity");
    assert_eq!(matches.len(), 2);

    // Get all lumen values
    let matches = store.get_matching("/lumen/**");
    assert_eq!(matches.len(), 4);
}

#[test]
fn test_state_store_remove() {
    let store = StateStore::new();

    store.set("/test/value", Value::Int(42));
    assert!(store.get("/test/value").is_some());

    store.remove("/test/value");
    assert!(store.get("/test/value").is_none());
}
