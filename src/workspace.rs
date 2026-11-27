//! Beat-relative takes and named musical workspaces. MIDI routing belongs to the rig.

use std::sync::Arc;

use crate::App;
use crate::input::InputAction;
use crate::midi::MidiEngine;
use crate::music::chord::Chord;
use crate::music::settings::MusicalSettings;
use crate::music::voicing::VoicingStyle;
use crate::performer::{ChordHold, Groove, PerformerEngine, Polyphony, TextureStyle, VoiceEvent};
use crate::phrase::{ChordEvent, EditGrid, LiveGrid, Phrase};
use crate::sync::{SyncMode, SyncTransport};
use crate::voice::VoiceParameters;

pub const MAX_LIBRARY_ITEMS: usize = 64;
const MAX_TAKE_EVENTS: usize = 48_000;
pub const NAME_LEN: usize = 20;
pub const ALPHABET: &[u8] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-";

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnsembleSetup {
    pub voices: [VoiceParameters; 4],
    pub chord_hold: ChordHold,
    #[serde(default)]
    pub groove: Groove,
    #[serde(default)]
    pub texture_style: TextureStyle,
    #[serde(default)]
    pub polyphony: Polyphony,
    pub voice_led_bass: bool,
    pub voicing: VoicingStyle,
}

impl EnsembleSetup {
    pub fn capture(app: &App) -> Self {
        Self {
            voices: app.voices.map(|voice| voice.parameters),
            chord_hold: app.performer_engine.hold_length,
            groove: app.performer_engine.groove,
            texture_style: app.performer_engine.texture_style,
            polyphony: app.performer_engine.polyphony,
            voice_led_bass: app.performer_engine.voice_led_bass,
            voicing: app.voice_engine.style,
        }
    }

    pub fn apply(self, app: &mut App) {
        for (voice, parameters) in app.voices.iter_mut().zip(self.voices) {
            voice.parameters = parameters;
        }
        app.performer_engine.hold_length = self.chord_hold;
        app.performer_engine.groove = self.groove;
        app.performer_engine.texture_style = self.texture_style;
        app.performer_engine.polyphony = self.polyphony;
        app.performer_engine.voice_led_bass = self.voice_led_bass;
        app.voice_engine.style = self.voicing;
        app.revoice_current();
    }

    pub fn valid(self) -> bool {
        self.polyphony.valid()
            && self.voices.iter().all(|voice| {
                [voice.density, voice.dynamics, voice.spread, voice.color]
                    .iter()
                    .all(|value| (0.0..=1.0).contains(value))
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TimedNote {
    pub beat: f64,
    pub event: VoiceEvent,
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Take {
    pub name: String,
    pub source_phrase: Option<u32>,
    pub settings: MusicalSettings,
    pub ensemble: EnsembleSetup,
    pub harmony: Vec<ChordEvent>,
    pub events: Vec<TimedNote>,
    pub duration: f64,
}

impl Take {
    pub fn valid(&self) -> bool {
        valid_name(&self.name)
            && self.ensemble.valid()
            && self.duration.is_finite()
            && self.duration > 0.0
            && !self.harmony.is_empty()
            && self.harmony.iter().all(valid_chord_event)
            && self.events.len() <= MAX_TAKE_EVENTS + 32
            && self
                .events
                .windows(2)
                .all(|pair| pair[0].beat <= pair[1].beat)
            && self.events.iter().all(|entry| {
                (0.0..=self.duration).contains(&entry.beat)
                    && match entry.event {
                        VoiceEvent::NoteOn {
                            voice_index,
                            note,
                            velocity,
                        } => voice_index < 4 && note < 128 && (1..=127).contains(&velocity),
                        VoiceEvent::NoteOff { voice_index, note } => voice_index < 4 && note < 128,
                    }
            })
    }
}

pub fn valid_chord_event(event: &ChordEvent) -> bool {
    event.chord.root.value() < 12
        && event.start_beat.is_finite()
        && event.start_beat >= 0.0
        && event.duration_beats.is_finite()
        && event.duration_beats > 0.0
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= NAME_LEN && name.bytes().all(|byte| ALPHABET.contains(&byte))
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub settings: MusicalSettings,
    pub chord: Chord,
    pub ensemble: EnsembleSetup,
    pub live_grid: LiveGrid,
    pub edit_grid: EditGrid,
    pub phrases: Vec<Phrase>,
    pub takes: Vec<Arc<Take>>,
}

impl Project {
    pub fn capture(app: &App) -> Self {
        Self {
            settings: app.settings,
            chord: app.current_chord,
            ensemble: EnsembleSetup::capture(app),
            live_grid: app.live_grid,
            edit_grid: app.editor_grid,
            phrases: app.phrases.clone(),
            takes: app.takes.clone(),
        }
    }

    pub fn apply(self, app: &mut App) {
        app.settings = self.settings;
        app.performer_engine =
            PerformerEngine::new(self.chord, self.settings.tonal, app.current_render.voicing);
        app.macro_group = [false; 4];
        app.scope = crate::voice::VoiceScope::All;
        app.phrases = self.phrases;
        app.takes = self.takes;
        app.live_grid = self.live_grid;
        app.editor_grid = self.edit_grid;
        app.phrase_player.stop();
        app.recorder.reset_timeline();
        app.recorder.continue_numbering(&app.phrases);
        app.phrase_undo.clear();
        app.editor_event_index = 0;
        app.editor_phrase_index = 0;
        app.pending_chord = None;
        app.pending_voicing = None;
        app.progression_history.clear();
        self.ensemble.apply(app);
        app.set_harmony(self.chord, 0.0, false, false);
    }

    fn valid(&self) -> bool {
        self.settings.tonal.tonic.value() < 12
            && crate::music::settings::TimeSignature::PRESETS.contains(&self.settings.meter)
            && (30.0..=300.0).contains(&self.settings.tempo_bpm)
            && self.chord.root.value() < 12
            && self.ensemble.valid()
            && self.phrases.iter().all(|phrase| {
                !phrase.events.is_empty() && phrase.events.iter().all(valid_chord_event)
            })
            && self.takes.len() <= MAX_LIBRARY_ITEMS
            && self.takes.iter().all(|take| take.valid())
    }
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Named<T> {
    pub name: String,
    pub value: T,
}

#[derive(Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Collection {
    pub projects: Vec<Named<Project>>,
    pub presets: Vec<Named<EnsembleSetup>>,
}

impl Collection {
    pub fn valid(&self) -> bool {
        self.projects.len() <= MAX_LIBRARY_ITEMS
            && self.presets.len() <= MAX_LIBRARY_ITEMS
            && self
                .projects
                .iter()
                .all(|entry| valid_name(&entry.name) && entry.value.valid())
            && self
                .presets
                .iter()
                .all(|entry| valid_name(&entry.name) && entry.value.valid())
    }
}

pub struct ActiveNotes([[u8; 128]; 4]);

impl Default for ActiveNotes {
    fn default() -> Self {
        Self([[0; 128]; 4])
    }
}

impl ActiveNotes {
    pub fn apply(&mut self, event: VoiceEvent) {
        match event {
            VoiceEvent::NoteOn {
                voice_index,
                note,
                velocity,
            } => self.0[voice_index][note as usize] = velocity,
            VoiceEvent::NoteOff { voice_index, note } => self.0[voice_index][note as usize] = 0,
        }
    }

    fn sounding(&self) -> impl Iterator<Item = (usize, u8, u8)> + '_ {
        self.0.iter().enumerate().flat_map(|(voice, notes)| {
            notes
                .iter()
                .enumerate()
                .filter(|(_, velocity)| **velocity > 0)
                .map(move |(note, &velocity)| (voice, note as u8, velocity))
        })
    }
}

#[derive(Clone, Copy)]
pub struct TakeArm {
    pub start: f64,
    pub stop: Option<f64>,
    pub source: Option<u32>,
}

pub struct TakeRecording {
    pub start: f64,
    pub stop: Option<f64>,
    current: Chord,
    chord_since: f64,
    take: Take,
}

impl TakeRecording {
    pub fn new(app: &App, start: f64, source_phrase: Option<u32>, name: String) -> Self {
        Self {
            start,
            stop: None,
            current: app.current_chord,
            chord_since: 0.0,
            take: Take {
                name,
                source_phrase,
                settings: app.settings,
                ensemble: EnsembleSetup::capture(app),
                harmony: Vec::new(),
                events: Vec::new(),
                duration: 0.0,
            },
        }
    }

    pub fn seed_notes(&mut self, notes: &ActiveNotes) {
        for (voice_index, note, velocity) in notes.sounding() {
            self.event(
                self.start,
                VoiceEvent::NoteOn {
                    voice_index,
                    note,
                    velocity,
                },
            );
        }
    }

    pub fn full(&self) -> bool {
        self.take.events.len() >= MAX_TAKE_EVENTS
    }

    pub fn event(&mut self, beat: f64, event: VoiceEvent) {
        if !self.full() {
            let relative = (beat - self.start).max(0.0);
            let previous = self.take.events.last().map_or(0.0, |event| event.beat);
            self.take.events.push(TimedNote {
                beat: relative.max(previous),
                event,
            });
        }
    }

    pub fn chord(&mut self, beat: f64, chord: Chord) {
        if self.current == chord {
            return;
        }
        self.close_chord(beat);
        self.current = chord;
    }

    fn close_chord(&mut self, beat: f64) {
        let relative = beat - self.start;
        if relative > self.chord_since {
            self.take.harmony.push(ChordEvent {
                chord: self.current,
                start_beat: self.chord_since,
                duration_beats: relative - self.chord_since,
            });
            self.chord_since = relative;
        }
    }

    pub fn finish(mut self, beat: f64) -> Option<Arc<Take>> {
        if beat <= self.start {
            return None;
        }
        self.close_chord(beat);
        self.take.duration = beat - self.start;
        self.take
            .events
            .retain(|event| event.beat <= self.take.duration);
        let mut notes = ActiveNotes::default();
        for entry in &self.take.events {
            notes.apply(entry.event);
        }
        for (voice_index, note, _) in notes.sounding() {
            self.take.events.push(TimedNote {
                beat: self.take.duration,
                event: VoiceEvent::NoteOff { voice_index, note },
            });
        }
        Some(Arc::new(self.take))
    }
}

pub struct TakePlayback {
    pub take: Arc<Take>,
    pub start: f64,
    pub event: usize,
    pub harmony: usize,
    pub started: bool,
}

impl TakePlayback {
    pub fn new(take: Arc<Take>, start: f64) -> Self {
        Self {
            take,
            start,
            event: 0,
            harmony: 0,
            started: false,
        }
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum Tab {
    #[default]
    Takes,
    Projects,
    Presets,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Self::Takes => Self::Projects,
            Self::Projects => Self::Presets,
            Self::Presets => Self::Takes,
        }
    }
}

pub struct NameEdit {
    pub bytes: [u8; NAME_LEN],
    pub cursor: usize,
    pub replacing: bool,
}

impl NameEdit {
    pub fn new(suggestion: &str) -> Self {
        let mut bytes = [b' '; NAME_LEN];
        let len = suggestion.len().min(NAME_LEN);
        bytes[..len].copy_from_slice(&suggestion.as_bytes()[..len]);
        Self {
            bytes,
            cursor: 0,
            replacing: false,
        }
    }

    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.bytes).trim().to_owned()
    }

    pub fn change(&mut self, step: i32) {
        let index = ALPHABET
            .iter()
            .position(|&letter| letter == self.bytes[self.cursor])
            .unwrap_or(0);
        self.bytes[self.cursor] =
            ALPHABET[(index as i32 + step).rem_euclid(ALPHABET.len() as i32) as usize];
    }
}

#[derive(Default)]
pub struct WorkspaceUi {
    pub tab: Tab,
    pub row: usize,
    pub name: Option<NameEdit>,
    pub message: String,
    load_confirm: bool,
}

pub fn toggle_capture(app: &mut App, transport: &mut SyncTransport) {
    if app.playing_take.is_some() || app.takes.len() >= MAX_LIBRARY_ITEMS {
        return;
    }
    let bar = app.settings.meter.beats_per_bar();
    let beat = transport.beat(bar);
    if let Some(active) = &mut app.recording_take {
        let boundary = crate::phrase::next_strict_boundary(beat, bar);
        active.stop = Some(active.stop.map_or(boundary, |stop| stop.min(boundary)));
        return;
    }
    if app.take_armed_at.take().is_some() {
        return;
    }
    if !transport.is_running() && !transport.start_pending() {
        if transport.mode() == SyncMode::MidiIn {
            return;
        }
        transport.start(bar);
    }
    let arm = app.phrase_player.loop_window(&app.phrases, beat).map_or(
        TakeArm {
            start: transport
                .pending_beat(bar)
                .unwrap_or_else(|| crate::phrase::next_boundary(beat, bar)),
            stop: None,
            source: app
                .phrase_player
                .playing_phrase()
                .map(|index| app.phrases[index].id),
        },
        |(start, stop, source)| TakeArm {
            start,
            stop: Some(stop),
            source: Some(source),
        },
    );
    app.take_armed_at = Some(arm);
}

pub fn begin_take(app: &mut App, beat: f64) {
    if let Some(arm) = app.take_armed_at.filter(|arm| beat >= arm.start) {
        app.take_armed_at = None;
        let mut take = TakeRecording::new(
            app,
            arm.start,
            arm.source,
            format!("TAKE {:02}", app.takes.len() + 1),
        );
        take.stop = arm.stop;
        take.seed_notes(&app.active_notes);
        app.recording_take = Some(take);
    }
}

pub fn handle_action(
    action: InputAction,
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
) -> Result<(), String> {
    use InputAction::*;
    let Some(mut ui) = app.workspace_ui.take() else {
        return Ok(());
    };
    let playing = transport.is_running() || transport.start_pending();
    if let Some(mut edit) = ui.name.take() {
        match action {
            NavigateLeft => edit.cursor = edit.cursor.saturating_sub(1),
            NavigateRight => edit.cursor = (edit.cursor + 1).min(NAME_LEN - 1),
            NavigateUp | ScopeNext => edit.change(1),
            NavigateDown | ScopePrevious => edit.change(-1),
            WorkspaceRecall | EditDelete => {
                edit = NameEdit::new("");
                ui.message.clear();
            }
            Secondary | MenuTap | ToggleSettings => {
                app.workspace_ui = Some(ui);
                return Ok(());
            }
            Primary if !playing => {
                let name = edit.name();
                let exists = match ui.tab {
                    Tab::Projects => app
                        .collection
                        .projects
                        .iter()
                        .any(|entry| entry.name == name),
                    Tab::Presets => app
                        .collection
                        .presets
                        .iter()
                        .any(|entry| entry.name == name),
                    Tab::Takes => false,
                };
                if !valid_name(&name) {
                    ui.message = "ENTER A NAME".into();
                } else if exists && !edit.replacing {
                    edit.replacing = true;
                    ui.message = "A REPLACE   B CANCEL".into();
                } else {
                    let saved = match ui.tab {
                        Tab::Projects => {
                            let mut project = Project::capture(app);
                            project.settings.tempo_bpm = transport.internal_tempo();
                            if save_named(&mut app.collection.projects, &name, project) {
                                app.project_name = Some(name);
                                true
                            } else {
                                false
                            }
                        }
                        Tab::Presets => {
                            let preset = EnsembleSetup::capture(app);
                            save_named(&mut app.collection.presets, &name, preset)
                        }
                        Tab::Takes => false,
                    };
                    ui.message = if saved { "SAVED" } else { "LIST FULL" }.into();
                    app.workspace_ui = Some(ui);
                    return Ok(());
                }
            }
            _ => {}
        }
        if matches!(
            action,
            NavigateDown | NavigateUp | NavigateLeft | NavigateRight | ScopeNext | ScopePrevious
        ) {
            edit.replacing = false;
            ui.message.clear();
        }
        ui.name = Some(edit);
        app.workspace_ui = Some(ui);
        return Ok(());
    }

    let confirmed = std::mem::take(&mut ui.load_confirm);
    ui.message.clear();
    match action {
        Secondary | MenuTap | ToggleSettings => {
            if confirmed {
                app.workspace_ui = Some(ui);
            }
            return Ok(());
        }
        NavigateLeft | NavigateRight => {
            ui.tab = if action == NavigateRight {
                ui.tab.next()
            } else {
                ui.tab.next().next()
            };
            ui.row = 0;
        }
        NavigateUp => ui.row = ui.row.saturating_sub(1),
        NavigateDown => {
            let len = match ui.tab {
                Tab::Takes => app.takes.len(),
                Tab::Projects => app.collection.projects.len(),
                Tab::Presets => app.collection.presets.len(),
            };
            ui.row = (ui.row + 1).min(len.saturating_sub(1));
        }
        Tertiary if ui.tab == Tab::Takes => {
            if app.take_armed_at.is_none()
                && app.recording_take.is_none()
                && matches!(app.recorder.status(), crate::phrase::RecorderStatus::Idle)
            {
                app.take_mode = !app.take_mode;
            } else {
                ui.message = "FINISH CAPTURE BEFORE SWITCHING".into();
            }
        }
        Tertiary if !playing => {
            let suggestion = match ui.tab {
                Tab::Projects => app
                    .project_name
                    .clone()
                    .unwrap_or_else(|| format!("PROJECT {:02}", app.collection.projects.len() + 1)),
                Tab::Presets => format!("PRESET {:02}", app.collection.presets.len() + 1),
                Tab::Takes => unreachable!(),
            };
            ui.name = Some(NameEdit::new(&suggestion));
        }
        Primary if ui.tab == Tab::Takes => {
            if let Some(take) = app.takes.get(ui.row).cloned() {
                if app.recording_take.is_some()
                    || app.take_armed_at.is_some()
                    || !matches!(app.recorder.status(), crate::phrase::RecorderStatus::Idle)
                {
                    ui.message = "FINISH CAPTURE BEFORE PLAYBACK".into();
                } else if !playing && transport.mode() == SyncMode::MidiIn {
                    ui.message = "START EXTERNAL TRANSPORT".into();
                } else {
                    let bar = app.settings.meter.beats_per_bar();
                    if !playing {
                        transport.start(bar);
                    }
                    let now = transport.beat(bar);
                    let start = transport.pending_beat(bar).unwrap_or_else(|| {
                        if playing {
                            crate::phrase::next_strict_boundary(now, bar)
                        } else {
                            now
                        }
                    });
                    if app.playing_take.take().is_some() {
                        app.stop_notes(midi)?;
                    }
                    app.playing_take = Some(TakePlayback::new(take, start));
                }
            }
        }
        Primary if !playing => match ui.tab {
            Tab::Projects => {
                if let Some(entry) = app.collection.projects.get(ui.row).cloned() {
                    if confirmed {
                        let tempo = entry.value.settings.tempo_bpm;
                        entry.value.apply(app);
                        if transport.mode() == SyncMode::Internal {
                            transport.set_tempo(tempo);
                        }
                        app.project_name = Some(entry.name);
                        ui.message = "PROJECT LOADED".into();
                    } else {
                        ui.load_confirm = true;
                        ui.message = "A LOAD SAVED PROJECT   B CANCEL".into();
                    }
                }
            }
            Tab::Presets => {
                if let Some(entry) = app.collection.presets.get(ui.row) {
                    entry.value.apply(app);
                    ui.message = "PRESET LOADED".into();
                }
            }
            Tab::Takes => unreachable!(),
        },
        WorkspaceRecall if ui.tab == Tab::Takes && !playing => {
            if let Some(take) = app.takes.get(ui.row) {
                take.ensemble.apply(app);
                ui.message = "TAKE SETTINGS RECALLED".into();
            }
        }
        LeftClick if ui.tab == Tab::Takes && !playing => {
            if let Some(take) = app.takes.get(ui.row) {
                let phrase = app.recorder.copy_harmony(&take.harmony);
                app.phrases.push(phrase);
                app.editor_phrase_index = app.phrases.len() - 1;
                app.editor_event_index = 0;
                ui.message = "COPIED TO PHRASE EDITOR".into();
            }
        }
        Primary | Tertiary | WorkspaceRecall | LeftClick if playing => {
            ui.message = "STOP TRANSPORT TO EDIT LIBRARY".into();
        }
        _ => {}
    }
    app.workspace_ui = Some(ui);
    Ok(())
}

fn save_named<T>(entries: &mut Vec<Named<T>>, name: &str, value: T) -> bool {
    if let Some(entry) = entries.iter_mut().find(|entry| entry.name == name) {
        entry.value = value;
    } else if entries.len() < MAX_LIBRARY_ITEMS {
        entries.push(Named {
            name: name.to_owned(),
            value,
        });
    } else {
        return false;
    }
    true
}
