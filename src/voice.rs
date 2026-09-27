//! Four performer states, macro handling, and MIDI realization.

use crate::music::chord::Chord;
use crate::music::voicing::{Voicing, VoicingEngine};

/// One of Khaṇa's four performer roles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceRole {
    Bass,
    Inner,
    Texture,
    Melody,
}

impl VoiceRole {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bass => "BASS",
            Self::Inner => "INNER",
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
    pub fn label(self) -> String {
        match self {
            Self::All => "ALL".to_owned(),
            Self::One(index) => format!("{}", index + 1),
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::All => Self::One(0),
            Self::One(0) => Self::One(1),
            Self::One(1) => Self::One(2),
            Self::One(2) => Self::One(3),
            Self::One(_) => Self::All,
        }
    }

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

/// Fully realized voice state ready for MIDI output and display.
#[derive(Clone, Copy, Debug)]
pub struct VoiceRender {
    pub realized_chord: Chord,
    pub voicing: Voicing,
    pub velocities: [u8; 4],
}

/// Converts harmonic state into four performer outputs.
pub struct VoiceEngine {
    voicing_engine: VoicingEngine,
    current_voicing: Option<Voicing>,
}

impl VoiceEngine {
    pub const fn new() -> Self {
        Self {
            voicing_engine: VoicingEngine::new(),
            current_voicing: None,
        }
    }

    /// Realizes a new harmonic event.
    ///
    /// In the sustained-chord prototype all four performers remain present.
    /// Density is reserved for the upcoming rhythmic performer engine and does
    /// not mute a voice at chord boundaries.
    pub fn harmonic_change(&mut self, chord: Chord, voices: &[Voice; 4]) -> VoiceRender {
        self.render(chord, voices)
    }

    /// Revoices the current harmony without creating a new harmonic event.
    pub fn revoice(&mut self, chord: Chord, voices: &[Voice; 4]) -> VoiceRender {
        self.render(chord, voices)
    }

    fn render(&mut self, chord: Chord, voices: &[Voice; 4]) -> VoiceRender {
        let color = average_color(voices);
        let spread = average_spread(voices);
        let realized_chord = chord.colored(color);
        let voicing = self
            .voicing_engine
            .realize(realized_chord, self.current_voicing, spread);

        let velocities =
            std::array::from_fn(|index| velocity_from_dynamics(voices[index].parameters.dynamics));

        self.current_voicing = Some(voicing);

        VoiceRender {
            realized_chord,
            voicing,
            velocities,
        }
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
                density: 0.72,
                dynamics: 0.72,
                spread: 0.72,
                color: 0.20,
            },
        },
        Voice {
            role: VoiceRole::Inner,
            parameters: VoiceParameters {
                density: 0.56,
                dynamics: 0.60,
                spread: 0.42,
                color: 0.42,
            },
        },
        Voice {
            role: VoiceRole::Texture,
            parameters: VoiceParameters {
                density: 0.44,
                dynamics: 0.52,
                spread: 0.62,
                color: 0.72,
            },
        },
        Voice {
            role: VoiceRole::Melody,
            parameters: VoiceParameters {
                density: 0.66,
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

fn average_spread(voices: &[Voice; 4]) -> f32 {
    voices
        .iter()
        .map(|voice| voice.parameters.spread)
        .sum::<f32>()
        / voices.len() as f32
}

fn average_color(voices: &[Voice; 4]) -> f32 {
    voices
        .iter()
        .map(|voice| voice.parameters.color)
        .sum::<f32>()
        / voices.len() as f32
}

fn velocity_from_dynamics(dynamics: f32) -> u8 {
    (35.0 + dynamics.clamp(0.0, 1.0) * 92.0).round() as u8
}
