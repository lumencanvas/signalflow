//! Gesture Move Coalescing
//!
//! Per the CLASP protocol, routers MAY coalesce `move` phases to reduce bandwidth.
//! This module implements a gesture registry that:
//! - Buffers Move phases, only forwarding the most recent
//! - Immediately forwards Start, End, and Cancel phases
//! - Flushes buffered Moves when a non-Move phase arrives or after a timeout
//!
//! # Example
//!
//! ```ignore
//! let mut registry = GestureRegistry::new(Duration::from_millis(16));
//!
//! // Move phases get buffered
//! registry.process(gesture_move); // -> None (buffered)
//! registry.process(gesture_move); // -> None (replaces previous)
//!
//! // End phase flushes the buffer
//! registry.process(gesture_end);  // -> Some([buffered_move, end])
//! ```

use clasp_core::{GesturePhase, PublishMessage, SignalType};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::debug;

/// Key for tracking active gestures
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GestureKey {
    /// The address this gesture is published to
    pub address: String,
    /// The gesture ID (for multi-touch)
    pub gesture_id: u32,
}

impl GestureKey {
    pub fn new(address: &str, gesture_id: u32) -> Self {
        Self {
            address: address.to_string(),
            gesture_id,
        }
    }
}

/// Buffered gesture state
#[derive(Debug, Clone)]
struct BufferedGesture {
    /// The most recent Move message (if any)
    pending_move: Option<PublishMessage>,
    /// When this gesture started
    started_at: Instant,
    /// When the last Move was buffered
    last_move_at: Option<Instant>,
}

impl BufferedGesture {
    fn new() -> Self {
        Self {
            pending_move: None,
            started_at: Instant::now(),
            last_move_at: None,
        }
    }
}

/// Result of processing a gesture message
#[derive(Debug)]
pub enum GestureResult {
    /// Forward these messages immediately
    Forward(Vec<PublishMessage>),
    /// Message was buffered, nothing to forward yet
    Buffered,
    /// Not a gesture message, pass through
    PassThrough,
}

/// Gesture registry for move coalescing
pub struct GestureRegistry {
    /// Active gestures indexed by (address, gesture_id)
    gestures: DashMap<GestureKey, BufferedGesture>,
    /// How long to buffer moves before flushing
    flush_interval: Duration,
}

impl GestureRegistry {
    /// Create a new gesture registry with the specified flush interval
    pub fn new(flush_interval: Duration) -> Self {
        Self {
            gestures: DashMap::new(),
            flush_interval,
        }
    }

    /// Create with default 16ms flush interval (60fps)
    pub fn default() -> Self {
        Self::new(Duration::from_millis(16))
    }

    /// Process a publish message, returning what should be forwarded
    pub fn process(&self, msg: &PublishMessage) -> GestureResult {
        // Only handle gesture messages
        if msg.signal != Some(SignalType::Gesture) {
            return GestureResult::PassThrough;
        }

        let phase = match msg.phase {
            Some(p) => p,
            None => return GestureResult::PassThrough,
        };

        let gesture_id = msg.id.unwrap_or(0);
        let key = GestureKey::new(&msg.address, gesture_id);

        match phase {
            GesturePhase::Start => {
                // Register new gesture and forward immediately
                self.gestures.insert(key, BufferedGesture::new());
                debug!("Gesture started: {}:{}", msg.address, gesture_id);
                GestureResult::Forward(vec![msg.clone()])
            }

            GesturePhase::Move => {
                // Buffer the move, replacing any previous
                if let Some(mut entry) = self.gestures.get_mut(&key) {
                    entry.pending_move = Some(msg.clone());
                    entry.last_move_at = Some(Instant::now());
                    GestureResult::Buffered
                } else {
                    // No active gesture - forward anyway (late join scenario)
                    GestureResult::Forward(vec![msg.clone()])
                }
            }

            GesturePhase::End | GesturePhase::Cancel => {
                // Flush any buffered move, then forward the end/cancel
                let mut to_forward = Vec::with_capacity(2);

                if let Some((_, buffered)) = self.gestures.remove(&key) {
                    if let Some(pending) = buffered.pending_move {
                        to_forward.push(pending);
                    }
                }

                to_forward.push(msg.clone());
                debug!(
                    "Gesture {:?}: {}:{}",
                    phase, msg.address, gesture_id
                );
                GestureResult::Forward(to_forward)
            }
        }
    }

    /// Flush all gestures that have pending moves older than the flush interval
    /// Returns messages that should be forwarded
    pub fn flush_stale(&self) -> Vec<PublishMessage> {
        let now = Instant::now();
        let mut to_forward = Vec::new();

        for mut entry in self.gestures.iter_mut() {
            if let Some(last_move) = entry.last_move_at {
                if now.duration_since(last_move) >= self.flush_interval {
                    if let Some(pending) = entry.pending_move.take() {
                        to_forward.push(pending);
                    }
                    entry.last_move_at = None;
                }
            }
        }

        to_forward
    }

    /// Get count of active gestures
    pub fn active_count(&self) -> usize {
        self.gestures.len()
    }

    /// Remove stale gestures (no activity for extended period)
    /// This prevents memory leaks from abandoned gestures
    pub fn cleanup_stale(&self, max_age: Duration) {
        let now = Instant::now();
        self.gestures.retain(|_, v| now.duration_since(v.started_at) < max_age);
    }
}

/// Background task that periodically flushes stale gestures
pub fn spawn_flush_task(
    registry: Arc<GestureRegistry>,
    flush_tx: mpsc::UnboundedSender<Vec<PublishMessage>>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        
        loop {
            ticker.tick().await;
            
            let to_flush = registry.flush_stale();
            if !to_flush.is_empty() {
                if flush_tx.send(to_flush).is_err() {
                    break; // Channel closed
                }
            }

            // Cleanup very old gestures (> 5 minutes with no end)
            registry.cleanup_stale(Duration::from_secs(300));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clasp_core::Value;
    use std::collections::HashMap;

    fn make_gesture(address: &str, id: u32, phase: GesturePhase) -> PublishMessage {
        PublishMessage {
            address: address.to_string(),
            signal: Some(SignalType::Gesture),
            phase: Some(phase),
            id: Some(id),
            value: None,
            payload: Some(Value::Map(Default::default())),
            samples: None,
            rate: None,
            timestamp: None,
            timeline: None,
        }
    }

    fn make_gesture_with_payload(address: &str, id: u32, phase: GesturePhase, payload: Value) -> PublishMessage {
        PublishMessage {
            address: address.to_string(),
            signal: Some(SignalType::Gesture),
            phase: Some(phase),
            id: Some(id),
            value: None,
            payload: Some(payload),
            samples: None,
            rate: None,
            timestamp: None,
            timeline: None,
        }
    }

    #[test]
    fn test_start_forwards_immediately() {
        let registry = GestureRegistry::default();
        let msg = make_gesture("/touch", 1, GesturePhase::Start);
        
        match registry.process(&msg) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 1);
                assert_eq!(msgs[0].phase, Some(GesturePhase::Start));
                assert_eq!(msgs[0].id, Some(1));
            }
            _ => panic!("Expected Forward"),
        }
    }

    #[test]
    fn test_move_gets_buffered() {
        let registry = GestureRegistry::default();
        
        // Start gesture
        let start = make_gesture("/touch", 1, GesturePhase::Start);
        registry.process(&start);
        
        // Move should be buffered
        let move1 = make_gesture("/touch", 1, GesturePhase::Move);
        match registry.process(&move1) {
            GestureResult::Buffered => {}
            _ => panic!("Expected Buffered"),
        }
        
        // Registry should have active gesture
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn test_move_replaces_previous_move() {
        let registry = GestureRegistry::default();
        
        // Start gesture
        registry.process(&make_gesture("/touch", 1, GesturePhase::Start));
        
        // First move
        let move1 = make_gesture_with_payload("/touch", 1, GesturePhase::Move, Value::Int(1));
        registry.process(&move1);
        
        // Second move (should replace first)
        let move2 = make_gesture_with_payload("/touch", 1, GesturePhase::Move, Value::Int(2));
        registry.process(&move2);
        
        // End should flush only the second move
        let end = make_gesture("/touch", 1, GesturePhase::End);
        match registry.process(&end) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 2);
                // First message should be the last move (value 2)
                if let Some(Value::Int(v)) = msgs[0].payload.as_ref().and_then(|p| match p {
                    Value::Int(i) => Some(Value::Int(*i)),
                    _ => None,
                }) {
                    assert_eq!(v, 2);
                } else {
                    panic!("Expected last move to have value 2");
                }
                assert_eq!(msgs[1].phase, Some(GesturePhase::End));
            }
            _ => panic!("Expected Forward with 2 messages"),
        }
    }

    #[test]
    fn test_end_flushes_buffered_move() {
        let registry = GestureRegistry::default();
        
        // Start gesture
        let start = make_gesture("/touch", 1, GesturePhase::Start);
        registry.process(&start);
        
        // Buffer some moves
        registry.process(&make_gesture("/touch", 1, GesturePhase::Move));
        registry.process(&make_gesture("/touch", 1, GesturePhase::Move));
        
        // End should flush the last move + end
        let end = make_gesture("/touch", 1, GesturePhase::End);
        match registry.process(&end) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 2);
                assert_eq!(msgs[0].phase, Some(GesturePhase::Move));
                assert_eq!(msgs[1].phase, Some(GesturePhase::End));
            }
            _ => panic!("Expected Forward with 2 messages"),
        }
        
        // Gesture should be removed
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_end_without_move() {
        let registry = GestureRegistry::default();
        
        // Start gesture
        registry.process(&make_gesture("/touch", 1, GesturePhase::Start));
        
        // End without any moves
        let end = make_gesture("/touch", 1, GesturePhase::End);
        match registry.process(&end) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 1);
                assert_eq!(msgs[0].phase, Some(GesturePhase::End));
            }
            _ => panic!("Expected Forward with 1 message"),
        }
    }

    #[test]
    fn test_cancel_flushes_buffered_move() {
        let registry = GestureRegistry::default();
        
        let start = make_gesture("/touch", 1, GesturePhase::Start);
        registry.process(&start);
        
        registry.process(&make_gesture("/touch", 1, GesturePhase::Move));
        
        let cancel = make_gesture("/touch", 1, GesturePhase::Cancel);
        match registry.process(&cancel) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 2);
                assert_eq!(msgs[0].phase, Some(GesturePhase::Move));
                assert_eq!(msgs[1].phase, Some(GesturePhase::Cancel));
            }
            _ => panic!("Expected Forward with 2 messages"),
        }
    }

    #[test]
    fn test_multiple_gestures_independent() {
        let registry = GestureRegistry::default();
        
        // Start two gestures
        registry.process(&make_gesture("/touch", 1, GesturePhase::Start));
        registry.process(&make_gesture("/touch", 2, GesturePhase::Start));
        
        // Buffer moves for both
        registry.process(&make_gesture("/touch", 1, GesturePhase::Move));
        registry.process(&make_gesture("/touch", 2, GesturePhase::Move));
        
        // End gesture 1 - should only flush gesture 1's move
        match registry.process(&make_gesture("/touch", 1, GesturePhase::End)) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 2);
                assert_eq!(msgs[0].id, Some(1));
                assert_eq!(msgs[1].id, Some(1));
            }
            _ => panic!("Expected Forward"),
        }
        
        // Gesture 2 should still be active
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn test_different_addresses_independent() {
        let registry = GestureRegistry::default();
        
        // Start gestures on different addresses
        registry.process(&make_gesture("/touch1", 1, GesturePhase::Start));
        registry.process(&make_gesture("/touch2", 1, GesturePhase::Start));
        
        // Buffer moves
        registry.process(&make_gesture("/touch1", 1, GesturePhase::Move));
        registry.process(&make_gesture("/touch2", 1, GesturePhase::Move));
        
        // End one
        match registry.process(&make_gesture("/touch1", 1, GesturePhase::End)) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 2);
                assert_eq!(msgs[0].address, "/touch1");
                assert_eq!(msgs[1].address, "/touch1");
            }
            _ => panic!("Expected Forward"),
        }
        
        // Other should still be active
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn test_move_without_start() {
        let registry = GestureRegistry::default();
        
        // Move without start (late join scenario)
        let move_msg = make_gesture("/touch", 1, GesturePhase::Move);
        match registry.process(&move_msg) {
            GestureResult::Forward(msgs) => {
                // Should forward anyway
                assert_eq!(msgs.len(), 1);
                assert_eq!(msgs[0].phase, Some(GesturePhase::Move));
            }
            _ => panic!("Expected Forward for late join"),
        }
    }

    #[test]
    fn test_rapid_start_end() {
        let registry = GestureRegistry::default();
        
        // Rapid start/end without moves
        registry.process(&make_gesture("/touch", 1, GesturePhase::Start));
        let end = make_gesture("/touch", 1, GesturePhase::End);
        
        match registry.process(&end) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 1);
                assert_eq!(msgs[0].phase, Some(GesturePhase::End));
            }
            _ => panic!("Expected Forward"),
        }
    }

    #[test]
    fn test_concurrent_gestures_same_address() {
        let registry = GestureRegistry::default();
        
        // Multiple concurrent gestures on same address (multitouch)
        for id in 1..=5 {
            registry.process(&make_gesture("/multitouch", id, GesturePhase::Start));
            registry.process(&make_gesture("/multitouch", id, GesturePhase::Move));
        }
        
        assert_eq!(registry.active_count(), 5);
        
        // End all
        for id in 1..=5 {
            let end = make_gesture("/multitouch", id, GesturePhase::End);
            match registry.process(&end) {
                GestureResult::Forward(msgs) => {
                    assert_eq!(msgs.len(), 2); // Move + End
                    assert_eq!(msgs[0].id, Some(id));
                    assert_eq!(msgs[1].id, Some(id));
                }
                _ => panic!("Expected Forward"),
            }
        }
        
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_flush_stale() {
        let registry = GestureRegistry::new(Duration::from_millis(1));
        
        // Start and buffer a move
        registry.process(&make_gesture("/touch", 1, GesturePhase::Start));
        registry.process(&make_gesture("/touch", 1, GesturePhase::Move));
        
        // Wait for flush interval
        std::thread::sleep(Duration::from_millis(5));
        
        let flushed = registry.flush_stale();
        assert_eq!(flushed.len(), 1);
        assert_eq!(flushed[0].phase, Some(GesturePhase::Move));
        
        // Second flush should be empty (no pending moves)
        let flushed2 = registry.flush_stale();
        assert!(flushed2.is_empty());
    }

    #[test]
    fn test_flush_stale_multiple_gestures() {
        let registry = GestureRegistry::new(Duration::from_millis(1));
        
        // Start multiple gestures
        for id in 1..=3 {
            registry.process(&make_gesture("/touch", id, GesturePhase::Start));
            registry.process(&make_gesture("/touch", id, GesturePhase::Move));
        }
        
        std::thread::sleep(Duration::from_millis(5));
        
        let flushed = registry.flush_stale();
        assert_eq!(flushed.len(), 3);
        
        // All should still be active
        assert_eq!(registry.active_count(), 3);
    }

    #[test]
    fn test_cleanup_stale() {
        let registry = GestureRegistry::default();
        
        // Start a gesture
        registry.process(&make_gesture("/touch", 1, GesturePhase::Start));
        assert_eq!(registry.active_count(), 1);
        
        // Cleanup shouldn't remove recent gesture
        registry.cleanup_stale(Duration::from_secs(300));
        assert_eq!(registry.active_count(), 1);
        
        // But should remove very old ones (simulated by setting started_at in past)
        // This is harder to test without exposing internals, so we test the behavior
        // by ensuring cleanup doesn't break active gestures
    }

    #[test]
    fn test_non_gesture_passes_through() {
        let registry = GestureRegistry::default();
        
        let msg = PublishMessage {
            address: "/test".to_string(),
            signal: Some(SignalType::Event),
            phase: None,
            id: None,
            value: Some(Value::Bool(true)),
            payload: None,
            samples: None,
            rate: None,
            timestamp: None,
            timeline: None,
        };
        
        match registry.process(&msg) {
            GestureResult::PassThrough => {}
            _ => panic!("Expected PassThrough"),
        }
    }

    #[test]
    fn test_gesture_without_phase() {
        let registry = GestureRegistry::default();
        
        let msg = PublishMessage {
            address: "/test".to_string(),
            signal: Some(SignalType::Gesture),
            phase: None, // Missing phase
            id: Some(1),
            value: None,
            payload: None,
            samples: None,
            rate: None,
            timestamp: None,
            timeline: None,
        };
        
        match registry.process(&msg) {
            GestureResult::PassThrough => {}
            _ => panic!("Expected PassThrough for gesture without phase"),
        }
    }

    #[test]
    fn test_gesture_without_id() {
        let registry = GestureRegistry::default();
        
        // Gesture without ID should use 0
        let msg = PublishMessage {
            address: "/test".to_string(),
            signal: Some(SignalType::Gesture),
            phase: Some(GesturePhase::Start),
            id: None, // No ID
            value: None,
            payload: None,
            samples: None,
            rate: None,
            timestamp: None,
            timeline: None,
        };
        
        match registry.process(&msg) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 1);
                assert_eq!(msgs[0].id, None); // ID is preserved as None
            }
            _ => panic!("Expected Forward"),
        }
    }

    #[test]
    fn test_stress_many_gestures() {
        let registry = GestureRegistry::default();
        
        // Create 100 concurrent gestures
        for id in 0..100 {
            registry.process(&make_gesture("/stress", id, GesturePhase::Start));
            registry.process(&make_gesture("/stress", id, GesturePhase::Move));
        }
        
        assert_eq!(registry.active_count(), 100);
        
        // End all
        for id in 0..100 {
            registry.process(&make_gesture("/stress", id, GesturePhase::End));
        }
        
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_rapid_move_updates() {
        let registry = GestureRegistry::default();
        
        registry.process(&make_gesture("/rapid", 1, GesturePhase::Start));
        
        // Send 1000 rapid moves
        for i in 0..1000 {
            let payload = Value::Map({
                let mut m = HashMap::new();
                m.insert("index".to_string(), Value::Int(i));
                m
            });
            registry.process(&make_gesture_with_payload("/rapid", 1, GesturePhase::Move, payload));
        }
        
        // Only last move should be buffered
        let end = make_gesture("/rapid", 1, GesturePhase::End);
        match registry.process(&end) {
            GestureResult::Forward(msgs) => {
                assert_eq!(msgs.len(), 2);
                // Last move should have index 999
                if let Some(Value::Map(map)) = msgs[0].payload.as_ref() {
                    if let Some(Value::Int(idx)) = map.get("index") {
                        assert_eq!(*idx, 999);
                    } else {
                        panic!("Expected index in payload");
                    }
                } else {
                    panic!("Expected Map payload");
                }
            }
            _ => panic!("Expected Forward"),
        }
    }
}
