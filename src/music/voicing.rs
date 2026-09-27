//! Four-voice chord realization with smooth voice leading.
//!
//! The default behavior is inspired by Chorder's nearest/smooth voice-leading
//! concepts: generate legal chord-tone voicings, then minimize motion from the
//! previous voicing while respecting an adjustable spread target.

use super::chord::Chord;

/// Four ascending MIDI notes, one for each Khaṇa voice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Voicing {
    pub notes: [u8; 4],
}

/// Produces smooth four-note chord voicings.
pub struct VoicingEngine;

impl VoicingEngine {
    /// Creates the voicing engine.
    pub const fn new() -> Self {
        Self
    }

    /// Finds a smooth ascending four-note realization.
    ///
    /// `spread` is clamped to 0..=1. At low values voices cluster around the
    /// middle register; at high values target registers fan outward.
    pub fn realize(&self, chord: Chord, previous: Option<Voicing>, spread: f32) -> Voicing {
        let spread = spread.clamp(0.0, 1.0);
        let targets = target_registers(spread);
        let candidates = candidate_notes(chord);

        let mut best = [48_u8, 55, 60, 67];
        let mut best_score = f32::MAX;

        for &a in &candidates {
            for &b in &candidates {
                if b <= a {
                    continue;
                }
                for &c in &candidates {
                    if c <= b {
                        continue;
                    }
                    for &d in &candidates {
                        if d <= c {
                            continue;
                        }

                        let notes = [a, b, c, d];
                        let score = score_voicing(notes, previous, targets);

                        if score < best_score {
                            best = notes;
                            best_score = score;
                        }
                    }
                }
            }
        }

        Voicing { notes: best }
    }
}

impl Default for VoicingEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn candidate_notes(chord: Chord) -> Vec<u8> {
    (36_u8..=88)
        .filter(|note| chord.contains_midi_pitch(*note))
        .collect()
}

fn target_registers(spread: f32) -> [f32; 4] {
    let compact = [48.0_f32, 55.0, 61.0, 67.0];
    let wide = [38.0_f32, 52.0, 68.0, 82.0];

    std::array::from_fn(|index| compact[index] + (wide[index] - compact[index]) * spread)
}

fn score_voicing(notes: [u8; 4], previous: Option<Voicing>, targets: [f32; 4]) -> f32 {
    let target_cost: f32 = notes
        .iter()
        .zip(targets)
        .map(|(note, target)| (f32::from(*note) - target).abs())
        .sum();

    let movement_cost = previous.map_or(0.0, |old| {
        notes
            .iter()
            .zip(old.notes)
            .map(|(new_note, old_note)| f32::from(new_note.abs_diff(old_note)))
            .sum::<f32>()
    });

    let largest_gap = notes
        .windows(2)
        .map(|window| window[1] - window[0])
        .max()
        .unwrap_or(0);

    // Motion dominates after the first chord. Register targeting keeps the
    // solution usable and prevents pathological clustering.
    movement_cost * if previous.is_some() { 2.4 } else { 0.0 }
        + target_cost
        + f32::from(largest_gap.saturating_sub(24)) * 2.0
}

#[cfg(test)]
mod tests {
    use super::{Voicing, VoicingEngine};
    use crate::music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};

    #[test]
    fn realized_notes_are_ascending_chord_tones() {
        let engine = VoicingEngine::new();
        let chord = Chord::new(PitchClass::C, ChordQuality::Major, ChordExtension::Seventh);
        let voicing = engine.realize(chord, None, 0.5);

        assert!(voicing.notes.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            voicing
                .notes
                .iter()
                .all(|note| chord.contains_midi_pitch(*note))
        );
    }

    #[test]
    fn smooth_realization_prefers_less_motion_than_fresh_realization() {
        let engine = VoicingEngine::new();
        let c_major = Chord::new(PitchClass::C, ChordQuality::Major, ChordExtension::Seventh);
        let g_seven = Chord::new(
            PitchClass::G,
            ChordQuality::Dominant,
            ChordExtension::Seventh,
        );

        let previous = engine.realize(c_major, None, 0.5);
        let smooth = engine.realize(g_seven, Some(previous), 0.5);
        let reference = Voicing {
            notes: [43, 55, 67, 79],
        };

        let smooth_motion: u16 = smooth
            .notes
            .iter()
            .zip(previous.notes)
            .map(|(left, right)| u16::from(left.abs_diff(right)))
            .sum();
        let reference_motion: u16 = reference
            .notes
            .iter()
            .zip(previous.notes)
            .map(|(left, right)| u16::from(left.abs_diff(right)))
            .sum();

        assert!(smooth_motion <= reference_motion);
    }
}
