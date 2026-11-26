//! Four performer states, macro handling, and chord-voicing realization.

use crate::music::chord::Chord;
use crate::music::voicing::{Voicing, VoicingEngine};

/// One of Khaṇa's four performer roles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceRole {
    Bass,
    Chords,
    Texture,
    Melody,
}

impl VoiceRole {
    /// Compact UI label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bass => "BASS",
            Self::Chords => "CHORDS",
            Self::Texture => "TEXTURE",
            Self::Melody => "MELODY",
        }
    }
}

/// Live parameters for one performer.
#[derive(Clone, Copy, Debug)]
pub struct VoiceParameters {
    pub density: f32,
    pub dynamics: f32,
    pub spread: f32,
    pub color: f32,
}

/// One MIDI performer.
#[derive(Clone, Copy, Debug)]
pub struct Voice {
    pub role: VoiceRole,
    pub parameters: VoiceParameters,
}

/// Scope for a live macro change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceScope {
    All,
    One(usize),
}

impl VoiceScope {
    /// Compact scope label.
    pub fn label(self) -> String {
        match self {
            Self::All => "ALL".to_owned(),
            Self::One(index) => format!("{}", index + 1),
        }
    }

    /// Selects the next voice scope.
    pub const fn next(self) -> Self {
        match self {
            Self::All => Self::One(0),
            Self::One(0) => Self::One(1),
            Self::One(1) => Self::One(2),
            Self::One(2) => Self::One(3),
            Self::One(_) => Self::All,
        }
    }

    /// Selects the previous voice scope.
    pub const fn previous(self) -> Self {
        match self {
            Self::All => Self::One(3),
            Self::One(0) => Self::All,
            Self::One(1) => Self::One(0),
            Self::One(2) => Self::One(1),
            Self::One(_) => Self::One(2),
        }
    }
}

/// Which live macro is being edited.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroKind {
    Density,
    Dynamics,
    Spread,
    Color,
}

/// Smooth polyphonic voicing used by the Chords performer and as a register
/// reference for the other performers.
#[derive(Clone, Copy, Debug)]
pub struct VoiceRender {
    pub voicing: Voicing,
}

/// Converts harmonic state into a smooth upper chord voicing.
pub struct VoiceEngine {
    voicing_engine: VoicingEngine,
    current_voicing: Option<Voicing>,
}

impl VoiceEngine {
    /// Creates an empty chord-voicing engine.
    pub const fn new() -> Self {
        Self {
            voicing_engine: VoicingEngine::new(),
            current_voicing: None,
        }
    }

    /// Realizes the Chords performer voicing for a new harmonic event.
    pub fn harmonic_change(&mut self, chord: Chord, voices: &[Voice; 4]) -> VoiceRender {
        self.render(chord, voices)
    }

    /// Revoices the current harmony without creating a new harmonic event.
    pub fn revoice(&mut self, chord: Chord, voices: &[Voice; 4]) -> VoiceRender {
        self.render(chord, voices)
    }

    fn render(&mut self, chord: Chord, voices: &[Voice; 4]) -> VoiceRender {
        let chord_spread = voices[1].parameters.spread;
        let bass_present = voices[0].parameters.density > 0.01;
        let voicing = self.voicing_engine.realize_with_bass(
            chord,
            self.current_voicing,
            chord_spread,
            bass_present,
        );

        self.current_voicing = Some(voicing);
        VoiceRender { voicing }
    }
}

impl Default for VoiceEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates the four default performer profiles.
pub fn default_voices() -> [Voice; 4] {
    [
        Voice {
            role: VoiceRole::Bass,
            parameters: VoiceParameters {
                density: 0.56,
                dynamics: 0.72,
                spread: 0.58,
                color: 0.20,
            },
        },
        Voice {
            role: VoiceRole::Chords,
            parameters: VoiceParameters {
                density: 0.52,
                dynamics: 0.62,
                spread: 0.48,
                color: 0.34,
            },
        },
        Voice {
            role: VoiceRole::Texture,
            parameters: VoiceParameters {
                density: 0.40,
                dynamics: 0.50,
                spread: 0.68,
                color: 0.72,
            },
        },
        Voice {
            role: VoiceRole::Melody,
            parameters: VoiceParameters {
                density: 0.62,
                dynamics: 0.70,
                spread: 0.80,
                color: 0.82,
            },
        },
    ]
}

/// Applies a macro delta to the chosen scope.
pub fn adjust_macro(voices: &mut [Voice; 4], scope: VoiceScope, kind: MacroKind, delta: f32) {
    match scope {
        VoiceScope::All => {
            for voice in voices {
                adjust_one(voice, kind, delta);
            }
        }
        VoiceScope::One(index) => {
            if let Some(voice) = voices.get_mut(index) {
                adjust_one(voice, kind, delta);
            }
        }
    }
}

fn adjust_one(voice: &mut Voice, kind: MacroKind, delta: f32) {
    let value = match kind {
        MacroKind::Density => &mut voice.parameters.density,
        MacroKind::Dynamics => &mut voice.parameters.dynamics,
        MacroKind::Spread => &mut voice.parameters.spread,
        MacroKind::Color => &mut voice.parameters.color,
    };

    *value = (*value + delta).clamp(0.0, 1.0);
}

#[cfg(test)]
mod tests {
    use super::{VoiceRole, default_voices};

    #[test]
    fn default_ensemble_is_bass_chords_texture_melody() {
        let voices = default_voices();
        assert_eq!(voices[0].role, VoiceRole::Bass);
        assert_eq!(voices[1].role, VoiceRole::Chords);
        assert_eq!(voices[2].role, VoiceRole::Texture);
        assert_eq!(voices[3].role, VoiceRole::Melody);
    }
}
