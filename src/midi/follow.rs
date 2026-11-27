//! MIDI realtime follower. Keep counting actual pulses and interpolate only
//! between them, so tempo smoothing cannot move the transport ahead by a beat.

use std::time::{Duration, Instant};

const PULSES_PER_QUARTER: f64 = 24.0;
const CLOCK_TIMEOUT: Duration = Duration::from_millis(750);

pub(crate) struct MidiFollower {
    enabled: bool,
    connected: bool,
    running: bool,
    waiting_for_first_pulse: bool,
    advancing_on_first_pulse: bool,
    restarted: bool,
    pulses: u64,
    last_pulse: Option<Instant>,
    start_requested_at: Option<Instant>,
    measured_at: Option<(Instant, u64)>,
    received: u64,
    seconds_per_pulse: f64,
    measured: bool,
}

impl MidiFollower {
    pub(crate) fn new(bpm: f64) -> Self {
        Self {
            enabled: true,
            connected: false,
            running: false,
            waiting_for_first_pulse: false,
            advancing_on_first_pulse: false,
            restarted: false,
            pulses: 0,
            last_pulse: None,
            start_requested_at: None,
            measured_at: None,
            received: 0,
            seconds_per_pulse: 60.0 / (bpm * PULSES_PER_QUARTER),
            measured: false,
        }
    }

    pub(crate) fn update_port(&mut self, connected: bool, now: Instant) {
        self.connected = connected;
        self.check_timeout(now);
    }

    pub(crate) fn check_timeout(&mut self, now: Instant) {
        let transport_timed_out = self
            .start_requested_at
            .is_some_and(|at| now.saturating_duration_since(at) >= CLOCK_TIMEOUT);
        let pulse_timed_out = self.last_pulse.is_some_and(|at| self.timed_out(now, at));
        if !self.connected || transport_timed_out || pulse_timed_out {
            self.running = false;
            self.waiting_for_first_pulse = false;
            self.restarted = false;
            self.start_requested_at = None;
            self.last_pulse = None;
            self.measured_at = None;
            self.measured = false;
        }
    }

    fn timed_out(&self, now: Instant, last: Instant) -> bool {
        let timeout = CLOCK_TIMEOUT.max(Duration::from_secs_f64(self.seconds_per_pulse * 8.0));
        now.saturating_duration_since(last) >= timeout
    }

    pub(crate) fn receive(&mut self, bytes: &[u8], now: Instant) {
        // Realtime bytes are valid even between the bytes of a note or SysEx.
        for &byte in bytes {
            match byte {
                0xF8 => self.pulse(now),
                0xFA if self.enabled => {
                    self.restarted = true;
                    self.running = false;
                    self.pulses = 0;
                    self.waiting_for_first_pulse = true;
                    self.advancing_on_first_pulse = false;
                    self.start_requested_at = Some(now);
                }
                0xFB if self.enabled && !self.running => {
                    self.waiting_for_first_pulse = true;
                    self.advancing_on_first_pulse = true;
                    self.start_requested_at = Some(now);
                }
                0xFC => {
                    self.running = false;
                    self.waiting_for_first_pulse = false;
                    self.restarted = false;
                    self.start_requested_at = None;
                }
                _ => {}
            }
        }
    }

    fn pulse(&mut self, now: Instant) {
        self.received += 1;
        if let Some((previous, pulse_count)) = self.measured_at {
            let elapsed = now.saturating_duration_since(previous);
            // A read can contain several clocks with the same host timestamp.
            // Wait for a distinct timestamp rather than measuring zero time.
            if !elapsed.is_zero() {
                let period = elapsed.as_secs_f64() / (self.received - pulse_count) as f64;
                if (60.0 / (400.0 * PULSES_PER_QUARTER)..=60.0 / (20.0 * PULSES_PER_QUARTER))
                    .contains(&period)
                {
                    self.seconds_per_pulse = if self.measured {
                        0.8 * self.seconds_per_pulse + 0.2 * period
                    } else {
                        period
                    };
                    self.measured = true;
                }
                self.measured_at = Some((now, self.received));
            }
        } else {
            self.measured_at = Some((now, self.received));
        }
        if self.waiting_for_first_pulse {
            self.waiting_for_first_pulse = false;
            self.start_requested_at = None;
            self.running = true;
            self.pulses += u64::from(self.advancing_on_first_pulse);
        } else if self.running {
            self.pulses += 1;
        }
        self.last_pulse = Some(now);
    }

    pub(crate) fn take_restart(&mut self) -> bool {
        self.running && std::mem::take(&mut self.restarted)
    }

    pub(crate) fn arm(&mut self) {
        self.enabled = true;
    }

    pub(crate) fn disarm(&mut self) {
        self.enabled = false;
        self.running = false;
        self.waiting_for_first_pulse = false;
        self.restarted = false;
        self.start_requested_at = None;
    }

    pub(crate) fn running(&self) -> bool {
        self.running
    }

    pub(crate) fn tempo(&self) -> f64 {
        60.0 / (self.seconds_per_pulse * PULSES_PER_QUARTER)
    }

    pub(crate) fn beat(&self, now: Instant) -> f64 {
        let fraction = self
            .last_pulse
            .filter(|_| self.running)
            .map_or(0.0, |pulse| {
                (now.saturating_duration_since(pulse).as_secs_f64() / self.seconds_per_pulse)
                    .min(0.999)
            });
        (self.pulses as f64 + fraction) / PULSES_PER_QUARTER
    }

    pub(crate) fn label(&self) -> &'static str {
        if !self.enabled {
            "DISARMED"
        } else if !self.connected {
            "NO PORT"
        } else if self.running && self.measured {
            "LOCKED"
        } else if self.running {
            "ACQUIRING"
        } else if self.last_pulse.is_some() {
            "WAIT START"
        } else {
            "WAIT CLOCK"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_tracks_tempo_and_start_stop_continue() {
        let now = Instant::now();
        let mut follower = MidiFollower::new(100.0);
        follower.update_port(true, now);
        follower.receive(&[0x90, 0xFA, 60, 90], now);
        assert!(!follower.running());
        // System realtime can arrive within a SysEx message.
        for pulse in 0..=24 {
            let time = now + Duration::from_millis(pulse * 20);
            follower.receive(&[0xF0, 0xF8, 0xF7], time);
        }
        let time = now + Duration::from_millis(480);
        assert_eq!(follower.beat(time), 1.0);
        assert!((follower.tempo() - 125.0).abs() < 0.01);
        follower.receive(&[0xFC, 0xF8, 0xF8], time);
        assert!(!follower.running());
        assert_eq!(follower.beat(time), 1.0);
        follower.receive(&[0xFB, 0xF8], time);
        assert!(follower.running());
        assert!((follower.beat(time) - f64::from(25) / 24.0).abs() < 0.000_001);
        follower.receive(&[0xFA, 0xF8], time);
        assert!(follower.take_restart());
        assert_eq!(follower.beat(time), 0.0);
    }

    #[test]
    fn clock_loss_and_local_stop_require_a_new_start_command() {
        let now = Instant::now();
        let later = now + CLOCK_TIMEOUT;
        let mut follower = MidiFollower::new(120.0);
        follower.update_port(true, now);
        follower.receive(&[0xFA, 0xF8], now);
        follower.update_port(true, later);
        assert!(!follower.running());
        follower.receive(&[0xF8], later);
        assert!(!follower.running());
        follower.disarm();
        follower.receive(&[0xFA, 0xF8], later);
        assert!(!follower.running());
        follower.arm();
        follower.receive(&[0xFA, 0xF8], later);
        assert!(follower.running());
        follower.update_port(false, later);
        assert!(!follower.running());
    }
}
