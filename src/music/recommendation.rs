//! Contextual next-chord recommendation.
//!
//! v0.6 keeps the scoring transparent while making the pools key/mode-aware.
//! Reroll changes ordering inside the ordinary vocabulary. Surprise broadens
//! the available vocabulary before ranking it.

use super::chord::{Chord, ChordExtension, ChordQuality, PitchClass};
use super::settings::TonalContext;

/// Visual recommendation family shown on the Performance screen.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RecommendationClass {
    Safe,
    Colorful,
    Bold,
}

impl RecommendationClass {
    pub const ALL: [Self; 3] = [Self::Safe, Self::Colorful, Self::Bold];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Safe => "SAFE",
            Self::Colorful => "COLORFUL",
            Self::Bold => "BOLD",
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Safe => 0,
            Self::Colorful => 1,
            Self::Bold => 2,
        }
    }

    pub const fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }
}

/// Directional intent independent of harmonic novelty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HarmonicDirection {
    Resolve,
    Neutral,
    Tension,
}

impl HarmonicDirection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Resolve => "RESOLVE",
            Self::Neutral => "NEUTRAL",
            Self::Tension => "TENSION",
        }
    }
}

/// Three recommendations for each visible class.
#[derive(Clone, Debug)]
pub struct RecommendationSet {
    safe: [Chord; 3],
    colorful: [Chord; 3],
    bold: [Chord; 3],
}

impl RecommendationSet {
    pub const fn for_class(&self, class: RecommendationClass) -> &[Chord; 3] {
        match class {
            RecommendationClass::Safe => &self.safe,
            RecommendationClass::Colorful => &self.colorful,
            RecommendationClass::Bold => &self.bold,
        }
    }

    pub const fn get(&self, class: RecommendationClass, index: usize) -> Chord {
        self.for_class(class)[index % 3]
    }
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn jitter(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}

/// Generates the current Safe / Colorful / Bold recommendation set.
pub struct RecommendationEngine {
    reroll_seed: u64,
}

impl RecommendationEngine {
    pub const fn new() -> Self {
        Self { reroll_seed: 1 }
    }

    /// Advances stochastic ordering without changing harmonic vocabulary.
    pub fn reroll(&mut self) {
        self.reroll_seed = self.reroll_seed.wrapping_add(1);
    }

    /// Generates recommendations for the current tonal context.
    pub fn recommend(
        &self,
        current: Chord,
        context: TonalContext,
        direction: HarmonicDirection,
        surprise: bool,
    ) -> RecommendationSet {
        let mut rng = SimpleRng::new(
            self.reroll_seed
                ^ u64::from(current.root.value() + 1)
                ^ u64::from(context.tonic.value() + 17)
                ^ match direction {
                    HarmonicDirection::Resolve => 0xAA11,
                    HarmonicDirection::Neutral => 0xBB22,
                    HarmonicDirection::Tension => 0xCC33,
                },
        );

        let safe = choose_three(
            safe_pool(context),
            current,
            context,
            direction,
            false,
            RecommendationClass::Safe,
            &mut rng,
        );

        let mut colorful_pool = colorful_pool(context);
        let mut bold_pool = bold_pool(context);

        if surprise {
            colorful_pool.extend(surprise_pool(context));
            bold_pool.extend(surprise_pool(context));
        }

        let colorful = choose_three(
            colorful_pool,
            current,
            context,
            direction,
            surprise,
            RecommendationClass::Colorful,
            &mut rng,
        );
        let bold = choose_three(
            bold_pool,
            current,
            context,
            direction,
            surprise,
            RecommendationClass::Bold,
            &mut rng,
        );

        RecommendationSet {
            safe,
            colorful,
            bold,
        }
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn choose_three(
    mut pool: Vec<Chord>,
    current: Chord,
    context: TonalContext,
    direction: HarmonicDirection,
    surprise: bool,
    class: RecommendationClass,
    rng: &mut SimpleRng,
) -> [Chord; 3] {
    pool.retain(|chord| *chord != current);
    pool.dedup();

    let novelty_bonus = match class {
        RecommendationClass::Safe => 0.0,
        RecommendationClass::Colorful => 0.16,
        RecommendationClass::Bold => 0.34,
    };

    let mut scored = pool
        .into_iter()
        .map(|candidate| {
            let candidate_score = score(
                candidate,
                current,
                context,
                direction,
                surprise,
                novelty_bonus,
                rng.jitter(),
            );
            (candidate, candidate_score)
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.1.total_cmp(&left.1));

    [scored[0].0, scored[1].0, scored[2].0]
}

fn score(
    candidate: Chord,
    current: Chord,
    context: TonalContext,
    direction: HarmonicDirection,
    surprise: bool,
    novelty_bonus: f32,
    jitter: f32,
) -> f32 {
    let candidate_tension = tension_score(candidate, context);
    let current_tension = tension_score(current, context);
    let delta = candidate_tension - current_tension;

    let directional = match direction {
        HarmonicDirection::Resolve => -delta,
        HarmonicDirection::Neutral => -delta.abs() * 0.25,
        HarmonicDirection::Tension => delta,
    };

    let root_motion = chromatic_distance(current.root, candidate.root);
    let smooth_root_bonus = (6.0 - root_motion.min(6.0)) / 6.0;
    let random_range = if surprise { 0.65 } else { 0.22 };
    let random_component = (jitter - 0.5) * random_range;

    directional * 0.9
        + smooth_root_bonus * 0.35
        + novelty_bonus * if surprise { 0.9 } else { 0.25 }
        + random_component
}

fn tension_score(chord: Chord, context: TonalContext) -> f32 {
    let quality = match chord.quality {
        ChordQuality::Major => 0.15,
        ChordQuality::Minor => 0.25,
        ChordQuality::Diminished => 0.9,
        ChordQuality::Dominant => 0.75,
    };

    let relative = (12 + chord.root.value() - context.tonic.value()) % 12;
    let function = match relative {
        0 => 0.0,
        5 => 0.25,
        7 => 0.6,
        11 => 0.85,
        _ => 0.45,
    };

    let extension = match chord.extension {
        ChordExtension::Triad => 0.0,
        ChordExtension::Seventh => 0.12,
        ChordExtension::Ninth => 0.2,
    };

    quality + function + extension
}

fn chromatic_distance(left: PitchClass, right: PitchClass) -> f32 {
    let raw = left.value().abs_diff(right.value());
    f32::from(raw.min(12 - raw))
}

fn safe_pool(context: TonalContext) -> Vec<Chord> {
    (0..7)
        .map(|degree| {
            let extension = if degree == 0 {
                ChordExtension::Triad
            } else {
                ChordExtension::Seventh
            };
            context.diatonic_chord(degree, extension)
        })
        .collect()
}

fn colorful_pool(context: TonalContext) -> Vec<Chord> {
    let mut pool = (0..7)
        .map(|degree| context.diatonic_chord(degree, ChordExtension::Ninth))
        .collect::<Vec<_>>();

    for degree in 0..7 {
        let target = context.diatonic_chord(degree, ChordExtension::Triad);
        pool.push(Chord::new(
            target.root.transpose(7),
            ChordQuality::Dominant,
            ChordExtension::Seventh,
        ));
    }

    pool
}

fn bold_pool(context: TonalContext) -> Vec<Chord> {
    let tonic = context.tonic;

    vec![
        Chord::new(
            tonic.transpose(3),
            ChordQuality::Major,
            ChordExtension::Seventh,
        ),
        Chord::new(
            tonic.transpose(8),
            ChordQuality::Major,
            ChordExtension::Seventh,
        ),
        Chord::new(
            tonic.transpose(10),
            ChordQuality::Major,
            ChordExtension::Seventh,
        ),
        Chord::new(
            tonic.transpose(1),
            ChordQuality::Dominant,
            ChordExtension::Seventh,
        ),
        Chord::new(
            tonic.transpose(6),
            ChordQuality::Dominant,
            ChordExtension::Seventh,
        ),
        Chord::new(
            tonic.transpose(5),
            ChordQuality::Minor,
            ChordExtension::Seventh,
        ),
    ]
}

fn surprise_pool(context: TonalContext) -> Vec<Chord> {
    let tonic = context.tonic;

    [1_u8, 2, 3, 6, 8, 9, 10, 11]
        .into_iter()
        .flat_map(|offset| {
            [
                Chord::new(
                    tonic.transpose(offset),
                    ChordQuality::Dominant,
                    ChordExtension::Ninth,
                ),
                Chord::new(
                    tonic.transpose(offset),
                    ChordQuality::Major,
                    ChordExtension::Ninth,
                ),
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{HarmonicDirection, RecommendationClass, RecommendationEngine};
    use crate::music::settings::MusicalSettings;

    #[test]
    fn recommendation_set_contains_three_per_class() {
        let settings = MusicalSettings::default();
        let current = settings.tonal.tonic_chord();
        let engine = RecommendationEngine::new();
        let set = engine.recommend(current, settings.tonal, HarmonicDirection::Neutral, false);

        for class in RecommendationClass::ALL {
            assert_eq!(set.for_class(class).len(), 3);
        }
    }
}
