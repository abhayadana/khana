//! MIDI Clock Out scheduling on the music worker's quarter-note timeline.

use std::time::{Duration, Instant};

const PULSES_PER_QUARTER: f64 = 24.0;
pub(super) const TRANSPORT_LEAD: Duration = Duration::from_millis(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum Realtime {
    Clock = 0xF8,
    Start = 0xFA,
    Continue = 0xFB,
    Stop = 0xFC,
}

#[derive(Debug, Default)]
enum Phase {
    #[default]
    Stopped,
    Waiting {
        pulse: i64,
        command: Realtime,
    },
    Armed {
        pulse: i64,
        first_clock_after: Instant,
    },
    Sending {
        next_pulse: i64,
        can_resume: bool,
    },
    Interrupted,
}

#[derive(Debug, Default)]
pub(super) struct ClockOut {
    enabled: bool,
    phase: Phase,
    resume_at: Option<i64>,
    skipped: u64,
}

impl ClockOut {
    pub(super) fn toggle(&mut self) {
        self.enabled = !self.enabled;
        self.forget_position();
    }

    /// Called while stopped when the destination or sync source changes.
    pub(super) fn forget_position(&mut self) {
        self.resume_at = None;
    }

    pub(super) fn label(&self, available: bool) -> &'static str {
        match (&self.phase, self.enabled, available) {
            (_, false, _) => "CLOCK OUT OFF",
            (_, _, false) => "CLOCK OUT NO PORT",
            (Phase::Interrupted, _, _) => "CLOCK OUT STOP/START",
            (Phase::Waiting { .. } | Phase::Armed { .. }, _, _) => "CLOCK OUT WAIT BEAT",
            (Phase::Sending { .. }, _, _) => "CLOCK OUT SENDING",
            (Phase::Stopped, _, _) => "CLOCK OUT READY",
        }
    }

    pub(super) fn take_skipped(&mut self) -> u64 {
        std::mem::take(&mut self.skipped)
    }

    pub(super) fn drop_queued_pulse(&mut self) {
        self.skipped = self.skipped.saturating_add(1);
        if let Phase::Sending { can_resume, .. } = &mut self.phase {
            *can_resume = false;
        }
    }

    /// Internal transport freezes on stop and can resume on an unchanged port.
    /// Link keeps moving, so each fresh Link launch waits for its bar boundary.
    pub(super) fn update(
        &mut self,
        beat: f64,
        running: bool,
        available: bool,
        link_launch: Option<(f64, Option<f64>)>,
        now: Instant,
    ) -> Option<Realtime> {
        if !self.enabled {
            return None;
        }
        if !available {
            self.forget_position();
            self.phase = if running {
                Phase::Interrupted
            } else {
                Phase::Stopped
            };
            return None;
        }
        let current = (beat * PULSES_PER_QUARTER).floor() as i64;
        if !running {
            if let Phase::Sending { next_pulse, .. } = self.phase
                && current >= next_pulse
            {
                self.skipped = self
                    .skipped
                    .saturating_add((current - next_pulse + 1) as u64);
                return self.stop(false);
            }
            return self.stop(link_launch.is_none());
        }

        if matches!(self.phase, Phase::Stopped) {
            let (pulse, command) = if let Some((quantum, target)) = link_launch {
                let bar = target.unwrap_or_else(|| ((beat / quantum).floor() + 1.0) * quantum);
                (
                    ((bar * PULSES_PER_QUARTER) - 1.0e-6).ceil() as i64,
                    Realtime::Start,
                )
            } else if let Some(resume_at) = self.resume_at {
                (resume_at.max(current + 1), Realtime::Continue)
            } else {
                (current + 1, Realtime::Start)
            };
            self.phase = Phase::Waiting { pulse, command };
        }

        match self.phase {
            Phase::Waiting { pulse, command } if current >= pulse - 1 => {
                self.phase = Phase::Armed {
                    pulse,
                    first_clock_after: now + TRANSPORT_LEAD,
                };
                Some(command)
            }
            Phase::Armed {
                pulse,
                first_clock_after,
            } if current >= pulse && now >= first_clock_after => {
                self.resume_at = None;
                let missed = (current - pulse) as u64;
                self.skipped = self.skipped.saturating_add(missed);
                self.phase = Phase::Sending {
                    next_pulse: current + 1,
                    can_resume: missed == 0,
                };
                Some(Realtime::Clock)
            }
            Phase::Sending {
                next_pulse,
                can_resume,
            } if current >= next_pulse => {
                let missed = (current - next_pulse) as u64;
                self.skipped = self.skipped.saturating_add(missed);
                // Send at most one pulse per poll. A burst cannot restore the
                // timing of clocks that were missed during a scheduling stall.
                self.phase = Phase::Sending {
                    next_pulse: current + 1,
                    can_resume: can_resume && missed == 0,
                };
                Some(Realtime::Clock)
            }
            Phase::Sending { next_pulse, .. } if current < next_pulse - 1 => {
                // A backwards jump in the source invalidates the outgoing phase.
                self.phase = Phase::Interrupted;
                Some(Realtime::Stop)
            }
            _ => None,
        }
    }

    pub(super) fn stop(&mut self, preserve_position: bool) -> Option<Realtime> {
        let phase = std::mem::take(&mut self.phase);
        match phase {
            Phase::Sending {
                next_pulse,
                can_resume,
            } => {
                self.resume_at = (preserve_position && can_resume).then_some(next_pulse);
                Some(Realtime::Stop)
            }
            Phase::Armed { .. } => Some(Realtime::Stop),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled() -> ClockOut {
        let mut clock = ClockOut::default();
        clock.toggle();
        clock
    }

    #[test]
    fn internal_clock_starts_resumes_and_sends_24_pulses_per_quarter() {
        let mut clock = enabled();
        let now = Instant::now();
        let later = now + TRANSPORT_LEAD;
        assert_eq!(
            clock.update(0.0, true, true, None, now),
            Some(Realtime::Start)
        );
        assert_eq!(clock.update(0.05, true, true, None, now), None);
        for pulse in 1..=24 {
            let beat = (f64::from(pulse) + 0.001) / PULSES_PER_QUARTER;
            assert_eq!(
                clock.update(beat, true, true, None, later),
                Some(Realtime::Clock)
            );
            assert_eq!(clock.update(beat, true, true, None, later), None);
        }
        assert_eq!(
            clock.update(1.01, false, true, None, later),
            Some(Realtime::Stop)
        );
        assert_eq!(
            clock.update(1.01, true, true, None, later),
            Some(Realtime::Continue)
        );
        assert_eq!(
            clock.update(1.05, true, true, None, later + TRANSPORT_LEAD),
            Some(Realtime::Clock)
        );
    }

    #[test]
    fn dropped_connection_requires_a_deliberate_restart() {
        let mut clock = enabled();
        let now = Instant::now();
        clock.update(0.0, true, true, None, now);
        assert_eq!(clock.update(0.5, true, false, None, now), None);
        assert_eq!(clock.update(1.0, true, true, None, now), None);
        assert_eq!(clock.update(1.0, false, true, None, now), None);
        assert_eq!(
            clock.update(1.0, true, true, None, now),
            Some(Realtime::Start)
        );
    }

    #[test]
    fn late_poll_does_not_burst_and_invalidates_continue() {
        let mut clock = enabled();
        let now = Instant::now();
        let later = now + TRANSPORT_LEAD;
        clock.update(0.0, true, true, None, now);
        assert_eq!(
            clock.update(0.5, true, true, None, later),
            Some(Realtime::Clock)
        );
        assert_eq!(clock.update(0.5, true, true, None, later), None);
        assert_eq!(clock.take_skipped(), 11);
        assert_eq!(clock.stop(true), Some(Realtime::Stop));
        assert_eq!(
            clock.update(0.51, true, true, None, later),
            Some(Realtime::Start)
        );
    }

    #[test]
    fn link_waits_for_the_local_meter_boundary_on_each_launch() {
        let mut clock = enabled();
        let quantum = Some((3.0, None));
        let now = Instant::now();
        let later = now + TRANSPORT_LEAD;
        assert_eq!(clock.update(1.5, true, true, quantum, now), None);
        assert_eq!(
            clock.update(2.99, true, true, quantum, now),
            Some(Realtime::Start)
        );
        assert_eq!(
            clock.update(3.0, true, true, quantum, later),
            Some(Realtime::Clock)
        );
        assert_eq!(
            clock.update(3.1, false, true, quantum, later),
            Some(Realtime::Stop)
        );
        assert_eq!(clock.update(3.2, true, true, quantum, later), None);
        assert_eq!(
            clock.update(5.99, true, true, quantum, later),
            Some(Realtime::Start)
        );
        assert_eq!(
            clock.update(6.0, true, true, quantum, later + TRANSPORT_LEAD),
            Some(Realtime::Clock)
        );
    }
}
