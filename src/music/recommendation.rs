//! Progression-context next-chord recommendation.
//!
//! v0.7 separates chord choice from voicing. Candidate chords are generated
//! broadly, scored primarily by progression/function context, classified by
//! contextual novelty, and only then exposed as Safe / Colorful / Bold.
//! Voice-leading contributes only a small chord-choice tie-breaker; detailed
//! voice-leading remains the responsibility of `VoicingEngine`.

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

/// Short explanation of why a recommendation ranked well.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecommendationReason {
    Diatonic,
    ProgressionPrior,
    DominantResolution,
    TwoFive,
    Cadence,
    SecondaryDominant,
    Borrowed,
    ChromaticMediant,
    DiminishedApproach,
    Tritone,
    Surprise,
}

impl RecommendationReason {
    /// Compact all-caps label suitable for the bitmap UI.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Diatonic => "DIATONIC",
            Self::ProgressionPrior => "PROGRESSION",
            Self::DominantResolution => "DOM RESOLVE",
            Self::TwoFive => "II TO V",
            Self::Cadence => "CADENCE",
            Self::SecondaryDominant => "SECONDARY DOM",
            Self::Borrowed => "BORROWED",
            Self::ChromaticMediant => "CHROM MEDIANT",
            Self::DiminishedApproach => "DIM APPROACH",
            Self::Tritone => "TRITONE",
            Self::Surprise => "RARE PATH",
        }
    }
}

/// One ranked recommendation plus diagnostics.
#[derive(Clone, Copy, Debug)]
pub struct Recommendation {
    pub chord: Chord,
    pub reason: RecommendationReason,
}

/// Three recommendations for each visible class.
#[derive(Clone, Debug)]
pub struct RecommendationSet {
    safe: [Recommendation; 3],
    colorful: [Recommendation; 3],
    bold: [Recommendation; 3],
}

impl RecommendationSet {
    pub const fn for_class(&self, class: RecommendationClass) -> &[Recommendation; 3] {
        match class {
            RecommendationClass::Safe => &self.safe,
            RecommendationClass::Colorful => &self.colorful,
            RecommendationClass::Bold => &self.bold,
        }
    }

    pub const fn recommendation(&self, class: RecommendationClass, index: usize) -> Recommendation {
        self.for_class(class)[index % 3]
    }

    pub const fn get(&self, class: RecommendationClass, index: usize) -> Chord {
        self.recommendation(class, index).chord
    }

    pub const fn reason(&self, class: RecommendationClass, index: usize) -> RecommendationReason {
        self.recommendation(class, index).reason
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateOrigin {
    Diatonic,
    SecondaryDominant,
    Borrowed,
    ChromaticMediant,
    DiminishedApproach,
    Tritone,
    Surprise,
}

impl CandidateOrigin {
    const fn novelty(self) -> f32 {
        match self {
            Self::Diatonic => 0.08,
            Self::SecondaryDominant => 0.38,
            Self::Borrowed => 0.50,
            Self::DiminishedApproach => 0.64,
            Self::ChromaticMediant => 0.72,
            Self::Tritone => 0.79,
            Self::Surprise => 0.88,
        }
    }

    const fn prior(self) -> f32 {
        match self {
            Self::Diatonic => 0.90,
            Self::SecondaryDominant => 0.72,
            Self::Borrowed => 0.62,
            Self::DiminishedApproach => 0.48,
            Self::ChromaticMediant => 0.42,
            Self::Tritone => 0.38,
            Self::Surprise => 0.26,
        }
    }

    const fn reason(self) -> RecommendationReason {
        match self {
            Self::Diatonic => RecommendationReason::Diatonic,
            Self::SecondaryDominant => RecommendationReason::SecondaryDominant,
            Self::Borrowed => RecommendationReason::Borrowed,
            Self::ChromaticMediant => RecommendationReason::ChromaticMediant,
            Self::DiminishedApproach => RecommendationReason::DiminishedApproach,
            Self::Tritone => RecommendationReason::Tritone,
            Self::Surprise => RecommendationReason::Surprise,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    chord: Chord,
    origin: CandidateOrigin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HarmonicFunction {
    Tonic,
    Predominant,
    Dominant,
    Chromatic,
}

#[derive(Clone, Copy, Debug)]
struct ScoredCandidate {
    recommendation: Recommendation,
    class: RecommendationClass,
    score: f32,
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

    /// Advances stochastic ordering without changing novelty thresholds.
    pub fn reroll(&mut self) {
        self.reroll_seed = self.reroll_seed.wrapping_add(1);
    }

    /// Generates recommendations from recent harmonic history.
    ///
    /// `history` is oldest-to-newest and normally contains the last few actual
    /// selected/played chords. When empty, the tonic chord is used as context.
    pub fn recommend(
        &self,
        history: &[Chord],
        context: TonalContext,
        direction: HarmonicDirection,
        surprise: bool,
    ) -> RecommendationSet {
        let current = history
            .last()
            .copied()
            .unwrap_or_else(|| context.tonic_chord());

        let history_hash = history.iter().rev().take(4).fold(0_u64, |state, chord| {
            state
                .wrapping_mul(37)
                .wrapping_add(u64::from(chord.root.value()) + 1)
        });

        let mut rng = SimpleRng::new(
            self.reroll_seed
                ^ history_hash
                ^ u64::from(context.tonic.value() + 17)
                ^ match direction {
                    HarmonicDirection::Resolve => 0xAA11,
                    HarmonicDirection::Neutral => 0xBB22,
                    HarmonicDirection::Tension => 0xCC33,
                },
        );

        let candidates = generate_candidates(current, context, surprise);
        let mut scored = candidates
            .into_iter()
            .filter(|candidate| candidate.chord != current)
            .map(|candidate| {
                score_candidate(
                    candidate,
                    history,
                    context,
                    direction,
                    surprise,
                    rng.jitter(),
                )
            })
            .collect::<Vec<_>>();

        scored.sort_by(|left, right| right.score.total_cmp(&left.score));

        RecommendationSet {
            safe: select_three(&scored, RecommendationClass::Safe, context),
            colorful: select_three(&scored, RecommendationClass::Colorful, context),
            bold: select_three(&scored, RecommendationClass::Bold, context),
        }
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn generate_candidates(current: Chord, context: TonalContext, surprise: bool) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let diatonic_extension = match current.extension {
        ChordExtension::Triad => ChordExtension::Triad,
        ChordExtension::Seventh | ChordExtension::Ninth => ChordExtension::Seventh,
    };

    for degree in 0..7 {
        add_candidate(
            &mut candidates,
            Candidate {
                chord: context.diatonic_chord(degree, diatonic_extension),
                origin: CandidateOrigin::Diatonic,
            },
        );
    }

    // Secondary dominants target each diatonic scale degree.
    for degree in 0..7 {
        let target = context.diatonic_chord(degree, ChordExtension::Triad);
        add_candidate(
            &mut candidates,
            Candidate {
                chord: Chord::new(
                    target.root.transpose(7),
                    ChordQuality::Dominant,
                    ChordExtension::Seventh,
                ),
                origin: CandidateOrigin::SecondaryDominant,
            },
        );
    }

    let tonic = context.tonic;

    // Common modal-interchange / borrowed colors relative to tonic.
    for chord in [
        Chord::new(
            tonic.transpose(3),
            ChordQuality::Major,
            ChordExtension::Seventh,
        ),
        Chord::new(
            tonic.transpose(5),
            ChordQuality::Minor,
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
            ChordQuality::Major,
            ChordExtension::Seventh,
        ),
    ] {
        add_candidate(
            &mut candidates,
            Candidate {
                chord,
                origin: CandidateOrigin::Borrowed,
            },
        );
    }

    // Chromatic mediants relative to the currently sounding root.
    for offset in [3_u8, 4, 8, 9] {
        for quality in [ChordQuality::Major, ChordQuality::Minor] {
            add_candidate(
                &mut candidates,
                Candidate {
                    chord: Chord::new(
                        current.root.transpose(offset),
                        quality,
                        ChordExtension::Seventh,
                    ),
                    origin: CandidateOrigin::ChromaticMediant,
                },
            );
        }
    }

    // Leading-tone diminished approaches into diatonic targets.
    for degree in 0..7 {
        let target = context.diatonic_chord(degree, ChordExtension::Triad);
        add_candidate(
            &mut candidates,
            Candidate {
                chord: Chord::new(
                    target.root.transpose(11),
                    ChordQuality::Diminished,
                    ChordExtension::Seventh,
                ),
                origin: CandidateOrigin::DiminishedApproach,
            },
        );
    }

    // Tritone-related dominant options.
    add_candidate(
        &mut candidates,
        Candidate {
            chord: Chord::new(
                current.root.transpose(6),
                ChordQuality::Dominant,
                ChordExtension::Seventh,
            ),
            origin: CandidateOrigin::Tritone,
        },
    );
    add_candidate(
        &mut candidates,
        Candidate {
            chord: Chord::new(
                tonic.transpose(1),
                ChordQuality::Dominant,
                ChordExtension::Seventh,
            ),
            origin: CandidateOrigin::Tritone,
        },
    );

    if surprise {
        for offset in [1_u8, 2, 6, 8, 10, 11] {
            for quality in [ChordQuality::Major, ChordQuality::Dominant] {
                add_candidate(
                    &mut candidates,
                    Candidate {
                        chord: Chord::new(tonic.transpose(offset), quality, ChordExtension::Ninth),
                        origin: CandidateOrigin::Surprise,
                    },
                );
            }
        }
    }

    candidates
}

fn add_candidate(candidates: &mut Vec<Candidate>, candidate: Candidate) {
    if let Some(existing) = candidates
        .iter_mut()
        .find(|existing| existing.chord == candidate.chord)
    {
        if candidate.origin.novelty() < existing.origin.novelty() {
            *existing = candidate;
        }
        return;
    }

    candidates.push(candidate);
}

fn score_candidate(
    candidate: Candidate,
    history: &[Chord],
    context: TonalContext,
    direction: HarmonicDirection,
    surprise: bool,
    jitter: f32,
) -> ScoredCandidate {
    let current = history
        .last()
        .copied()
        .unwrap_or_else(|| context.tonic_chord());

    let (progression, progression_reason) = progression_score(history, candidate.chord, context);
    let function = functional_coherence(current, candidate.chord, context);
    let directional = directional_fit(current, candidate.chord, context, direction);
    let history_fit = history_fit(history, candidate.chord);
    let chord_motion = chord_choice_voice_leading_hint(current, candidate.chord);
    let origin_prior = candidate.origin.prior();

    let surprise_boost = if surprise
        && matches!(
            candidate.origin,
            CandidateOrigin::Borrowed
                | CandidateOrigin::ChromaticMediant
                | CandidateOrigin::DiminishedApproach
                | CandidateOrigin::Tritone
                | CandidateOrigin::Surprise
        ) {
        0.08
    } else {
        0.0
    };

    let random_range = if surprise { 0.07 } else { 0.025 };
    let random_component = (jitter - 0.5) * random_range;

    // Progression/function dominate. The chord-tone movement hint is kept at
    // only 5%; detailed voice-leading happens after chord selection.
    let total = progression.clamp(0.0, 1.0) * 0.38
        + function * 0.22
        + directional * 0.18
        + origin_prior * 0.10
        + history_fit * 0.07
        + chord_motion * 0.05
        + surprise_boost
        + random_component;

    let novelty = contextual_novelty(candidate, history, context, progression);
    let class = classify_novelty(novelty);

    let reason = if progression_reason != RecommendationReason::ProgressionPrior {
        progression_reason
    } else {
        candidate.origin.reason()
    };

    ScoredCandidate {
        recommendation: Recommendation {
            chord: candidate.chord,
            reason,
        },
        class,
        score: total,
    }
}

fn progression_score(
    history: &[Chord],
    candidate: Chord,
    context: TonalContext,
) -> (f32, RecommendationReason) {
    let current = history
        .last()
        .copied()
        .unwrap_or_else(|| context.tonic_chord());

    // A dominant resolving down a fifth (up a fourth) is strong regardless of
    // whether it is the primary dominant or a secondary dominant.
    if current.quality == ChordQuality::Dominant && candidate.root == current.root.transpose(5) {
        return (1.0, RecommendationReason::DominantResolution);
    }

    let current_degree = degree_of_root(context, current.root);
    let candidate_degree = degree_of_root(context, candidate.root);

    if let (Some(from), Some(to)) = (current_degree, candidate_degree) {
        if from == 1 && to == 4 {
            return (0.98, RecommendationReason::TwoFive);
        }

        if from == 4 && matches!(to, 0 | 5) {
            return (1.0, RecommendationReason::Cadence);
        }
    }

    if history.len() >= 2 {
        let previous = history[history.len() - 2];
        let previous_degree = degree_of_root(context, previous.root);

        if let (Some(previous), Some(current), Some(next)) =
            (previous_degree, current_degree, candidate_degree)
        {
            // ii -> V -> I
            if previous == 1 && current == 4 && next == 0 {
                return (1.0, RecommendationReason::Cadence);
            }

            // IV -> V -> I
            if previous == 3 && current == 4 && next == 0 {
                return (0.98, RecommendationReason::Cadence);
            }

            // vi -> ii -> V
            if previous == 5 && current == 1 && next == 4 {
                return (0.97, RecommendationReason::TwoFive);
            }

            // I -> vi -> ii
            if previous == 0 && current == 5 && next == 1 {
                return (0.92, RecommendationReason::ProgressionPrior);
            }
        }
    }

    if history.len() >= 3 {
        let degrees = history[history.len() - 3..]
            .iter()
            .map(|chord| degree_of_root(context, chord.root))
            .collect::<Vec<_>>();

        if degrees.len() == 3
            && degrees[0] == Some(0)
            && degrees[1] == Some(5)
            && degrees[2] == Some(1)
            && candidate_degree == Some(4)
        {
            return (1.0, RecommendationReason::ProgressionPrior);
        }
    }

    let pair_score = match (current_degree, candidate_degree) {
        (Some(0), Some(5)) => 0.88, // I -> vi
        (Some(0), Some(3)) => 0.82, // I -> IV
        (Some(0), Some(4)) => 0.84, // I -> V
        (Some(0), Some(1)) => 0.72, // I -> ii
        (Some(1), Some(4)) => 0.98, // ii -> V
        (Some(2), Some(5)) => 0.90, // iii -> vi
        (Some(3), Some(4)) => 0.94, // IV -> V
        (Some(3), Some(0)) => 0.82, // IV -> I
        (Some(4), Some(0)) => 1.00, // V -> I
        (Some(4), Some(5)) => 0.86, // V -> vi
        (Some(5), Some(1)) => 0.92, // vi -> ii
        (Some(5), Some(3)) => 0.84, // vi -> IV
        (Some(6), Some(0)) => 0.94, // vii -> I
        (Some(_), Some(_)) => 0.58,
        _ => 0.35,
    };

    (pair_score, RecommendationReason::ProgressionPrior)
}

fn functional_coherence(current: Chord, candidate: Chord, context: TonalContext) -> f32 {
    if current.quality == ChordQuality::Dominant && candidate.root == current.root.transpose(5) {
        return 1.0;
    }

    let from = function_of(current, context);
    let to = function_of(candidate, context);

    let transition = match (from, to) {
        (HarmonicFunction::Tonic, HarmonicFunction::Predominant) => 1.0,
        (HarmonicFunction::Tonic, HarmonicFunction::Dominant) => 0.88,
        (HarmonicFunction::Tonic, HarmonicFunction::Tonic) => 0.70,
        (HarmonicFunction::Predominant, HarmonicFunction::Dominant) => 1.0,
        (HarmonicFunction::Predominant, HarmonicFunction::Tonic) => 0.58,
        (HarmonicFunction::Predominant, HarmonicFunction::Predominant) => 0.52,
        (HarmonicFunction::Dominant, HarmonicFunction::Tonic) => 1.0,
        (HarmonicFunction::Dominant, HarmonicFunction::Predominant) => 0.30,
        (HarmonicFunction::Dominant, HarmonicFunction::Dominant) => 0.45,
        (_, HarmonicFunction::Chromatic) => 0.50,
        (HarmonicFunction::Chromatic, _) => 0.55,
    };

    if candidate.quality == ChordQuality::Dominant
        && degree_of_root(context, candidate.root.transpose(5)).is_some()
    {
        (transition + 0.12_f32).min(1.0_f32)
    } else {
        transition
    }
}

fn function_of(chord: Chord, context: TonalContext) -> HarmonicFunction {
    match degree_of_root(context, chord.root) {
        Some(0 | 2 | 5) => HarmonicFunction::Tonic,
        Some(1 | 3) => HarmonicFunction::Predominant,
        Some(4 | 6) => HarmonicFunction::Dominant,
        Some(_) | None => HarmonicFunction::Chromatic,
    }
}

fn directional_fit(
    current: Chord,
    candidate: Chord,
    context: TonalContext,
    direction: HarmonicDirection,
) -> f32 {
    let delta = tension_score(candidate, context) - tension_score(current, context);

    match direction {
        HarmonicDirection::Resolve => (0.55_f32 - delta * 0.42_f32).clamp(0.0_f32, 1.0_f32),
        HarmonicDirection::Neutral => (1.0_f32 - delta.abs() * 0.35_f32).clamp(0.0_f32, 1.0_f32),
        HarmonicDirection::Tension => (0.55_f32 + delta * 0.42_f32).clamp(0.0_f32, 1.0_f32),
    }
}

fn tension_score(chord: Chord, context: TonalContext) -> f32 {
    let quality = match chord.quality {
        ChordQuality::Major => 0.16,
        ChordQuality::Minor => 0.24,
        ChordQuality::Diminished => 0.92,
        ChordQuality::HalfDiminished => 0.76,
        ChordQuality::Dominant => 0.80,
    };

    let function = match function_of(chord, context) {
        HarmonicFunction::Tonic => 0.05,
        HarmonicFunction::Predominant => 0.30,
        HarmonicFunction::Dominant => 0.62,
        HarmonicFunction::Chromatic => 0.48,
    };

    let extension = match chord.extension {
        ChordExtension::Triad => 0.0,
        ChordExtension::Seventh => 0.10,
        ChordExtension::Ninth => 0.18,
    };

    quality + function + extension
}

fn history_fit(history: &[Chord], candidate: Chord) -> f32 {
    if history.len() >= 2 && history[history.len() - 2] == candidate {
        return 0.22;
    }

    if history
        .iter()
        .rev()
        .skip(1)
        .take(6)
        .any(|chord| *chord == candidate)
    {
        0.82
    } else {
        0.62
    }
}

fn chord_choice_voice_leading_hint(current: Chord, candidate: Chord) -> f32 {
    let current_pitch_classes = current.essential_pitch_classes();
    let candidate_pitch_classes = candidate.essential_pitch_classes();

    let common = current_pitch_classes
        .iter()
        .filter(|pitch| candidate_pitch_classes.contains(pitch))
        .count();

    common as f32 / 4.0
}

fn contextual_novelty(
    candidate: Candidate,
    history: &[Chord],
    context: TonalContext,
    progression_score: f32,
) -> f32 {
    let mut novelty = candidate.origin.novelty();

    if progression_score >= 0.90 {
        novelty -= 0.10;
    }

    if degree_of_root(context, candidate.chord.root).is_some() {
        novelty -= 0.06;
    }

    if history
        .iter()
        .rev()
        .skip(1)
        .take(6)
        .any(|chord| *chord == candidate.chord)
    {
        novelty -= 0.14;
    }

    novelty.clamp(0.0, 1.0)
}

const fn classify_novelty(novelty: f32) -> RecommendationClass {
    if novelty < 0.30 {
        RecommendationClass::Safe
    } else if novelty < 0.67 {
        RecommendationClass::Colorful
    } else {
        RecommendationClass::Bold
    }
}

fn degree_of_root(context: TonalContext, root: PitchClass) -> Option<usize> {
    context
        .mode
        .intervals()
        .iter()
        .position(|interval| context.tonic.transpose(*interval) == root)
}

fn select_three(
    scored: &[ScoredCandidate],
    class: RecommendationClass,
    context: TonalContext,
) -> [Recommendation; 3] {
    let matching = scored
        .iter()
        .filter(|candidate| candidate.class == class)
        .map(|candidate| candidate.recommendation)
        .collect::<Vec<_>>();

    if let [first, second, third, ..] = matching.as_slice() {
        return [*first, *second, *third];
    }

    // This should be rare because each novelty region has multiple generators.
    // Use deterministic nearby candidates rather than panicking.
    let mut fallback = matching;
    for candidate in scored {
        if fallback
            .iter()
            .any(|existing| existing.chord == candidate.recommendation.chord)
        {
            continue;
        }

        fallback.push(candidate.recommendation);
        if fallback.len() == 3 {
            break;
        }
    }

    if let [first, second, third, ..] = fallback.as_slice() {
        [*first, *second, *third]
    } else {
        fallback_recommendations(context)
    }
}

fn fallback_recommendations(context: TonalContext) -> [Recommendation; 3] {
    [0_usize, 3, 4].map(|degree| Recommendation {
        chord: context.diatonic_chord(degree, ChordExtension::Seventh),
        reason: RecommendationReason::Diatonic,
    })
}

#[cfg(test)]
mod tests {
    use super::{HarmonicDirection, RecommendationClass, RecommendationEngine};
    use crate::music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};
    use crate::music::settings::MusicalSettings;

    fn chord(root: u8, quality: ChordQuality, extension: ChordExtension) -> Chord {
        Chord::new(PitchClass::from_value(root), quality, extension)
    }

    #[test]
    fn recommendation_set_contains_three_per_class() {
        let settings = MusicalSettings::default();
        let current = settings.tonal.tonic_chord();
        let engine = RecommendationEngine::new();
        let set = engine.recommend(
            &[current],
            settings.tonal,
            HarmonicDirection::Neutral,
            false,
        );

        for class in RecommendationClass::ALL {
            assert_eq!(set.for_class(class).len(), 3);
        }
    }

    #[test]
    fn two_five_with_resolve_prefers_tonic() {
        let settings = MusicalSettings::default();
        let dm7 = chord(2, ChordQuality::Minor, ChordExtension::Seventh);
        let g7 = chord(7, ChordQuality::Dominant, ChordExtension::Seventh);
        let engine = RecommendationEngine::new();

        let set = engine.recommend(
            &[dm7, g7],
            settings.tonal,
            HarmonicDirection::Resolve,
            false,
        );

        assert_eq!(set.get(RecommendationClass::Safe, 0).root.value(), 0);
    }

    #[test]
    fn secondary_dominant_prefers_its_resolution() {
        let settings = MusicalSettings::default();
        let cmaj7 = settings.tonal.tonic_chord();
        let e7 = chord(4, ChordQuality::Dominant, ChordExtension::Seventh);
        let engine = RecommendationEngine::new();

        let set = engine.recommend(
            &[cmaj7, e7],
            settings.tonal,
            HarmonicDirection::Neutral,
            false,
        );

        let top_safe = set.get(RecommendationClass::Safe, 0);
        assert_eq!(top_safe.root.value(), 9);
        assert_eq!(top_safe.quality, ChordQuality::Minor);
    }

    #[test]
    fn one_six_two_context_prefers_five() {
        let settings = MusicalSettings::default();
        let cmaj7 = settings.tonal.tonic_chord();
        let am7 = chord(9, ChordQuality::Minor, ChordExtension::Seventh);
        let dm7 = chord(2, ChordQuality::Minor, ChordExtension::Seventh);
        let engine = RecommendationEngine::new();

        let set = engine.recommend(
            &[cmaj7, am7, dm7],
            settings.tonal,
            HarmonicDirection::Neutral,
            false,
        );

        let top_safe = set.get(RecommendationClass::Safe, 0);
        assert_eq!(top_safe.root.value(), 7);
        assert_eq!(top_safe.quality, ChordQuality::Dominant);
    }

    #[test]
    fn categories_do_not_duplicate_the_same_chord() {
        let settings = MusicalSettings::default();
        let engine = RecommendationEngine::new();
        let set = engine.recommend(
            &[settings.tonal.tonic_chord()],
            settings.tonal,
            HarmonicDirection::Neutral,
            true,
        );

        let mut chords = Vec::new();
        for class in RecommendationClass::ALL {
            for recommendation in set.for_class(class) {
                assert!(!chords.contains(&recommendation.chord));
                chords.push(recommendation.chord);
            }
        }
    }
}
