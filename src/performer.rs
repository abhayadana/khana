//! Role-aware performers with evolving pattern families and variants.
//!
//! v0.9 separates Density's high-level musical intent from the exact rhythm:
//! Density selects a role-specific [`PatternFamily`], while continuity, age,
//! recent pattern history, and deterministic variation select a sibling
//! [`PatternVariant`] within that family.
//!
//! The four roles are Bass / Chords / Texture / Melody. Chords is genuinely
//! polyphonic and states the selected harmony on MIDI channel 2. Melody remains
//! phrase-generated rather than being reduced to a hit mask.

use crate::music::chord::Chord;
use crate::music::settings::TonalContext;
use crate::music::voicing::Voicing;
use crate::voice::{Voice, VoiceRole};

const VOICE_COUNT: usize = 4;
const MAX_NOTES_PER_VOICE: usize = 4;
const MAX_EVENTS_PER_UPDATE: usize = 32;
const MAX_MELODY_STEPS: usize = 64;

/// When a performer's larger rhythmic state naturally renews.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResetRule {
    Chord,
    Bar,
    TwoBars,
    Phrase,
}

impl ResetRule {
    /// Compact UI label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Chord => "CHD",
            Self::Bar => "BAR",
            Self::TwoBars => "2BAR",
            Self::Phrase => "PHR",
        }
    }
}

/// High-level behavior region selected primarily by Density.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PatternFamily {
    BassFoundation,
    BassMoving,
    BassConnecting,
    ChordsSustain,
    ChordsComp,
    ChordsActive,
    TextureSparse,
    TextureMotion,
    TextureActive,
    MelodyMotif,
    MelodyPhrase,
    MelodyOrnament,
}

impl PatternFamily {
    /// Compact family label shown on the Performance screen.
    pub const fn label(self) -> &'static str {
        match self {
            Self::BassFoundation => "FOUND",
            Self::BassMoving => "MOVING",
            Self::BassConnecting => "CONNECT",
            Self::ChordsSustain => "SUSTAIN",
            Self::ChordsComp => "COMP",
            Self::ChordsActive => "ACTIVE",
            Self::TextureSparse => "SPARSE",
            Self::TextureMotion => "MOTION",
            Self::TextureActive => "ACTIVE",
            Self::MelodyMotif => "MOTIF",
            Self::MelodyPhrase => "PHRASE",
            Self::MelodyOrnament => "ORNAMENT",
        }
    }
}

/// Specific pattern inside a role-specific family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PatternVariant {
    BassRootHold,
    BassRootPulse,
    BassRootOctave,
    BassRootFifth,
    BassRootFifthOctave,
    BassBroken,
    BassScaleApproach,
    BassChromaticApproach,
    BassWalkingFragment,

    ChordsSustain,
    ChordsSinglePulse,
    ChordsTwoPulse,
    ChordsBackbeat,
    ChordsOffbeat,
    ChordsCharleston,
    ChordsAnticipationResponse,
    ChordsSyncopatedA,
    ChordsSyncopatedB,
    ChordsActiveStabs,

    TextureIsolatedUpper,
    TextureDelayedEcho,
    TextureChordSparkle,
    TextureTwoNoteResponse,
    TextureSlowArp,
    TextureStaggeredExtensions,
    TextureShimmer,
    TextureRepeatingArp,
    TextureExtensionFigure,

    MelodyMotifA,
    MelodyMotifB,
    MelodySparseSkeleton,
    MelodyDevelopedPhrase,
    MelodyMotifVariation,
    MelodyAnsweringPhrase,
    MelodyOrnamentedPhrase,
    MelodyPassingDense,
    MelodyChromaticPhrase,
}

const BASS_FOUNDATION: [PatternVariant; 3] = [
    PatternVariant::BassRootHold,
    PatternVariant::BassRootPulse,
    PatternVariant::BassRootOctave,
];
const BASS_MOVING: [PatternVariant; 3] = [
    PatternVariant::BassRootFifth,
    PatternVariant::BassRootFifthOctave,
    PatternVariant::BassBroken,
];
const BASS_CONNECTING: [PatternVariant; 3] = [
    PatternVariant::BassScaleApproach,
    PatternVariant::BassChromaticApproach,
    PatternVariant::BassWalkingFragment,
];
const CHORDS_SUSTAIN: [PatternVariant; 3] = [
    PatternVariant::ChordsSustain,
    PatternVariant::ChordsSinglePulse,
    PatternVariant::ChordsTwoPulse,
];
const CHORDS_COMP: [PatternVariant; 4] = [
    PatternVariant::ChordsBackbeat,
    PatternVariant::ChordsOffbeat,
    PatternVariant::ChordsCharleston,
    PatternVariant::ChordsAnticipationResponse,
];
const CHORDS_ACTIVE: [PatternVariant; 3] = [
    PatternVariant::ChordsSyncopatedA,
    PatternVariant::ChordsSyncopatedB,
    PatternVariant::ChordsActiveStabs,
];
const TEXTURE_SPARSE: [PatternVariant; 3] = [
    PatternVariant::TextureIsolatedUpper,
    PatternVariant::TextureDelayedEcho,
    PatternVariant::TextureChordSparkle,
];
const TEXTURE_MOTION: [PatternVariant; 3] = [
    PatternVariant::TextureTwoNoteResponse,
    PatternVariant::TextureSlowArp,
    PatternVariant::TextureStaggeredExtensions,
];
const TEXTURE_ACTIVE: [PatternVariant; 3] = [
    PatternVariant::TextureShimmer,
    PatternVariant::TextureRepeatingArp,
    PatternVariant::TextureExtensionFigure,
];
const MELODY_MOTIF: [PatternVariant; 3] = [
    PatternVariant::MelodyMotifA,
    PatternVariant::MelodyMotifB,
    PatternVariant::MelodySparseSkeleton,
];
const MELODY_PHRASE: [PatternVariant; 3] = [
    PatternVariant::MelodyDevelopedPhrase,
    PatternVariant::MelodyMotifVariation,
    PatternVariant::MelodyAnsweringPhrase,
];
const MELODY_ORNAMENT: [PatternVariant; 3] = [
    PatternVariant::MelodyOrnamentedPhrase,
    PatternVariant::MelodyPassingDense,
    PatternVariant::MelodyChromaticPhrase,
];

/// One channel-specific MIDI intention emitted by a performer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceEvent {
    NoteOn {
        voice_index: usize,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        voice_index: usize,
        note: u8,
    },
}

/// Fixed-capacity event batch for the real-time update path.
pub struct VoiceEventBatch {
    events: [Option<VoiceEvent>; MAX_EVENTS_PER_UPDATE],
    len: usize,
}

impl VoiceEventBatch {
    const fn new() -> Self {
        Self {
            events: [None; MAX_EVENTS_PER_UPDATE],
            len: 0,
        }
    }

    fn push(&mut self, event: VoiceEvent) {
        if self.len < self.events.len() {
            self.events[self.len] = Some(event);
            self.len += 1;
        }
    }

    /// Iterates over emitted events without allocating.
    pub fn iter(&self) -> impl Iterator<Item = VoiceEvent> + '_ {
        self.events[..self.len].iter().filter_map(|event| *event)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NoteSet {
    notes: [u8; MAX_NOTES_PER_VOICE],
    len: usize,
}

impl NoteSet {
    const fn single(note: u8) -> Self {
        Self {
            notes: [note, 0, 0, 0],
            len: 1,
        }
    }

    const fn chord(notes: [u8; 4]) -> Self {
        Self { notes, len: 4 }
    }

    fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        self.notes[..self.len].iter().copied()
    }

    fn snapshot(self) -> [Option<u8>; MAX_NOTES_PER_VOICE] {
        std::array::from_fn(|index| {
            if index < self.len {
                Some(self.notes[index])
            } else {
                None
            }
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct ScheduledNotes {
    notes: NoteSet,
    off_beat: f64,
}

#[derive(Clone, Copy, Debug)]
struct PendingAttack {
    notes: NoteSet,
    velocity: u8,
    start_beat: f64,
    gate_beats: f64,
}

#[derive(Clone, Copy, Debug)]
struct Behavior {
    activity: f32,
    space: f32,
    repeat: f32,
}

#[derive(Clone, Copy, Debug)]
struct PatternState {
    family: PatternFamily,
    variant: PatternVariant,
    previous_variant: PatternVariant,
    age_cycles: u8,
    last_cycle_index: i64,
    seed: u64,
}

impl PatternState {
    const fn new(family: PatternFamily, variant: PatternVariant, seed: u64) -> Self {
        Self {
            family,
            variant,
            previous_variant: variant,
            age_cycles: 0,
            last_cycle_index: i64::MIN,
            seed,
        }
    }

    fn update(
        &mut self,
        family: PatternFamily,
        role: VoiceRole,
        cycle_index: i64,
        activity: f32,
        repeat: f32,
    ) {
        let family_changed = family != self.family;
        let cycle_changed = cycle_index != self.last_cycle_index;
        if !family_changed && !cycle_changed {
            return;
        }

        let next = choose_variant(VariantRequest {
            family,
            role,
            current: if family_changed {
                None
            } else {
                Some(self.variant)
            },
            previous: self.previous_variant,
            age_cycles: self.age_cycles,
            cycle_index,
            activity,
            repeat,
            seed: self.seed,
        });

        if next == self.variant && !family_changed {
            self.age_cycles = self.age_cycles.saturating_add(1);
        } else {
            self.previous_variant = self.variant;
            self.variant = next;
            self.age_cycles = 0;
        }

        self.family = family;
        self.last_cycle_index = cycle_index;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MelodyIntent {
    Rest,
    Skeleton(i8),
    Fill(i8),
    Neighbor(i8),
    ChromaticApproach(i8),
}

#[derive(Clone, Copy, Debug)]
struct MelodyPlan {
    steps: [MelodyIntent; MAX_MELODY_STEPS],
    len: usize,
    cycle_index: i64,
    generation: u64,
}

impl MelodyPlan {
    const fn empty() -> Self {
        Self {
            steps: [MelodyIntent::Rest; MAX_MELODY_STEPS],
            len: 0,
            cycle_index: i64::MIN,
            generation: 0,
        }
    }

    fn intent(&self, position: usize) -> MelodyIntent {
        if self.len == 0 {
            MelodyIntent::Rest
        } else {
            self.steps[position % self.len]
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PerformerState {
    last_step_key: Option<i64>,
    sounding: Option<ScheduledNotes>,
    pending: Option<PendingAttack>,
    previous_note: Option<u8>,
    pending_resolution: Option<u8>,
    force_attack: bool,
    pattern: PatternState,
    melody_plan: MelodyPlan,
}

impl PerformerState {
    const fn new(pattern: PatternState) -> Self {
        Self {
            last_step_key: None,
            sounding: None,
            pending: None,
            previous_note: None,
            pending_resolution: None,
            force_attack: false,
            pattern,
            melody_plan: MelodyPlan::empty(),
        }
    }
}

/// Read-only state shown in the Performance UI.
#[derive(Clone, Copy, Debug)]
pub struct PerformerSnapshot {
    pub active_notes: [Option<u8>; MAX_NOTES_PER_VOICE],
    pub reset_rule: ResetRule,
    pub family: PatternFamily,
}

/// Four independent role-aware performers sharing one active harmony.
pub struct PerformerEngine {
    chord: Chord,
    tonal: TonalContext,
    chord_voicing: Voicing,
    chord_origin_beat: f64,
    enabled: bool,
    intensity: f32,
    states: [PerformerState; VOICE_COUNT],
}

impl PerformerEngine {
    /// Creates a stopped performer engine around an initial harmony.
    pub const fn new(chord: Chord, tonal: TonalContext, chord_voicing: Voicing) -> Self {
        Self {
            chord,
            tonal,
            chord_voicing,
            chord_origin_beat: 0.0,
            enabled: false,
            intensity: 0.0,
            states: [
                PerformerState::new(PatternState::new(
                    PatternFamily::BassMoving,
                    PatternVariant::BassRootFifth,
                    0xB455_0001,
                )),
                PerformerState::new(PatternState::new(
                    PatternFamily::ChordsComp,
                    PatternVariant::ChordsCharleston,
                    0xC40D_0002,
                )),
                PerformerState::new(PatternState::new(
                    PatternFamily::TextureMotion,
                    PatternVariant::TextureDelayedEcho,
                    0x7E87_0003,
                )),
                PerformerState::new(PatternState::new(
                    PatternFamily::MelodyPhrase,
                    PatternVariant::MelodyDevelopedPhrase,
                    0x4E10_0004,
                )),
            ],
        }
    }

    /// Updates shared harmony without erasing longer musical identity.
    ///
    /// Bass and Chords force an immediate new-harmony attack. Texture retains
    /// its two-bar pattern identity and Melody retains its phrase plan.
    pub fn harmony_changed(
        &mut self,
        chord: Chord,
        tonal: TonalContext,
        chord_voicing: Voicing,
        now_beat: f64,
    ) {
        self.chord = chord;
        self.tonal = tonal;
        self.chord_voicing = chord_voicing;
        self.chord_origin_beat = now_beat;

        for state in &mut self.states {
            state.sounding = None;
            state.pending = None;
            state.pending_resolution = None;
        }

        self.states[0].last_step_key = None;
        self.states[0].force_attack = true;
        self.states[1].force_attack = true;
    }

    /// Changes the Chords voicing/register reference without retriggering.
    pub fn update_anchor(&mut self, tonal: TonalContext, chord_voicing: Voicing) {
        self.tonal = tonal;
        self.chord_voicing = chord_voicing;
    }

    /// Enables performer generation at the current transport position.
    pub fn transport_started(&mut self, now_beat: f64) {
        self.enabled = true;
        self.chord_origin_beat = now_beat;

        for state in &mut self.states {
            state.sounding = None;
            state.pending = None;
            state.last_step_key = None;
            state.pending_resolution = None;
        }
        self.states[0].force_attack = true;
        self.states[1].force_attack = true;
    }

    /// Disables generation and clears internal note ownership.
    pub fn transport_stopped(&mut self) {
        self.enabled = false;
        for state in &mut self.states {
            state.sounding = None;
            state.pending = None;
            state.last_step_key = None;
            state.pending_resolution = None;
            state.force_attack = false;
        }
    }

    /// Enables/disables generation without changing transport state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            for state in &mut self.states {
                state.sounding = None;
                state.pending = None;
                state.pending_resolution = None;
                state.force_attack = false;
            }
        }
    }

    /// Returns current UI state for all four performers.
    pub fn snapshots(&self) -> [PerformerSnapshot; VOICE_COUNT] {
        std::array::from_fn(|index| PerformerSnapshot {
            active_notes: self.states[index]
                .sounding
                .map_or([None; MAX_NOTES_PER_VOICE], |sounding| {
                    sounding.notes.snapshot()
                }),
            reset_rule: reset_rule(index),
            family: self.states[index].pattern.family,
        })
    }

    /// Advances all performers to `now_beat`.
    pub fn update(
        &mut self,
        now_beat: f64,
        beats_per_bar: f64,
        voices: &[Voice; VOICE_COUNT],
    ) -> VoiceEventBatch {
        let mut batch = VoiceEventBatch::new();
        if !self.enabled {
            return batch;
        }

        for (index, voice) in voices.iter().copied().enumerate() {
            self.release_due_notes(index, now_beat, &mut batch);
            self.start_due_attack(index, now_beat, &mut batch);

            let role = voice.role;
            let behavior = behavior_for(role, voice.parameters.density, self.intensity);
            let family = family_for(role, behavior.activity);
            let timing = step_timing(
                role,
                family,
                now_beat,
                beats_per_bar,
                self.chord_origin_beat,
            );

            self.states[index].pattern.update(
                family,
                role,
                timing.cycle_index,
                behavior.activity,
                behavior.repeat,
            );
            let variant = self.states[index].pattern.variant;

            if self.states[index].last_step_key == Some(timing.step_key)
                && !self.states[index].force_attack
            {
                continue;
            }
            self.states[index].last_step_key = Some(timing.step_key);

            if role == VoiceRole::Melody {
                self.ensure_melody_plan(timing, voice.parameters.color, behavior, variant);
            }

            let forced = self.states[index].force_attack;
            self.states[index].force_attack = false;
            if behavior.activity <= 0.01
                || (!forced && !self.should_attack(index, role, variant, timing, behavior))
            {
                continue;
            }

            let notes = match role {
                VoiceRole::Chords => NoteSet::chord(self.chord_voicing.notes),
                VoiceRole::Melody => {
                    NoteSet::single(self.choose_melody_note(index, timing.position, voice))
                }
                VoiceRole::Bass | VoiceRole::Texture => {
                    NoteSet::single(choose_role_note(RoleNoteRequest {
                        voice,
                        variant,
                        chord: self.chord,
                        tonal: self.tonal,
                        position: timing.position,
                        chord_voicing: self.chord_voicing,
                        previous_note: self.states[index].previous_note,
                    }))
                }
            };

            let velocity = attack_velocity(
                role,
                voice.parameters.dynamics,
                timing.position,
                self.intensity,
            );
            let gate = gate_beats(role, variant, timing, behavior.space, self.intensity);
            let start = now_beat + timing_offset_beats(role);

            self.states[index].pending = Some(PendingAttack {
                notes,
                velocity,
                start_beat: start,
                gate_beats: gate,
            });
            self.start_due_attack(index, now_beat, &mut batch);
        }

        batch
    }

    fn release_due_notes(&mut self, index: usize, now_beat: f64, batch: &mut VoiceEventBatch) {
        if let Some(active) = self.states[index].sounding
            && now_beat + f64::EPSILON >= active.off_beat
        {
            for note in active.notes.iter() {
                batch.push(VoiceEvent::NoteOff {
                    voice_index: index,
                    note,
                });
            }
            self.states[index].sounding = None;
        }
    }

    fn start_due_attack(&mut self, index: usize, now_beat: f64, batch: &mut VoiceEventBatch) {
        let Some(pending) = self.states[index].pending else {
            return;
        };
        if now_beat + f64::EPSILON < pending.start_beat {
            return;
        }

        if let Some(active) = self.states[index].sounding.take() {
            for note in active.notes.iter() {
                batch.push(VoiceEvent::NoteOff {
                    voice_index: index,
                    note,
                });
            }
        }

        for note in pending.notes.iter() {
            batch.push(VoiceEvent::NoteOn {
                voice_index: index,
                note,
                velocity: pending.velocity,
            });
        }

        self.states[index].sounding = Some(ScheduledNotes {
            notes: pending.notes,
            off_beat: pending.start_beat + pending.gate_beats,
        });
        if pending.notes.len == 1 {
            self.states[index].previous_note = Some(pending.notes.notes[0]);
        }
        self.states[index].pending = None;
    }

    fn should_attack(
        &self,
        index: usize,
        role: VoiceRole,
        variant: PatternVariant,
        timing: StepTiming,
        behavior: Behavior,
    ) -> bool {
        if role == VoiceRole::Melody {
            return self.states[index].melody_plan.intent(timing.position) != MelodyIntent::Rest;
        }

        let base_hit = pattern_hit(variant, timing.position, timing.steps_per_cycle);
        if !base_hit {
            return false;
        }

        // Bass and Chords play their selected pattern coherently. Texture is
        // intentionally more porous and can omit deterministic sibling hits.
        if role != VoiceRole::Texture {
            return true;
        }

        let repeat_group = repeat_group(timing.cycle_index, behavior.repeat);
        let chance = deterministic_unit(
            role_seed(role)
                ^ (repeat_group as u64).wrapping_mul(0x9E37_79B9)
                ^ (timing.position as u64).wrapping_mul(0x85EB_CA6B),
        );
        let keep = (0.52 + behavior.activity * 0.62 - behavior.space * 0.32).clamp(0.15, 1.0);
        chance <= keep
    }

    fn ensure_melody_plan(
        &mut self,
        timing: StepTiming,
        color: f32,
        behavior: Behavior,
        variant: PatternVariant,
    ) {
        let state = &mut self.states[3];
        if state.melody_plan.cycle_index == timing.cycle_index
            && state.melody_plan.len == timing.steps_per_cycle
        {
            return;
        }

        let previous = state.melody_plan;
        state.melody_plan = generate_melody_plan(
            timing.steps_per_cycle.min(MAX_MELODY_STEPS),
            behavior,
            color,
            previous,
            timing.cycle_index,
            variant,
        );
    }

    fn choose_melody_note(&mut self, index: usize, position: usize, voice: Voice) -> u8 {
        if let Some(resolution) = self.states[index].pending_resolution.take() {
            return resolution;
        }

        let intent = self.states[index].melody_plan.intent(position);
        if let MelodyIntent::ChromaticApproach(contour) = intent {
            let resolution = melody_target_note(
                MelodyIntent::Skeleton(contour),
                self.chord,
                self.tonal,
                self.chord_voicing.notes[3],
                self.states[index].previous_note,
                voice.parameters.spread,
                0.45,
            );
            self.states[index].pending_resolution = Some(resolution);
            return chromatic_approach_note(resolution, self.chord, self.tonal);
        }

        melody_target_note(
            intent,
            self.chord,
            self.tonal,
            self.chord_voicing.notes[3],
            self.states[index].previous_note,
            voice.parameters.spread,
            voice.parameters.color,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct StepTiming {
    step_key: i64,
    position: usize,
    steps_per_cycle: usize,
    step_length: f64,
    cycle_index: i64,
}

#[derive(Clone, Copy, Debug)]
struct RoleNoteRequest {
    voice: Voice,
    variant: PatternVariant,
    chord: Chord,
    tonal: TonalContext,
    position: usize,
    chord_voicing: Voicing,
    previous_note: Option<u8>,
}

fn variants_for(family: PatternFamily) -> &'static [PatternVariant] {
    match family {
        PatternFamily::BassFoundation => &BASS_FOUNDATION,
        PatternFamily::BassMoving => &BASS_MOVING,
        PatternFamily::BassConnecting => &BASS_CONNECTING,
        PatternFamily::ChordsSustain => &CHORDS_SUSTAIN,
        PatternFamily::ChordsComp => &CHORDS_COMP,
        PatternFamily::ChordsActive => &CHORDS_ACTIVE,
        PatternFamily::TextureSparse => &TEXTURE_SPARSE,
        PatternFamily::TextureMotion => &TEXTURE_MOTION,
        PatternFamily::TextureActive => &TEXTURE_ACTIVE,
        PatternFamily::MelodyMotif => &MELODY_MOTIF,
        PatternFamily::MelodyPhrase => &MELODY_PHRASE,
        PatternFamily::MelodyOrnament => &MELODY_ORNAMENT,
    }
}

#[derive(Clone, Copy, Debug)]
struct VariantRequest {
    family: PatternFamily,
    role: VoiceRole,
    current: Option<PatternVariant>,
    previous: PatternVariant,
    age_cycles: u8,
    cycle_index: i64,
    activity: f32,
    repeat: f32,
    seed: u64,
}

fn choose_variant(request: VariantRequest) -> PatternVariant {
    let candidates = variants_for(request.family);
    let local = family_local_activity(request.role, request.activity);
    let target = local * (candidates.len().saturating_sub(1)) as f32;
    let mut best = candidates[0];
    let mut best_score = f32::MIN;

    for (index, candidate) in candidates.iter().copied().enumerate() {
        let distance = (index as f32 - target).abs();
        let density_fit = 1.0 - distance / candidates.len().max(1) as f32;
        let continuity = if Some(candidate) == request.current {
            let age_decay = f32::from(request.age_cycles.min(5)) * 0.11;
            (0.62 + request.repeat * 0.52 - age_decay).max(0.12)
        } else {
            0.0
        };
        let history_penalty = if candidate == request.previous {
            0.08
        } else {
            0.0
        };
        let jitter = deterministic_unit(
            request.seed
                ^ (request.cycle_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03),
        ) * 0.18;
        let score = density_fit * 0.72 + continuity + jitter - history_penalty;

        if score > best_score {
            best = candidate;
            best_score = score;
        }
    }

    best
}

fn family_for(role: VoiceRole, activity: f32) -> PatternFamily {
    match role {
        VoiceRole::Bass if activity < 0.32 => PatternFamily::BassFoundation,
        VoiceRole::Bass if activity < 0.72 => PatternFamily::BassMoving,
        VoiceRole::Bass => PatternFamily::BassConnecting,

        VoiceRole::Chords if activity < 0.30 => PatternFamily::ChordsSustain,
        VoiceRole::Chords if activity < 0.75 => PatternFamily::ChordsComp,
        VoiceRole::Chords => PatternFamily::ChordsActive,

        VoiceRole::Texture if activity < 0.38 => PatternFamily::TextureSparse,
        VoiceRole::Texture if activity < 0.75 => PatternFamily::TextureMotion,
        VoiceRole::Texture => PatternFamily::TextureActive,

        VoiceRole::Melody if activity < 0.30 => PatternFamily::MelodyMotif,
        VoiceRole::Melody if activity < 0.76 => PatternFamily::MelodyPhrase,
        VoiceRole::Melody => PatternFamily::MelodyOrnament,
    }
}

fn family_local_activity(role: VoiceRole, activity: f32) -> f32 {
    let (low, high) = match family_for(role, activity) {
        PatternFamily::BassFoundation => (0.0, 0.32),
        PatternFamily::BassMoving => (0.32, 0.72),
        PatternFamily::BassConnecting => (0.72, 1.0),
        PatternFamily::ChordsSustain => (0.0, 0.30),
        PatternFamily::ChordsComp => (0.30, 0.75),
        PatternFamily::ChordsActive => (0.75, 1.0),
        PatternFamily::TextureSparse => (0.0, 0.38),
        PatternFamily::TextureMotion => (0.38, 0.75),
        PatternFamily::TextureActive => (0.75, 1.0),
        PatternFamily::MelodyMotif => (0.0, 0.30),
        PatternFamily::MelodyPhrase => (0.30, 0.76),
        PatternFamily::MelodyOrnament => (0.76, 1.0),
    };

    ((activity - low) / (high - low)).clamp(0.0, 1.0)
}

fn step_timing(
    role: VoiceRole,
    family: PatternFamily,
    now_beat: f64,
    beats_per_bar: f64,
    chord_origin_beat: f64,
) -> StepTiming {
    let step_length = step_length(family);
    let safe_bar = beats_per_bar.max(step_length);
    let absolute_step = (now_beat / step_length).floor() as i64;

    match role {
        VoiceRole::Bass => {
            let relative = (now_beat - chord_origin_beat).max(0.0);
            let relative_step = (relative / step_length).floor() as i64;
            let steps = cycle_steps(safe_bar, step_length);
            StepTiming {
                step_key: relative_step,
                position: relative_step.rem_euclid(steps as i64) as usize,
                steps_per_cycle: steps,
                step_length,
                cycle_index: (relative / safe_bar).floor() as i64,
            }
        }
        VoiceRole::Chords => {
            let steps = cycle_steps(safe_bar, step_length);
            let bar_position = now_beat.rem_euclid(safe_bar);
            StepTiming {
                step_key: absolute_step,
                position: ((bar_position / step_length).floor() as usize).min(steps - 1),
                steps_per_cycle: steps,
                step_length,
                cycle_index: (now_beat / safe_bar).floor() as i64,
            }
        }
        VoiceRole::Texture => {
            let cycle_beats = safe_bar * 2.0;
            let steps = cycle_steps(cycle_beats, step_length);
            let cycle_position = now_beat.rem_euclid(cycle_beats);
            StepTiming {
                step_key: absolute_step,
                position: ((cycle_position / step_length).floor() as usize).min(steps - 1),
                steps_per_cycle: steps,
                step_length,
                cycle_index: (now_beat / cycle_beats).floor() as i64,
            }
        }
        VoiceRole::Melody => {
            let phrase_beats = safe_bar * 2.0;
            let steps = cycle_steps(phrase_beats, step_length).min(MAX_MELODY_STEPS);
            let phrase_position = now_beat.rem_euclid(phrase_beats);
            StepTiming {
                step_key: absolute_step,
                position: ((phrase_position / step_length).floor() as usize).min(steps - 1),
                steps_per_cycle: steps,
                step_length,
                cycle_index: (now_beat / phrase_beats).floor() as i64,
            }
        }
    }
}

fn step_length(family: PatternFamily) -> f64 {
    match family {
        PatternFamily::TextureActive | PatternFamily::MelodyOrnament => 0.25,
        _ => 0.5,
    }
}

fn cycle_steps(cycle_beats: f64, step_length: f64) -> usize {
    ((cycle_beats / step_length).round() as usize).max(1)
}

fn reset_rule(index: usize) -> ResetRule {
    match index {
        0 => ResetRule::Chord,
        1 => ResetRule::Bar,
        2 => ResetRule::TwoBars,
        _ => ResetRule::Phrase,
    }
}

fn behavior_for(role: VoiceRole, density: f32, intensity: f32) -> Behavior {
    let density = density.clamp(0.0, 1.0);
    let intensity = intensity.clamp(-1.0, 1.0);

    let base_space = match role {
        VoiceRole::Bass => 0.40,
        VoiceRole::Chords => 0.34,
        VoiceRole::Texture => 0.70,
        VoiceRole::Melody => 0.46,
    };
    let repeat = match role {
        VoiceRole::Bass => 0.76,
        VoiceRole::Chords => 0.82,
        VoiceRole::Texture => 0.62,
        VoiceRole::Melody => 0.74,
    };

    Behavior {
        activity: (density + intensity * 0.18).clamp(0.0, 1.0),
        space: (base_space - density * 0.20 - intensity * 0.12).clamp(0.08, 0.92),
        repeat,
    }
}

fn pattern_hit(variant: PatternVariant, position: usize, steps: usize) -> bool {
    if steps == 0 {
        return false;
    }

    let half = (steps / 2).max(1);
    let quarter = (steps / 4).max(1);
    let last = steps.saturating_sub(1);

    match variant {
        PatternVariant::BassRootHold => position == 0,
        PatternVariant::BassRootPulse => position.is_multiple_of(2),
        PatternVariant::BassRootOctave => position == 0 || position == half,
        PatternVariant::BassRootFifth => position == 0 || position == half,
        PatternVariant::BassRootFifthOctave => {
            position == 0 || position == quarter || position == half
        }
        PatternVariant::BassBroken => matches!(position % 8, 0 | 3 | 5 | 7),
        PatternVariant::BassScaleApproach => matches!(position % 8, 0 | 4 | 6 | 7),
        PatternVariant::BassChromaticApproach => {
            matches!(position % 8, 0 | 4 | 7)
        }
        PatternVariant::BassWalkingFragment => true,

        PatternVariant::ChordsSustain => position == 0,
        PatternVariant::ChordsSinglePulse => position == 0,
        PatternVariant::ChordsTwoPulse => position == 0 || position == half,
        PatternVariant::ChordsBackbeat => position == quarter || position == quarter * 3,
        PatternVariant::ChordsOffbeat => !position.is_multiple_of(2),
        PatternVariant::ChordsCharleston => position == 0 || position == quarter + 1,
        PatternVariant::ChordsAnticipationResponse => position == half || position == last,
        PatternVariant::ChordsSyncopatedA => matches!(position % 8, 0 | 3 | 6),
        PatternVariant::ChordsSyncopatedB => matches!(position % 8, 1 | 4 | 7),
        PatternVariant::ChordsActiveStabs => matches!(position % 8, 0 | 2 | 3 | 5 | 7),

        PatternVariant::TextureIsolatedUpper => position == 0,
        PatternVariant::TextureDelayedEcho => position == quarter || position == half + quarter,
        PatternVariant::TextureChordSparkle => position == 1,
        PatternVariant::TextureTwoNoteResponse => matches!(position % 8, 3 | 7),
        PatternVariant::TextureSlowArp => position.is_multiple_of(2),
        PatternVariant::TextureStaggeredExtensions => matches!(position % 8, 1 | 4 | 6),
        PatternVariant::TextureShimmer => position.is_multiple_of(2),
        PatternVariant::TextureRepeatingArp => position % 3 != 2,
        PatternVariant::TextureExtensionFigure => matches!(position % 8, 0 | 3 | 5 | 7),

        // Melody rhythm comes from the phrase planner.
        PatternVariant::MelodyMotifA
        | PatternVariant::MelodyMotifB
        | PatternVariant::MelodySparseSkeleton
        | PatternVariant::MelodyDevelopedPhrase
        | PatternVariant::MelodyMotifVariation
        | PatternVariant::MelodyAnsweringPhrase
        | PatternVariant::MelodyOrnamentedPhrase
        | PatternVariant::MelodyPassingDense
        | PatternVariant::MelodyChromaticPhrase => true,
    }
}

fn repeat_group(cycle_index: i64, repeat: f32) -> i64 {
    if repeat >= 0.72 {
        cycle_index.div_euclid(2)
    } else if repeat >= 0.48 {
        cycle_index
    } else {
        cycle_index.wrapping_mul(3)
    }
}

fn role_seed(role: VoiceRole) -> u64 {
    match role {
        VoiceRole::Bass => 0xB455_0001,
        VoiceRole::Chords => 0xC40D_0002,
        VoiceRole::Texture => 0x7E87_0003,
        VoiceRole::Melody => 0x4E10_0004,
    }
}

fn deterministic_unit(mut value: u64) -> f32 {
    value ^= value >> 33;
    value = value.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    value ^= value >> 33;
    value = value.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    value ^= value >> 33;
    (value as u32) as f32 / u32::MAX as f32
}

#[derive(Clone, Copy, Debug)]
struct PitchPool {
    values: [u8; 12],
    len: usize,
}

impl PitchPool {
    const fn new() -> Self {
        Self {
            values: [0; 12],
            len: 0,
        }
    }

    fn push(&mut self, pitch_class: u8) {
        let pitch_class = pitch_class % 12;
        if self.contains(pitch_class) || self.len >= self.values.len() {
            return;
        }
        self.values[self.len] = pitch_class;
        self.len += 1;
    }

    fn contains(&self, pitch_class: u8) -> bool {
        self.as_slice().contains(&(pitch_class % 12))
    }

    fn get(&self, index: usize) -> u8 {
        self.values[index % self.len.max(1)]
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn as_slice(&self) -> &[u8] {
        &self.values[..self.len]
    }
}

fn role_note_pool(
    role: VoiceRole,
    variant: PatternVariant,
    chord: Chord,
    tonal: TonalContext,
    color: f32,
) -> PitchPool {
    let color = color.clamp(0.0, 1.0);
    let intervals = chord.intervals();
    let mut pool = PitchPool::new();

    match role {
        VoiceRole::Bass => {
            pool.push(chord.root.value());
            if intervals.len() >= 3 {
                pool.push((chord.root.value() + intervals[2]) % 12);
            }
            if color >= 0.55 && intervals.len() >= 2 {
                pool.push((chord.root.value() + intervals[1]) % 12);
            }
            if color >= 0.82 && intervals.len() >= 4 {
                pool.push((chord.root.value() + intervals[3]) % 12);
            }
            if matches!(
                variant,
                PatternVariant::BassScaleApproach | PatternVariant::BassWalkingFragment
            ) && color >= 0.35
            {
                for interval in tonal.mode.intervals() {
                    pool.push(tonal.tonic.transpose(interval).value());
                }
            }
        }
        VoiceRole::Texture => {
            for interval in &intervals {
                pool.push((chord.root.value() + interval) % 12);
            }
            if color >= 0.32 {
                pool.push(chord.root.transpose(2).value());
            }
            if color >= 0.55 {
                pool.push(chord.root.transpose(5).value());
            }
            if color >= 0.76 {
                pool.push(chord.root.transpose(9).value());
            }
        }
        VoiceRole::Melody => {
            for interval in &intervals {
                pool.push((chord.root.value() + interval) % 12);
            }
            if color >= 0.38 {
                for interval in tonal.mode.intervals() {
                    pool.push(tonal.tonic.transpose(interval).value());
                }
            }
        }
        VoiceRole::Chords => {
            // Chords uses the dedicated four-note voicing rather than a pool.
        }
    }

    if pool.is_empty() && role != VoiceRole::Chords {
        pool.push(chord.root.value());
    }
    pool
}

fn choose_role_note(request: RoleNoteRequest) -> u8 {
    let role = request.voice.role;
    let pool = role_note_pool(
        role,
        request.variant,
        request.chord,
        request.tonal,
        request.voice.parameters.color,
    );
    let spread = request.voice.parameters.spread.clamp(0.0, 1.0);
    let target = match role {
        VoiceRole::Bass => 36.0 + spread * 8.0,
        VoiceRole::Texture => f32::from(request.chord_voicing.notes[3]) + 3.0 + spread * 10.0,
        VoiceRole::Melody => f32::from(request.chord_voicing.notes[3]) + 5.0 + spread * 12.0,
        VoiceRole::Chords => f32::from(request.chord_voicing.notes[1]),
    };
    let preferred_index = pattern_note_offset(request.variant, request.position);
    let preferred = pool.get(preferred_index);
    let movement_weight = match role {
        VoiceRole::Bass => 0.24,
        VoiceRole::Texture => 0.38,
        VoiceRole::Melody => 0.82,
        VoiceRole::Chords => 0.0,
    };
    let mut note = nearest_allowed_note(
        &pool,
        preferred,
        target,
        request.previous_note,
        movement_weight,
    );

    if matches!(request.variant, PatternVariant::BassRootOctave) && request.position > 0 {
        note = note.saturating_add(12).min(88);
    }
    if matches!(request.variant, PatternVariant::BassRootFifthOctave) && request.position >= 4 {
        note = note.saturating_add(12).min(88);
    }
    if matches!(request.variant, PatternVariant::BassChromaticApproach) && request.position % 8 == 7
    {
        let root_target = nearest_pitch_class_near(request.chord.root.value(), target as u8);
        note = root_target.saturating_sub(1).max(24);
    }

    note
}

fn pattern_note_offset(variant: PatternVariant, position: usize) -> usize {
    match variant {
        PatternVariant::BassRootHold
        | PatternVariant::BassRootPulse
        | PatternVariant::BassRootOctave => 0,
        PatternVariant::BassRootFifth => {
            if position == 0 {
                0
            } else {
                1
            }
        }
        PatternVariant::BassRootFifthOctave => match position {
            0 => 0,
            1..=3 => 1,
            _ => 0,
        },
        PatternVariant::BassBroken
        | PatternVariant::BassScaleApproach
        | PatternVariant::BassWalkingFragment => position / 2,
        PatternVariant::BassChromaticApproach => 0,
        PatternVariant::TextureIsolatedUpper | PatternVariant::TextureChordSparkle => 0,
        PatternVariant::TextureDelayedEcho | PatternVariant::TextureTwoNoteResponse => {
            position / 2 + 1
        }
        PatternVariant::TextureSlowArp
        | PatternVariant::TextureStaggeredExtensions
        | PatternVariant::TextureShimmer
        | PatternVariant::TextureRepeatingArp
        | PatternVariant::TextureExtensionFigure => position + 1,
        _ => 0,
    }
}

fn nearest_pitch_class_near(pitch_class: u8, target: u8) -> u8 {
    let mut best = target;
    let mut best_distance = u8::MAX;
    for note in 24_u8..=88 {
        if note % 12 != pitch_class % 12 {
            continue;
        }
        let distance = note.abs_diff(target);
        if distance < best_distance {
            best = note;
            best_distance = distance;
        }
    }
    best
}

fn nearest_allowed_note(
    pool: &PitchPool,
    preferred_pitch_class: u8,
    target: f32,
    previous_note: Option<u8>,
    movement_weight: f32,
) -> u8 {
    let mut best_note = target.round().clamp(24.0, 100.0) as u8;
    let mut best_score = f32::MAX;

    for note in 24_u8..=100 {
        if !pool.contains(note % 12) {
            continue;
        }
        let register_cost = (f32::from(note) - target).abs();
        let movement_cost = previous_note.map_or(0.0, |previous| {
            f32::from(note.abs_diff(previous)) * movement_weight
        });
        let preference_cost = if note % 12 == preferred_pitch_class {
            0.0
        } else {
            3.5
        };
        let score = register_cost + movement_cost + preference_cost;
        if score < best_score {
            best_score = score;
            best_note = note;
        }
    }
    best_note
}

fn generate_melody_plan(
    len: usize,
    behavior: Behavior,
    color: f32,
    previous: MelodyPlan,
    cycle_index: i64,
    variant: PatternVariant,
) -> MelodyPlan {
    let len = len.clamp(1, MAX_MELODY_STEPS);
    let repeat_threshold = if matches!(variant, PatternVariant::MelodyMotifVariation) {
        0.58
    } else {
        0.70
    };

    if previous.len == len
        && previous.cycle_index != i64::MIN
        && behavior.repeat >= repeat_threshold
        && cycle_index.rem_euclid(2) == 1
    {
        let mut repeated = previous;
        repeated.cycle_index = cycle_index;
        repeated.generation = previous.generation.wrapping_add(1);
        let variation = ((cycle_index.unsigned_abs() as usize * 5) + 3) % len;
        if matches!(repeated.steps[variation], MelodyIntent::Fill(_)) {
            repeated.steps[variation] = MelodyIntent::Neighbor(1);
        }
        return repeated;
    }

    let mut plan = MelodyPlan {
        steps: [MelodyIntent::Rest; MAX_MELODY_STEPS],
        len,
        cycle_index,
        generation: previous.generation.wrapping_add(1),
    };

    let quarter = (len / 4).min(len - 1);
    let half = (len / 2).min(len - 1);
    let peak = ((len * 5) / 8).min(len - 1);
    let three_quarter = ((len * 3) / 4).min(len - 1);
    let cadence = len - 1;
    let skeleton = melody_skeleton(variant);

    for (position, contour) in [
        (0, skeleton[0]),
        (quarter, skeleton[1]),
        (half, skeleton[2]),
        (peak, skeleton[3]),
        (three_quarter, skeleton[4]),
        (cadence, skeleton[5]),
    ] {
        plan.steps[position] = MelodyIntent::Skeleton(contour);
    }

    let variant_fill = match variant {
        PatternVariant::MelodySparseSkeleton => -0.28,
        PatternVariant::MelodyMotifA | PatternVariant::MelodyMotifB => -0.16,
        PatternVariant::MelodyDevelopedPhrase
        | PatternVariant::MelodyMotifVariation
        | PatternVariant::MelodyAnsweringPhrase => 0.0,
        PatternVariant::MelodyOrnamentedPhrase => 0.12,
        PatternVariant::MelodyPassingDense => 0.22,
        PatternVariant::MelodyChromaticPhrase => 0.18,
        _ => 0.0,
    };
    let fill_probability =
        (0.14 + behavior.activity * 0.70 - behavior.space * 0.22 + variant_fill).clamp(0.03, 0.96);

    for position in 1..cadence {
        if plan.steps[position] != MelodyIntent::Rest {
            continue;
        }
        let chance = deterministic_unit(
            plan.generation.wrapping_mul(0xD6E8_FEB8_6659_FD93)
                ^ (position as u64).wrapping_mul(0xA5A3_58B5),
        );
        if chance > fill_probability {
            continue;
        }

        let contour = arch_contour(position, len);
        let before_skeleton =
            position + 1 < len && matches!(plan.steps[position + 1], MelodyIntent::Skeleton(_));
        let chromatic_permission =
            color >= 0.80 || matches!(variant, PatternVariant::MelodyChromaticPhrase);

        if chromatic_permission && before_skeleton {
            let target_contour = match plan.steps[position + 1] {
                MelodyIntent::Skeleton(value) => value,
                _ => contour,
            };
            plan.steps[position] = MelodyIntent::ChromaticApproach(target_contour);
        } else if behavior.activity >= 0.68 && position % 4 == 2 {
            plan.steps[position] = MelodyIntent::Neighbor(contour);
        } else {
            plan.steps[position] = MelodyIntent::Fill(contour);
        }
    }

    plan
}

fn melody_skeleton(variant: PatternVariant) -> [i8; 6] {
    match variant {
        PatternVariant::MelodyMotifA => [0, 1, 0, 2, 1, 0],
        PatternVariant::MelodyMotifB => [0, 2, 1, 3, 1, 0],
        PatternVariant::MelodySparseSkeleton => [0, 1, 2, 4, 2, 0],
        PatternVariant::MelodyAnsweringPhrase => [2, 1, 0, 2, 1, 0],
        PatternVariant::MelodyOrnamentedPhrase => [0, 2, 3, 5, 2, 0],
        _ => [0, 1, 2, 4, 2, 0],
    }
}

fn arch_contour(position: usize, len: usize) -> i8 {
    if len <= 1 {
        return 0;
    }
    let midpoint = (len - 1) as f32 / 2.0;
    let distance = ((position as f32 - midpoint).abs() / midpoint.max(1.0)).clamp(0.0, 1.0);
    ((1.0 - distance) * 4.0).round() as i8
}

fn melody_target_note(
    intent: MelodyIntent,
    chord: Chord,
    tonal: TonalContext,
    anchor_note: u8,
    previous_note: Option<u8>,
    spread: f32,
    color: f32,
) -> u8 {
    let contour = match intent {
        MelodyIntent::Skeleton(value)
        | MelodyIntent::Fill(value)
        | MelodyIntent::Neighbor(value)
        | MelodyIntent::ChromaticApproach(value) => value,
        MelodyIntent::Rest => 0,
    };
    let stable_only = matches!(intent, MelodyIntent::Skeleton(_)) && color < 0.38;
    let mut pool = PitchPool::new();

    if stable_only || color < 0.38 {
        for interval in chord.intervals() {
            pool.push((chord.root.value() + interval) % 12);
        }
    } else {
        for interval in tonal.mode.intervals() {
            pool.push(tonal.tonic.transpose(interval).value());
        }
        for interval in chord.intervals() {
            pool.push((chord.root.value() + interval) % 12);
        }
    }
    if pool.is_empty() {
        pool.push(chord.root.value());
    }

    let base_target = f32::from(anchor_note) + 4.0 + spread.clamp(0.0, 1.0) * 10.0;
    let contour_target = base_target + f32::from(contour) * 1.7;
    let preferred = match intent {
        MelodyIntent::Skeleton(_) => pool.get(contour.unsigned_abs() as usize),
        MelodyIntent::Neighbor(_) => pool.get(contour.unsigned_abs() as usize + 1),
        MelodyIntent::Fill(_) | MelodyIntent::ChromaticApproach(_) => {
            pool.get(contour.unsigned_abs() as usize + 2)
        }
        MelodyIntent::Rest => chord.root.value(),
    };

    nearest_allowed_note(&pool, preferred, contour_target, previous_note, 0.82)
}

fn chromatic_approach_note(resolution: u8, chord: Chord, tonal: TonalContext) -> u8 {
    let mut stable = PitchPool::new();
    for interval in tonal.mode.intervals() {
        stable.push(tonal.tonic.transpose(interval).value());
    }
    for interval in chord.intervals() {
        stable.push((chord.root.value() + interval) % 12);
    }

    let lower = resolution.saturating_sub(1).max(24);
    let upper = resolution.saturating_add(1).min(100);
    let lower_is_chromatic = !stable.contains(lower % 12);
    let upper_is_chromatic = !stable.contains(upper % 12);

    match (lower_is_chromatic, upper_is_chromatic) {
        (true, _) if lower != resolution => lower,
        (_, true) if upper != resolution => upper,
        _ if lower != resolution => lower,
        _ => upper,
    }
}

fn gate_beats(
    role: VoiceRole,
    variant: PatternVariant,
    timing: StepTiming,
    space: f32,
    intensity: f32,
) -> f64 {
    if matches!(
        variant,
        PatternVariant::BassRootHold | PatternVariant::ChordsSustain
    ) {
        let cycle_beats = timing.step_length * timing.steps_per_cycle as f64;
        return cycle_beats * (0.93 - f64::from(space.clamp(0.0, 1.0)) * 0.08);
    }

    timing.step_length * gate_ratio(role, variant, space, intensity)
}

fn attack_velocity(role: VoiceRole, dynamics: f32, position: usize, intensity: f32) -> u8 {
    let role_adjustment = match role {
        VoiceRole::Bass => 6.0,
        VoiceRole::Chords => 0.0,
        VoiceRole::Texture => -8.0,
        VoiceRole::Melody => 3.0,
    };
    let accent = if position == 0 { 8.0 } else { 0.0 };
    let velocity = 30.0
        + dynamics.clamp(0.0, 1.0) * 84.0
        + role_adjustment
        + accent
        + intensity.clamp(-1.0, 1.0) * 10.0;
    velocity.round().clamp(1.0, 127.0) as u8
}

fn gate_ratio(role: VoiceRole, variant: PatternVariant, space: f32, intensity: f32) -> f64 {
    let base = match role {
        VoiceRole::Bass => 0.78,
        VoiceRole::Chords => 0.70,
        VoiceRole::Texture => 0.42,
        VoiceRole::Melody => 0.56,
    };
    let pattern_adjustment = match variant {
        PatternVariant::ChordsSinglePulse
        | PatternVariant::ChordsBackbeat
        | PatternVariant::ChordsOffbeat
        | PatternVariant::ChordsActiveStabs => -0.20,
        PatternVariant::TextureChordSparkle
        | PatternVariant::TextureShimmer
        | PatternVariant::TextureRepeatingArp => -0.12,
        PatternVariant::MelodyOrnamentedPhrase
        | PatternVariant::MelodyPassingDense
        | PatternVariant::MelodyChromaticPhrase => -0.08,
        _ => 0.0,
    };

    (base + pattern_adjustment
        - f64::from(space) * 0.22
        - f64::from(intensity.clamp(-1.0, 1.0)) * 0.04)
        .clamp(0.14, 0.96)
}

fn timing_offset_beats(role: VoiceRole) -> f64 {
    match role {
        VoiceRole::Melody => 0.0,
        VoiceRole::Bass => 0.004,
        VoiceRole::Chords => 0.010,
        VoiceRole::Texture => 0.018,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PatternFamily, PatternState, PatternVariant, PerformerEngine, ResetRule, VoiceEvent,
        family_for, variants_for,
    };
    use crate::music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};
    use crate::music::settings::MusicalSettings;
    use crate::music::voicing::Voicing;
    use crate::voice::{VoiceRole, default_voices};

    fn c_major() -> Chord {
        Chord::new(
            PitchClass::from_value(0),
            ChordQuality::Major,
            ChordExtension::Triad,
        )
    }

    #[test]
    fn density_selects_behavior_family_not_exact_pattern() {
        assert_eq!(
            family_for(VoiceRole::Bass, 0.10),
            PatternFamily::BassFoundation
        );
        assert_eq!(family_for(VoiceRole::Bass, 0.55), PatternFamily::BassMoving);
        assert_eq!(
            family_for(VoiceRole::Chords, 0.50),
            PatternFamily::ChordsComp
        );
        assert_eq!(
            family_for(VoiceRole::Texture, 0.85),
            PatternFamily::TextureActive
        );
        assert_eq!(
            family_for(VoiceRole::Melody, 0.60),
            PatternFamily::MelodyPhrase
        );
    }

    #[test]
    fn comp_family_contains_multiple_sibling_variants() {
        assert!(variants_for(PatternFamily::ChordsComp).len() >= 3);
    }

    #[test]
    fn pattern_state_stays_inside_selected_family() {
        let mut state = PatternState::new(
            PatternFamily::ChordsComp,
            PatternVariant::ChordsCharleston,
            42,
        );
        for cycle in 0..12 {
            state.update(
                PatternFamily::ChordsComp,
                VoiceRole::Chords,
                cycle,
                0.55,
                0.82,
            );
            assert!(variants_for(PatternFamily::ChordsComp).contains(&state.variant));
        }
    }

    #[test]
    fn chords_performer_emits_polyphonic_attack() {
        let settings = MusicalSettings::default();
        let voices = default_voices();
        let mut engine = PerformerEngine::new(
            c_major(),
            settings.tonal,
            Voicing {
                notes: [52, 55, 60, 64],
            },
        );
        engine.transport_started(0.0);
        let _ = engine.update(0.0, 4.0, &voices);
        let events = engine.update(0.02, 4.0, &voices);
        let chord_note_ons = events
            .iter()
            .filter(|event| matches!(event, VoiceEvent::NoteOn { voice_index: 1, .. }))
            .count();

        assert_eq!(chord_note_ons, 4);
    }

    #[test]
    fn default_reset_rules_match_new_ensemble() {
        let settings = MusicalSettings::default();
        let engine = PerformerEngine::new(
            c_major(),
            settings.tonal,
            Voicing {
                notes: [52, 55, 60, 64],
            },
        );
        let snapshots = engine.snapshots();

        assert_eq!(snapshots[0].reset_rule, ResetRule::Chord);
        assert_eq!(snapshots[1].reset_rule, ResetRule::Bar);
        assert_eq!(snapshots[2].reset_rule, ResetRule::TwoBars);
        assert_eq!(snapshots[3].reset_rule, ResetRule::Phrase);
    }

    #[test]
    fn same_frame_is_processed_only_once_after_pending_attacks_start() {
        let settings = MusicalSettings::default();
        let voices = default_voices();
        let mut engine = PerformerEngine::new(
            c_major(),
            settings.tonal,
            Voicing {
                notes: [52, 55, 60, 64],
            },
        );
        engine.transport_started(0.0);
        let _ = engine.update(0.0, 4.0, &voices);
        let first_started = engine.update(0.02, 4.0, &voices);
        let second = engine.update(0.02, 4.0, &voices);

        assert!(first_started.iter().count() > 0);
        assert_eq!(second.iter().count(), 0);
    }
}
