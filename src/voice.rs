//! Four performer states and macro handling.

/// One of Khaṇa's four performer roles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceRole {
    Bass,
    Inner,
    Texture,
    Melody,
}

impl VoiceRole {
    /// Compact uppercase label.
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
    /// Compact label for the performance UI.
    pub fn label(self) -> String {
        match self {
            Self::All => "ALL".to_owned(),
            Self::One(index) => format!("{}", index + 1),
        }
    }

    /// Moves to the next scope in ALL -> 1 -> 2 -> 3 -> 4 -> ALL order.
    pub const fn next(self) -> Self {
        match self {
            Self::All => Self::One(0),
            Self::One(0) => Self::One(1),
            Self::One(1) => Self::One(2),
            Self::One(2) => Self::One(3),
            Self::One(_) => Self::All,
        }
    }

    /// Moves to the previous scope.
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

/// Returns the average spread across the ensemble.
pub fn average_spread(voices: &[Voice; 4]) -> f32 {
    voices
        .iter()
        .map(|voice| voice.parameters.spread)
        .sum::<f32>()
        / voices.len() as f32
}

/// Returns the average color across the ensemble.
pub fn average_color(voices: &[Voice; 4]) -> f32 {
    voices
        .iter()
        .map(|voice| voice.parameters.color)
        .sum::<f32>()
        / voices.len() as f32
}
