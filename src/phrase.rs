//! Semantic phrase capture, editing, and playback.

use crate::music::chord::Chord;

/// One performed chord event with phrase-relative timing.
#[derive(Clone, Copy, Debug)]
pub struct ChordEvent {
    pub chord: Chord,
    pub start_beat: f64,
    pub duration_beats: f64,
}

/// Auto-numbered phrase containing semantic chord events.
#[derive(Clone, Debug)]
pub struct Phrase {
    pub id: u32,
    pub events: Vec<ChordEvent>,
}

impl Phrase {
    pub fn duration_beats(&self) -> f64 {
        self.events
            .last()
            .map(|event| event.start_beat + event.duration_beats)
            .unwrap_or(0.0)
    }

    pub fn replace_chord(&mut self, index: usize, chord: Chord) {
        if let Some(event) = self.events.get_mut(index) {
            event.chord = chord;
        }
    }

    pub fn duplicate_event(&mut self, index: usize) {
        let Some(event) = self.events.get(index).copied() else {
            return;
        };

        self.events.insert(index + 1, event);
        self.reflow();
    }

    pub fn delete_event(&mut self, index: usize) {
        if self.events.len() <= 1 || index >= self.events.len() {
            return;
        }

        self.events.remove(index);
        self.reflow();
    }

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
    pub const fn new() -> Self {
        Self {
            next_phrase_id: 1,
            pending_start: None,
            active: None,
            arm_next: false,
            quantize_beats: 4.0,
        }
    }

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

    pub const fn quantize_beats(&self) -> f64 {
        self.quantize_beats
    }

    pub const fn next_is_armed(&self) -> bool {
        self.arm_next
    }

    /// Toggles recording using the supplied quantization period.
    pub fn toggle(&mut self, now_beat: f64, quantize_beats: f64) {
        self.quantize_beats = quantize_beats.max(0.25);

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

    pub fn toggle_arm_next(&mut self) {
        self.arm_next = !self.arm_next;
    }

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

#[derive(Clone, Copy, Debug)]
struct PlayingPhrase {
    phrase_index: usize,
    started_beat: f64,
    next_event_index: usize,
}

#[derive(Clone, Copy, Debug)]
struct ScheduledPhrase {
    phrase_index: usize,
    start_beat: f64,
}

/// Result of one phrase-player update.
#[derive(Debug, Default)]
pub struct PhrasePlaybackUpdate {
    pub chord: Option<Chord>,
    pub ended: bool,
}

/// One-shot semantic phrase player with quantized replacement queueing.
pub struct PhrasePlayer {
    playing: Option<PlayingPhrase>,
    queued: Option<ScheduledPhrase>,
}

impl PhrasePlayer {
    pub const fn new() -> Self {
        Self {
            playing: None,
            queued: None,
        }
    }

    pub fn playing_phrase(&self) -> Option<usize> {
        self.playing.map(|state| state.phrase_index)
    }

    pub fn queued_phrase(&self) -> Option<usize> {
        self.queued.map(|state| state.phrase_index)
    }

    /// Starts immediately when idle; otherwise queues for the next bar boundary.
    pub fn request(&mut self, phrase_index: usize, now_beat: f64, beats_per_bar: f64) {
        let start_beat = if self.playing.is_some() {
            next_strict_boundary(now_beat, beats_per_bar)
        } else {
            now_beat
        };

        self.queued = Some(ScheduledPhrase {
            phrase_index,
            start_beat,
        });
    }

    /// Cancels phrase playback and queue state.
    pub fn stop(&mut self) {
        self.playing = None;
        self.queued = None;
    }

    /// Advances semantic playback and emits the most recent due chord.
    pub fn update(&mut self, phrases: &[Phrase], now_beat: f64) -> PhrasePlaybackUpdate {
        if let Some(queued) = self.queued
            && now_beat >= queued.start_beat
        {
            self.queued = None;
            self.playing = Some(PlayingPhrase {
                phrase_index: queued.phrase_index,
                started_beat: queued.start_beat,
                next_event_index: 0,
            });
        }

        let Some(mut playing) = self.playing else {
            return PhrasePlaybackUpdate::default();
        };

        let Some(phrase) = phrases.get(playing.phrase_index) else {
            self.playing = None;
            return PhrasePlaybackUpdate {
                chord: None,
                ended: true,
            };
        };

        let relative_beat = now_beat - playing.started_beat;
        let mut chord = None;

        while let Some(event) = phrase.events.get(playing.next_event_index) {
            if relative_beat + f64::EPSILON < event.start_beat {
                break;
            }

            chord = Some(event.chord);
            playing.next_event_index += 1;
        }

        let ended = relative_beat >= phrase.duration_beats();

        if ended {
            self.playing = None;
        } else {
            self.playing = Some(playing);
        }

        PhrasePlaybackUpdate { chord, ended }
    }
}

impl Default for PhrasePlayer {
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

fn next_strict_boundary(now_beat: f64, quantum: f64) -> f64 {
    ((now_beat / quantum).floor() + 1.0) * quantum
}

#[cfg(test)]
mod tests {
    use super::{ChordEvent, Phrase, PhrasePlayer, PhraseRecorder, RecorderStatus};
    use crate::music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};

    fn c_major() -> Chord {
        Chord::new(
            PitchClass::from_value(0),
            ChordQuality::Major,
            ChordExtension::Seventh,
        )
    }

    fn d_minor() -> Chord {
        Chord::new(
            PitchClass::from_value(2),
            ChordQuality::Minor,
            ChordExtension::Seventh,
        )
    }

    #[test]
    fn recording_uses_supplied_bar_quantization() {
        let mut recorder = PhraseRecorder::new();
        recorder.toggle(1.2, 3.0);

        assert_eq!(recorder.status(), RecorderStatus::Armed { start_beat: 3.0 });
    }

    #[test]
    fn phrase_player_emits_semantic_chords() {
        let phrases = vec![Phrase {
            id: 1,
            events: vec![
                ChordEvent {
                    chord: c_major(),
                    start_beat: 0.0,
                    duration_beats: 2.0,
                },
                ChordEvent {
                    chord: d_minor(),
                    start_beat: 2.0,
                    duration_beats: 2.0,
                },
            ],
        }];

        let mut player = PhrasePlayer::new();
        player.request(0, 0.0, 4.0);

        assert_eq!(player.update(&phrases, 0.0).chord, Some(c_major()));
        assert_eq!(player.update(&phrases, 2.0).chord, Some(d_minor()));
        assert!(player.update(&phrases, 4.0).ended);
    }
}
