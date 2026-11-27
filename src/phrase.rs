//! Semantic phrase capture, editing, and playback.

use crate::music::chord::Chord;

/// One performed chord event with phrase-relative timing.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChordEvent {
    pub chord: Chord,
    pub start_beat: f64,
    pub duration_beats: f64,
}

/// Phrase Editor duration grid.
///
/// Note-value grids are expressed in quarter-note beats. `Beat` is
/// meter-aware: compound x/8 meters divisible by three use a dotted-quarter
/// beat, while other meters use the denominator note. `Bar` always uses the
/// full current measure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EditGrid {
    Sixteenth,
    Eighth,
    Quarter,
    Half,
    Beat,
    Bar,
}

impl EditGrid {
    pub const ALL: [Self; 6] = [
        Self::Sixteenth,
        Self::Eighth,
        Self::Quarter,
        Self::Half,
        Self::Beat,
        Self::Bar,
    ];

    /// Compact UI label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sixteenth => "1/16",
            Self::Eighth => "1/8",
            Self::Quarter => "1/4",
            Self::Half => "1/2",
            Self::Beat => "BEAT",
            Self::Bar => "BAR",
        }
    }

    /// Selects the next edit grid.
    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(2);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    /// Selects the previous edit grid.
    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(2);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// Returns the grid duration in quarter-note beats.
    pub fn beats(self, numerator: u8, denominator: u8) -> f64 {
        match self {
            Self::Sixteenth => 0.25,
            Self::Eighth => 0.5,
            Self::Quarter => 1.0,
            Self::Half => 2.0,
            Self::Beat => {
                if denominator == 8 && numerator >= 6 && numerator.is_multiple_of(3) {
                    1.5
                } else {
                    4.0 / f64::from(denominator.max(1))
                }
            }
            Self::Bar => f64::from(numerator) * 4.0 / f64::from(denominator.max(1)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LiveGrid {
    #[default]
    Immediate,
    Eighth,
    Quarter,
    Beat,
    Bar,
}

impl LiveGrid {
    pub const fn next(self) -> Self {
        match self {
            Self::Immediate => Self::Eighth,
            Self::Eighth => Self::Quarter,
            Self::Quarter => Self::Beat,
            Self::Beat => Self::Bar,
            Self::Bar => Self::Immediate,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Immediate => "NOW",
            Self::Eighth => "1/8",
            Self::Quarter => "1/4",
            Self::Beat => "BEAT",
            Self::Bar => "BAR",
        }
    }

    pub fn quantum(self, meter: crate::music::settings::TimeSignature) -> Option<f64> {
        let grid = match self {
            Self::Immediate => return None,
            Self::Eighth => EditGrid::Eighth,
            Self::Quarter => EditGrid::Quarter,
            Self::Beat => EditGrid::Beat,
            Self::Bar => EditGrid::Bar,
        };
        Some(grid.beats(meter.numerator, meter.denominator))
    }
}

/// Auto-numbered phrase containing semantic chord events.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Phrase {
    pub looping: bool,
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

    /// Snaps a chord duration to the adjacent edit-grid boundary.
    ///
    /// Off-grid performed durations are preserved until edited. Extending an
    /// off-grid event snaps upward; shortening snaps downward. If already on a
    /// grid boundary, the duration moves by exactly one grid unit.
    pub fn snap_resize_event(&mut self, index: usize, grid_beats: f64, extend: bool) {
        let Some(event) = self.events.get_mut(index) else {
            return;
        };

        let grid = grid_beats.max(0.25);
        let units = event.duration_beats / grid;
        let nearest = units.round();
        let on_grid = (units - nearest).abs() < 1.0e-6;

        let snapped_units = if extend {
            if on_grid { nearest + 1.0 } else { units.ceil() }
        } else if on_grid {
            nearest - 1.0
        } else {
            units.floor()
        };

        event.duration_beats = (snapped_units * grid).max(0.25);
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

    pub fn continue_numbering(&mut self, phrases: &[Phrase]) {
        self.next_phrase_id = phrases
            .iter()
            .map(|phrase| phrase.id)
            .max()
            .unwrap_or_default()
            .saturating_add(1);
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

    pub fn cancel_pending_start(&mut self) {
        self.pending_start = None;
    }

    pub fn reset_timeline(&mut self) {
        self.pending_start = None;
        self.active = None;
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
            looping: false,
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
    pub chord_beat: Option<f64>,
    pub next_chord: Option<Chord>,
    pub next_chord_beat: Option<f64>,
    pub ended: bool,
}

/// Semantic phrase player supporting one-shot playback, loops, and bar-quantized replacement.
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

    pub fn phrase_edited(&mut self, phrase_index: usize) {
        if let Some(playing) = &mut self.playing
            && playing.phrase_index == phrase_index
        {
            // Re-read the edited timeline at the current playhead.
            playing.next_event_index = 0;
        }
    }

    pub fn clear_queue(&mut self) {
        self.queued = None;
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
                ended: true,
                ..PhrasePlaybackUpdate::default()
            };
        };

        let duration = phrase.duration_beats();
        if phrase.looping && duration > 0.0 && now_beat >= playing.started_beat + duration {
            let loops = ((now_beat - playing.started_beat) / duration)
                .floor()
                .max(1.0);
            playing.started_beat += loops * duration;
            playing.next_event_index = 0;
        }
        let mut current = None;

        while let Some(event) = phrase.events.get(playing.next_event_index) {
            let start = playing.started_beat + event.start_beat;
            if now_beat < start {
                break;
            }

            current = Some((event.chord, start));
            playing.next_event_index += 1;
        }

        let ended = now_beat >= playing.started_beat + duration;

        let next_event = if ended {
            None
        } else {
            phrase.events.get(playing.next_event_index)
        };
        let next = next_event
            .map(|event| (event.chord, playing.started_beat + event.start_beat))
            .or_else(|| {
                phrase
                    .looping
                    .then(|| phrase.events.first())
                    .flatten()
                    .map(|event| (event.chord, playing.started_beat + duration))
            });
        let queued = self.queued.and_then(|entry| {
            let chord = phrases.get(entry.phrase_index)?.events.first()?.chord;
            Some((chord, entry.start_beat))
        });
        let next = match (next, queued) {
            (Some(a), Some(b)) => Some(if a.1 < b.1 { a } else { b }),
            (a, b) => a.or(b),
        };
        let (next_chord, next_chord_beat) = next.unzip();

        if ended {
            self.playing = None;
        } else {
            self.playing = Some(playing);
        }

        let (chord, chord_beat) = current.unzip();
        PhrasePlaybackUpdate {
            chord,
            chord_beat,
            next_chord,
            next_chord_beat,
            ended,
        }
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

pub fn next_strict_boundary(now_beat: f64, quantum: f64) -> f64 {
    ((now_beat / quantum).floor() + 1.0) * quantum
}

#[cfg(test)]
mod tests {
    use super::{ChordEvent, EditGrid, Phrase, PhrasePlayer, PhraseRecorder, RecorderStatus};
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
            looping: false,
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

        let first = player.update(&phrases, 0.0);
        assert_eq!(first.chord, Some(c_major()));
        assert_eq!(first.next_chord, Some(d_minor()));
        assert_eq!(first.next_chord_beat, Some(2.0));

        let second = player.update(&phrases, 2.0);
        assert_eq!(second.chord, Some(d_minor()));
        assert_eq!(second.next_chord, None);
        assert_eq!(second.next_chord_beat, None);
        assert!(player.update(&phrases, 4.0).ended);
    }

    #[test]
    fn loop_wraps_at_recorded_length_and_queued_phrase_takes_the_next_bar() {
        let events = [
            ChordEvent {
                chord: c_major(),
                start_beat: 0.0,
                duration_beats: 1.0,
            },
            ChordEvent {
                chord: d_minor(),
                start_beat: 1.0,
                duration_beats: 1.3,
            },
        ];
        let phrases = [
            Phrase {
                looping: true,
                id: 1,
                events: events.to_vec(),
            },
            Phrase {
                looping: false,
                id: 2,
                events: vec![ChordEvent {
                    start_beat: 0.0,
                    ..events[1]
                }],
            },
        ];
        let mut player = PhrasePlayer::new();
        player.request(0, 4.0, 4.0);
        assert_eq!(player.update(&phrases, 4.0).chord, Some(c_major()));
        let last = player.update(&phrases, 5.0);
        assert_eq!(last.chord, Some(d_minor()));
        assert_eq!(last.next_chord_beat, Some(6.3));
        let seam = player.update(&phrases, 6.3);
        assert_eq!(seam.chord, Some(c_major()));
        assert_eq!(seam.chord_beat, Some(6.3));
        assert!(!seam.ended);
        player.request(1, 6.5, 4.0);
        assert_eq!(player.update(&phrases, 7.31).next_chord_beat, Some(8.0));
        assert_eq!(player.update(&phrases, 8.0).chord, Some(d_minor()));
        assert_eq!(player.playing_phrase(), Some(1));
        assert_eq!(player.queued_phrase(), None);
    }

    #[test]
    fn resizing_a_playing_phrase_resolves_the_chord_at_the_current_beat() {
        let mut phrases = [Phrase {
            looping: true,
            id: 1,
            events: vec![
                ChordEvent {
                    chord: c_major(),
                    start_beat: 0.0,
                    duration_beats: 1.0,
                },
                ChordEvent {
                    chord: d_minor(),
                    start_beat: 1.0,
                    duration_beats: 2.0,
                },
            ],
        }];
        let mut player = PhrasePlayer::new();
        player.request(0, 4.0, 4.0);
        assert_eq!(player.update(&phrases, 5.5).chord, Some(d_minor()));
        phrases[0].snap_resize_event(0, 1.0, true);
        player.phrase_edited(0);
        let update = player.update(&phrases, 5.5);
        assert_eq!(update.chord, Some(c_major()));
        assert_eq!(update.next_chord, Some(d_minor()));
        assert_eq!(update.next_chord_beat, Some(6.0));
    }

    #[test]
    fn off_grid_duration_snaps_to_adjacent_boundary() {
        let chord = crate::music::settings::MusicalSettings::default()
            .tonal
            .tonic_chord();
        let mut phrase = Phrase {
            looping: false,
            id: 1,
            events: vec![ChordEvent {
                chord,
                start_beat: 0.0,
                duration_beats: 4.1,
            }],
        };

        phrase.snap_resize_event(0, 1.0, true);
        assert!((phrase.events[0].duration_beats - 5.0).abs() < 1.0e-9);

        phrase.events[0].duration_beats = 4.1;
        phrase.snap_resize_event(0, 1.0, false);
        assert!((phrase.events[0].duration_beats - 4.0).abs() < 1.0e-9);
    }

    #[test]
    fn compound_meter_beat_grid_is_dotted_quarter() {
        assert!((EditGrid::Beat.beats(6, 8) - 1.5).abs() < 1.0e-9);
        assert!((EditGrid::Beat.beats(9, 8) - 1.5).abs() < 1.0e-9);
        assert!((EditGrid::Beat.beats(12, 8) - 1.5).abs() < 1.0e-9);
    }

    #[test]
    fn bar_grid_tracks_meter() {
        assert!((EditGrid::Bar.beats(4, 4) - 4.0).abs() < 1.0e-9);
        assert!((EditGrid::Bar.beats(3, 4) - 3.0).abs() < 1.0e-9);
        assert!((EditGrid::Bar.beats(6, 8) - 3.0).abs() < 1.0e-9);
    }
}
