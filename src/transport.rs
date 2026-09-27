//! Start/stop musical transport with tempo-preserving position changes.

use std::time::Instant;

/// Shared quarter-note transport used by playback and recording.
pub struct Transport {
    running: bool,
    bpm: f64,
    stored_beat: f64,
    anchor: Instant,
}

impl Transport {
    /// Creates a stopped transport at beat zero.
    pub fn new(bpm: f64) -> Self {
        Self {
            running: false,
            bpm,
            stored_beat: 0.0,
            anchor: Instant::now(),
        }
    }

    /// Returns whether the transport is running.
    pub const fn is_running(&self) -> bool {
        self.running
    }

    /// Returns the current beat position.
    pub fn beat(&self) -> f64 {
        if self.running {
            self.stored_beat + self.anchor.elapsed().as_secs_f64() * self.bpm / 60.0
        } else {
            self.stored_beat
        }
    }

    /// Starts transport without changing beat position.
    pub fn start(&mut self) {
        if self.running {
            return;
        }

        self.anchor = Instant::now();
        self.running = true;
    }

    /// Stops transport while preserving beat position.
    pub fn stop(&mut self) {
        if !self.running {
            return;
        }

        self.stored_beat = self.beat();
        self.running = false;
    }

    /// Updates tempo while preserving the current beat position.
    pub fn set_bpm(&mut self, bpm: f64) {
        let beat = self.beat();
        self.stored_beat = beat;
        self.bpm = bpm.clamp(30.0, 300.0);
        self.anchor = Instant::now();
    }
}
