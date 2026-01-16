//! Timing utilities for SignalFlow
//!
//! Provides clock synchronization and timestamp handling.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Timestamp type (microseconds)
pub type Timestamp = u64;

/// Get current Unix timestamp in microseconds
pub fn now() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as Timestamp
}

/// Convert microseconds to Duration
pub fn to_duration(micros: Timestamp) -> Duration {
    Duration::from_micros(micros)
}

/// Convert Duration to microseconds
pub fn from_duration(duration: Duration) -> Timestamp {
    duration.as_micros() as Timestamp
}

/// Clock synchronization state
#[derive(Debug, Clone)]
pub struct ClockSync {
    /// Estimated offset from server time (microseconds)
    offset: i64,
    /// Round-trip time (microseconds)
    rtt: u64,
    /// Jitter estimate (microseconds)
    jitter: u64,
    /// Number of sync samples
    samples: u32,
    /// Last sync time (local)
    last_sync: Instant,
    /// Recent RTT samples for jitter calculation
    rtt_history: Vec<u64>,
}

impl Default for ClockSync {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockSync {
    /// Create a new clock sync instance
    pub fn new() -> Self {
        Self {
            offset: 0,
            rtt: 0,
            jitter: 0,
            samples: 0,
            last_sync: Instant::now(),
            rtt_history: Vec::with_capacity(10),
        }
    }

    /// Process a sync response
    ///
    /// # Arguments
    /// * `t1` - Client send time
    /// * `t2` - Server receive time
    /// * `t3` - Server send time
    /// * `t4` - Client receive time
    pub fn process_sync(&mut self, t1: u64, t2: u64, t3: u64, t4: u64) {
        // Calculate round-trip time
        let rtt = (t4 - t1) - (t3 - t2);

        // Calculate offset using NTP algorithm
        let offset = ((t2 as i64 - t1 as i64) + (t3 as i64 - t4 as i64)) / 2;

        // Update RTT history
        self.rtt_history.push(rtt);
        if self.rtt_history.len() > 10 {
            self.rtt_history.remove(0);
        }

        // Calculate jitter (variance of RTT)
        if self.rtt_history.len() >= 2 {
            let mean: u64 = self.rtt_history.iter().sum::<u64>() / self.rtt_history.len() as u64;
            let variance: u64 = self.rtt_history
                .iter()
                .map(|&x| {
                    let diff = x as i64 - mean as i64;
                    (diff * diff) as u64
                })
                .sum::<u64>() / self.rtt_history.len() as u64;
            self.jitter = (variance as f64).sqrt() as u64;
        }

        // Use exponential moving average for offset
        if self.samples == 0 {
            self.offset = offset;
            self.rtt = rtt;
        } else {
            // Weight newer samples more
            let alpha = 0.3;
            self.offset = ((1.0 - alpha) * self.offset as f64 + alpha * offset as f64) as i64;
            self.rtt = ((1.0 - alpha) * self.rtt as f64 + alpha * rtt as f64) as u64;
        }

        self.samples += 1;
        self.last_sync = Instant::now();
    }

    /// Get estimated server time
    pub fn server_time(&self) -> Timestamp {
        let local = now();
        (local as i64 + self.offset) as Timestamp
    }

    /// Convert local time to server time
    pub fn to_server_time(&self, local: Timestamp) -> Timestamp {
        (local as i64 + self.offset) as Timestamp
    }

    /// Convert server time to local time
    pub fn to_local_time(&self, server: Timestamp) -> Timestamp {
        (server as i64 - self.offset) as Timestamp
    }

    /// Get current offset estimate
    pub fn offset(&self) -> i64 {
        self.offset
    }

    /// Get current RTT estimate
    pub fn rtt(&self) -> u64 {
        self.rtt
    }

    /// Get current jitter estimate
    pub fn jitter(&self) -> u64 {
        self.jitter
    }

    /// Check if sync is needed (e.g., every 30 seconds)
    pub fn needs_sync(&self, interval_secs: u64) -> bool {
        self.samples == 0 || self.last_sync.elapsed().as_secs() >= interval_secs
    }

    /// Get sync quality (0.0 = poor, 1.0 = excellent)
    pub fn quality(&self) -> f64 {
        if self.samples == 0 {
            return 0.0;
        }

        // Based on RTT and jitter
        let rtt_score = (10000.0 - self.rtt.min(10000) as f64) / 10000.0;
        let jitter_score = (1000.0 - self.jitter.min(1000) as f64) / 1000.0;
        let sample_score = (self.samples.min(10) as f64) / 10.0;

        (rtt_score * 0.4 + jitter_score * 0.4 + sample_score * 0.2).clamp(0.0, 1.0)
    }
}

/// Session time tracker (time since session start)
#[derive(Debug, Clone)]
pub struct SessionTime {
    start: Instant,
    start_unix: Timestamp,
}

impl Default for SessionTime {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionTime {
    /// Create a new session time tracker
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            start_unix: now(),
        }
    }

    /// Get microseconds since session start
    pub fn elapsed(&self) -> Timestamp {
        self.start.elapsed().as_micros() as Timestamp
    }

    /// Get the session start time (Unix timestamp)
    pub fn start_time(&self) -> Timestamp {
        self.start_unix
    }

    /// Convert session time to Unix timestamp
    pub fn to_unix(&self, session_time: Timestamp) -> Timestamp {
        self.start_unix + session_time
    }

    /// Convert Unix timestamp to session time
    pub fn from_unix(&self, unix_time: Timestamp) -> Timestamp {
        unix_time.saturating_sub(self.start_unix)
    }
}

/// Jitter buffer for smoothing high-rate streams
#[derive(Debug)]
pub struct JitterBuffer<T> {
    buffer: Vec<(Timestamp, T)>,
    capacity: usize,
    window_us: u64,
}

impl<T: Clone> JitterBuffer<T> {
    /// Create a new jitter buffer
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of items
    /// * `window_ms` - Buffer window in milliseconds
    pub fn new(capacity: usize, window_ms: u64) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            window_us: window_ms * 1000,
        }
    }

    /// Add a sample with timestamp
    pub fn push(&mut self, timestamp: Timestamp, value: T) {
        // Remove old samples
        let cutoff = now().saturating_sub(self.window_us);
        self.buffer.retain(|(ts, _)| *ts > cutoff);

        // Add new sample (maintain sorted order)
        let pos = self.buffer.partition_point(|(ts, _)| *ts < timestamp);
        if self.buffer.len() < self.capacity {
            self.buffer.insert(pos, (timestamp, value));
        } else if pos > 0 {
            // Replace oldest
            self.buffer.remove(0);
            let new_pos = pos.saturating_sub(1);
            self.buffer.insert(new_pos, (timestamp, value));
        }
    }

    /// Get the next sample ready for playback
    pub fn pop(&mut self, playback_time: Timestamp) -> Option<T> {
        if let Some((ts, _)) = self.buffer.first() {
            if *ts <= playback_time {
                return Some(self.buffer.remove(0).1);
            }
        }
        None
    }

    /// Get all samples ready for playback
    pub fn drain_ready(&mut self, playback_time: Timestamp) -> Vec<T> {
        let mut ready = Vec::new();
        while let Some((ts, _)) = self.buffer.first() {
            if *ts <= playback_time {
                ready.push(self.buffer.remove(0).1);
            } else {
                break;
            }
        }
        ready
    }

    /// Current buffer depth (samples)
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Current buffer depth (time span in microseconds)
    pub fn depth_us(&self) -> u64 {
        if self.buffer.len() < 2 {
            0
        } else {
            self.buffer.last().unwrap().0 - self.buffer.first().unwrap().0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_sync() {
        let mut sync = ClockSync::new();

        // Simulate sync exchange
        // Client sends at T1, server receives at T2, server sends at T3, client receives at T4
        let t1 = 1000000u64;
        let t2 = 1000050u64; // Server is 50µs ahead
        let t3 = 1000051u64;
        let t4 = 1000100u64;

        sync.process_sync(t1, t2, t3, t4);

        assert!(sync.samples > 0);
        assert!(sync.rtt > 0);
    }

    #[test]
    fn test_session_time() {
        let session = SessionTime::new();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let elapsed = session.elapsed();
        assert!(elapsed >= 10000); // At least 10ms in microseconds
    }

    #[test]
    fn test_jitter_buffer() {
        let mut buffer: JitterBuffer<f64> = JitterBuffer::new(10, 100);

        let base = now();
        buffer.push(base + 10000, 0.1);
        buffer.push(base + 20000, 0.2);
        buffer.push(base + 5000, 0.05); // Out of order

        assert_eq!(buffer.len(), 3);

        // Should return in timestamp order
        let first = buffer.pop(base + 10000);
        assert_eq!(first, Some(0.05));
    }
}
