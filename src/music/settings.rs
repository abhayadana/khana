//! Tonal and metric settings shared by harmony, transport, and capture.

use super::chord::{Chord, ChordExtension, ChordQuality, PitchClass};

/// Seven diatonic modes supported by the MVP.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScaleMode {
    Ionian,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Aeolian,
    Locrian,
}

impl ScaleMode {
    pub const ALL: [Self; 7] = [
        Self::Ionian,
        Self::Dorian,
        Self::Phrygian,
        Self::Lydian,
        Self::Mixolydian,
        Self::Aeolian,
        Self::Locrian,
    ];

    /// Compact mode name.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ionian => "IONIAN",
            Self::Dorian => "DORIAN",
            Self::Phrygian => "PHRYGIAN",
            Self::Lydian => "LYDIAN",
            Self::Mixolydian => "MIXOLYDIAN",
            Self::Aeolian => "AEOLIAN",
            Self::Locrian => "LOCRIAN",
        }
    }

    /// Seven scale intervals in semitones from tonic.
    pub const fn intervals(self) -> [u8; 7] {
        match self {
            Self::Ionian => [0, 2, 4, 5, 7, 9, 11],
            Self::Dorian => [0, 2, 3, 5, 7, 9, 10],
            Self::Phrygian => [0, 1, 3, 5, 7, 8, 10],
            Self::Lydian => [0, 2, 4, 6, 7, 9, 11],
            Self::Mixolydian => [0, 2, 4, 5, 7, 9, 10],
            Self::Aeolian => [0, 2, 3, 5, 7, 8, 10],
            Self::Locrian => [0, 1, 3, 5, 6, 8, 10],
        }
    }

    /// Returns the next mode, wrapping after Locrian.
    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    /// Returns the previous mode, wrapping before Ionian.
    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Supported time signatures for the MVP.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

impl TimeSignature {
    pub const PRESETS: [Self; 8] = [
        Self::new(2, 4),
        Self::new(3, 4),
        Self::new(4, 4),
        Self::new(5, 4),
        Self::new(6, 8),
        Self::new(7, 8),
        Self::new(9, 8),
        Self::new(12, 8),
    ];

    /// Creates a time signature.
    pub const fn new(numerator: u8, denominator: u8) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Returns the bar duration in quarter-note beats.
    pub fn beats_per_bar(self) -> f64 {
        f64::from(self.numerator) * 4.0 / f64::from(self.denominator)
    }

    /// Returns a compact `N/D` label.
    pub fn label(self) -> String {
        format!("{}/{}", self.numerator, self.denominator)
    }

    /// Returns the next preset.
    pub fn next(self) -> Self {
        let index = Self::PRESETS
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(2);
        Self::PRESETS[(index + 1) % Self::PRESETS.len()]
    }

    /// Returns the previous preset.
    pub fn previous(self) -> Self {
        let index = Self::PRESETS
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(2);
        Self::PRESETS[(index + Self::PRESETS.len() - 1) % Self::PRESETS.len()]
    }
}

/// Tonal context used by the recommendation engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TonalContext {
    pub tonic: PitchClass,
    pub mode: ScaleMode,
}

impl TonalContext {
    /// Builds a diatonic chord on `degree` where 0 is scale degree I.
    pub fn diatonic_chord(self, degree: usize, extension: ChordExtension) -> Chord {
        let scale = self.mode.intervals();
        let root_interval = scale[degree % 7];

        let third_index = (degree + 2) % 7;
        let fifth_index = (degree + 4) % 7;
        let seventh_index = (degree + 6) % 7;

        let third = interval_above(scale[third_index], root_interval, degree + 2 >= 7);
        let fifth = interval_above(scale[fifth_index], root_interval, degree + 4 >= 7);
        let seventh = interval_above(scale[seventh_index], root_interval, degree + 6 >= 7);

        let quality = match (third, fifth, seventh) {
            (4, 7, 10) => ChordQuality::Dominant,
            (4, 7, _) => ChordQuality::Major,
            (3, 6, 10) => ChordQuality::HalfDiminished,
            (3, 6, 9) => ChordQuality::Diminished,
            _ => ChordQuality::Minor,
        };

        Chord::new(self.tonic.transpose(root_interval), quality, extension)
    }

    /// Returns the tonic seventh chord for the current mode.
    pub fn tonic_chord(self) -> Chord {
        self.diatonic_chord(0, ChordExtension::Seventh)
    }

    /// Returns a context-aware compact spelling for a pitch class.
    ///
    /// Diatonic notes follow the key's broad sharp/flat tendency. Chromatic
    /// b2/b3/b6/b7 relationships prefer flats, which keeps common borrowed
    /// harmony readable (e.g. Ab/Eb/Bb in C rather than G#/D#/A#).
    pub fn pitch_label(self, pitch: PitchClass) -> &'static str {
        let relative = (12 + pitch.value() - self.tonic.value()) % 12;
        let diatonic = self
            .mode
            .intervals()
            .iter()
            .any(|interval| self.tonic.transpose(*interval) == pitch);

        let key_prefers_flats = matches!(self.tonic.value(), 1 | 3 | 5 | 8 | 10);

        if diatonic {
            if key_prefers_flats {
                pitch.flat_label()
            } else {
                pitch.label()
            }
        } else if matches!(relative, 1 | 3 | 8 | 10) {
            pitch.flat_label()
        } else {
            pitch.label()
        }
    }

    /// Returns a chord symbol using tonal-context pitch spelling.
    pub fn chord_symbol(self, chord: Chord) -> String {
        format!("{}{}", self.pitch_label(chord.root), chord.suffix())
    }

    /// Returns a context-aware MIDI note label where MIDI 60 is C4.
    pub fn midi_note_label(self, note: u8) -> String {
        let pitch = PitchClass::from_value(note % 12);
        let octave = i16::from(note / 12) - 1;
        format!("{}{}", self.pitch_label(pitch), octave)
    }
}

fn interval_above(note_interval: u8, root_interval: u8, wrapped: bool) -> u8 {
    let note = if wrapped {
        note_interval + 12
    } else {
        note_interval
    };
    note - root_interval
}

/// Editable musical settings for Khaṇa.
#[derive(Clone, Copy, Debug)]
pub struct MusicalSettings {
    pub tonal: TonalContext,
    pub tempo_bpm: f64,
    pub meter: TimeSignature,
}

impl Default for MusicalSettings {
    fn default() -> Self {
        Self {
            tonal: TonalContext {
                tonic: PitchClass::from_value(0),
                mode: ScaleMode::Ionian,
            },
            tempo_bpm: 120.0,
            meter: TimeSignature::new(4, 4),
        }
    }
}

/// One top-bar field being edited.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingField {
    Tonic,
    Mode,
    Tempo,
    Meter,
    Sync,
    StartStopSync,
}

impl SettingField {
    pub const ALL: [Self; 6] = [
        Self::Tonic,
        Self::Mode,
        Self::Tempo,
        Self::Meter,
        Self::Sync,
        Self::StartStopSync,
    ];

    /// Moves focus one field to the right.
    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    /// Moves focus one field to the left.
    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::{ScaleMode, TimeSignature, TonalContext};
    use crate::music::chord::{ChordExtension, ChordQuality, PitchClass};

    #[test]
    fn ionian_builds_a_dominant_on_degree_five() {
        let context = TonalContext {
            tonic: PitchClass::from_value(0),
            mode: ScaleMode::Ionian,
        };
        let chord = context.diatonic_chord(4, ChordExtension::Seventh);

        assert_eq!(chord.root, PitchClass::from_value(7));
        assert_eq!(chord.quality, ChordQuality::Dominant);
    }

    #[test]
    fn six_eight_is_three_quarter_note_beats_per_bar() {
        assert_eq!(TimeSignature::new(6, 8).beats_per_bar(), 3.0);
    }

    #[test]
    fn c_context_spells_common_borrowed_roots_with_flats() {
        let context = TonalContext {
            tonic: PitchClass::from_value(0),
            mode: ScaleMode::Ionian,
        };

        assert_eq!(context.pitch_label(PitchClass::from_value(3)), "EB");
        assert_eq!(context.pitch_label(PitchClass::from_value(8)), "AB");
        assert_eq!(context.pitch_label(PitchClass::from_value(10)), "BB");
    }

    #[test]
    fn context_midi_note_labels_include_octave() {
        let context = TonalContext {
            tonic: PitchClass::from_value(0),
            mode: ScaleMode::Ionian,
        };

        assert_eq!(context.midi_note_label(60), "C4");
        assert_eq!(context.midi_note_label(68), "AB4");
    }

    #[test]
    fn every_modal_diatonic_seventh_matches_stacked_scale_tones() {
        for mode in ScaleMode::ALL {
            let context = TonalContext {
                tonic: PitchClass::from_value(0),
                mode,
            };
            let scale = mode.intervals();

            for degree in 0..7 {
                let chord = context.diatonic_chord(degree, ChordExtension::Seventh);
                let root = scale[degree];

                let expected = [0_usize, 2, 4, 6].map(|offset| {
                    let index = degree + offset;
                    let wrapped_note = scale[index % 7] + if index >= 7 { 12 } else { 0 };
                    (wrapped_note - root) % 12
                });

                let actual = chord
                    .intervals()
                    .into_iter()
                    .take(4)
                    .map(|interval| interval % 12)
                    .collect::<Vec<_>>();

                assert_eq!(
                    actual,
                    expected.to_vec(),
                    "mode {:?}, degree {} produced {}",
                    mode,
                    degree + 1,
                    chord.symbol()
                );
            }
        }
    }
}
