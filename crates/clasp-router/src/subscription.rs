//! Subscription management

use dashmap::DashMap;
use clasp_core::{address::Pattern, SignalType, SubscribeOptions};
use std::collections::HashSet;

use crate::SessionId;

/// A subscription entry
#[derive(Debug, Clone)]
pub struct Subscription {
    /// Subscription ID (unique per session)
    pub id: u32,
    /// Session that owns this subscription
    pub session_id: SessionId,
    /// Pattern to match
    pub pattern: Pattern,
    /// Signal types to filter (empty = all)
    pub types: HashSet<SignalType>,
    /// Subscription options
    pub options: SubscribeOptions,
}

impl Subscription {
    pub fn new(
        id: u32,
        session_id: SessionId,
        pattern: &str,
        types: Vec<SignalType>,
        options: SubscribeOptions,
    ) -> Result<Self, clasp_core::Error> {
        let pattern = Pattern::compile(pattern)?;

        Ok(Self {
            id,
            session_id,
            pattern,
            types: types.into_iter().collect(),
            options,
        })
    }

    /// Check if this subscription matches an address
    pub fn matches(&self, address: &str, signal_type: Option<SignalType>) -> bool {
        // Check address pattern
        if !self.pattern.matches(address) {
            return false;
        }

        // Check signal type filter
        if !self.types.is_empty() {
            if let Some(st) = signal_type {
                if !self.types.contains(&st) {
                    return false;
                }
            }
        }

        true
    }
}

/// Manages all subscriptions
pub struct SubscriptionManager {
    /// All subscriptions by (session_id, subscription_id)
    subscriptions: DashMap<(SessionId, u32), Subscription>,
    /// Index by address prefix for faster lookup
    by_prefix: DashMap<String, Vec<(SessionId, u32)>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: DashMap::new(),
            by_prefix: DashMap::new(),
        }
    }

    /// Add a subscription
    pub fn add(&self, sub: Subscription) {
        let key = (sub.session_id.clone(), sub.id);

        // Add to prefix index (use first segment as prefix)
        let prefix = sub
            .pattern
            .address()
            .segments()
            .first()
            .map(|s| format!("/{}", s))
            .unwrap_or_else(|| "/".to_string());

        self.by_prefix
            .entry(prefix)
            .or_insert_with(Vec::new)
            .push(key.clone());

        self.subscriptions.insert(key, sub);
    }

    /// Remove a subscription
    pub fn remove(&self, session_id: &SessionId, id: u32) -> Option<Subscription> {
        let key = (session_id.clone(), id);
        self.subscriptions.remove(&key).map(|(_, sub)| sub)
    }

    /// Remove all subscriptions for a session
    pub fn remove_session(&self, session_id: &SessionId) {
        let keys: Vec<_> = self
            .subscriptions
            .iter()
            .filter(|entry| entry.key().0 == *session_id)
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys {
            self.subscriptions.remove(&key);
        }
    }

    /// Find all sessions subscribed to an address
    pub fn find_subscribers(
        &self,
        address: &str,
        signal_type: Option<SignalType>,
    ) -> Vec<SessionId> {
        let mut subscribers = HashSet::new();

        // Check all subscriptions (could be optimized with better indexing)
        for entry in self.subscriptions.iter() {
            let sub = entry.value();
            if sub.matches(address, signal_type) {
                subscribers.insert(sub.session_id.clone());
            }
        }

        subscribers.into_iter().collect()
    }

    /// Get subscription count
    pub fn len(&self) -> usize {
        self.subscriptions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.subscriptions.is_empty()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_matching() {
        let sub = Subscription::new(
            1,
            "session1".to_string(),
            "/lumen/scene/*/layer/*/opacity",
            vec![],
            SubscribeOptions::default(),
        )
        .unwrap();

        assert!(sub.matches("/lumen/scene/0/layer/3/opacity", None));
        assert!(!sub.matches("/lumen/scene/0/opacity", None));
    }

    #[test]
    fn test_manager() {
        let manager = SubscriptionManager::new();

        let sub = Subscription::new(
            1,
            "session1".to_string(),
            "/test/**",
            vec![],
            SubscribeOptions::default(),
        )
        .unwrap();

        manager.add(sub);

        let subscribers = manager.find_subscribers("/test/foo/bar", None);
        assert!(subscribers.contains(&"session1".to_string()));
    }
}
