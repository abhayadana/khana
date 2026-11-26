//! Complete semantic-chord realization for the polyphonic Chords performer.
//!
//! Khaṇa's Chords performer states the chord that is actually displayed:
//! triads use three unique chord tones, sevenths four, and ninths five.
//! Register and Spread may reorder those tones by inversion/octave, but they
//! never silently omit or double a semantic chord member.

use super::chord::Chord;

/// Maximum semantic chord size supported by the current fixed-capacity voicing.
///
/// The current chord vocabulary tops out at ninth chords (five tones). Seven
/// slots leave room for future 11th/13th support without changing real-time
/// note-storage types again.
pub const MAX_VOICING_NOTES: usize = 7;

/// Ascending MIDI realization of one semantic chord.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Voicing {
    pub notes: [u8; MAX_VOICING_NOTES],
    pub len: usize,
}

impl Voicing {
    /// Builds a voicing from an already ascending note slice.
    pub fn from_slice(notes: &[u8]) -> Self {
        let mut stored = [0_u8; MAX_VOICING_NOTES];
        let len = notes.len().min(MAX_VOICING_NOTES);
        stored[..len].copy_from_slice(&notes[..len]);
        Self { notes: stored, len }
    }

    /// Returns only the active MIDI notes.
    pub fn as_slice(&self) -> &[u8] {
        &self.notes[..self.len]
    }

    /// Iterates over active MIDI notes.
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        self.as_slice().iter().copied()
    }

    /// Number of sounding chord tones.
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns true when no notes are stored.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Highest note in the current realization.
    pub fn top_note(self) -> u8 {
        self.as_slice().last().copied().unwrap_or(72)
    }

    /// Returns a note by index, clamped to the highest available voice.
    pub fn note(self, index: usize) -> u8 {
        if self.len == 0 {
            60
        } else {
            self.notes[index.min(self.len - 1)]
        }
    }
}

/// Produces smooth complete chord voicings.
pub struct VoicingEngine;

impl VoicingEngine {
    /// Creates a voicing engine.
    pub const fn new() -> Self {
        Self
    }

    /// Finds a smooth complete realization for the Chords performer.
    ///
    /// Every semantic chord tone occurs exactly once. Spread influences
    /// register width, while the previous voicing biases smooth motion.
    pub fn realize(&self, chord: Chord, previous: Option<Voicing>, spread: f32) -> Voicing {
        let pitch_classes = chord_pitch_classes(chord);
        let len = pitch_classes.len().min(MAX_VOICING_NOTES);
        let spread = spread.clamp(0.0, 1.0);

        if len == 0 {
            return Voicing::from_slice(&[60]);
        }

        let mut best = Voicing::from_slice(&[60]);
        let mut best_score = f32::MAX;

        for (inversion, first_pc) in pitch_classes.iter().copied().take(len).enumerate() {
            for start in 47_u8..=71 {
                if start % 12 != first_pc {
                    continue;
                }

                let mut compact = [0_u8; MAX_VOICING_NOTES];
                compact[0] = start;
                let mut valid = true;

                let mut voice = 1;
                while voice < len {
                    let pc = pitch_classes[(inversion + voice) % len];
                    let Some(next) = next_pitch_class_above(pc, compact[voice - 1]) else {
                        valid = false;
                        break;
                    };
                    compact[voice] = next;
                    voice += 1;
                }

                if !valid {
                    continue;
                }

                let expansion_count = 1_usize << len.saturating_sub(1);
                for mask in 0..expansion_count {
                    let mut notes = compact;
                    let mut candidate_valid = true;

                    let mut voice = 1;
                    while voice < len {
                        if mask & (1 << (voice - 1)) != 0 {
                            notes[voice] = notes[voice].saturating_add(12);
                        }

                        while notes[voice] <= notes[voice - 1] {
                            notes[voice] = notes[voice].saturating_add(12);
                            if notes[voice] > 96 {
                                candidate_valid = false;
                                break;
                            }
                        }

                        if !candidate_valid {
                            break;
                        }
                        voice += 1;
                    }

                    if !candidate_valid || notes[len - 1] > 96 {
                        continue;
                    }

                    let candidate = Voicing { notes, len };
                    if !contains_each_pitch_class_once(&pitch_classes, candidate) {
                        continue;
                    }

                    let score =
                        score_voicing(candidate, previous, spread) + inversion as f32 * 0.35;
                    if score < best_score {
                        best = candidate;
                        best_score = score;
                    }
                }
            }
        }

        best
    }
}

impl Default for VoicingEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn chord_pitch_classes(chord: Chord) -> Vec<u8> {
    chord
        .intervals()
        .into_iter()
        .map(|interval| (chord.root.value() + interval) % 12)
        .collect()
}

fn next_pitch_class_above(pitch_class: u8, previous: u8) -> Option<u8> {
    ((previous.saturating_add(1))..=96).find(|note| note % 12 == pitch_class % 12)
}

fn contains_each_pitch_class_once(expected: &[u8], voicing: Voicing) -> bool {
    if voicing.len() != expected.len() {
        return false;
    }

    expected.iter().copied().all(|pitch_class| {
        voicing
            .iter()
            .filter(|note| note % 12 == pitch_class)
            .count()
            == 1
    })
}

fn target_register(index: usize, len: usize, spread: f32) -> f32 {
    if len <= 1 {
        return 62.0;
    }

    let low = 53.0 - spread * 5.0;
    let high = 66.0 + spread * 18.0;
    let position = index as f32 / (len - 1) as f32;
    low + (high - low) * position
}

fn score_voicing(voicing: Voicing, previous: Option<Voicing>, spread: f32) -> f32 {
    let target_cost: f32 = voicing
        .iter()
        .enumerate()
        .map(|(index, note)| {
            (f32::from(note) - target_register(index, voicing.len(), spread)).abs()
        })
        .sum();

    let movement_cost = previous.map_or(0.0, |old| {
        if old.is_empty() {
            return 0.0;
        }

        voicing
            .iter()
            .enumerate()
            .map(|(index, note)| {
                let old_index = if voicing.len() <= 1 || old.len() <= 1 {
                    0
                } else {
                    ((index as f32 * (old.len() - 1) as f32 / (voicing.len() - 1) as f32).round()
                        as usize)
                        .min(old.len() - 1)
                };
                f32::from(note.abs_diff(old.note(old_index)))
            })
            .sum::<f32>()
            + (voicing.len().abs_diff(old.len()) as f32 * 3.0)
    });

    let largest_gap = voicing
        .as_slice()
        .windows(2)
        .map(|window| window[1] - window[0])
        .max()
        .unwrap_or(0);

    target_cost
        + movement_cost * if previous.is_some() { 2.25 } else { 0.0 }
        + f32::from(largest_gap.saturating_sub(24)) * 2.0
}

#[cfg(test)]
mod tests {
    use super::VoicingEngine;
    use crate::music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};

    fn pitch_classes(voicing: super::Voicing) -> Vec<u8> {
        voicing.iter().map(|note| note % 12).collect()
    }

    #[test]
    fn major_triad_uses_exactly_three_unique_chord_tones() {
        let chord = Chord::new(
            PitchClass::from_value(0),
            ChordQuality::Major,
            ChordExtension::Triad,
        );
        let voicing = VoicingEngine::new().realize(chord, None, 0.5);
        let pcs = pitch_classes(voicing);

        assert_eq!(voicing.len(), 3);
        assert_eq!(pcs.len(), 3);
        for required in [0_u8, 4, 7] {
            assert_eq!(pcs.iter().filter(|pc| **pc == required).count(), 1);
        }
    }

    #[test]
    fn seventh_uses_all_four_chord_tones_once() {
        let chord = Chord::new(
            PitchClass::from_value(0),
            ChordQuality::Major,
            ChordExtension::Seventh,
        );
        let voicing = VoicingEngine::new().realize(chord, None, 0.5);
        let pcs = pitch_classes(voicing);

        assert_eq!(voicing.len(), 4);
        for required in [0_u8, 4, 7, 11] {
            assert_eq!(pcs.iter().filter(|pc| **pc == required).count(), 1);
        }
    }

    #[test]
    fn dominant_ninth_uses_all_five_chord_tones_once() {
        let chord = Chord::new(
            PitchClass::from_value(7),
            ChordQuality::Dominant,
            ChordExtension::Ninth,
        );
        let voicing = VoicingEngine::new().realize(chord, None, 0.5);
        let pcs = pitch_classes(voicing);

        assert_eq!(voicing.len(), 5);
        for required in [7_u8, 11, 2, 5, 9] {
            assert_eq!(pcs.iter().filter(|pc| **pc == required).count(), 1);
        }
    }

    #[test]
    fn spread_changes_register_without_changing_membership() {
        let chord = Chord::new(
            PitchClass::from_value(0),
            ChordQuality::Minor,
            ChordExtension::Seventh,
        );
        let close = VoicingEngine::new().realize(chord, None, 0.0);
        let wide = VoicingEngine::new().realize(chord, None, 1.0);

        let mut close_pcs = pitch_classes(close);
        close_pcs.sort_unstable();
        let mut wide_pcs = pitch_classes(wide);
        wide_pcs.sort_unstable();

        assert_eq!(close_pcs, wide_pcs);
        assert!(
            *wide.as_slice().last().unwrap() - wide.as_slice()[0]
                >= *close.as_slice().last().unwrap() - close.as_slice()[0]
        );
    }

    #[test]
    fn every_current_chord_type_realizes_complete_unique_membership() {
        let qualities = [
            ChordQuality::Major,
            ChordQuality::Minor,
            ChordQuality::Dominant,
            ChordQuality::Diminished,
            ChordQuality::HalfDiminished,
        ];
        let extensions = [
            ChordExtension::Triad,
            ChordExtension::Seventh,
            ChordExtension::Ninth,
        ];
        let engine = VoicingEngine::new();

        for root in 0_u8..12 {
            for quality in qualities {
                for extension in extensions {
                    let chord = Chord::new(PitchClass::from_value(root), quality, extension);
                    let voicing = engine.realize(chord, None, 0.5);
                    let expected = chord.essential_pitch_classes();
                    let actual = pitch_classes(voicing);

                    assert_eq!(voicing.len(), expected.len());
                    for pitch_class in expected {
                        assert_eq!(
                            actual
                                .iter()
                                .filter(|candidate| **candidate == pitch_class)
                                .count(),
                            1
                        );
                    }
                }
            }
        }
    }
}
