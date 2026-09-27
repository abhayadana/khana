//! Chord representation and compact pitch naming.

use std::fmt;

/// One of the twelve chromatic pitch classes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PitchClass(u8);

impl PitchClass {
    /// Creates a pitch class by wrapping a semitone value into 0..=11.
    pub const fn from_value(value: u8) -> Self {
        Self(value % 12)
    }

    /// Returns the chromatic semitone value in 0..=11.
    pub const fn value(self) -> u8 {
        self.0
    }

    /// Returns a compact uppercase pitch name.
    pub const fn label(self) -> &'static str {
        match self.0 {
            0 => "C",
            1 => "C#",
            2 => "D",
            3 => "D#",
            4 => "E",
            5 => "F",
            6 => "F#",
            7 => "G",
            8 => "G#",
            9 => "A",
            10 => "BB",
            _ => "B",
        }
    }

    /// Returns a compact flat-oriented pitch name.
    pub const fn flat_label(self) -> &'static str {
        match self.0 {
            0 => "C",
            1 => "DB",
            2 => "D",
            3 => "EB",
            4 => "E",
            5 => "F",
            6 => "GB",
            7 => "G",
            8 => "AB",
            9 => "A",
            10 => "BB",
            _ => "B",
        }
    }

    /// Returns this pitch class transposed upward by `semitones`.
    pub const fn transpose(self, semitones: u8) -> Self {
        Self::from_value(self.0 + semitones)
    }
}

/// Chord quality independent of extension.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
    HalfDiminished,
    Dominant,
}

/// Amount of chordal color encoded directly in the chord identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChordExtension {
    Triad,
    Seventh,
    Ninth,
}

/// A chord identity before register-specific voicing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Chord {
    pub root: PitchClass,
    pub quality: ChordQuality,
    pub extension: ChordExtension,
}

impl Chord {
    /// Creates a chord identity.
    pub const fn new(root: PitchClass, quality: ChordQuality, extension: ChordExtension) -> Self {
        Self {
            root,
            quality,
            extension,
        }
    }

    /// Returns semitone intervals from the root used to realize this chord.
    pub fn intervals(self) -> Vec<u8> {
        use ChordExtension::{Ninth, Seventh, Triad};
        use ChordQuality::{Diminished, Dominant, HalfDiminished, Major, Minor};

        match (self.quality, self.extension) {
            (Major, Triad) => vec![0, 4, 7],
            (Major, Seventh) => vec![0, 4, 7, 11],
            (Major, Ninth) => vec![0, 4, 7, 11, 14],
            (Minor, Triad) => vec![0, 3, 7],
            (Minor, Seventh) => vec![0, 3, 7, 10],
            (Minor, Ninth) => vec![0, 3, 7, 10, 14],
            (Dominant, Triad) => vec![0, 4, 7],
            (Dominant, Seventh) => vec![0, 4, 7, 10],
            (Dominant, Ninth) => vec![0, 4, 7, 10, 14],
            (Diminished, Triad) => vec![0, 3, 6],
            (Diminished, Seventh) => vec![0, 3, 6, 9],
            (Diminished, Ninth) => vec![0, 3, 6, 9, 14],
            (HalfDiminished, Triad) => vec![0, 3, 6],
            (HalfDiminished, Seventh) => vec![0, 3, 6, 10],
            (HalfDiminished, Ninth) => vec![0, 3, 6, 10, 14],
        }
    }

    /// Returns the chord-quality / extension suffix without its root.
    pub const fn suffix(self) -> &'static str {
        match self.quality {
            ChordQuality::Major => match self.extension {
                ChordExtension::Triad => "",
                ChordExtension::Seventh => "MAJ7",
                ChordExtension::Ninth => "MAJ9",
            },
            ChordQuality::Minor => match self.extension {
                ChordExtension::Triad => "M",
                ChordExtension::Seventh => "M7",
                ChordExtension::Ninth => "M9",
            },
            ChordQuality::Dominant => match self.extension {
                ChordExtension::Triad => "",
                ChordExtension::Seventh => "7",
                ChordExtension::Ninth => "9",
            },
            ChordQuality::Diminished => match self.extension {
                ChordExtension::Triad => "DIM",
                ChordExtension::Seventh => "DIM7",
                ChordExtension::Ninth => "DIM9",
            },
            ChordQuality::HalfDiminished => match self.extension {
                ChordExtension::Triad => "DIM",
                ChordExtension::Seventh => "M7B5",
                ChordExtension::Ninth => "M9B5",
            },
        }
    }

    /// Returns a compact chord symbol using the default sharp-oriented root.
    pub fn symbol(self) -> String {
        format!("{}{}", self.root.label(), self.suffix())
    }

    /// Returns true when `midi_note` belongs to this chord realization.
    pub fn contains_midi_pitch(self, midi_note: u8) -> bool {
        let pitch_class = midi_note % 12;

        self.intervals()
            .iter()
            .any(|interval| (self.root.value() + interval) % 12 == pitch_class)
    }

    /// Returns the pitch classes that a four-voice realization must contain.
    ///
    /// Triads retain root/third/fifth and may double one tone. Sevenths retain
    /// all four chord tones. Ninth chords retain root/third/seventh/ninth and
    /// omit the fifth by default, a conventional four-voice reduction.
    pub fn essential_pitch_classes(self) -> Vec<u8> {
        let intervals = self.intervals();

        let essential_intervals: Vec<u8> = match self.extension {
            ChordExtension::Triad => intervals[..3].to_vec(),
            ChordExtension::Seventh => intervals[..4].to_vec(),
            ChordExtension::Ninth => {
                vec![intervals[0], intervals[1], intervals[3], intervals[4]]
            }
        };

        essential_intervals
            .into_iter()
            .map(|interval| (self.root.value() + interval) % 12)
            .collect()
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.symbol())
    }
}

#[cfg(test)]
mod tests {
    use super::{Chord, ChordExtension, ChordQuality, PitchClass};

    #[test]
    fn pitch_classes_transpose_with_wraparound() {
        assert_eq!(
            PitchClass::from_value(11).transpose(2),
            PitchClass::from_value(1)
        );
    }

    #[test]
    fn major_ninth_contains_expected_intervals() {
        let chord = Chord::new(
            PitchClass::from_value(0),
            ChordQuality::Major,
            ChordExtension::Ninth,
        );

        assert_eq!(chord.intervals(), vec![0, 4, 7, 11, 14]);
    }

    #[test]
    fn half_diminished_seventh_has_minor_seventh_not_diminished_seventh() {
        let chord = Chord::new(
            PitchClass::from_value(11),
            ChordQuality::HalfDiminished,
            ChordExtension::Seventh,
        );

        assert_eq!(chord.intervals(), vec![0, 3, 6, 10]);
    }
}
