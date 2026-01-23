//! Timeline execution engine
//!
//! Provides interpolation and playback of timeline automation data.
//!
//! # Example
//!
//! ```ignore
//! use clasp_core::{TimelineData, TimelineKeyframe, EasingType, Value, timeline::TimelinePlayer};
//!
//! let timeline = TimelineData::new(vec![
//!     TimelineKeyframe { time: 0, value: Value::Float(0.0), easing: EasingType::Linear, bezier: None },
//!     TimelineKeyframe { time: 1_000_000, value: Value::Float(1.0), easing: EasingType::EaseOut, bezier: None },
//! ]);
//!
//! let mut player = TimelinePlayer::new(timeline);
//! player.start(current_time_us);
//!
//! // Later...
//! if let Some(value) = player.sample(current_time_us) {
//!     // Use interpolated value
//! }
//! ```

use crate::{EasingType, TimelineData, TimelineKeyframe, Value};

/// State of timeline playback
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    /// Not started
    Stopped,
    /// Playing forward
    Playing,
    /// Paused at current position
    Paused,
    /// Completed (not looping)
    Finished,
}

/// Timeline player for real-time interpolation
#[derive(Debug, Clone)]
pub struct TimelinePlayer {
    /// The timeline data
    timeline: TimelineData,
    /// Current playback state
    state: PlaybackState,
    /// Start time in microseconds (server time)
    start_time: u64,
    /// Pause time (for resume)
    pause_time: Option<u64>,
    /// Current loop iteration
    loop_count: u32,
}

impl TimelinePlayer {
    /// Create a new timeline player
    pub fn new(timeline: TimelineData) -> Self {
        Self {
            timeline,
            state: PlaybackState::Stopped,
            start_time: 0,
            pause_time: None,
            loop_count: 0,
        }
    }

    /// Start playback from the beginning
    pub fn start(&mut self, current_time_us: u64) {
        self.start_time = current_time_us;
        self.state = PlaybackState::Playing;
        self.pause_time = None;
        self.loop_count = 0;
    }

    /// Start playback at a specific time
    pub fn start_at(&mut self, start_time_us: u64) {
        self.start_time = start_time_us;
        self.state = PlaybackState::Playing;
        self.pause_time = None;
        self.loop_count = 0;
    }

    /// Pause playback
    pub fn pause(&mut self, current_time_us: u64) {
        if self.state == PlaybackState::Playing {
            self.pause_time = Some(current_time_us);
            self.state = PlaybackState::Paused;
        }
    }

    /// Resume playback
    pub fn resume(&mut self, current_time_us: u64) {
        if self.state == PlaybackState::Paused {
            if let Some(pause_time) = self.pause_time {
                // Adjust start time to account for pause duration
                let pause_duration = current_time_us.saturating_sub(pause_time);
                self.start_time = self.start_time.saturating_add(pause_duration);
            }
            self.state = PlaybackState::Playing;
            self.pause_time = None;
        }
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.pause_time = None;
    }

    /// Get current playback state
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Get current loop count
    pub fn loop_count(&self) -> u32 {
        self.loop_count
    }

    /// Get timeline duration
    pub fn duration(&self) -> u64 {
        self.timeline.duration()
    }

    /// Sample the timeline at the current time
    ///
    /// Returns None if stopped or no keyframes exist.
    /// Returns the interpolated value based on current time.
    pub fn sample(&mut self, current_time_us: u64) -> Option<Value> {
        if self.state == PlaybackState::Stopped {
            return None;
        }

        if self.timeline.keyframes.is_empty() {
            return None;
        }

        // Calculate elapsed time
        let elapsed = if self.state == PlaybackState::Paused {
            self.pause_time.unwrap_or(current_time_us) - self.start_time
        } else {
            current_time_us.saturating_sub(self.start_time)
        };

        let duration = self.timeline.duration();
        if duration == 0 {
            return Some(self.timeline.keyframes[0].value.clone());
        }

        // Handle looping
        let position = if self.timeline.loop_ {
            let new_loop_count = (elapsed / duration) as u32;
            if new_loop_count > self.loop_count {
                self.loop_count = new_loop_count;
            }
            elapsed % duration
        } else if elapsed >= duration {
            self.state = PlaybackState::Finished;
            return Some(self.timeline.keyframes.last()?.value.clone());
        } else {
            elapsed
        };

        // Find surrounding keyframes
        let (prev_kf, next_kf) = self.find_keyframes(position)?;

        // Calculate interpolation factor
        let segment_duration = next_kf.time.saturating_sub(prev_kf.time);
        if segment_duration == 0 {
            return Some(prev_kf.value.clone());
        }

        let local_t = (position - prev_kf.time) as f64 / segment_duration as f64;
        let eased_t = apply_easing(local_t, prev_kf.easing, prev_kf.bezier);

        // Interpolate value
        Some(interpolate_value(&prev_kf.value, &next_kf.value, eased_t))
    }

    /// Find the keyframes surrounding the given position
    fn find_keyframes(&self, position: u64) -> Option<(&TimelineKeyframe, &TimelineKeyframe)> {
        let keyframes = &self.timeline.keyframes;

        if keyframes.is_empty() {
            return None;
        }

        if keyframes.len() == 1 {
            return Some((&keyframes[0], &keyframes[0]));
        }

        // Find the first keyframe after position
        let next_idx = keyframes
            .iter()
            .position(|kf| kf.time > position)
            .unwrap_or(keyframes.len());

        if next_idx == 0 {
            // Before first keyframe
            Some((&keyframes[0], &keyframes[0]))
        } else if next_idx >= keyframes.len() {
            // After last keyframe
            let last = &keyframes[keyframes.len() - 1];
            Some((last, last))
        } else {
            Some((&keyframes[next_idx - 1], &keyframes[next_idx]))
        }
    }
}

/// Apply easing function to normalized time (0.0 - 1.0)
fn apply_easing(t: f64, easing: EasingType, bezier: Option<[f64; 4]>) -> f64 {
    let t = t.clamp(0.0, 1.0);

    match easing {
        EasingType::Linear => t,
        EasingType::EaseIn => t * t,
        EasingType::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        EasingType::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        EasingType::Step => {
            if t < 1.0 {
                0.0
            } else {
                1.0
            }
        }
        EasingType::CubicBezier => {
            if let Some([x1, y1, x2, y2]) = bezier {
                cubic_bezier(t, x1, y1, x2, y2)
            } else {
                t // Fall back to linear if no control points
            }
        }
    }
}

/// Cubic bezier interpolation
/// Control points define the curve: P0=(0,0), P1=(x1,y1), P2=(x2,y2), P3=(1,1)
fn cubic_bezier(t: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    // For simplicity, we approximate by using Newton-Raphson to find t for x, then compute y
    // This is a simplified implementation - full CSS cubic-bezier would use more iterations

    // Solve for parameter u where bezier_x(u) = t
    let mut u = t;
    for _ in 0..8 {
        let x = bezier_sample(u, x1, x2);
        let dx = bezier_derivative(u, x1, x2);
        if dx.abs() < 1e-10 {
            break;
        }
        u -= (x - t) / dx;
        u = u.clamp(0.0, 1.0);
    }

    // Compute y at parameter u
    bezier_sample(u, y1, y2)
}

fn bezier_sample(t: f64, p1: f64, p2: f64) -> f64 {
    // B(t) = 3(1-t)²t*P1 + 3(1-t)t²*P2 + t³
    let mt = 1.0 - t;
    3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t
}

fn bezier_derivative(t: f64, p1: f64, p2: f64) -> f64 {
    // B'(t) = 3(1-t)²*P1 + 6(1-t)t*(P2-P1) + 3t²*(1-P2)
    let mt = 1.0 - t;
    3.0 * mt * mt * p1 + 6.0 * mt * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

/// Interpolate between two values
fn interpolate_value(a: &Value, b: &Value, t: f64) -> Value {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) => Value::Float(a + (b - a) * t),
        (Value::Int(a), Value::Int(b)) => {
            Value::Int(*a + ((*b - *a) as f64 * t) as i64)
        }
        (Value::Array(arr_a), Value::Array(arr_b)) if arr_a.len() == arr_b.len() => {
            Value::Array(
                arr_a
                    .iter()
                    .zip(arr_b.iter())
                    .map(|(a, b)| interpolate_value(a, b, t))
                    .collect(),
            )
        }
        // For non-interpolatable values, use step at 0.5
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_timeline() -> TimelineData {
        TimelineData::new(vec![
            TimelineKeyframe {
                time: 0,
                value: Value::Float(0.0),
                easing: EasingType::Linear,
                bezier: None,
            },
            TimelineKeyframe {
                time: 1_000_000, // 1 second
                value: Value::Float(100.0),
                easing: EasingType::Linear,
                bezier: None,
            },
        ])
    }

    #[test]
    fn test_timeline_player_creation() {
        let timeline = make_simple_timeline();
        let player = TimelinePlayer::new(timeline);
        assert_eq!(player.state(), PlaybackState::Stopped);
    }

    #[test]
    fn test_timeline_player_start() {
        let timeline = make_simple_timeline();
        let mut player = TimelinePlayer::new(timeline);

        player.start(0);
        assert_eq!(player.state(), PlaybackState::Playing);
    }

    #[test]
    fn test_timeline_linear_interpolation() {
        let timeline = make_simple_timeline();
        let mut player = TimelinePlayer::new(timeline);

        player.start(0);

        // At t=0
        let val = player.sample(0).unwrap();
        assert!(matches!(val, Value::Float(v) if (v - 0.0).abs() < 0.01));

        // At t=0.5s (500ms)
        let val = player.sample(500_000).unwrap();
        assert!(matches!(val, Value::Float(v) if (v - 50.0).abs() < 0.01));

        // At t=1s
        let val = player.sample(1_000_000).unwrap();
        assert!(matches!(val, Value::Float(v) if (v - 100.0).abs() < 0.01));
    }

    #[test]
    fn test_timeline_finished_state() {
        let timeline = make_simple_timeline();
        let mut player = TimelinePlayer::new(timeline);

        player.start(0);

        // After timeline ends
        let _ = player.sample(2_000_000);
        assert_eq!(player.state(), PlaybackState::Finished);
    }

    #[test]
    fn test_timeline_looping() {
        let timeline = make_simple_timeline().with_loop(true);
        let mut player = TimelinePlayer::new(timeline);

        player.start(0);

        // First loop
        let val = player.sample(500_000).unwrap();
        assert!(matches!(val, Value::Float(v) if (v - 50.0).abs() < 0.01));

        // Second loop (at 1.5s = 500ms into second loop)
        let val = player.sample(1_500_000).unwrap();
        assert!(matches!(val, Value::Float(v) if (v - 50.0).abs() < 0.01));

        assert_eq!(player.loop_count(), 1);
    }

    #[test]
    fn test_timeline_pause_resume() {
        let timeline = make_simple_timeline();
        let mut player = TimelinePlayer::new(timeline);

        player.start(0);

        // Play to 250ms
        let _ = player.sample(250_000);

        // Pause at 250ms
        player.pause(250_000);
        assert_eq!(player.state(), PlaybackState::Paused);

        // Time passes (500ms later)
        let val = player.sample(750_000).unwrap();
        // Should still be at 250ms position (25.0)
        assert!(matches!(val, Value::Float(v) if (v - 25.0).abs() < 0.01));

        // Resume at 750ms
        player.resume(750_000);
        assert_eq!(player.state(), PlaybackState::Playing);

        // 250ms later (1000ms total, but only 500ms of playback)
        let val = player.sample(1_000_000).unwrap();
        // Should be at 500ms position (50.0)
        assert!(matches!(val, Value::Float(v) if (v - 50.0).abs() < 0.01));
    }

    #[test]
    fn test_easing_ease_in() {
        let timeline = TimelineData::new(vec![
            TimelineKeyframe {
                time: 0,
                value: Value::Float(0.0),
                easing: EasingType::EaseIn,
                bezier: None,
            },
            TimelineKeyframe {
                time: 1_000_000,
                value: Value::Float(100.0),
                easing: EasingType::Linear,
                bezier: None,
            },
        ]);
        let mut player = TimelinePlayer::new(timeline);
        player.start(0);

        // At t=0.5, ease-in should be less than linear (25 instead of 50)
        let val = player.sample(500_000).unwrap();
        assert!(matches!(val, Value::Float(v) if v < 50.0));
    }

    #[test]
    fn test_easing_ease_out() {
        let timeline = TimelineData::new(vec![
            TimelineKeyframe {
                time: 0,
                value: Value::Float(0.0),
                easing: EasingType::EaseOut,
                bezier: None,
            },
            TimelineKeyframe {
                time: 1_000_000,
                value: Value::Float(100.0),
                easing: EasingType::Linear,
                bezier: None,
            },
        ]);
        let mut player = TimelinePlayer::new(timeline);
        player.start(0);

        // At t=0.5, ease-out should be more than linear (75 instead of 50)
        let val = player.sample(500_000).unwrap();
        assert!(matches!(val, Value::Float(v) if v > 50.0));
    }

    #[test]
    fn test_array_interpolation() {
        let timeline = TimelineData::new(vec![
            TimelineKeyframe {
                time: 0,
                value: Value::Array(vec![Value::Float(0.0), Value::Float(0.0)]),
                easing: EasingType::Linear,
                bezier: None,
            },
            TimelineKeyframe {
                time: 1_000_000,
                value: Value::Array(vec![Value::Float(100.0), Value::Float(200.0)]),
                easing: EasingType::Linear,
                bezier: None,
            },
        ]);
        let mut player = TimelinePlayer::new(timeline);
        player.start(0);

        let val = player.sample(500_000).unwrap();
        if let Value::Array(arr) = val {
            assert!(matches!(arr[0], Value::Float(v) if (v - 50.0).abs() < 0.01));
            assert!(matches!(arr[1], Value::Float(v) if (v - 100.0).abs() < 0.01));
        } else {
            panic!("Expected array value");
        }
    }
}
