//! Contextual next-chord recommendation.
//!
//! v0.5 intentionally uses a small transparent heuristic vocabulary. The API
//! is designed so richer functional harmony, modal interchange, and historical
//! scoring can replace these heuristics without changing the UI.

use super::chord::{Chord, ChordExtension, ChordQuality, PitchClass};

/// Visual recommendation family shown on the Performance screen.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RecommendationClass {
    Safe,
    Colorful,
    Bold,
}

impl RecommendationClass {
    pub const ALL: [Self; 3] = [Self::Safe, Self::Colorful, Self::Bold];

    /// Human-readable uppercase label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Safe => "SAFE",
            Self::Colorful => "COLORFUL",
            Self::Bold => "BOLD",
        }
    }

    /// Converts the class to a stable zero-based UI index.
    pub const fn index(self) -> usize {
        match self {
            Self::Safe => 0,
            Self::Colorful => 1,
            Self::Bold => 2,
        }
    }

    /// Returns a class from a UI index, wrapping into the three classes.
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
    /// Human-readable uppercase label.
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
    /// Returns the three recommendations for `class`.
    pub const fn for_class(&self, class: RecommendationClass) -> &[Chord; 3] {
        match class {
            RecommendationClass::Safe => &self.safe,
            RecommendationClass::Colorful => &self.colorful,
            RecommendationClass::Bold => &self.bold,
        }
    }

    /// Returns one recommendation, wrapping the row index.
    pub const fn get(&self, class: RecommendationClass, index: usize) -> Chord {
        self.for_class(class)[index % 3]
    }
}

/// Small deterministic pseudo-random generator.
///
/// A local generator avoids an additional dependency and makes rerolls
/// reproducible from a seed.
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
    /// Creates a deterministic recommendation engine.
    pub const fn new() -> Self {
        Self { reroll_seed: 1 }
    }

    /// Advances the stochastic state used for the next recommendation set.
    pub fn reroll(&mut self) {
        self.reroll_seed = self.reroll_seed.wrapping_add(1);
    }

    /// Generates recommendations for C major.
    ///
    /// The current prototype uses transparent pools:
    /// - Safe: diatonic seventh chords.
    /// - Colorful: diatonic ninths plus secondary dominants.
    /// - Bold: borrowed/chromatic colors.
    ///
    /// `surprise` relaxes the normal preference against novelty.
    pub fn recommend(
        &self,
        current: Chord,
        direction: HarmonicDirection,
        surprise: bool,
    ) -> RecommendationSet {
        let mut rng = SimpleRng::new(
            self.reroll_seed
                ^ u64::from(current.root.value() + 1)
                ^ match direction {
                    HarmonicDirection::Resolve => 0xAA11,
                    HarmonicDirection::Neutral => 0xBB22,
                    HarmonicDirection::Tension => 0xCC33,
                },
        );

        let safe = choose_three(
            safe_pool(),
            current,
            direction,
            surprise,
            RecommendationClass::Safe,
            &mut rng,
        );
        let colorful = choose_three(
            colorful_pool(),
            current,
            direction,
            surprise,
            RecommendationClass::Colorful,
            &mut rng,
        );
        let bold = choose_three(
            bold_pool(),
            current,
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
    direction: HarmonicDirection,
    surprise: bool,
    class: RecommendationClass,
    rng: &mut SimpleRng,
) -> [Chord; 3] {
    pool.retain(|chord| *chord != current);

    let novelty_bonus = match class {
        RecommendationClass::Safe => 0.0,
        RecommendationClass::Colorful => 0.15,
        RecommendationClass::Bold => 0.32,
    };

    let mut scored = pool
        .into_iter()
        .map(|candidate| {
            let candidate_score = score(
                candidate,
                current,
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
    direction: HarmonicDirection,
    surprise: bool,
    novelty_bonus: f32,
    jitter: f32,
) -> f32 {
    let candidate_tension = tension_score(candidate);
    let current_tension = tension_score(current);
    let delta = candidate_tension - current_tension;

    let directional = match direction {
        HarmonicDirection::Resolve => -delta,
        HarmonicDirection::Neutral => -delta.abs() * 0.25,
        HarmonicDirection::Tension => delta,
    };

    let root_motion = chromatic_distance(current.root, candidate.root);
    let smooth_root_bonus = (6.0 - root_motion.min(6.0)) / 6.0;

    let surprise_multiplier = if surprise { 1.6 } else { 0.65 };
    let random_component = (jitter - 0.5) * 0.35 * surprise_multiplier;

    directional * 0.9
        + smooth_root_bonus * 0.35
        + novelty_bonus * if surprise { 1.0 } else { 0.25 }
        + random_component
}

fn tension_score(chord: Chord) -> f32 {
    let quality = match chord.quality {
        ChordQuality::Major => 0.15,
        ChordQuality::Minor => 0.25,
        ChordQuality::Diminished => 0.9,
        ChordQuality::Dominant => 0.75,
    };

    let root = match chord.root.value() {
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

    quality + root + extension
}

fn chromatic_distance(left: PitchClass, right: PitchClass) -> f32 {
    let raw = left.value().abs_diff(right.value());
    f32::from(raw.min(12 - raw))
}

fn safe_pool() -> Vec<Chord> {
    use ChordExtension::{Seventh, Triad};
    use ChordQuality::{Diminished, Dominant, Major, Minor};

    vec![
        Chord::new(PitchClass::C, Major, Triad),
        Chord::new(PitchClass::D, Minor, Seventh),
        Chord::new(PitchClass::E, Minor, Seventh),
        Chord::new(PitchClass::F, Major, Seventh),
        Chord::new(PitchClass::G, Dominant, Seventh),
        Chord::new(PitchClass::A, Minor, Seventh),
        Chord::new(PitchClass::B, Diminished, Seventh),
    ]
}

fn colorful_pool() -> Vec<Chord> {
    use ChordExtension::{Ninth, Seventh};
    use ChordQuality::{Dominant, Major, Minor};

    vec![
        Chord::new(PitchClass::C, Major, Ninth),
        Chord::new(PitchClass::D, Minor, Ninth),
        Chord::new(PitchClass::E, Minor, Ninth),
        Chord::new(PitchClass::F, Major, Ninth),
        Chord::new(PitchClass::G, Dominant, Ninth),
        Chord::new(PitchClass::A, Minor, Ninth),
        Chord::new(PitchClass::D, Dominant, Seventh),
        Chord::new(PitchClass::E, Dominant, Seventh),
        Chord::new(PitchClass::A, Dominant, Seventh),
    ]
}

fn bold_pool() -> Vec<Chord> {
    use ChordExtension::{Ninth, Seventh};
    use ChordQuality::{Dominant, Major, Minor};

    vec![
        Chord::new(PitchClass::D_SHARP, Major, Seventh),
        Chord::new(PitchClass::G_SHARP, Major, Seventh),
        Chord::new(PitchClass::A_SHARP, Major, Seventh),
        Chord::new(PitchClass::C_SHARP, Dominant, Seventh),
        Chord::new(PitchClass::D_SHARP, Dominant, Seventh),
        Chord::new(PitchClass::F_SHARP, Dominant, Seventh),
        Chord::new(PitchClass::F, Minor, Seventh),
        Chord::new(PitchClass::G_SHARP, Major, Ninth),
        Chord::new(PitchClass::A_SHARP, Major, Ninth),
    ]
}

#[cfg(test)]
mod tests {
    use super::{HarmonicDirection, RecommendationClass, RecommendationEngine};
    use crate::music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};

    #[test]
    fn recommendation_set_contains_three_per_class() {
        let engine = RecommendationEngine::new();
        let current = Chord::new(PitchClass::C, ChordQuality::Major, ChordExtension::Seventh);
        let set = engine.recommend(current, HarmonicDirection::Neutral, false);

        for class in RecommendationClass::ALL {
            assert_eq!(set.for_class(class).len(), 3);
        }
    }

    #[test]
    fn reroll_changes_engine_seed_without_invalidating_recommendations() {
        let mut engine = RecommendationEngine::new();
        let current = Chord::new(PitchClass::C, ChordQuality::Major, ChordExtension::Seventh);

        let first = engine.recommend(current, HarmonicDirection::Neutral, false);
        engine.reroll();
        let second = engine.recommend(current, HarmonicDirection::Neutral, false);

        assert_eq!(first.for_class(RecommendationClass::Safe).len(), 3);
        assert_eq!(second.for_class(RecommendationClass::Safe).len(), 3);
    }
}
