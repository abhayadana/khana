//! Role-aware performers with evolving pattern families and variants.
//!
//! v0.11 turns the four role generators into a lightweight ensemble.
//! Chords states every semantic chord tone exactly once. Melody phrases carry
//! rhythmic identity, varied gate lengths, and articulation. Texture uses
//! persistent cyclic cells that may continue across bar/chord boundaries.
//! An EnsembleContext gives Bass, Texture, and Melody soft awareness of one
//! another for rhythmic complement, register separation, and motion scoring.

use crate::music::chord::Chord;
use crate::music::settings::TonalContext;
use crate::music::voicing::{MAX_VOICING_NOTES, Voicing};
use crate::voice::{Voice, VoiceRole};

const VOICE_COUNT: usize = 4;
const MAX_NOTES_PER_VOICE: usize = MAX_VOICING_NOTES;
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
    BassAnchor,
    BassFoundation,
    BassMoving,
    BassConnecting,
    ChordsLongSustain,
    ChordsBarSustain,
    ChordsPulse,
    ChordsComp,
    ChordsActive,
    TextureDrone,
    TextureSparse,
    TexturePulse,
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
            Self::BassAnchor => "ANCHOR",
            Self::BassFoundation => "FOUND",
            Self::BassMoving => "MOVING",
            Self::BassConnecting => "CONNECT",
            Self::ChordsLongSustain => "LONG",
            Self::ChordsBarSustain => "BAR",
            Self::ChordsPulse => "PULSE",
            Self::ChordsComp => "COMP",
            Self::ChordsActive => "ACTIVE",
            Self::TextureDrone => "DRONE",
            Self::TextureSparse => "SPARSE",
            Self::TexturePulse => "PULSE",
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
    BassRootPedal,
    BassRootFifth,
    BassRootOctave,
    BassRootBounce,
    BassRootFifthOctave,
    BassBroken,
    BassChordToneOstinato,
    BassScaleApproach,
    BassChromaticApproach,
    BassWalkingFragment,

    ChordsFourBarHold,
    ChordsTwoBarHold,
    ChordsOneBarHold,
    ChordsTwoPulse,
    ChordsFourPulse,
    ChordsBackbeat,
    ChordsOffbeat,
    ChordsCharleston,
    ChordsAnticipationResponse,
    ChordsSyncopatedA,
    ChordsSyncopatedB,
    ChordsActiveStabs,

    TextureLongDrone,
    TextureBarDrone,
    TextureIsolatedUpper,
    TextureDelayedEcho,
    TextureHalfPulse,
    TextureQuarterPulse,
    TextureThreeStepArp,
    TextureSlowArp,
    TextureExtensionFigure,
    TextureShimmer,
    TextureRepeatingArp,
    TextureRotatingThree,

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

const BASS_ANCHOR: [PatternVariant; 2] =
    [PatternVariant::BassRootHold, PatternVariant::BassRootPedal];
const BASS_FOUNDATION: [PatternVariant; 3] = [
    PatternVariant::BassRootFifth,
    PatternVariant::BassRootOctave,
    PatternVariant::BassRootBounce,
];
const BASS_MOVING: [PatternVariant; 3] = [
    PatternVariant::BassRootFifthOctave,
    PatternVariant::BassBroken,
    PatternVariant::BassChordToneOstinato,
];
const BASS_CONNECTING: [PatternVariant; 3] = [
    PatternVariant::BassScaleApproach,
    PatternVariant::BassChromaticApproach,
    PatternVariant::BassWalkingFragment,
];
const CHORDS_LONG_SUSTAIN: [PatternVariant; 2] = [
    PatternVariant::ChordsFourBarHold,
    PatternVariant::ChordsTwoBarHold,
];
const CHORDS_BAR_SUSTAIN: [PatternVariant; 1] = [PatternVariant::ChordsOneBarHold];
const CHORDS_PULSE: [PatternVariant; 2] = [
    PatternVariant::ChordsTwoPulse,
    PatternVariant::ChordsFourPulse,
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
const TEXTURE_DRONE: [PatternVariant; 2] = [
    PatternVariant::TextureLongDrone,
    PatternVariant::TextureBarDrone,
];
const TEXTURE_SPARSE: [PatternVariant; 2] = [
    PatternVariant::TextureIsolatedUpper,
    PatternVariant::TextureDelayedEcho,
];
const TEXTURE_PULSE: [PatternVariant; 2] = [
    PatternVariant::TextureHalfPulse,
    PatternVariant::TextureQuarterPulse,
];
const TEXTURE_MOTION: [PatternVariant; 3] = [
    PatternVariant::TextureThreeStepArp,
    PatternVariant::TextureSlowArp,
    PatternVariant::TextureExtensionFigure,
];
const TEXTURE_ACTIVE: [PatternVariant; 3] = [
    PatternVariant::TextureShimmer,
    PatternVariant::TextureRepeatingArp,
    PatternVariant::TextureRotatingThree,
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
        let mut notes = [0_u8; MAX_NOTES_PER_VOICE];
        notes[0] = note;
        Self { notes, len: 1 }
    }

    fn chord(voicing: Voicing) -> Self {
        Self {
            notes: voicing.notes,
            len: voicing.len,
        }
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
    preserve_common: bool,
}

#[derive(Clone, Copy, Debug)]
struct Behavior {
    activity: f32,
    space: f32,
    repeat: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextureRelationship {
    Independent,
    Complement,
}

#[derive(Clone, Copy, Debug)]
struct PatternState {
    family: PatternFamily,
    variant: PatternVariant,
    previous_variant: PatternVariant,
    age_cycles: u8,
    last_cycle_index: i64,
    last_family_boundary: i64,
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
            last_family_boundary: i64::MIN,
            seed,
        }
    }

    fn update(
        &mut self,
        desired_family: PatternFamily,
        family_boundary: i64,
        cycle_index: i64,
        activity: f32,
        repeat: f32,
    ) -> bool {
        let boundary_changed = family_boundary != self.last_family_boundary;
        let family_changed = desired_family != self.family && boundary_changed;

        if family_changed {
            self.previous_variant = self.variant;
            self.family = desired_family;
            self.variant = choose_variant(VariantRequest {
                family: self.family,
                current: None,
                previous: self.previous_variant,
                age_cycles: 0,
                cycle_index,
                activity,
                repeat,
                seed: self.seed,
            });
            self.age_cycles = 0;
            self.last_cycle_index = cycle_index;
        } else if (cycle_index != self.last_cycle_index
            && (!matches!(
                self.family,
                PatternFamily::MelodyMotif
                    | PatternFamily::MelodyPhrase
                    | PatternFamily::MelodyOrnament
            ) || cycle_index.rem_euclid(4) == 0
                || self.last_cycle_index == i64::MIN))
            || (self.family == PatternFamily::ChordsLongSustain && boundary_changed)
        {
            let next = choose_variant(VariantRequest {
                family: self.family,
                current: Some(self.variant),
                previous: self.previous_variant,
                age_cycles: self.age_cycles,
                cycle_index,
                activity,
                repeat,
                seed: self.seed,
            });

            if next == self.variant {
                self.age_cycles = self.age_cycles.saturating_add(1);
            } else {
                self.previous_variant = self.variant;
                self.variant = next;
                self.age_cycles = 0;
            }
            self.last_cycle_index = cycle_index;
        }

        self.last_family_boundary = family_boundary;
        family_changed
    }

    fn force_next_family_boundary(&mut self) {
        self.last_family_boundary = i64::MIN;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MelodyIntent {
    Rest,
    Skeleton(i8),
    Fill(i8),
    Neighbor(i8),
    Suspension(i8),
    ModalTension(i8),
    ChromaticApproach(i8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MelodyArticulation {
    Short,
    Normal,
    Legato,
    Held,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MelodyStep {
    intent: MelodyIntent,
    gate_steps: u8,
    articulation: MelodyArticulation,
}

impl MelodyStep {
    const REST: Self = Self {
        intent: MelodyIntent::Rest,
        gate_steps: 0,
        articulation: MelodyArticulation::Short,
    };

    const fn new(intent: MelodyIntent) -> Self {
        Self {
            intent,
            gate_steps: 1,
            articulation: MelodyArticulation::Normal,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct MelodyPlan {
    steps: [MelodyStep; MAX_MELODY_STEPS],
    len: usize,
    cycle_index: i64,
    generation: u64,
}

impl MelodyPlan {
    const fn empty() -> Self {
        Self {
            steps: [MelodyStep::REST; MAX_MELODY_STEPS],
            len: 0,
            cycle_index: i64::MIN,
            generation: 0,
        }
    }

    fn step(&self, position: usize) -> MelodyStep {
        if self.len == 0 {
            MelodyStep::REST
        } else {
            self.steps[position % self.len]
        }
    }

    fn intent(&self, position: usize) -> MelodyIntent {
        self.step(position).intent
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
    legato_harmony: bool,
    last_attack_beat: Option<f64>,
    motion_direction: i8,
    pattern: PatternState,
    melody_plan: MelodyPlan,
    melody_motif: MelodyPlan,
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
            legato_harmony: false,
            last_attack_beat: None,
            motion_direction: 0,
            pattern,
            melody_plan: MelodyPlan::empty(),
            melody_motif: MelodyPlan::empty(),
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

/// Lightweight snapshot of what the other performers are doing.
///
/// This is intentionally advisory rather than a centralized composition engine:
/// each role preserves its own musical identity, then uses ensemble state as a
/// soft scoring influence.
#[derive(Clone, Copy, Debug)]
struct EnsembleContext {
    active_notes: [Option<u8>; VOICE_COUNT],
    last_attack_beat: [Option<f64>; VOICE_COUNT],
    motion_direction: [i8; VOICE_COUNT],
}

impl EnsembleContext {
    fn recent_attack(self, voice_index: usize, now_beat: f64, window: f64) -> bool {
        self.last_attack_beat[voice_index]
            .is_some_and(|beat| now_beat - beat <= window && now_beat >= beat)
    }
}

#[derive(Clone, Copy, Debug)]
struct FutureHarmony {
    chord: Chord,
    start_beat: f64,
}

/// Four independent role-aware performers sharing one active harmony.
pub struct PerformerEngine {
    chord: Chord,
    tonal: TonalContext,
    chord_voicing: Voicing,
    chord_origin_beat: f64,
    next_chord_target: Option<FutureHarmony>,
    enabled: bool,
    intensity: f32,
    states: [PerformerState; VOICE_COUNT],
    expired_attacks: u64,
    max_attack_lateness_beats: f64,
}

impl PerformerEngine {
    /// Creates a stopped performer engine around an initial harmony.
    pub const fn new(chord: Chord, tonal: TonalContext, chord_voicing: Voicing) -> Self {
        Self {
            chord,
            tonal,
            chord_voicing,
            chord_origin_beat: 0.0,
            next_chord_target: None,
            enabled: false,
            intensity: 0.0,
            expired_attacks: 0,
            max_attack_lateness_beats: 0.0,
            states: [
                PerformerState::new(PatternState::new(
                    PatternFamily::BassFoundation,
                    PatternVariant::BassRootFifth,
                    0xB455_0001,
                )),
                PerformerState::new(PatternState::new(
                    PatternFamily::ChordsBarSustain,
                    PatternVariant::ChordsOneBarHold,
                    0xC40D_0002,
                )),
                PerformerState::new(PatternState::new(
                    PatternFamily::TexturePulse,
                    PatternVariant::TextureHalfPulse,
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
        self.states[1].legato_harmony = chord != self.chord;
        self.chord = chord;
        self.tonal = tonal;
        self.chord_voicing = chord_voicing;
        self.chord_origin_beat = now_beat;
        self.next_chord_target = None;

        // Keep already-sounding notes owned so the next scheduled attack or
        // gate release can emit the correct Note Off. This is especially
        // important for Texture/Melody continuity across harmony changes.
        // Pending attacks were calculated against the old harmony, so cancel
        // those before remapping future events to the new chord.
        for state in &mut self.states {
            state.pending = None;
            state.pending_resolution = None;
        }

        self.states[0].last_step_key = None;
        self.states[0].pattern.force_next_family_boundary();
        self.states[1].pattern.force_next_family_boundary();
        self.states[0].force_attack = true;
        self.states[1].force_attack = true;
    }

    /// Changes the Chords voicing/register reference without retriggering.
    pub fn update_anchor(&mut self, tonal: TonalContext, chord_voicing: Voicing) {
        self.tonal = tonal;
        self.chord_voicing = chord_voicing;
    }

    /// Supplies a future harmony only when it is genuinely known.
    ///
    /// Live improvisation normally leaves this `None`; semantic Phrase
    /// playback can expose its next event so CONNECT Bass patterns may prepare
    /// a destination without inventing one.
    pub fn set_next_chord_target(&mut self, chord: Option<Chord>, start_beat: Option<f64>) {
        self.next_chord_target = chord
            .zip(start_beat)
            .map(|(chord, start_beat)| FutureHarmony { chord, start_beat });
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
            state.legato_harmony = false;
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

    /// Return and reset scheduler diagnostics for the reporting interval.
    pub fn take_timing_diagnostics(&mut self) -> (u64, f64) {
        (
            std::mem::take(&mut self.expired_attacks),
            std::mem::take(&mut self.max_attack_lateness_beats),
        )
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

        // First settle every already-scheduled event so EnsembleContext never
        // observes a note that should have been released this frame.
        for (index, voice) in voices.iter().enumerate() {
            // A muted voice must not emit an already-queued Note On.
            if voice.parameters.density <= 0.01 {
                self.mute_voice(index, &mut batch);
                continue;
            }
            self.release_due_notes(index, now_beat, &mut batch);
            self.start_due_attack(index, now_beat, &mut batch);
        }

        // Melody is evaluated before Texture so complementary Texture patterns
        // can respond to a Melody attack on the same transport frame.
        for index in [0_usize, 1, 3, 2] {
            let voice = voices[index];
            let role = voice.role;

            if voice.parameters.density <= 0.01 {
                continue;
            }

            let behavior = behavior_for(role, voice.parameters.density, self.intensity);
            let desired_family = family_for(role, behavior.activity);
            let family_boundary = family_boundary_index(role, now_beat, beats_per_bar);

            let active_family = self.states[index].pattern.family;
            let mut timing = step_timing(
                role,
                active_family,
                now_beat,
                beats_per_bar,
                self.chord_origin_beat,
            );

            let family_changed = self.states[index].pattern.update(
                desired_family,
                family_boundary,
                timing.cycle_index,
                behavior.activity,
                behavior.repeat,
            );

            if family_changed {
                self.stop_sounding_now(index, &mut batch);
                self.states[index].pending = None;
                timing = step_timing(
                    role,
                    self.states[index].pattern.family,
                    now_beat,
                    beats_per_bar,
                    self.chord_origin_beat,
                );
                self.states[index].pattern.last_cycle_index = timing.cycle_index;
                if role == VoiceRole::Melody {
                    self.states[index].melody_motif = MelodyPlan::empty();
                    self.states[index].melody_plan = MelodyPlan::empty();
                }
            } else if self.states[index].pattern.family != active_family {
                timing = step_timing(
                    role,
                    self.states[index].pattern.family,
                    now_beat,
                    beats_per_bar,
                    self.chord_origin_beat,
                );
            }

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

            let ensemble = self.ensemble_context();
            let forced = self.states[index].force_attack;
            self.states[index].force_attack = false;
            let preserve_common = std::mem::take(&mut self.states[index].legato_harmony)
                && matches!(
                    self.states[index].pattern.family,
                    PatternFamily::ChordsLongSustain | PatternFamily::ChordsBarSustain
                );
            if behavior.activity <= 0.01
                || (!forced
                    && !self.should_attack(index, role, timing, behavior, ensemble, now_beat))
            {
                continue;
            }

            let notes = match role {
                VoiceRole::Chords => NoteSet::chord(self.chord_voicing),
                VoiceRole::Melody => NoteSet::single(self.choose_melody_note(
                    index,
                    timing.position,
                    voice,
                    ensemble,
                )),
                VoiceRole::Bass if forced => NoteSet::single(nearest_pitch_class_near(
                    self.chord.root.value(),
                    (36.0 + voice.parameters.spread * 8.0).round() as u8,
                )),
                VoiceRole::Bass | VoiceRole::Texture => {
                    NoteSet::single(choose_role_note(RoleNoteRequest {
                        voice,
                        variant,
                        chord: self.chord,
                        tonal: self.tonal,
                        position: timing.position,
                        chord_voicing: self.chord_voicing,
                        previous_note: self.states[index].previous_note,
                        next_chord: self.next_chord_target,
                        now_beat,
                        ensemble,
                    }))
                }
            };

            let mut velocity = attack_velocity(
                role,
                voice.parameters.dynamics,
                timing.position,
                self.intensity,
            );
            if role == VoiceRole::Melody {
                let plan = &self.states[index].melody_plan;
                velocity = match plan.intent(timing.position) {
                    MelodyIntent::Skeleton(_) if timing.position == (plan.len * 5) / 8 => {
                        velocity.saturating_add(9).min(127)
                    }
                    MelodyIntent::Skeleton(_) => velocity,
                    _ => velocity.saturating_sub(9).max(1),
                };
            }
            let gate = if role == VoiceRole::Melody {
                melody_gate_beats(
                    self.states[index].melody_plan.step(timing.position),
                    timing.step_length,
                )
            } else {
                gate_beats(role, variant, timing, behavior.space, self.intensity)
            };
            let start = now_beat + timing_offset_beats(role);

            self.states[index].pending = Some(PendingAttack {
                notes,
                velocity,
                start_beat: start,
                gate_beats: gate,
                preserve_common,
            });
            self.start_due_attack(index, now_beat, &mut batch);
        }

        batch
    }

    fn ensemble_context(&self) -> EnsembleContext {
        EnsembleContext {
            active_notes: std::array::from_fn(|index| {
                self.states[index].sounding.and_then(|sounding| {
                    if index == 1 {
                        sounding.notes.iter().last()
                    } else {
                        sounding.notes.iter().next()
                    }
                })
            }),
            last_attack_beat: std::array::from_fn(|index| self.states[index].last_attack_beat),
            motion_direction: std::array::from_fn(|index| self.states[index].motion_direction),
        }
    }

    fn stop_sounding_now(&mut self, index: usize, batch: &mut VoiceEventBatch) {
        if let Some(active) = self.states[index].sounding.take() {
            for note in active.notes.iter() {
                batch.push(VoiceEvent::NoteOff {
                    voice_index: index,
                    note,
                });
            }
        }
    }

    fn mute_voice(&mut self, index: usize, batch: &mut VoiceEventBatch) {
        self.states[index].pending = None;
        self.states[index].pending_resolution = None;
        self.states[index].force_attack = false;

        self.stop_sounding_now(index, batch);
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
        self.max_attack_lateness_beats = self
            .max_attack_lateness_beats
            .max(now_beat - pending.start_beat);
        if now_beat >= pending.start_beat + pending.gate_beats {
            // A missed gate is silence, not a late burst of Note On/Off.
            self.states[index].pending = None;
            self.states[index].pending_resolution = None;
            self.expired_attacks = self.expired_attacks.saturating_add(1);
            return;
        }

        let active = self.states[index].sounding.take();
        if let Some(active) = active {
            for note in active.notes.iter() {
                if pending.preserve_common && pending.notes.iter().any(|next| next == note) {
                    continue;
                }
                batch.push(VoiceEvent::NoteOff {
                    voice_index: index,
                    note,
                });
            }
        }

        for note in pending.notes.iter() {
            if pending.preserve_common
                && active.is_some_and(|old| old.notes.iter().any(|prior| prior == note))
            {
                continue;
            }
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
        self.states[index].last_attack_beat = Some(now_beat);
        if pending.notes.len == 1 {
            let next_note = pending.notes.notes[0];
            self.states[index].motion_direction =
                self.states[index].previous_note.map_or(0, |previous| {
                    match next_note.cmp(&previous) {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    }
                });
            self.states[index].previous_note = Some(next_note);
        }
        self.states[index].pending = None;
    }

    fn should_attack(
        &self,
        index: usize,
        role: VoiceRole,
        timing: StepTiming,
        behavior: Behavior,
        ensemble: EnsembleContext,
        now_beat: f64,
    ) -> bool {
        let variant = self.states[index].pattern.variant;
        if role == VoiceRole::Melody {
            return self.states[index].melody_plan.intent(timing.position) != MelodyIntent::Rest;
        }

        let base_hit = pattern_hit(variant, timing.position, timing.steps_per_cycle);
        if !base_hit {
            return false;
        }

        if role != VoiceRole::Texture {
            return true;
        }

        // Texture rhythm is intentionally regular in v0.11. Complementary
        // patterns create space by relationship rule rather than random holes.
        if texture_relationship(variant, behavior) == TextureRelationship::Complement
            && ensemble.recent_attack(3, now_beat, 0.125)
            && !matches!(
                variant,
                PatternVariant::TextureLongDrone | PatternVariant::TextureBarDrone
            )
        {
            return false;
        }

        true
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

        let len = timing.steps_per_cycle.min(MAX_MELODY_STEPS);
        let group = timing.cycle_index.div_euclid(4);
        if state.melody_motif.len != len || state.melody_motif.cycle_index != group {
            state.melody_motif =
                generate_melody_plan(len, behavior, color, state.melody_motif, group, variant);
        }
        state.melody_plan = vary_melody_plan(state.melody_motif, timing.cycle_index, variant);
    }

    fn choose_melody_note(
        &mut self,
        index: usize,
        position: usize,
        voice: Voice,
        ensemble: EnsembleContext,
    ) -> u8 {
        if let Some(resolution) = self.states[index].pending_resolution.take() {
            return resolution;
        }

        let mut intent = self.states[index].melody_plan.intent(position);
        if let MelodyIntent::ChromaticApproach(contour) = intent
            && voice.parameters.color >= 0.80
        {
            let resolution = melody_target_note(
                MelodyIntent::Skeleton(contour),
                self.chord,
                self.tonal,
                self.chord_voicing.top_note(),
                self.states[index].previous_note,
                voice.parameters.spread,
                NoteScoringContext {
                    role: VoiceRole::Melody,
                    color: 0.45,
                    ensemble,
                },
            );
            self.states[index].pending_resolution = Some(resolution);
            return chromatic_approach_note(resolution, self.chord, self.tonal);
        }
        if let MelodyIntent::ChromaticApproach(contour) = intent {
            intent = MelodyIntent::Neighbor(contour);
        }

        melody_target_note(
            intent,
            self.chord,
            self.tonal,
            self.chord_voicing.top_note(),
            self.states[index].previous_note,
            voice.parameters.spread,
            NoteScoringContext {
                role: VoiceRole::Melody,
                color: voice.parameters.color,
                ensemble,
            },
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
    next_chord: Option<FutureHarmony>,
    now_beat: f64,
    ensemble: EnsembleContext,
}

fn variants_for(family: PatternFamily) -> &'static [PatternVariant] {
    match family {
        PatternFamily::BassAnchor => &BASS_ANCHOR,
        PatternFamily::BassFoundation => &BASS_FOUNDATION,
        PatternFamily::BassMoving => &BASS_MOVING,
        PatternFamily::BassConnecting => &BASS_CONNECTING,
        PatternFamily::ChordsLongSustain => &CHORDS_LONG_SUSTAIN,
        PatternFamily::ChordsBarSustain => &CHORDS_BAR_SUSTAIN,
        PatternFamily::ChordsPulse => &CHORDS_PULSE,
        PatternFamily::ChordsComp => &CHORDS_COMP,
        PatternFamily::ChordsActive => &CHORDS_ACTIVE,
        PatternFamily::TextureDrone => &TEXTURE_DRONE,
        PatternFamily::TextureSparse => &TEXTURE_SPARSE,
        PatternFamily::TexturePulse => &TEXTURE_PULSE,
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
    let local = family_local_activity(request.family, request.activity);
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
        VoiceRole::Bass if activity < 0.16 => PatternFamily::BassAnchor,
        VoiceRole::Bass if activity < 0.42 => PatternFamily::BassFoundation,
        VoiceRole::Bass if activity < 0.78 => PatternFamily::BassMoving,
        VoiceRole::Bass => PatternFamily::BassConnecting,

        VoiceRole::Chords if activity < 0.14 => PatternFamily::ChordsLongSustain,
        VoiceRole::Chords if activity < 0.34 => PatternFamily::ChordsBarSustain,
        VoiceRole::Chords if activity < 0.58 => PatternFamily::ChordsPulse,
        VoiceRole::Chords if activity < 0.82 => PatternFamily::ChordsComp,
        VoiceRole::Chords => PatternFamily::ChordsActive,

        VoiceRole::Texture if activity < 0.12 => PatternFamily::TextureDrone,
        VoiceRole::Texture if activity < 0.30 => PatternFamily::TextureSparse,
        VoiceRole::Texture if activity < 0.52 => PatternFamily::TexturePulse,
        VoiceRole::Texture if activity < 0.78 => PatternFamily::TextureMotion,
        VoiceRole::Texture => PatternFamily::TextureActive,

        VoiceRole::Melody if activity < 0.30 => PatternFamily::MelodyMotif,
        VoiceRole::Melody if activity < 0.76 => PatternFamily::MelodyPhrase,
        VoiceRole::Melody => PatternFamily::MelodyOrnament,
    }
}

fn family_local_activity(family: PatternFamily, activity: f32) -> f32 {
    let (low, high) = match family {
        PatternFamily::BassAnchor => (0.0, 0.16),
        PatternFamily::BassFoundation => (0.16, 0.42),
        PatternFamily::BassMoving => (0.42, 0.78),
        PatternFamily::BassConnecting => (0.78, 1.0),
        PatternFamily::ChordsLongSustain => (0.0, 0.14),
        PatternFamily::ChordsBarSustain => (0.14, 0.34),
        PatternFamily::ChordsPulse => (0.34, 0.58),
        PatternFamily::ChordsComp => (0.58, 0.82),
        PatternFamily::ChordsActive => (0.82, 1.0),
        PatternFamily::TextureDrone => (0.0, 0.12),
        PatternFamily::TextureSparse => (0.12, 0.30),
        PatternFamily::TexturePulse => (0.30, 0.52),
        PatternFamily::TextureMotion => (0.52, 0.78),
        PatternFamily::TextureActive => (0.78, 1.0),
        PatternFamily::MelodyMotif => (0.0, 0.30),
        PatternFamily::MelodyPhrase => (0.30, 0.76),
        PatternFamily::MelodyOrnament => (0.76, 1.0),
    };

    ((activity - low) / (high - low)).clamp(0.0, 1.0)
}

fn family_boundary_index(role: VoiceRole, now_beat: f64, beats_per_bar: f64) -> i64 {
    let bar = beats_per_bar.max(0.25);
    match role {
        VoiceRole::Bass | VoiceRole::Chords | VoiceRole::Texture => (now_beat / bar).floor() as i64,
        VoiceRole::Melody => (now_beat / (bar * 2.0)).floor() as i64,
    }
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
            let cycle_beats = if family == PatternFamily::ChordsLongSustain {
                safe_bar * 4.0
            } else {
                safe_bar
            };
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
        VoiceRole::Texture => {
            let cycle_beats = safe_bar * 2.0;
            let steps = cycle_steps(cycle_beats, step_length);
            StepTiming {
                step_key: absolute_step,
                // Use absolute phase so odd-length Texture cells continue
                // through bar and harmony boundaries instead of restarting.
                position: absolute_step.rem_euclid(96) as usize,
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
        PatternFamily::TextureDrone => 0.5,
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
        PatternVariant::BassRootPedal => position == 0,
        PatternVariant::BassRootFifth => position == 0 || position == half,
        PatternVariant::BassRootOctave => position == 0 || position == half,
        PatternVariant::BassRootBounce => matches!(position % 8, 0 | 2 | 4 | 6),
        PatternVariant::BassRootFifthOctave => {
            position == 0 || position == quarter || position == half
        }
        PatternVariant::BassBroken => matches!(position % 8, 0 | 3 | 5 | 7),
        PatternVariant::BassChordToneOstinato => matches!(position % 8, 0 | 2 | 4 | 6),
        PatternVariant::BassScaleApproach => matches!(position % 8, 0 | 4 | 6 | 7),
        PatternVariant::BassChromaticApproach => matches!(position % 8, 0 | 4 | 7),
        PatternVariant::BassWalkingFragment => true,

        PatternVariant::ChordsFourBarHold => position == 0,
        PatternVariant::ChordsTwoBarHold => position == 0 || position == half,
        PatternVariant::ChordsOneBarHold => position == 0,
        PatternVariant::ChordsTwoPulse => position == 0 || position == half,
        PatternVariant::ChordsFourPulse => position.is_multiple_of(2),
        PatternVariant::ChordsBackbeat => position == quarter || position == quarter * 3,
        PatternVariant::ChordsOffbeat => !position.is_multiple_of(2),
        PatternVariant::ChordsCharleston => position == 0 || position == quarter + 1,
        PatternVariant::ChordsAnticipationResponse => position == half || position == last,
        PatternVariant::ChordsSyncopatedA => matches!(position % 8, 0 | 3 | 6),
        PatternVariant::ChordsSyncopatedB => matches!(position % 8, 1 | 4 | 7),
        PatternVariant::ChordsActiveStabs => matches!(position % 8, 0 | 2 | 3 | 5 | 7),

        PatternVariant::TextureLongDrone => position.is_multiple_of(steps.max(1)),
        PatternVariant::TextureBarDrone => position.is_multiple_of((steps / 2).max(1)),
        PatternVariant::TextureIsolatedUpper => position.is_multiple_of(steps.max(1)),
        PatternVariant::TextureDelayedEcho => matches!(position % 8, 3 | 7),
        PatternVariant::TextureHalfPulse => position.is_multiple_of(4),
        PatternVariant::TextureQuarterPulse => position.is_multiple_of(2),
        PatternVariant::TextureThreeStepArp => true,
        PatternVariant::TextureSlowArp => position.is_multiple_of(2),
        PatternVariant::TextureExtensionFigure => matches!(position % 8, 0 | 3 | 5 | 7),
        PatternVariant::TextureShimmer => position.is_multiple_of(2),
        PatternVariant::TextureRepeatingArp => position % 4 != 3,
        PatternVariant::TextureRotatingThree => true,

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

fn texture_relationship(variant: PatternVariant, behavior: Behavior) -> TextureRelationship {
    if matches!(
        variant,
        PatternVariant::TextureDelayedEcho
            | PatternVariant::TextureExtensionFigure
            | PatternVariant::TextureRotatingThree
    ) || behavior.space >= 0.62
    {
        TextureRelationship::Complement
    } else {
        TextureRelationship::Independent
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
            // Chords uses the complete semantic-chord voicing rather than a pool.
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
        VoiceRole::Texture => f32::from(request.chord_voicing.top_note()) + 3.0 + spread * 10.0,
        VoiceRole::Melody => f32::from(request.chord_voicing.top_note()) + 5.0 + spread * 12.0,
        VoiceRole::Chords => f32::from(request.chord_voicing.note(1)),
    };
    let preferred_index = pattern_note_offset(request.variant, request.position);
    let preferred = pool.get(preferred_index);
    let preferred_weight = match request.variant {
        PatternVariant::BassChordToneOstinato | PatternVariant::BassRootBounce => 9.0,
        PatternVariant::TextureThreeStepArp | PatternVariant::TextureRotatingThree => 6.0,
        _ => 3.5,
    };
    let movement_weight = match role {
        VoiceRole::Bass => 0.24,
        VoiceRole::Texture => 0.68,
        VoiceRole::Melody => 0.82,
        VoiceRole::Chords => 0.0,
    };
    let mut note = nearest_allowed_note(
        &pool,
        preferred,
        target,
        request.previous_note,
        movement_weight,
        preferred_weight,
        NoteScoringContext {
            role,
            color: request.voice.parameters.color,
            ensemble: request.ensemble,
        },
    );

    if matches!(request.variant, PatternVariant::BassRootOctave) && request.position > 0 {
        note = note.saturating_add(12).min(88);
    }
    if matches!(request.variant, PatternVariant::BassRootFifthOctave) && request.position >= 4 {
        note = note.saturating_add(12).min(88);
    }
    if let Some(future) = request.next_chord
        && future.start_beat >= request.now_beat
        && future.start_beat - request.now_beat <= 1.0
        && matches!(
            request.variant,
            PatternVariant::BassScaleApproach
                | PatternVariant::BassChromaticApproach
                | PatternVariant::BassWalkingFragment
        )
        && request.position % 8 == 7
    {
        let destination = nearest_pitch_class_near(future.chord.root.value(), target as u8);
        note = if matches!(request.variant, PatternVariant::BassChromaticApproach)
            && request.voice.parameters.color >= 0.80
        {
            destination.saturating_sub(1).max(24)
        } else {
            diatonic_approach_note(destination, request.tonal, request.previous_note)
        };
    } else if matches!(request.variant, PatternVariant::BassChromaticApproach)
        && request.voice.parameters.color >= 0.80
        && request.position % 8 == 7
    {
        // With no known future chord, CONNECT remains local to the current
        // harmony rather than fabricating a destination.
        let root_target = nearest_pitch_class_near(request.chord.root.value(), target as u8);
        note = root_target.saturating_sub(1).max(24);
    }

    note
}

fn diatonic_approach_note(destination: u8, tonal: TonalContext, previous_note: Option<u8>) -> u8 {
    let mut best = destination.saturating_sub(2).max(24);
    let mut best_score = f32::MAX;

    for candidate in 24_u8..=88 {
        if candidate == destination {
            continue;
        }
        let in_scale = tonal
            .mode
            .intervals()
            .iter()
            .any(|interval| tonal.tonic.transpose(*interval).value() == candidate % 12);
        if !in_scale {
            continue;
        }

        let destination_distance = f32::from(candidate.abs_diff(destination));
        if destination_distance > 4.0 {
            continue;
        }
        let movement = previous_note.map_or(0.0, |previous| {
            f32::from(candidate.abs_diff(previous)) * 0.25
        });
        let score = destination_distance + movement;
        if score < best_score {
            best = candidate;
            best_score = score;
        }
    }

    best
}

fn pattern_note_offset(variant: PatternVariant, position: usize) -> usize {
    match variant {
        PatternVariant::BassRootHold | PatternVariant::BassRootPedal => 0,
        PatternVariant::BassRootFifth => {
            if position == 0 {
                0
            } else {
                1
            }
        }
        PatternVariant::BassRootOctave => 0,
        PatternVariant::BassRootBounce => {
            if position.is_multiple_of(4) {
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
        PatternVariant::BassChordToneOstinato => [0, 1, 0, 2][(position / 2) % 4],
        PatternVariant::BassBroken
        | PatternVariant::BassScaleApproach
        | PatternVariant::BassWalkingFragment => position / 2,
        PatternVariant::BassChromaticApproach => 0,
        PatternVariant::TextureLongDrone
        | PatternVariant::TextureBarDrone
        | PatternVariant::TextureIsolatedUpper => 0,
        PatternVariant::TextureDelayedEcho => position / 2 + 1,
        PatternVariant::TextureHalfPulse | PatternVariant::TextureQuarterPulse => position / 2,
        PatternVariant::TextureThreeStepArp | PatternVariant::TextureRotatingThree => position % 3,
        PatternVariant::TextureSlowArp
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

/// Performer-specific inputs for soft contrapuntal note scoring.
#[derive(Clone, Copy, Debug)]
struct NoteScoringContext {
    role: VoiceRole,
    color: f32,
    ensemble: EnsembleContext,
}

fn nearest_allowed_note(
    pool: &PitchPool,
    preferred_pitch_class: u8,
    target: f32,
    previous_note: Option<u8>,
    movement_weight: f32,
    preferred_weight: f32,
    scoring: NoteScoringContext,
) -> u8 {
    let NoteScoringContext {
        role,
        color,
        ensemble,
    } = scoring;
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
            preferred_weight
        };
        let ensemble_cost = counterpoint_cost(note, previous_note, role, color, ensemble);
        let score = register_cost + movement_cost + preference_cost + ensemble_cost;
        if score < best_score {
            best_score = score;
            best_note = note;
        }
    }
    best_note
}

fn counterpoint_cost(
    note: u8,
    previous_note: Option<u8>,
    role: VoiceRole,
    color: f32,
    ensemble: EnsembleContext,
) -> f32 {
    let other_index = match role {
        VoiceRole::Texture => 3,
        VoiceRole::Melody => 2,
        VoiceRole::Bass => 3,
        VoiceRole::Chords => return 0.0,
    };
    let Some(other) = ensemble.active_notes[other_index] else {
        return 0.0;
    };

    let strength = match role {
        VoiceRole::Texture => 1.0,
        VoiceRole::Melody => 0.82,
        VoiceRole::Bass => 0.42,
        VoiceRole::Chords => 0.0,
    };
    let color = color.clamp(0.0, 1.0);
    let absolute_distance = note.abs_diff(other);
    let raw_pc_interval = (note % 12).abs_diff(other % 12);
    let interval_class = raw_pc_interval.min(12_u8.saturating_sub(raw_pc_interval));

    let mut cost = 0.0;

    // Same-register collisions are the strongest generic avoidance rule.
    if absolute_distance <= 2 {
        cost += (8.0 - color * 4.0) * strength;
    }

    // Dissonance remains available as Color increases; it is discouraged,
    // not prohibited.
    cost += match interval_class {
        0 => (2.2 - color * 1.5) * strength,
        1 => (4.5 - color * 3.8) * strength,
        2 => (2.2 - color * 1.8) * strength,
        6 => (2.8 - color * 2.2) * strength,
        _ => 0.0,
    };

    if let Some(previous) = previous_note {
        let candidate_motion = match note.cmp(&previous) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
        let other_motion = ensemble.motion_direction[other_index];

        if candidate_motion != 0 && other_motion != 0 {
            if candidate_motion == -other_motion {
                cost -= 1.1 * strength;
            } else if candidate_motion == other_motion {
                cost += 0.35 * strength;

                // Repeated exposed perfect parallel motion gets a modest
                // penalty, never a hard classical prohibition.
                if matches!(interval_class, 0 | 5) {
                    cost += 0.8 * strength;
                }
            }
        }
    }

    cost
}

fn melody_cadence_position(len: usize) -> usize {
    if len <= 1 {
        return 0;
    }

    // Leave roughly one quarter-note beat after the cadence so a held arrival
    // is audible before the next two-bar phrase begins. MelodyOrnament uses a
    // sixteenth-note grid; the other Melody families use eighth notes.
    let trailing_steps = if len >= 24 { 4 } else { 2 };
    len.saturating_sub(trailing_steps).min(len - 1)
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
    let mut plan = MelodyPlan {
        steps: [MelodyStep::REST; MAX_MELODY_STEPS],
        len,
        cycle_index,
        generation: previous.generation.wrapping_add(1),
    };

    let quarter = (len / 4).min(len - 1);
    let half = (len / 2).min(len - 1);
    let peak = ((len * 5) / 8).min(len - 1);
    let three_quarter = ((len * 3) / 4).min(len - 1);
    let cadence = melody_cadence_position(len);
    let skeleton = melody_skeleton(variant);

    for (position, contour) in [
        (0, skeleton[0]),
        (quarter, skeleton[1]),
        (half, skeleton[2]),
        (peak, skeleton[3]),
        (three_quarter, skeleton[4]),
        (cadence, skeleton[5]),
    ] {
        plan.steps[position] = MelodyStep::new(MelodyIntent::Skeleton(contour));
    }

    let variant_fill = match variant {
        PatternVariant::MelodySparseSkeleton => -0.32,
        PatternVariant::MelodyMotifA | PatternVariant::MelodyMotifB => -0.16,
        PatternVariant::MelodyDevelopedPhrase
        | PatternVariant::MelodyMotifVariation
        | PatternVariant::MelodyAnsweringPhrase => 0.0,
        PatternVariant::MelodyOrnamentedPhrase => 0.14,
        PatternVariant::MelodyPassingDense => 0.24,
        PatternVariant::MelodyChromaticPhrase => 0.18,
        _ => 0.0,
    };
    let base_fill =
        (0.14 + behavior.activity * 0.70 - behavior.space * 0.22 + variant_fill).clamp(0.02, 0.96);

    let mut position = 1;
    while position < cadence {
        if plan.steps[position].intent != MelodyIntent::Rest {
            position += 1;
            continue;
        }

        let fill_probability =
            (base_fill + melody_rhythm_bias(variant, position, len)).clamp(0.01, 0.98);
        let chance = deterministic_unit(
            plan.generation.wrapping_mul(0xD6E8_FEB8_6659_FD93)
                ^ (position as u64).wrapping_mul(0xA5A3_58B5),
        );
        if chance > fill_probability {
            position += 1;
            continue;
        }

        let contour = arch_contour(position, len);
        let before_skeleton = position + 1 < len
            && matches!(plan.steps[position + 1].intent, MelodyIntent::Skeleton(_));
        let chromatic_permission = color >= 0.80;

        let intent = if chromatic_permission && before_skeleton {
            let target_contour = match plan.steps[position + 1].intent {
                MelodyIntent::Skeleton(value) => value,
                _ => contour,
            };
            MelodyIntent::ChromaticApproach(target_contour)
        } else if color >= 0.58 && before_skeleton {
            MelodyIntent::Suspension(contour)
        } else if color >= 0.68 && position % 5 == 3 {
            MelodyIntent::ModalTension(contour)
        } else if (matches!(variant, PatternVariant::MelodyOrnamentedPhrase) && position % 3 == 1)
            || (behavior.activity >= 0.68 && position % 4 == 2)
        {
            MelodyIntent::Neighbor(contour)
        } else {
            MelodyIntent::Fill(contour)
        };

        plan.steps[position] = MelodyStep::new(intent);
        position += 1;
    }

    apply_melody_articulation(&mut plan, variant);
    plan
}

fn vary_melody_plan(mut plan: MelodyPlan, cycle: i64, variant: PatternVariant) -> MelodyPlan {
    plan.cycle_index = cycle;
    let peak = (plan.len * 5) / 8;
    let cadence = melody_cadence_position(plan.len);
    match cycle.rem_euclid(4) {
        // Keep rhythmic identity and change one real note, even in a sparse motif.
        1 => {
            let variation = (1..peak).rev().find(|&index| {
                matches!(
                    plan.steps[index].intent,
                    MelodyIntent::Fill(_) | MelodyIntent::Neighbor(_)
                )
            });
            if let Some(index) = variation {
                plan.steps[index].intent = match plan.steps[index].intent {
                    MelodyIntent::Fill(contour) => MelodyIntent::Neighbor(contour),
                    MelodyIntent::Neighbor(contour) => MelodyIntent::Fill(contour),
                    intent => intent,
                };
            } else if let MelodyIntent::Skeleton(contour) = plan.steps[peak].intent {
                plan.steps[peak].intent = MelodyIntent::Skeleton(contour + 1);
            }
        }
        // Answer the opening with a lower peak and a breath before the cadence.
        3 if peak < cadence => {
            if let MelodyIntent::Skeleton(contour) = plan.steps[peak].intent {
                plan.steps[peak].intent = MelodyIntent::Skeleton(contour.saturating_sub(2).max(0));
            }
            if let Some(index) = ((peak + 1)..cadence).rev().find(|&index| {
                !matches!(
                    plan.steps[index].intent,
                    MelodyIntent::Rest | MelodyIntent::Skeleton(_)
                )
            }) {
                plan.steps[index] = MelodyStep::REST;
            }
            apply_melody_articulation(&mut plan, variant);
        }
        _ => {}
    }
    plan
}

/// Phrase-level rhythmic identity layered on top of Density.
///
/// These are not exposed as extra UI modes: the selected Melody variant keeps
/// a recognizable rhythmic character while Density controls overall activity.
fn melody_rhythm_bias(variant: PatternVariant, position: usize, len: usize) -> f32 {
    let half = len / 2;
    let peak = (len * 5) / 8;
    match variant {
        // CALL: more activity in the opening statement, then space.
        PatternVariant::MelodyMotifA => {
            if position < half {
                0.08
            } else {
                -0.20
            }
        }
        // PULSE: a recognizable alternating rhythmic lattice.
        PatternVariant::MelodyMotifB => {
            if position.is_multiple_of(2) {
                0.16
            } else {
                -0.20
            }
        }
        // SPARSE ARCH: structural notes do most of the work.
        PatternVariant::MelodySparseSkeleton => -0.34,
        // ARCH: modest activity around the climb and peak.
        PatternVariant::MelodyDevelopedPhrase => {
            if position.abs_diff(peak) <= len / 8 {
                0.10
            } else {
                0.0
            }
        }
        // WAVE / motif development: recurring internal rhythmic landmarks.
        PatternVariant::MelodyMotifVariation => {
            if position % 4 == 1 {
                0.12
            } else {
                -0.02
            }
        }
        // ANSWER: leave room early, become more active in the response half.
        PatternVariant::MelodyAnsweringPhrase => {
            if position < half {
                -0.14
            } else {
                0.18
            }
        }
        // PASSING / FLOURISH families increase local connective activity.
        PatternVariant::MelodyOrnamentedPhrase => {
            if position.abs_diff(peak) <= len / 6 {
                0.26
            } else {
                0.02
            }
        }
        PatternVariant::MelodyPassingDense => 0.28,
        PatternVariant::MelodyChromaticPhrase if position + 1 < len => 0.18,
        _ => 0.0,
    }
}

fn apply_melody_articulation(plan: &mut MelodyPlan, variant: PatternVariant) {
    let mut position = 0;
    while position < plan.len {
        let intent = plan.steps[position].intent;
        if intent == MelodyIntent::Rest {
            position += 1;
            continue;
        }

        let desired_gate = melody_desired_gate_steps(intent, variant, position, plan.len);
        let next_attack =
            ((position + 1)..plan.len).find(|next| plan.steps[*next].intent != MelodyIntent::Rest);
        let gate_steps = next_attack.map_or(desired_gate, |next| {
            desired_gate.min((next - position).max(1))
        });

        plan.steps[position].gate_steps = gate_steps.clamp(1, usize::from(u8::MAX)) as u8;
        plan.steps[position].articulation =
            melody_articulation(intent, gate_steps, position, plan.len);
        position += 1;
    }
}

fn melody_desired_gate_steps(
    intent: MelodyIntent,
    variant: PatternVariant,
    position: usize,
    len: usize,
) -> usize {
    match intent {
        MelodyIntent::Rest => 0,
        MelodyIntent::ChromaticApproach(_) | MelodyIntent::Neighbor(_) => 1,
        MelodyIntent::Fill(_) => {
            if matches!(variant, PatternVariant::MelodyMotifB) && position.is_multiple_of(2) {
                2
            } else {
                1
            }
        }
        MelodyIntent::Suspension(_) => 3,
        MelodyIntent::ModalTension(_) => 3,
        MelodyIntent::Skeleton(_) if position == melody_cadence_position(len) => 6,
        MelodyIntent::Skeleton(_) if position.abs_diff((len * 5) / 8) <= 1 => 4,
        MelodyIntent::Skeleton(_) if matches!(variant, PatternVariant::MelodySparseSkeleton) => 4,
        MelodyIntent::Skeleton(_) => 2,
    }
}

fn melody_articulation(
    intent: MelodyIntent,
    gate_steps: usize,
    position: usize,
    len: usize,
) -> MelodyArticulation {
    match intent {
        MelodyIntent::Rest => MelodyArticulation::Short,
        MelodyIntent::ChromaticApproach(_) | MelodyIntent::Neighbor(_) => MelodyArticulation::Short,
        MelodyIntent::Suspension(_) | MelodyIntent::ModalTension(_) => MelodyArticulation::Legato,
        MelodyIntent::Skeleton(_)
            if position == melody_cadence_position(len) || gate_steps >= 4 =>
        {
            MelodyArticulation::Held
        }
        MelodyIntent::Skeleton(_) if gate_steps >= 2 => MelodyArticulation::Legato,
        MelodyIntent::Fill(_) | MelodyIntent::Skeleton(_) => MelodyArticulation::Normal,
    }
}

fn melody_gate_beats(step: MelodyStep, step_length: f64) -> f64 {
    if step.intent == MelodyIntent::Rest || step.gate_steps == 0 {
        return 0.0;
    }

    let ratio = match step.articulation {
        MelodyArticulation::Short => 0.54,
        MelodyArticulation::Normal => 0.78,
        MelodyArticulation::Legato => 0.94,
        MelodyArticulation::Held => 0.985,
    };

    f64::from(step.gate_steps) * step_length * ratio
}

fn melody_skeleton(variant: PatternVariant) -> [i8; 6] {
    match variant {
        PatternVariant::MelodyMotifA => [0, 1, 0, 2, 1, 0],
        PatternVariant::MelodyMotifB => [0, 2, 1, 3, 1, 0],
        PatternVariant::MelodySparseSkeleton => [0, 1, 2, 4, 2, 0],
        PatternVariant::MelodyAnsweringPhrase => [2, 1, 0, 2, 1, 0],
        PatternVariant::MelodyMotifVariation => [0, 2, 1, 3, 2, 0],
        PatternVariant::MelodyOrnamentedPhrase => [0, 2, 3, 5, 2, 0],
        PatternVariant::MelodyPassingDense => [0, 1, 3, 4, 2, 0],
        PatternVariant::MelodyChromaticPhrase => [0, 2, 2, 5, 1, 0],
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
    scoring: NoteScoringContext,
) -> u8 {
    let color = scoring.color;
    let contour = match intent {
        MelodyIntent::Skeleton(value)
        | MelodyIntent::Fill(value)
        | MelodyIntent::Neighbor(value)
        | MelodyIntent::Suspension(value)
        | MelodyIntent::ModalTension(value)
        | MelodyIntent::ChromaticApproach(value) => value,
        MelodyIntent::Rest => 0,
    };
    let stable_only = matches!(intent, MelodyIntent::Skeleton(_));
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
        MelodyIntent::Neighbor(_) | MelodyIntent::Suspension(_) => {
            pool.get(contour.unsigned_abs() as usize + 1)
        }
        MelodyIntent::ModalTension(_)
        | MelodyIntent::Fill(_)
        | MelodyIntent::ChromaticApproach(_) => pool.get(contour.unsigned_abs() as usize + 2),
        MelodyIntent::Rest => chord.root.value(),
    };

    nearest_allowed_note(
        &pool,
        preferred,
        contour_target,
        previous_note,
        0.82,
        if stable_only { 6.0 } else { 3.5 },
        scoring,
    )
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
    let cycle_beats = timing.step_length * timing.steps_per_cycle as f64;
    match variant {
        PatternVariant::BassRootHold => {
            return cycle_beats * (0.94 - f64::from(space.clamp(0.0, 1.0)) * 0.06);
        }
        PatternVariant::ChordsFourBarHold => return cycle_beats * 0.985,
        PatternVariant::ChordsTwoBarHold => return cycle_beats * 0.49,
        PatternVariant::ChordsOneBarHold => return cycle_beats * 0.97,
        PatternVariant::TextureLongDrone => return cycle_beats * 0.985,
        PatternVariant::TextureBarDrone => return cycle_beats * 0.49,
        _ => {}
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
        VoiceRole::Texture => 0.78,
        // Melody has its own phrase-step articulation in v0.11.
        VoiceRole::Melody => 0.78,
    };
    let pattern_adjustment = match variant {
        PatternVariant::ChordsTwoPulse
        | PatternVariant::ChordsBackbeat
        | PatternVariant::ChordsOffbeat
        | PatternVariant::ChordsActiveStabs => -0.20,

        PatternVariant::TextureIsolatedUpper | PatternVariant::TextureDelayedEcho => -0.18,
        PatternVariant::TextureHalfPulse | PatternVariant::TextureQuarterPulse => -0.10,
        PatternVariant::TextureThreeStepArp | PatternVariant::TextureSlowArp => 0.08,
        PatternVariant::TextureExtensionFigure => -0.04,
        PatternVariant::TextureShimmer => -0.38,
        PatternVariant::TextureRepeatingArp => -0.12,
        PatternVariant::TextureRotatingThree => -0.18,

        _ => 0.0,
    };

    (base + pattern_adjustment
        - f64::from(space) * 0.12
        - f64::from(intensity.clamp(-1.0, 1.0)) * 0.04)
        .clamp(0.14, 0.98)
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
        assert_eq!(family_for(VoiceRole::Bass, 0.10), PatternFamily::BassAnchor);
        assert_eq!(family_for(VoiceRole::Bass, 0.55), PatternFamily::BassMoving);
        assert_eq!(
            family_for(VoiceRole::Chords, 0.10),
            PatternFamily::ChordsLongSustain
        );
        assert_eq!(
            family_for(VoiceRole::Chords, 0.50),
            PatternFamily::ChordsPulse
        );
        assert_eq!(
            family_for(VoiceRole::Chords, 0.70),
            PatternFamily::ChordsComp
        );
        assert_eq!(
            family_for(VoiceRole::Texture, 0.08),
            PatternFamily::TextureDrone
        );
        assert_eq!(
            family_for(VoiceRole::Texture, 0.40),
            PatternFamily::TexturePulse
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
            state.update(PatternFamily::ChordsComp, cycle, cycle, 0.70, 0.82);
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
            Voicing::from_slice(&[52, 55, 60]),
        );
        engine.transport_started(0.0);
        let _ = engine.update(0.0, 4.0, &voices);
        let events = engine.update(0.02, 4.0, &voices);
        let chord_note_ons = events
            .iter()
            .filter(|event| matches!(event, VoiceEvent::NoteOn { voice_index: 1, .. }))
            .count();

        assert_eq!(chord_note_ons, 3);
    }

    #[test]
    fn default_reset_rules_match_new_ensemble() {
        let settings = MusicalSettings::default();
        let engine = PerformerEngine::new(
            c_major(),
            settings.tonal,
            Voicing::from_slice(&[52, 55, 60]),
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
            Voicing::from_slice(&[52, 55, 60]),
        );
        engine.transport_started(0.0);
        let _ = engine.update(0.0, 4.0, &voices);
        let first_started = engine.update(0.02, 4.0, &voices);
        let second = engine.update(0.02, 4.0, &voices);

        assert!(first_started.iter().count() > 0);
        assert_eq!(second.iter().count(), 0);
    }

    #[test]
    fn chord_density_continuum_includes_sustain_pulse_comp_and_active() {
        assert_eq!(
            family_for(VoiceRole::Chords, 0.08),
            PatternFamily::ChordsLongSustain
        );
        assert_eq!(
            family_for(VoiceRole::Chords, 0.22),
            PatternFamily::ChordsBarSustain
        );
        assert_eq!(
            family_for(VoiceRole::Chords, 0.48),
            PatternFamily::ChordsPulse
        );
        assert_eq!(
            family_for(VoiceRole::Chords, 0.70),
            PatternFamily::ChordsComp
        );
        assert_eq!(
            family_for(VoiceRole::Chords, 0.92),
            PatternFamily::ChordsActive
        );
    }

    #[test]
    fn family_change_waits_for_new_density_boundary() {
        let mut state = PatternState::new(
            PatternFamily::ChordsBarSustain,
            PatternVariant::ChordsOneBarHold,
            7,
        );
        state.update(PatternFamily::ChordsBarSustain, 0, 0, 0.20, 0.82);
        state.update(PatternFamily::ChordsComp, 0, 0, 0.70, 0.82);
        assert_eq!(state.family, PatternFamily::ChordsBarSustain);

        state.update(PatternFamily::ChordsComp, 1, 1, 0.70, 0.82);
        assert_eq!(state.family, PatternFamily::ChordsComp);
    }

    #[test]
    fn texture_absolute_phase_does_not_restart_at_bar_boundary() {
        let before = super::step_timing(
            VoiceRole::Texture,
            PatternFamily::TextureMotion,
            3.5,
            4.0,
            0.0,
        );
        let boundary = super::step_timing(
            VoiceRole::Texture,
            PatternFamily::TextureMotion,
            4.0,
            4.0,
            0.0,
        );

        assert_eq!(before.position, 7);
        assert_eq!(boundary.position, 8);
    }

    #[test]
    fn melody_plan_contains_short_and_sustained_gates() {
        let plan = super::generate_melody_plan(
            32,
            super::Behavior {
                activity: 0.92,
                space: 0.10,
                repeat: 0.40,
            },
            0.82,
            super::MelodyPlan::empty(),
            0,
            PatternVariant::MelodyPassingDense,
        );

        let gates = plan
            .steps
            .iter()
            .take(plan.len)
            .filter(|step| step.intent != super::MelodyIntent::Rest)
            .map(|step| step.gate_steps)
            .collect::<Vec<_>>();

        assert!(gates.contains(&1));
        assert!(gates.iter().any(|gate| *gate >= 3));
    }

    #[test]
    fn melody_repeats_the_motif_and_answers_without_replacing_its_opening() {
        let mut engine = PerformerEngine::new(
            c_major(),
            MusicalSettings::default().tonal,
            Voicing::from_slice(&[52, 55, 60]),
        );
        let behavior = super::Behavior {
            activity: 0.54,
            space: 0.38,
            repeat: 0.75,
        };
        let variant = PatternVariant::MelodyDevelopedPhrase;
        let phrases = std::array::from_fn::<_, 5, _>(|cycle| {
            let timing = super::StepTiming {
                cycle_index: cycle as i64,
                step_key: cycle as i64 * 16,
                position: 0,
                steps_per_cycle: 16,
                step_length: 0.5,
            };
            engine.ensure_melody_plan(timing, 0.34, behavior, variant);
            engine.states[3].melody_plan
        });
        let [a, varied, repeat, answer, next] = phrases;
        assert_eq!(a.steps, repeat.steps);
        assert_ne!(a.steps, varied.steps);
        assert_eq!(
            a.steps.map(|step| step.gate_steps),
            varied.steps.map(|step| step.gate_steps)
        );
        assert_eq!(a.steps[..8], answer.steps[..8]);
        assert_ne!(a.steps[8..16], answer.steps[8..16]);
        assert_ne!(a.generation, next.generation);
    }

    #[test]
    fn sustained_chords_keep_common_notes_until_the_new_gate_ends() {
        let tonal = MusicalSettings::default().tonal;
        let old_voicing = Voicing::from_slice(&[48, 52, 55]);
        let mut engine = PerformerEngine::new(c_major(), tonal, old_voicing);
        let mut voices = default_voices();
        for (index, voice) in voices.iter_mut().enumerate() {
            voice.parameters.density = if index == 1 { 0.15 } else { 0.0 };
        }
        engine.transport_started(0.0);
        engine.update(0.0, 4.0, &voices);
        engine.update(0.02, 4.0, &voices);
        let new_chord = tonal.diatonic_chord(5, ChordExtension::Triad);
        let current = Voicing::from_slice(&[48, 52, 57]);
        engine.harmony_changed(new_chord, tonal, current, 0.5);
        engine.update(0.5, 4.0, &voices);
        let notes = engine.update(0.52, 4.0, &voices);
        assert_eq!(notes.iter().count(), 2);
        assert!(matches!(
            notes.iter().next(),
            Some(VoiceEvent::NoteOff {
                voice_index: 1,
                note: 55
            })
        ));
        assert!(matches!(
            notes.iter().last(),
            Some(VoiceEvent::NoteOn {
                voice_index: 1,
                note: 57,
                ..
            })
        ));
        let released = engine.update(
            0.53,
            4.0,
            &std::array::from_fn(|index| {
                let mut voice = voices[index];
                voice.parameters.density = 0.0;
                voice
            }),
        );
        assert_eq!(released.iter().count(), 3);
    }

    #[test]
    fn counterpoint_scoring_prefers_contrary_motion_when_other_costs_match() {
        let context = super::EnsembleContext {
            active_notes: [None, None, None, Some(60)],
            last_attack_beat: [None; 4],
            motion_direction: [0, 0, 0, 1],
        };

        let contrary = super::counterpoint_cost(64, Some(67), VoiceRole::Texture, 0.5, context);
        let parallel = super::counterpoint_cost(69, Some(67), VoiceRole::Texture, 0.5, context);

        assert!(contrary < parallel);
    }

    #[test]
    fn harmony_change_preserves_texture_and_melody_note_ownership() {
        let settings = MusicalSettings::default();
        let mut engine = PerformerEngine::new(
            c_major(),
            settings.tonal,
            Voicing::from_slice(&[52, 55, 60]),
        );

        engine.states[2].sounding = Some(super::ScheduledNotes {
            notes: super::NoteSet::single(76),
            off_beat: 3.0,
        });
        engine.states[3].sounding = Some(super::ScheduledNotes {
            notes: super::NoteSet::single(79),
            off_beat: 2.0,
        });

        let d_minor = Chord::new(
            PitchClass::from_value(2),
            ChordQuality::Minor,
            ChordExtension::Triad,
        );
        engine.harmony_changed(
            d_minor,
            settings.tonal,
            Voicing::from_slice(&[50, 53, 57]),
            1.0,
        );

        assert_eq!(engine.snapshots()[2].active_notes[0], Some(76));
        assert_eq!(engine.snapshots()[3].active_notes[0], Some(79));
        assert!(engine.states[0].force_attack);
        assert!(engine.states[1].force_attack);
    }

    #[test]
    fn melody_cadence_leaves_time_for_an_audible_hold() {
        assert_eq!(super::melody_cadence_position(16), 14);
        assert_eq!(super::melody_cadence_position(32), 28);

        let plan = super::generate_melody_plan(
            16,
            super::Behavior {
                activity: 0.55,
                space: 0.35,
                repeat: 0.50,
            },
            0.45,
            super::MelodyPlan::empty(),
            0,
            PatternVariant::MelodyDevelopedPhrase,
        );
        let cadence = super::melody_cadence_position(plan.len);
        assert!(plan.steps[cadence].gate_steps >= 2);
        assert_eq!(
            plan.steps[cadence].articulation,
            super::MelodyArticulation::Held
        );
    }

    #[test]
    fn expired_attack_does_not_retrigger_or_release_a_held_note() {
        let mut engine = PerformerEngine::new(
            c_major(),
            MusicalSettings::default().tonal,
            Voicing::from_slice(&[52, 55, 60]),
        );
        engine.states[2].sounding = Some(super::ScheduledNotes {
            notes: super::NoteSet::single(76),
            off_beat: 4.0,
        });
        engine.states[2].pending = Some(super::PendingAttack {
            notes: super::NoteSet::single(79),
            velocity: 90,
            start_beat: 1.0,
            gate_beats: 0.25,
            preserve_common: false,
        });
        let mut batch = super::VoiceEventBatch::new();
        engine.start_due_attack(2, 1.5, &mut batch);
        assert_eq!(batch.iter().count(), 0);
        assert!(engine.states[2].pending.is_none());
        assert_eq!(engine.snapshots()[2].active_notes[0], Some(76));
        assert_eq!(engine.take_timing_diagnostics(), (1, 0.5));
        assert_eq!(engine.take_timing_diagnostics(), (0, 0.0));
    }

    #[test]
    fn mute_cancels_pending_attack_before_dispatch() {
        let mut engine = PerformerEngine::new(
            c_major(),
            MusicalSettings::default().tonal,
            Voicing::from_slice(&[52, 55, 60]),
        );
        engine.transport_started(0.0);
        engine.states[2].pending = Some(super::PendingAttack {
            notes: super::NoteSet::single(76),
            velocity: 90,
            start_beat: 1.0,
            gate_beats: 0.5,
            preserve_common: false,
        });
        let mut voices = default_voices();
        voices[2].parameters.density = 0.0;
        let batch = engine.update(1.0, 4.0, &voices);
        assert!(
            !batch
                .iter()
                .any(|event| matches!(event, VoiceEvent::NoteOn { voice_index: 2, .. }))
        );
        assert!(engine.states[2].pending.is_none());
    }

    #[test]
    fn texture_pulse_and_rotating_cells_are_regular() {
        for position in 0..16 {
            assert_eq!(
                super::pattern_hit(PatternVariant::TextureQuarterPulse, position, 16,),
                position.is_multiple_of(2)
            );
            assert!(super::pattern_hit(
                PatternVariant::TextureRotatingThree,
                position,
                16,
            ));
        }
    }
}
