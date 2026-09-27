//! Phrase capture and chord-event editing.
//!
//! Phrases store semantic chord events rather than frozen rendered MIDI.

use crate::music::chord::Chord;

/// One performed chord event with phrase-relative timing.
#[derive(Clone, Copy, Debug)]
pub struct ChordEvent {
    pub chord: Chord,
    pub start_beat: f64,
    pub duration_beats: f64,
}

/// Auto-numbered phrase containing chord events.
#[derive(Clone, Debug)]
pub struct Phrase {
    pub id: u32,
    pub events: Vec<ChordEvent>,
}

impl Phrase {
    /// Returns total phrase duration in beats.
    pub fn duration_beats(&self) -> f64 {
        self.events
            .last()
            .map(|event| event.start_beat + event.duration_beats)
            .unwrap_or(0.0)
    }

    /// Replaces one chord while preserving event timing.
    pub fn replace_chord(&mut self, index: usize, chord: Chord) {
        if let Some(event) = self.events.get_mut(index) {
            event.chord = chord;
        }
    }

    /// Duplicates an event immediately after itself.
    pub fn duplicate_event(&mut self, index: usize) {
        let Some(event) = self.events.get(index).copied() else {
            return;
        };

        self.events.insert(index + 1, event);
        self.reflow();
    }

    /// Deletes an event if the phrase has more than one event.
    pub fn delete_event(&mut self, index: usize) {
        if self.events.len() <= 1 || index >= self.events.len() {
            return;
        }

        self.events.remove(index);
        self.reflow();
    }

    /// Adjusts event duration by `delta_beats`, with a quarter-beat minimum.
    pub fn resize_event(&mut self, index: usize, delta_beats: f64) {
        let Some(event) = self.events.get_mut(index) else {
            return;
        };

        event.duration_beats = (event.duration_beats + delta_beats).max(0.25);
        self.reflow();
    }

    fn reflow(&mut self) {
        let mut cursor = 0.0;

        for event in &mut self.events {
            event.start_beat = cursor;
            cursor += event.duration_beats;
        }
    }
}

#[derive(Clone, Debug)]
struct ActiveRecording {
    phrase_id: u32,
    phrase_start_beat: f64,
    current_chord: Chord,
    current_start_beat: f64,
    events: Vec<ChordEvent>,
    stop_at_beat: Option<f64>,
}

/// Recorder lifecycle shown on the Performance screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecorderStatus {
    Idle,
    Armed { start_beat: f64 },
    Recording { phrase_id: u32 },
    Stopping { phrase_id: u32, stop_beat: f64 },
}

/// Quantized semantic phrase recorder.
pub struct PhraseRecorder {
    next_phrase_id: u32,
    pending_start: Option<f64>,
    active: Option<ActiveRecording>,
    arm_next: bool,
    quantize_beats: f64,
}

impl PhraseRecorder {
    /// Creates a recorder quantized to one 4/4 bar.
    pub const fn new() -> Self {
        Self {
            next_phrase_id: 1,
            pending_start: None,
            active: None,
            arm_next: false,
            quantize_beats: 4.0,
        }
    }

    /// Returns the current recorder status.
    pub fn status(&self) -> RecorderStatus {
        if let Some(active) = &self.active {
            if let Some(stop_beat) = active.stop_at_beat {
                return RecorderStatus::Stopping {
                    phrase_id: active.phrase_id,
                    stop_beat,
                };
            }

            return RecorderStatus::Recording {
                phrase_id: active.phrase_id,
            };
        }

        if let Some(start_beat) = self.pending_start {
            return RecorderStatus::Armed { start_beat };
        }

        RecorderStatus::Idle
    }

    /// Returns the fixed MVP quantization in beats.
    pub const fn quantize_beats(&self) -> f64 {
        self.quantize_beats
    }

    /// Returns whether another phrase is armed to begin after the current one.
    pub const fn next_is_armed(&self) -> bool {
        self.arm_next
    }

    /// Toggles quantized recording start/stop.
    pub fn toggle(&mut self, now_beat: f64) {
        if let Some(active) = self.active.as_mut() {
            active.stop_at_beat = match active.stop_at_beat {
                Some(_) => None,
                None => Some(next_boundary(now_beat, self.quantize_beats)),
            };
            return;
        }

        self.pending_start = match self.pending_start {
            Some(_) => None,
            None => Some(next_boundary(now_beat, self.quantize_beats)),
        };
    }

    /// Toggles whether another phrase should start at the current stop boundary.
    pub fn toggle_arm_next(&mut self) {
        self.arm_next = !self.arm_next;
    }

    /// Records a harmonic transition when recording is active.
    pub fn chord_changed(&mut self, now_beat: f64, new_chord: Chord) {
        let Some(active) = self.active.as_mut() else {
            return;
        };

        if active.stop_at_beat.is_some_and(|stop| now_beat >= stop) {
            return;
        }

        close_current_event(active, now_beat);
        active.current_chord = new_chord;
        active.current_start_beat = now_beat;
    }

    /// Advances recorder state and returns a completed phrase when one closes.
    pub fn update(&mut self, now_beat: f64, current_chord: Chord) -> Option<Phrase> {
        if let Some(start_beat) = self.pending_start
            && now_beat >= start_beat
        {
            self.pending_start = None;
            let phrase_id = self.next_phrase_id;
            self.next_phrase_id += 1;
            self.active = Some(ActiveRecording {
                phrase_id,
                phrase_start_beat: start_beat,
                current_chord,
                current_start_beat: start_beat,
                events: Vec::new(),
                stop_at_beat: None,
            });
        }

        let stop_beat = self.active.as_ref()?.stop_at_beat?;

        if now_beat < stop_beat {
            return None;
        }

        let mut completed = self.active.take()?;
        close_current_event(&mut completed, stop_beat);

        let phrase = Phrase {
            id: completed.phrase_id,
            events: completed.events,
        };

        if self.arm_next {
            self.arm_next = false;
            let phrase_id = self.next_phrase_id;
            self.next_phrase_id += 1;
            self.active = Some(ActiveRecording {
                phrase_id,
                phrase_start_beat: stop_beat,
                current_chord,
                current_start_beat: stop_beat,
                events: Vec::new(),
                stop_at_beat: None,
            });
        }

        Some(phrase)
    }
}

impl Default for PhraseRecorder {
    fn default() -> Self {
        Self::new()
    }
}

fn close_current_event(active: &mut ActiveRecording, end_beat: f64) {
    let duration = (end_beat - active.current_start_beat).max(0.0);

    if duration <= f64::EPSILON {
        return;
    }

    active.events.push(ChordEvent {
        chord: active.current_chord,
        start_beat: active.current_start_beat - active.phrase_start_beat,
        duration_beats: duration,
    });
}

fn next_boundary(now_beat: f64, quantum: f64) -> f64 {
    let quotient = now_beat / quantum;
    let rounded = quotient.ceil() * quantum;

    if (rounded - now_beat).abs() < 0.000_001 {
        now_beat
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use super::{Phrase, PhraseRecorder, RecorderStatus};
    use crate::music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};

    fn c_major() -> Chord {
        Chord::new(PitchClass::C, ChordQuality::Major, ChordExtension::Seventh)
    }

    fn d_minor() -> Chord {
        Chord::new(PitchClass::D, ChordQuality::Minor, ChordExtension::Seventh)
    }

    #[test]
    fn recording_is_quantized_to_bar_boundary() {
        let mut recorder = PhraseRecorder::new();
        recorder.toggle(1.2);

        assert_eq!(recorder.status(), RecorderStatus::Armed { start_beat: 4.0 });

        recorder.update(4.0, c_major());
        assert_eq!(
            recorder.status(),
            RecorderStatus::Recording { phrase_id: 1 }
        );
    }

    #[test]
    fn phrase_capture_preserves_performed_chord_durations() {
        let mut recorder = PhraseRecorder::new();
        recorder.toggle(0.0);
        recorder.update(0.0, c_major());
        recorder.chord_changed(2.0, d_minor());
        recorder.toggle(5.0);

        let phrase = recorder
            .update(8.0, d_minor())
            .expect("phrase should close at beat 8");

        assert_eq!(phrase.events.len(), 2);
        assert_eq!(phrase.events[0].duration_beats, 2.0);
        assert_eq!(phrase.events[1].duration_beats, 6.0);
    }

    #[test]
    fn phrase_resize_reflows_following_events() {
        let mut phrase = Phrase {
            id: 1,
            events: vec![
                super::ChordEvent {
                    chord: c_major(),
                    start_beat: 0.0,
                    duration_beats: 2.0,
                },
                super::ChordEvent {
                    chord: d_minor(),
                    start_beat: 2.0,
                    duration_beats: 2.0,
                },
            ],
        };

        phrase.resize_event(0, 1.0);

        assert_eq!(phrase.events[0].duration_beats, 3.0);
        assert_eq!(phrase.events[1].start_beat, 3.0);
    }
}
