//! Simple always-running beat clock for the desktop prototype.

use std::time::Instant;

/// Monotonic musical clock expressed in beats.
pub struct BeatClock {
    started: Instant,
    bpm: f64,
}

impl BeatClock {
    /// Creates an always-running clock.
    pub fn new(bpm: f64) -> Self {
        Self {
            started: Instant::now(),
            bpm,
        }
    }

    /// Returns beats elapsed since application start.
    pub fn beat(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * self.bpm / 60.0
    }
}
