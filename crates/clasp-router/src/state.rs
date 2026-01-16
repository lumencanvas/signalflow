//! Router state management

use clasp_core::state::{ParamState, StateStore, UpdateError};
use clasp_core::{Message, ParamValue, SetMessage, SnapshotMessage, Value};
use dashmap::DashMap;
use parking_lot::RwLock;

use crate::SessionId;

/// Global router state
pub struct RouterState {
    /// Parameter state store
    params: RwLock<StateStore>,
    /// Change listeners (for reactive updates)
    listeners: DashMap<String, Vec<Box<dyn Fn(&str, &Value) + Send + Sync>>>,
}

impl RouterState {
    pub fn new() -> Self {
        Self {
            params: RwLock::new(StateStore::new()),
            listeners: DashMap::new(),
        }
    }

    /// Get a parameter value
    pub fn get(&self, address: &str) -> Option<Value> {
        self.params.read().get_value(address).cloned()
    }

    /// Get full parameter state
    pub fn get_state(&self, address: &str) -> Option<ParamState> {
        self.params.read().get(address).cloned()
    }

    /// Set a parameter value
    pub fn set(
        &self,
        address: &str,
        value: Value,
        writer: &SessionId,
        revision: Option<u64>,
        lock: bool,
        unlock: bool,
    ) -> Result<u64, UpdateError> {
        let result =
            self.params
                .write()
                .set(address, value.clone(), writer, revision, lock, unlock)?;

        // Notify listeners
        if let Some(listeners) = self.listeners.get(address) {
            for listener in listeners.iter() {
                listener(address, &value);
            }
        }

        Ok(result)
    }

    /// Apply a SET message
    pub fn apply_set(&self, msg: &SetMessage, writer: &SessionId) -> Result<u64, UpdateError> {
        self.set(
            &msg.address,
            msg.value.clone(),
            writer,
            msg.revision,
            msg.lock,
            msg.unlock,
        )
    }

    /// Get all parameters matching a pattern
    pub fn get_matching(&self, pattern: &str) -> Vec<(String, ParamState)> {
        self.params
            .read()
            .get_matching(pattern)
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    /// Create a snapshot of all params matching a pattern
    pub fn snapshot(&self, pattern: &str) -> SnapshotMessage {
        let params: Vec<ParamValue> = self
            .get_matching(pattern)
            .into_iter()
            .map(|(address, state)| ParamValue {
                address,
                value: state.value,
                revision: state.revision,
                writer: Some(state.writer),
                timestamp: Some(state.timestamp),
            })
            .collect();

        SnapshotMessage { params }
    }

    /// Create a full snapshot
    pub fn full_snapshot(&self) -> SnapshotMessage {
        self.snapshot("**")
    }

    /// Number of parameters
    pub fn len(&self) -> usize {
        self.params.read().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.params.read().is_empty()
    }

    /// Clear all state
    pub fn clear(&self) {
        self.params.write().clear();
    }
}

impl Default for RouterState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_state() {
        let state = RouterState::new();

        state
            .set(
                "/test/value",
                Value::Float(0.5),
                &"session1".to_string(),
                None,
                false,
                false,
            )
            .unwrap();

        let value = state.get("/test/value").unwrap();
        assert_eq!(value, Value::Float(0.5));
    }

    #[test]
    fn test_snapshot() {
        let state = RouterState::new();

        state
            .set(
                "/test/a",
                Value::Float(1.0),
                &"s1".to_string(),
                None,
                false,
                false,
            )
            .unwrap();
        state
            .set(
                "/test/b",
                Value::Float(2.0),
                &"s1".to_string(),
                None,
                false,
                false,
            )
            .unwrap();
        state
            .set(
                "/other/c",
                Value::Float(3.0),
                &"s1".to_string(),
                None,
                false,
                false,
            )
            .unwrap();

        let snapshot = state.snapshot("/test/**");
        assert_eq!(snapshot.params.len(), 2);
    }
}
