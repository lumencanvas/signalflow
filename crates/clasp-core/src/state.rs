//! State management for SignalFlow params
//!
//! Provides conflict resolution and revision tracking for stateful parameters.

use crate::{ConflictStrategy, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// State of a single parameter
#[derive(Debug, Clone)]
pub struct ParamState {
    /// Current value
    pub value: Value,
    /// Monotonic revision number
    pub revision: u64,
    /// Session ID of last writer
    pub writer: String,
    /// Timestamp of last write (microseconds)
    pub timestamp: u64,
    /// Conflict resolution strategy
    pub strategy: ConflictStrategy,
    /// Lock holder (if locked)
    pub lock_holder: Option<String>,
    /// Metadata
    pub meta: Option<ParamMeta>,
}

/// Parameter metadata
#[derive(Debug, Clone)]
pub struct ParamMeta {
    pub unit: Option<String>,
    pub range: Option<(f64, f64)>,
    pub default: Option<Value>,
}

impl ParamState {
    /// Create a new param state
    pub fn new(value: Value, writer: String) -> Self {
        Self {
            value,
            revision: 1,
            writer,
            timestamp: current_timestamp(),
            strategy: ConflictStrategy::Lww,
            lock_holder: None,
            meta: None,
        }
    }

    /// Create with specific strategy
    pub fn with_strategy(mut self, strategy: ConflictStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Create with metadata
    pub fn with_meta(mut self, meta: ParamMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Attempt to update the value
    ///
    /// Returns Ok(new_revision) if update was accepted,
    /// Err with reason if rejected.
    pub fn try_update(
        &mut self,
        new_value: Value,
        writer: &str,
        expected_revision: Option<u64>,
        request_lock: bool,
        release_lock: bool,
    ) -> Result<u64, UpdateError> {
        let timestamp = current_timestamp();

        // Check optimistic lock (if revision specified)
        if let Some(expected) = expected_revision {
            if expected != self.revision {
                return Err(UpdateError::RevisionConflict {
                    expected,
                    actual: self.revision,
                });
            }
        }

        // Check lock
        if let Some(ref holder) = self.lock_holder {
            if holder != writer && !release_lock {
                return Err(UpdateError::LockHeld {
                    holder: holder.clone(),
                });
            }
        }

        // Handle lock release
        if release_lock {
            if self.lock_holder.as_deref() == Some(writer) {
                self.lock_holder = None;
            }
        }

        // Apply conflict resolution
        let should_update = match self.strategy {
            ConflictStrategy::Lww => timestamp >= self.timestamp,
            ConflictStrategy::Max => {
                match (&new_value, &self.value) {
                    (Value::Float(new), Value::Float(old)) => new > old,
                    (Value::Int(new), Value::Int(old)) => new > old,
                    _ => true, // Fall back to LWW for non-numeric
                }
            }
            ConflictStrategy::Min => {
                match (&new_value, &self.value) {
                    (Value::Float(new), Value::Float(old)) => new < old,
                    (Value::Int(new), Value::Int(old)) => new < old,
                    _ => true,
                }
            }
            ConflictStrategy::Lock => {
                self.lock_holder.is_none() || self.lock_holder.as_deref() == Some(writer)
            }
            ConflictStrategy::Merge => true, // App handles merge
        };

        if !should_update {
            return Err(UpdateError::ConflictRejected);
        }

        // Handle lock request
        if request_lock {
            if self.lock_holder.is_some() && self.lock_holder.as_deref() != Some(writer) {
                return Err(UpdateError::LockHeld {
                    holder: self.lock_holder.clone().unwrap(),
                });
            }
            self.lock_holder = Some(writer.to_string());
        }

        // Apply update
        self.value = new_value;
        self.revision += 1;
        self.writer = writer.to_string();
        self.timestamp = timestamp;

        Ok(self.revision)
    }

    /// Check if value is within range (if specified)
    pub fn validate_range(&self, value: &Value) -> bool {
        if let Some(meta) = &self.meta {
            if let Some((min, max)) = meta.range {
                if let Some(v) = value.as_f64() {
                    return v >= min && v <= max;
                }
            }
        }
        true
    }
}

/// Errors that can occur during state updates
#[derive(Debug, Clone)]
pub enum UpdateError {
    RevisionConflict { expected: u64, actual: u64 },
    LockHeld { holder: String },
    ConflictRejected,
    OutOfRange,
}

/// State store for multiple params
#[derive(Debug, Default)]
pub struct StateStore {
    params: HashMap<String, ParamState>,
}

impl StateStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a param's current state
    pub fn get(&self, address: &str) -> Option<&ParamState> {
        self.params.get(address)
    }

    /// Get a param's current value
    pub fn get_value(&self, address: &str) -> Option<&Value> {
        self.params.get(address).map(|p| &p.value)
    }

    /// Set a param value, creating if necessary
    pub fn set(
        &mut self,
        address: &str,
        value: Value,
        writer: &str,
        revision: Option<u64>,
        lock: bool,
        unlock: bool,
    ) -> Result<u64, UpdateError> {
        if let Some(param) = self.params.get_mut(address) {
            param.try_update(value, writer, revision, lock, unlock)
        } else {
            // Create new param
            let mut param = ParamState::new(value, writer.to_string());
            if lock {
                param.lock_holder = Some(writer.to_string());
            }
            let rev = param.revision;
            self.params.insert(address.to_string(), param);
            Ok(rev)
        }
    }

    /// Get all params matching a pattern
    pub fn get_matching(&self, pattern: &str) -> Vec<(&str, &ParamState)> {
        use crate::address::glob_match;

        self.params
            .iter()
            .filter(|(addr, _)| glob_match(pattern, addr))
            .map(|(addr, state)| (addr.as_str(), state))
            .collect()
    }

    /// Get all params as a snapshot
    pub fn snapshot(&self) -> Vec<(&str, &ParamState)> {
        self.params.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }

    /// Number of params
    pub fn len(&self) -> usize {
        self.params.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Remove a param
    pub fn remove(&mut self, address: &str) -> Option<ParamState> {
        self.params.remove(address)
    }

    /// Clear all params
    pub fn clear(&mut self) {
        self.params.clear();
    }
}

/// Get current timestamp in microseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_update() {
        let mut state = ParamState::new(Value::Float(0.5), "session1".to_string());

        let result = state.try_update(
            Value::Float(0.75),
            "session2",
            None,
            false,
            false,
        );

        assert!(result.is_ok());
        assert_eq!(state.revision, 2);
        assert_eq!(state.value, Value::Float(0.75));
        assert_eq!(state.writer, "session2");
    }

    #[test]
    fn test_revision_conflict() {
        let mut state = ParamState::new(Value::Float(0.5), "session1".to_string());

        let result = state.try_update(
            Value::Float(0.75),
            "session2",
            Some(999), // Wrong revision
            false,
            false,
        );

        assert!(matches!(result, Err(UpdateError::RevisionConflict { .. })));
    }

    #[test]
    fn test_locking() {
        let mut state = ParamState::new(Value::Float(0.5), "session1".to_string());

        // Session 1 takes lock
        let result = state.try_update(
            Value::Float(0.6),
            "session1",
            None,
            true, // Request lock
            false,
        );
        assert!(result.is_ok());
        assert_eq!(state.lock_holder, Some("session1".to_string()));

        // Session 2 tries to update - should fail
        let result = state.try_update(
            Value::Float(0.7),
            "session2",
            None,
            false,
            false,
        );
        assert!(matches!(result, Err(UpdateError::LockHeld { .. })));

        // Session 1 can still update
        let result = state.try_update(
            Value::Float(0.8),
            "session1",
            None,
            false,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_strategy() {
        let mut state = ParamState::new(Value::Float(0.5), "session1".to_string())
            .with_strategy(ConflictStrategy::Max);

        // Higher value wins
        let result = state.try_update(Value::Float(0.8), "session2", None, false, false);
        assert!(result.is_ok());
        assert_eq!(state.value, Value::Float(0.8));

        // Lower value rejected
        let result = state.try_update(Value::Float(0.3), "session3", None, false, false);
        assert!(matches!(result, Err(UpdateError::ConflictRejected)));
        assert_eq!(state.value, Value::Float(0.8)); // Unchanged
    }

    #[test]
    fn test_state_store() {
        let mut store = StateStore::new();

        store.set("/test/a", Value::Float(1.0), "s1", None, false, false).unwrap();
        store.set("/test/b", Value::Float(2.0), "s1", None, false, false).unwrap();
        store.set("/other/c", Value::Float(3.0), "s1", None, false, false).unwrap();

        assert_eq!(store.len(), 3);

        let matching = store.get_matching("/test/*");
        assert_eq!(matching.len(), 2);
    }
}
