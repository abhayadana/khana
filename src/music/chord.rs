//! Chord representation and chord-tone construction.
//!
//! The model is intentionally compact for the MVP. It borrows the proven
//! Chorder distinction between chord identity and later voicing transforms.

use std::fmt;

/// One of the twelve chromatic pitch classes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PitchClass(u8);

impl PitchClass {
    pub const C: Self = Self(0);
    pub const C_SHARP: Self = Self(1);
    pub const D: Self = Self(2);
    pub const D_SHARP: Self = Self(3);
    pub const E: Self = Self(4);
    pub const F: Self = Self(5);
    pub const F_SHARP: Self = Self(6);
    pub const G: Self = Self(7);
    pub const G_SHARP: Self = Self(8);
    pub const A: Self = Self(9);
    pub const A_SHARP: Self = Self(10);
    pub const B: Self = Self(11);

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
}

/// Chord quality independent of extension.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
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
    ///
    /// The current vocabulary covers triads, sevenths, and ninths. More
    /// modifiers from Chorder (sus2/sus4, inversions, quartal/quintal, etc.)
    /// can be added without changing the recommendation API.
    pub fn intervals(self) -> Vec<u8> {
        use ChordExtension::{Ninth, Seventh, Triad};
        use ChordQuality::{Diminished, Dominant, Major, Minor};

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
        }
    }

    /// Applies the live Color macro without mutating the underlying chord.
    ///
    /// Low color preserves the written extension; higher values may enrich a
    /// simpler chord to a seventh or ninth. This keeps phrase data semantic
    /// while allowing performance color to alter its realization.
    pub fn colored(self, color: f32) -> Self {
        let extension = match self.extension {
            ChordExtension::Triad if color >= 0.72 => ChordExtension::Ninth,
            ChordExtension::Triad if color >= 0.34 => ChordExtension::Seventh,
            ChordExtension::Seventh if color >= 0.68 => ChordExtension::Ninth,
            other => other,
        };

        Self { extension, ..self }
    }

    /// Returns a compact chord symbol suitable for the small UI.
    pub fn symbol(self) -> String {
        let quality = match self.quality {
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
        };

        format!("{}{}", self.root.label(), quality)
    }

    /// Returns true when the MIDI pitch belongs to this chord realization.
    pub fn contains_midi_pitch(self, midi_note: u8) -> bool {
        let pitch_class = midi_note % 12;

        self.intervals()
            .iter()
            .any(|interval| (self.root.value() + interval) % 12 == pitch_class)
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
    fn major_ninth_contains_expected_intervals() {
        let chord = Chord::new(PitchClass::C, ChordQuality::Major, ChordExtension::Ninth);

        assert_eq!(chord.intervals(), vec![0, 4, 7, 11, 14]);
    }

    #[test]
    fn color_can_enrich_a_triad_without_changing_identity_fields() {
        let chord = Chord::new(PitchClass::D, ChordQuality::Minor, ChordExtension::Triad);

        let colored = chord.colored(0.8);

        assert_eq!(colored.root, PitchClass::D);
        assert_eq!(colored.quality, ChordQuality::Minor);
        assert_eq!(colored.extension, ChordExtension::Ninth);
    }

    #[test]
    fn chord_symbol_is_compact() {
        let chord = Chord::new(
            PitchClass::G,
            ChordQuality::Dominant,
            ChordExtension::Seventh,
        );

        assert_eq!(chord.symbol(), "G7");
    }
}
