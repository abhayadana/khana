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
            (3, 6, _) => ChordQuality::Diminished,
            _ => ChordQuality::Minor,
        };

        Chord::new(self.tonic.transpose(root_interval), quality, extension)
    }

    /// Returns the tonic seventh chord for the current mode.
    pub fn tonic_chord(self) -> Chord {
        self.diatonic_chord(0, ChordExtension::Seventh)
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
}

impl SettingField {
    pub const ALL: [Self; 4] = [Self::Tonic, Self::Mode, Self::Tempo, Self::Meter];

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
}
