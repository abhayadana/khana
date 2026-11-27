//! Semantic phrase capture, editing, and playback.

use std::collections::VecDeque;

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

        self.insert_event(index + 1, event);
    }

    pub fn insert_event(&mut self, index: usize, event: ChordEvent) {
        self.events.insert(index.min(self.events.len()), event);
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

    pub fn copy_harmony(&mut self, events: &[ChordEvent]) -> Phrase {
        let id = self.next_phrase_id;
        self.next_phrase_id = self.next_phrase_id.saturating_add(1);
        Phrase {
            id,
            looping: true,
            events: events.to_vec(),
        }
    }

    pub fn continue_numbering(&mut self, phrases: &[Phrase]) {
        self.next_phrase_id = phrases
            .iter()
            .map(|phrase| phrase.id)
            .max()
            .unwrap_or_default()
            .saturating_add(1);
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transition {
    #[default]
    PhraseEnd,
    Bar,
}

impl Transition {
    pub const fn label(self) -> &'static str {
        match self {
            Self::PhraseEnd => "PHRASE END",
            Self::Bar => "BAR",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChainSection {
    pub phrase_index: usize,
    pub passes: u8,
    pub transition: Transition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhrasePass {
    pub phrase_index: usize,
    pub phrase_id: u32,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug)]
struct PlayingPhrase {
    pass: PhrasePass,
    phrase: Phrase,
    next_event_index: usize,
    passes_left: Option<u8>,
    edits_pending: bool,
}

impl PlayingPhrase {
    fn repeats(&self) -> bool {
        self.passes_left
            .map_or(self.phrase.looping, |left| left > 1)
    }
}

#[derive(Clone, Copy, Debug)]
struct ScheduledPhrase {
    phrase_index: usize,
    start_beat: f64,
    passes: Option<u8>,
}

/// Position within the current pass, in quarter-note beats.
#[derive(Clone, Copy, Debug)]
pub struct PhrasePosition {
    pub elapsed: f64,
    pub remaining: f64,
}

impl PhrasePosition {
    pub fn opening(self, bar: f64) -> bool {
        self.elapsed < (bar / 2.0).min((self.elapsed + self.remaining) / 4.0)
    }

    pub fn closing(self, bar: f64) -> bool {
        self.remaining <= bar.min((self.elapsed + self.remaining) / 2.0)
    }
}

/// Result of one phrase-player update.
#[derive(Debug, Default)]
pub struct PhrasePlaybackUpdate {
    pub chord: Option<Chord>,
    pub chord_beat: Option<f64>,
    pub next_chord: Option<Chord>,
    pub next_chord_beat: Option<f64>,
    pub ended: bool,
    pub position: Option<PhrasePosition>,
}

/// Plays a stable snapshot for each pass of a phrase.
pub struct PhrasePlayer {
    playing: Option<PlayingPhrase>,
    queued: Option<ScheduledPhrase>,
    chain: VecDeque<ChainSection>,
    bar: f64,
}

impl PhrasePlayer {
    pub const CHAIN_LIMIT: usize = 16;

    pub const fn new() -> Self {
        Self {
            playing: None,
            queued: None,
            chain: VecDeque::new(),
            bar: 4.0,
        }
    }

    pub fn loop_window(&self, now: f64) -> Option<(f64, f64, u32)> {
        let playing = self.playing.as_ref()?;
        let duration = playing.phrase.duration_beats();
        if playing.edits_pending || !playing.repeats() || duration <= 0.0 {
            return None;
        }
        let start = playing.pass.start + next_strict_boundary(now - playing.pass.start, duration);
        if self
            .queued
            .is_some_and(|entry| entry.start_beat < start + duration)
        {
            return None;
        }
        Some((start, start + duration, playing.pass.phrase_id))
    }

    pub fn pass(&self) -> Option<PhrasePass> {
        self.playing.as_ref().map(|playing| playing.pass)
    }

    pub fn playing_phrase(&self) -> Option<usize> {
        self.pass().map(|pass| pass.phrase_index)
    }

    pub fn edit_pending(&self, phrase_index: usize) -> bool {
        self.playing.as_ref().is_some_and(|playing| {
            playing.pass.phrase_index == phrase_index && playing.edits_pending
        })
    }

    pub fn phrase_edited(&mut self, phrase_index: usize) {
        if let Some(playing) = &mut self.playing
            && playing.pass.phrase_index == phrase_index
        {
            playing.edits_pending = true;
        }
    }

    pub fn clear_queue(&mut self) {
        self.queued = None;
        self.chain.clear();
    }

    pub fn queued_phrase(&self) -> Option<usize> {
        self.queued
            .map(|state| state.phrase_index)
            .or_else(|| self.chain.front().map(|section| section.phrase_index))
    }

    pub fn chain_sections(&self) -> impl Iterator<Item = ChainSection> + '_ {
        self.chain.iter().copied()
    }

    pub fn section_passes_left(&self) -> Option<u8> {
        self.playing.as_ref()?.passes_left
    }

    /// Starts immediately when idle; otherwise replaces the queue at the next bar.
    pub fn request(&mut self, phrase_index: usize, now_beat: f64, beats_per_bar: f64) {
        self.clear_queue();
        self.bar = beats_per_bar;
        let start_beat = if self.playing.is_some() {
            next_strict_boundary(now_beat, self.bar)
        } else {
            now_beat
        };
        self.queued = Some(ScheduledPhrase {
            phrase_index,
            start_beat,
            passes: None,
        });
    }

    pub fn append_section(&mut self, section: ChainSection) -> bool {
        if self.chain.len() >= Self::CHAIN_LIMIT {
            return false;
        }
        self.chain.push_back(section);
        true
    }

    pub fn arm_chain(&mut self, now: f64, bar: f64) {
        self.bar = bar;
        self.arm_section(now);
    }

    fn arm_section(&mut self, now: f64) {
        if self.queued.is_some() {
            return;
        }
        let Some(section) = self.chain.front() else {
            return;
        };
        let start_beat = match &self.playing {
            Some(playing) if playing.passes_left.is_some_and(|passes| passes > 1) => return,
            Some(playing) => match section.transition {
                Transition::PhraseEnd => playing.pass.end,
                Transition::Bar if playing.repeats() => next_strict_boundary(now, self.bar),
                Transition::Bar => next_boundary(playing.pass.end, self.bar),
            },
            None => now,
        };
        self.queued = Some(ScheduledPhrase {
            phrase_index: section.phrase_index,
            start_beat,
            passes: Some(section.passes),
        });
    }

    pub fn stop(&mut self) {
        self.playing = None;
        self.clear_queue();
    }

    /// Advances playback and emits the most recent due chord.
    pub fn update(&mut self, phrases: &[Phrase], now: f64) -> PhrasePlaybackUpdate {
        self.arm_section(now);
        let mut ended = false;
        loop {
            if let Some(queued) = self.queued
                && now >= queued.start_beat
                && self
                    .playing
                    .as_ref()
                    .is_none_or(|playing| queued.start_beat <= playing.pass.end)
            {
                self.queued = None;
                if queued.passes.is_some() {
                    self.chain.pop_front();
                }
                self.playing = phrases
                    .get(queued.phrase_index)
                    .filter(|phrase| phrase.duration_beats() > 0.0)
                    .map(|phrase| PlayingPhrase {
                        phrase: phrase.clone(),
                        pass: PhrasePass {
                            phrase_index: queued.phrase_index,
                            phrase_id: phrase.id,
                            start: queued.start_beat,
                            end: queued.start_beat + phrase.duration_beats(),
                        },
                        next_event_index: 0,
                        passes_left: queued.passes,
                        edits_pending: false,
                    });
                ended = self.playing.is_none();
                self.arm_section(queued.start_beat);
                continue;
            }
            let Some(playing) = &mut self.playing else {
                break;
            };
            if now < playing.pass.end {
                break;
            }
            if playing.edits_pending
                && let Some(phrase) = phrases.get(playing.pass.phrase_index)
            {
                playing.phrase = phrase.clone();
                playing.edits_pending = false;
            }
            if !playing.repeats() || playing.phrase.duration_beats() <= 0.0 {
                self.playing = None;
                ended = true;
                continue;
            }
            if let Some(passes_left) = &mut playing.passes_left {
                *passes_left -= 1;
            }
            let start = playing.pass.end;
            let duration = playing.phrase.duration_beats();
            // Fast-forward unscheduled, unbounded loops after a transport jump.
            let skipped = if playing.passes_left.is_none() && self.queued.is_none() {
                ((now - start) / duration).floor()
            } else {
                0.0
            };
            playing.pass.start = start + skipped * duration;
            playing.pass.end = playing.pass.start + duration;
            playing.next_event_index = 0;
            self.arm_section(start);
        }

        let mut result = PhrasePlaybackUpdate {
            ended,
            ..Default::default()
        };
        let queued = self.queued.and_then(|entry| {
            let chord = phrases.get(entry.phrase_index)?.events.first()?.chord;
            Some((chord, entry.start_beat))
        });
        let mut next = None;
        if let Some(playing) = &mut self.playing {
            while let Some(event) = playing.phrase.events.get(playing.next_event_index) {
                let start = playing.pass.start + event.start_beat;
                if now < start {
                    next = Some((event.chord, start));
                    break;
                }
                result.chord = Some(event.chord);
                result.chord_beat = Some(start);
                playing.next_event_index += 1;
            }
            let upcoming = &phrases[playing.pass.phrase_index];
            let repeats = playing
                .passes_left
                .map_or(upcoming.looping, |left| left > 1);
            if next.is_none() && repeats {
                next = upcoming
                    .events
                    .first()
                    .map(|event| (event.chord, playing.pass.end));
            }
            result.position = Some(PhrasePosition {
                elapsed: now - playing.pass.start,
                remaining: playing.pass.end - now,
            });
        }
        let next = match (next, queued) {
            (Some(a), Some(b)) => Some(if a.1 < b.1 { a } else { b }),
            (a, b) => a.or(b),
        };
        (result.next_chord, result.next_chord_beat) = next.unzip();
        result
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

pub(crate) fn next_boundary(now_beat: f64, quantum: f64) -> f64 {
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
        let (start, stop, id) = player.loop_window(5.0).unwrap();
        assert!((start - 6.3).abs() < 1.0e-9);
        assert!((stop - 8.6).abs() < 1.0e-9);
        assert_eq!(id, 1);
        let last = player.update(&phrases, 5.0);
        assert_eq!(last.chord, Some(d_minor()));
        assert_eq!(last.next_chord_beat, Some(6.3));
        assert!(last.position.unwrap().remaining > 1.29);
        assert!(!last.position.unwrap().closing(4.0));
        let seam = player.update(&phrases, 6.3);
        assert_eq!(seam.chord, Some(c_major()));
        assert_eq!(seam.chord_beat, Some(6.3));
        assert!(!seam.ended);
        assert_eq!(seam.position.unwrap().elapsed, 0.0);
        assert!(seam.position.unwrap().opening(4.0));
        player.request(1, 6.5, 4.0);
        assert_eq!(player.update(&phrases, 7.31).next_chord_beat, Some(8.0));
        assert_eq!(player.update(&phrases, 8.0).chord, Some(d_minor()));
        assert_eq!(player.playing_phrase(), Some(1));
        assert_eq!(player.queued_phrase(), None);
    }

    #[test]
    fn resizing_a_playing_phrase_applies_at_the_original_loop_seam() {
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
        assert_eq!(update.chord, None);
        assert_eq!(update.next_chord_beat, Some(7.0));
        let seam = player.update(&phrases, 7.0);
        assert_eq!(seam.chord, Some(c_major()));
        assert_eq!(seam.next_chord_beat, Some(9.0));
    }

    #[test]
    fn chain_honors_full_pass_counts_and_bar_transitions() {
        use super::{ChainSection, Transition};
        let phrases = [c_major(), d_minor(), c_major()].map(|chord| Phrase {
            id: u32::from(chord.root.value()) + 1,
            looping: true,
            events: vec![ChordEvent {
                chord,
                start_beat: 0.0,
                duration_beats: 2.5,
            }],
        });
        let mut player = PhrasePlayer::new();
        for (phrase_index, passes, transition) in [
            (0, 2, Transition::PhraseEnd),
            (1, 1, Transition::Bar),
            (2, 1, Transition::PhraseEnd),
        ] {
            assert!(player.append_section(ChainSection {
                phrase_index,
                passes,
                transition
            }));
        }
        player.arm_chain(0.0, 4.0);
        assert_eq!(player.update(&phrases, 0.0).chord, Some(c_major()));
        assert_eq!(player.section_passes_left(), Some(2));
        assert_eq!(player.update(&phrases, 2.5).chord_beat, Some(2.5));
        assert_eq!(player.section_passes_left(), Some(1));
        assert_eq!(player.update(&phrases, 4.5).next_chord_beat, Some(8.0));
        assert!(player.update(&phrases, 5.0).ended);
        assert!(player.update(&phrases, 7.0).position.is_none());
        assert_eq!(player.update(&phrases, 8.0).chord, Some(d_minor()));
        assert_eq!(player.update(&phrases, 10.5).chord, Some(c_major()));
        assert_eq!(player.playing_phrase(), Some(2));
        assert!(player.update(&phrases, 13.0).ended);
        assert_eq!(player.playing_phrase(), None);
    }

    #[test]
    fn next_bar_can_exit_an_unbounded_loop_before_its_seam() {
        use super::{ChainSection, Transition};
        let phrases = [c_major(), d_minor()].map(|chord| Phrase {
            id: u32::from(chord.root.value()) + 1,
            looping: true,
            events: vec![ChordEvent {
                chord,
                start_beat: 0.0,
                duration_beats: 6.5,
            }],
        });
        let mut player = PhrasePlayer::new();
        player.request(0, 0.0, 4.0);
        player.update(&phrases, 0.0);
        assert!(player.append_section(ChainSection {
            phrase_index: 1,
            passes: 1,
            transition: Transition::Bar,
        }));
        player.arm_chain(1.0, 4.0);
        assert_eq!(player.update(&phrases, 4.0).chord, Some(d_minor()));
        assert_eq!(player.pass().unwrap().start, 4.0);
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
