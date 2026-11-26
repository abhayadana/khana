//! Four-note chord realization for the polyphonic Chords performer.

use super::chord::{Chord, ChordExtension};

/// Four ascending MIDI notes forming the current Chords voicing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Voicing {
    pub notes: [u8; 4],
}

/// Produces smooth four-note chord voicings.
pub struct VoicingEngine;

impl VoicingEngine {
    /// Creates a voicing engine.
    pub const fn new() -> Self {
        Self
    }

    /// Finds a smooth upper realization for the Chords performer.
    ///
    /// When `bass_present` is true, the dedicated Bass performer is allowed to
    /// supply the root. Chords then prioritize the tones that identify quality
    /// and extension: third, seventh, and explicit upper extensions before an
    /// otherwise redundant root doubling.
    pub fn realize_with_bass(
        &self,
        chord: Chord,
        previous: Option<Voicing>,
        spread: f32,
        bass_present: bool,
    ) -> Voicing {
        let spread = spread.clamp(0.0, 1.0);
        let targets = target_registers(spread);
        let candidates = candidate_notes(chord);

        let mut best = [52_u8, 60, 67, 72];
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
                        if !covers_required_tones(chord, notes, bass_present) {
                            continue;
                        }

                        let score = score_voicing(notes, previous, targets)
                            + root_doubling_cost(chord, notes, bass_present);
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
    (47_u8..=91)
        .filter(|note| chord.contains_midi_pitch(*note))
        .collect()
}

fn covers_required_tones(chord: Chord, notes: [u8; 4], bass_present: bool) -> bool {
    let pitch_classes = notes.map(|note| note % 12);
    let intervals = chord.intervals();

    let required_intervals: Vec<u8> = if bass_present {
        match chord.extension {
            ChordExtension::Triad => vec![intervals[1], intervals[2]],
            ChordExtension::Seventh => {
                vec![intervals[1], intervals[2], intervals[3]]
            }
            ChordExtension::Ninth => {
                vec![intervals[1], intervals[3], intervals[4]]
            }
        }
    } else {
        match chord.extension {
            ChordExtension::Triad => intervals[..3].to_vec(),
            ChordExtension::Seventh => intervals[..4].to_vec(),
            ChordExtension::Ninth => {
                vec![intervals[0], intervals[1], intervals[3], intervals[4]]
            }
        }
    };

    required_intervals.into_iter().all(|interval| {
        let pitch_class = (chord.root.value() + interval) % 12;
        pitch_classes.contains(&pitch_class)
    })
}

fn root_doubling_cost(chord: Chord, notes: [u8; 4], bass_present: bool) -> f32 {
    if !bass_present {
        return 0.0;
    }

    let root = chord.root.value();
    let root_count = notes.iter().filter(|note| **note % 12 == root).count();
    root_count as f32 * 2.5
}

fn target_registers(spread: f32) -> [f32; 4] {
    let compact = [52.0_f32, 57.0, 62.0, 67.0];
    let wide = [48.0_f32, 59.0, 72.0, 84.0];

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

    movement_cost * if previous.is_some() { 2.5 } else { 0.0 }
        + target_cost
        + f32::from(largest_gap.saturating_sub(24)) * 2.0
}

#[cfg(test)]
mod tests {
    use super::VoicingEngine;
    use crate::music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};

    #[test]
    fn seventh_voicing_without_bass_contains_all_four_chord_tones() {
        let chord = Chord::new(
            PitchClass::from_value(0),
            ChordQuality::Major,
            ChordExtension::Seventh,
        );
        let voicing = VoicingEngine::new().realize_with_bass(chord, None, 0.5, false);
        let pcs = voicing.notes.map(|note| note % 12);

        for essential in chord.essential_pitch_classes() {
            assert!(pcs.contains(&essential));
        }
    }

    #[test]
    fn dominant_ninth_with_bass_keeps_third_seventh_and_ninth() {
        let chord = Chord::new(
            PitchClass::from_value(7),
            ChordQuality::Dominant,
            ChordExtension::Ninth,
        );
        let voicing = VoicingEngine::new().realize_with_bass(chord, None, 0.5, true);
        let pcs = voicing.notes.map(|note| note % 12);

        // G9 with a dedicated bass must still state B, F, and A upstairs.
        for required in [11_u8, 5, 9] {
            assert!(pcs.contains(&required));
        }
    }

    #[test]
    fn chord_voicing_stays_in_upper_register() {
        let chord = Chord::new(
            PitchClass::from_value(0),
            ChordQuality::Major,
            ChordExtension::Triad,
        );
        let voicing = VoicingEngine::new().realize_with_bass(chord, None, 0.5, true);

        assert!(voicing.notes[0] >= 47);
    }
}
