//! Khaṇa v0.12.0 performer transposition and musical continuity.
//!
//! v0.11 adds complete semantic chord voicings, rhythmic/gated Melody phrases,
//! persistent cyclic Texture, and lightweight performer-to-performer awareness.

mod editor;
mod input;
mod midi;
mod music;
mod performer;
mod phrase;
mod runtime;
mod session;
mod sync;
mod ui;
mod voice;
mod workspace;

use std::thread;
use std::time::{Duration, Instant};

use editor::{ChordEdit, EditorModal};
use input::{HandheldInput, InputAction, InputPage, desktop_event, open_input_devices};
use midi::MidiEngine;
use music::chord::Chord;
use music::recommendation::{
    HarmonicDirection, RecommendationClass, RecommendationEngine, RecommendationSet,
};
use music::settings::{MusicalSettings, SettingField};
use music::voicing::VoicingStyle;
use performer::PerformerEngine;
use phrase::{ChainSection, ChordEvent, EditGrid, LiveGrid, Phrase, PhrasePlayer, PhraseRecorder};
use sdl2::event::Event;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use sync::{SyncMode, SyncTransport};
use ui::Ui;
use voice::{MacroKind, Voice, VoiceEngine, VoiceRender, VoiceScope, adjust_macro, default_voices};
use workspace::{ActiveNotes, Collection, Take, TakePlayback, TakeRecording, WorkspaceUi};

const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 768;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Screen {
    Performance,
    PhraseEditor,
}

struct PhraseEdit {
    events: Vec<ChordEvent>,
    selection: usize,
}

struct App {
    screen: Screen,
    settings: MusicalSettings,
    setting_focus: Option<SettingField>,
    workspace_ui: Option<WorkspaceUi>,
    collection: Collection,
    project_name: Option<String>,
    takes: Vec<Arc<Take>>,
    take_mode: bool,
    take_armed_at: Option<workspace::TakeArm>,
    recording_take: Option<TakeRecording>,
    retrospective_recording: Option<TakeRecording>,
    retrospective_take: Option<Arc<Take>>,
    playing_take: Option<TakePlayback>,
    active_notes: ActiveNotes,
    panic_confirm: bool,
    current_chord: Chord,
    chord_started_beat: f64,
    current_render: VoiceRender,
    progression_history: Vec<Chord>,
    recommendation_engine: RecommendationEngine,
    recommendations: RecommendationSet,
    selected_class: RecommendationClass,
    selected_row: usize,
    direction: HarmonicDirection,
    surprise: bool,
    voice_engine: VoiceEngine,
    performer_engine: PerformerEngine,
    voices: [Voice; 4],
    scope: VoiceScope,
    macro_group: [bool; 4],
    recorder: PhraseRecorder,
    phrase_player: PhrasePlayer,
    phrases: Vec<Phrase>,
    phrase_undo: HashMap<u32, VecDeque<PhraseEdit>>,
    editor_phrase_index: usize,
    editor_event_index: usize,
    editor_zoom: f32,
    editor_fit: bool,
    editor_modal: Option<EditorModal>,
    editor_clipboard: Option<ChordEvent>,
    editor_message: &'static str,
    editor_grid: EditGrid,
    live_grid: LiveGrid,
    pending_chord: Option<(Chord, f64)>,
    pending_voicing: Option<(VoicingStyle, f64)>,
}

impl App {
    fn input_page(&self) -> InputPage {
        if self.workspace_ui.is_some() {
            InputPage::Workspace
        } else if self.setting_focus.is_some() {
            InputPage::Settings
        } else if self.screen == Screen::Performance {
            InputPage::Performance
        } else if self.editor_modal.is_some() {
            InputPage::EditorModal
        } else {
            InputPage::Editor
        }
    }

    fn new() -> Self {
        let settings = MusicalSettings::default();
        let current_chord = settings.tonal.tonic_chord();
        let recommendation_engine = RecommendationEngine::new();
        let direction = HarmonicDirection::Neutral;
        let progression_history = vec![current_chord];
        let recommendations =
            recommendation_engine.recommend(&progression_history, settings.tonal, direction, false);
        let voices = default_voices();
        let mut voice_engine = VoiceEngine::new();
        let current_render = voice_engine.harmonic_change(current_chord, &voices);
        let performer_engine =
            PerformerEngine::new(current_chord, settings.tonal, current_render.voicing);

        Self {
            screen: Screen::Performance,
            settings,
            setting_focus: None,
            workspace_ui: None,
            collection: Collection::default(),
            project_name: None,
            takes: Vec::new(),
            take_mode: false,
            take_armed_at: None,
            recording_take: None,
            retrospective_recording: None,
            retrospective_take: None,
            playing_take: None,
            active_notes: ActiveNotes::default(),
            panic_confirm: false,
            current_chord,
            chord_started_beat: 0.0,
            current_render,
            progression_history,
            recommendation_engine,
            recommendations,
            selected_class: RecommendationClass::Safe,
            selected_row: 0,
            direction,
            surprise: false,
            voice_engine,
            performer_engine,
            voices,
            scope: VoiceScope::All,
            macro_group: [false; 4],
            recorder: PhraseRecorder::new(),
            phrase_player: PhrasePlayer::new(),
            phrases: Vec::new(),
            phrase_undo: HashMap::new(),
            editor_phrase_index: 0,
            editor_event_index: 0,
            editor_zoom: 82.0,
            editor_fit: false,
            editor_modal: None,
            editor_clipboard: None,
            editor_message: "",
            editor_grid: EditGrid::Quarter,
            live_grid: LiveGrid::Immediate,
            pending_chord: None,
            pending_voicing: None,
        }
    }

    fn finish_take(&mut self, beat: f64) -> bool {
        if let Some(take) = self
            .recording_take
            .take()
            .and_then(|take| take.finish(beat))
        {
            println!("CAPTURED {}", take.name);
            self.takes.push(take);
            true
        } else {
            false
        }
    }

    fn finish_retrospective_pass(&mut self, now: f64) {
        if let Some(stop) = self
            .retrospective_recording
            .as_ref()
            .and_then(|take| take.stop)
            && now >= stop
            && let Some(take) = self.retrospective_recording.take()
            && let Some(take) = take.finish_phrase_pass(stop)
        {
            self.retrospective_take = Some(take);
        }
    }

    fn stop_notes(&mut self, midi: &mut MidiEngine) -> Result<(), String> {
        self.active_notes = ActiveNotes::default();
        midi.stop_all()
    }

    fn macro_targets(&self) -> [bool; 4] {
        if self.macro_group.iter().any(|&member| member) {
            self.macro_group
        } else {
            std::array::from_fn(|index| {
                self.scope == VoiceScope::All || self.scope == VoiceScope::One(index)
            })
        }
    }

    fn set_direction(&mut self, direction: HarmonicDirection) {
        if self.direction != direction {
            self.direction = direction;
            self.refresh_recommendations();
        }
    }

    fn refresh_recommendations(&mut self) {
        self.recommendations = self.recommendation_engine.recommend(
            &self.progression_history,
            self.settings.tonal,
            self.direction,
            self.surprise,
        );
        self.selected_row = 0;
    }

    fn chosen_chord(&self) -> Chord {
        self.recommendations
            .get(self.selected_class, self.selected_row)
    }

    fn set_harmony(&mut self, chord: Chord, now_beat: f64, transport_running: bool, record: bool) {
        self.current_chord = chord;
        self.chord_started_beat = now_beat;
        self.current_render = self.voice_engine.harmonic_change(chord, &self.voices);

        self.performer_engine.harmony_changed(
            chord,
            self.settings.tonal,
            self.current_render.voicing,
            now_beat,
        );
        self.performer_engine.set_enabled(transport_running);

        for take in [&mut self.recording_take, &mut self.retrospective_recording]
            .into_iter()
            .flatten()
        {
            take.chord(now_beat, chord);
        }
        if record {
            self.recorder.chord_changed(now_beat, chord);
        }

        if self.progression_history.last().copied() != Some(chord) {
            self.progression_history.push(chord);
            if self.progression_history.len() > 8 {
                self.progression_history.remove(0);
            }
        }

        self.refresh_recommendations();

        let chord_name = self.settings.tonal.chord_symbol(self.current_chord);
        let note_names = self
            .current_render
            .voicing
            .iter()
            .map(|note| self.settings.tonal.midi_note_label(note))
            .collect::<Vec<_>>();

        println!(
            "CHORD {} | CHORDS VOICING {:?} MIDI {:?}",
            chord_name,
            note_names,
            self.current_render.voicing.as_slice()
        );
    }

    fn revoice_current(&mut self) {
        self.current_render = self.voice_engine.revoice(self.current_chord, &self.voices);
        self.performer_engine
            .update_anchor(self.settings.tonal, self.current_render.voicing);
    }

    fn update_performers(&mut self, now_beat: f64, midi: &mut MidiEngine) -> Result<(), String> {
        let batch = self.performer_engine.update(
            now_beat,
            self.settings.meter.beats_per_bar(),
            &self.voices,
        );

        for event in batch.iter() {
            self.active_notes.apply(event);
            for take in [&mut self.recording_take, &mut self.retrospective_recording]
                .into_iter()
                .flatten()
            {
                take.event(now_beat, event);
            }
            midi.apply_event(event)?;
        }

        Ok(())
    }

    fn selected_phrase(&self) -> Option<&Phrase> {
        self.phrases.get(self.editor_phrase_index)
    }

    fn selected_phrase_mut(&mut self) -> Option<&mut Phrase> {
        self.phrases.get_mut(self.editor_phrase_index)
    }

    fn edit_phrase(&mut self, edit: impl FnOnce(&mut Phrase, usize)) -> bool {
        const UNDO_LIMIT: usize = 32;
        let selection = self.editor_event_index;
        let Some(phrase) = self.phrases.get_mut(self.editor_phrase_index) else {
            return false;
        };
        let events = phrase.events.clone();
        edit(phrase, selection);
        if events == phrase.events {
            return false;
        }
        let history = self.phrase_undo.entry(phrase.id).or_default();
        if history.len() == UNDO_LIMIT {
            history.pop_front();
        }
        history.push_back(PhraseEdit { events, selection });
        self.phrase_player.phrase_edited(self.editor_phrase_index);
        true
    }

    fn undo_phrase_edit(&mut self) {
        let Some(phrase) = self.phrases.get_mut(self.editor_phrase_index) else {
            return;
        };
        if let Some(edit) = self
            .phrase_undo
            .get_mut(&phrase.id)
            .and_then(VecDeque::pop_back)
        {
            phrase.events = edit.events;
            self.editor_event_index = edit.selection;
            self.phrase_player.phrase_edited(self.editor_phrase_index);
        }
    }

    fn can_undo_phrase(&self) -> bool {
        self.selected_phrase()
            .and_then(|phrase| self.phrase_undo.get(&phrase.id))
            .is_some_and(|history| !history.is_empty())
    }

    fn clamp_editor_selection(&mut self) {
        if self.phrases.is_empty() {
            self.editor_phrase_index = 0;
            self.editor_event_index = 0;
            return;
        }

        self.editor_phrase_index = self.editor_phrase_index.min(self.phrases.len() - 1);

        let event_count = self.phrases[self.editor_phrase_index].events.len();

        if event_count == 0 {
            self.editor_event_index = 0;
        } else {
            self.editor_event_index = self.editor_event_index.min(event_count - 1);
        }
    }

    fn reset_harmony_for_context(
        &mut self,
        now_beat: f64,
        midi: &mut MidiEngine,
    ) -> Result<(), String> {
        self.current_chord = self.settings.tonal.tonic_chord();
        self.chord_started_beat = now_beat;
        self.current_render = self
            .voice_engine
            .harmonic_change(self.current_chord, &self.voices);
        self.performer_engine.harmony_changed(
            self.current_chord,
            self.settings.tonal,
            self.current_render.voicing,
            now_beat,
        );
        self.performer_engine.set_enabled(false);
        self.progression_history.clear();
        self.progression_history.push(self.current_chord);
        midi.stop_all()?;
        self.refresh_recommendations();
        Ok(())
    }
}

/// Runs Khaṇa v0.11.0.
///
/// # Errors
///
/// Returns an error if SDL or MIDI initialization/output fails.
fn ui_period(value: Option<&str>) -> Result<Duration, String> {
    match value.unwrap_or("30") {
        "30" => Ok(Duration::from_secs_f64(1.0 / 30.0)),
        "60" => Ok(Duration::from_secs_f64(1.0 / 60.0)),
        _ => Err("BRICK_UI_FPS must be 30 or 60".to_owned()),
    }
}

fn main() -> Result<(), String> {
    let brick = std::env::var("KHANA_BRICK").as_deref() == Ok("1")
        || std::env::var("PLATFORM").as_deref() == Ok("tg5040");
    // Headless diagnostics deliberately run before SDL/video initialization.
    let mut args = std::env::args().skip(1);
    if let Some(command) = args.next() {
        if args.next().is_some() {
            return Err("Expected a single diagnostic option".to_owned());
        }
        return match command.as_str() {
            "--midi-list" => {
                midi::list_destinations();
                Ok(())
            }
            "--midi-test" => midi::smoke_test(brick),
            _ => Err(format!("Unknown option: {command}")),
        };
    }
    let frame_period = ui_period(std::env::var("BRICK_UI_FPS").ok().as_deref())?;
    let input_log = std::env::var("KHANA_INPUT_LOG").as_deref() == Ok("1");
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let title = format!("Khana v{}", env!("CARGO_PKG_VERSION"));
    let mut builder = video.window(&title, WINDOW_WIDTH, WINDOW_HEIGHT);
    builder.position_centered();
    if brick {
        builder.fullscreen_desktop();
    }
    let window = builder.build().map_err(|error| error.to_string())?;
    // No vsync wait in the musical/control polling loop. Presentation may
    // still block on a device driver; measure that rather than assuming RT.
    let mut canvas = window
        .into_canvas()
        .build()
        .map_err(|error| error.to_string())?;
    canvas
        .set_logical_size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .map_err(|error| error.to_string())?;
    let mut event_pump = sdl_context.event_pump()?;
    let (controllers, joysticks) = open_input_devices(&sdl_context, brick)?;
    let joystick = if let Ok(index) = std::env::var("KHANA_JOYSTICK_INDEX") {
        let index = index
            .parse::<usize>()
            .map_err(|_| "KHANA_JOYSTICK_INDEX must be an integer".to_owned())?;
        Some(
            joysticks
                .get(index)
                .ok_or_else(|| "KHANA_JOYSTICK_INDEX not found".to_owned())?,
        )
    } else {
        joysticks
            .iter()
            .find(|joystick| joystick.num_axes() >= 6 && joystick.num_buttons() >= 9)
            .or_else(|| joysticks.first())
    };
    let mut handheld = HandheldInput::new(
        joystick.map(sdl2::joystick::Joystick::instance_id),
        Instant::now(),
    );
    println!(
        "BUILD {} | brick={brick} | SDL driver={} | output={:?} | UI FPS={:.0}",
        title,
        video.current_video_driver(),
        canvas.output_size(),
        1.0 / frame_period.as_secs_f64()
    );
    if brick {
        println!(
            "INPUT: selected raw joystick {:?}",
            joystick.map(sdl2::joystick::Joystick::instance_id)
        );
    }

    let ui = Ui::new(WINDOW_WIDTH, brick);
    let mut worker = runtime::Worker::spawn(brick)?;
    print_controls();
    let mut next_render = Instant::now();
    let mut last_report = next_render;
    let mut max_render = Duration::ZERO;

    'running: loop {
        for event in event_pump.poll_iter() {
            if matches!(event, Event::Quit { .. }) {
                break 'running;
            }
            if input_log
                && matches!(
                    event,
                    Event::JoyButtonDown { .. }
                        | Event::JoyButtonUp { .. }
                        | Event::JoyAxisMotion { .. }
                        | Event::JoyHatMotion { .. }
                        | Event::ControllerButtonDown { .. }
                        | Event::KeyDown { .. }
                )
            {
                println!("INPUT {event:?}");
            }
            let action = if brick {
                handheld.event(&event, worker.input_page(), Instant::now())
            } else {
                desktop_event(&event, &controllers, worker.input_page())
            };
            if let Some(action) = action
                && worker.action(action)?
            {
                break 'running;
            }
        }
        if brick {
            for action in handheld
                .tick(worker.input_page(), Instant::now())
                .into_iter()
                .flatten()
            {
                if worker.action(action)? {
                    break 'running;
                }
            }
        }

        // Render only when due; do not sleep for the entire UI frame budget.
        let render_now = Instant::now();
        if render_now < next_render {
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        next_render = render_now + frame_period;

        // The worker owns all mutable music state. The renderer holds only a
        // snapshot, so a slow SDL present cannot delay its clock or MIDI poll.
        let frame = worker.snapshot()?;
        frame.draw(&ui, &mut canvas, brick && handheld.select_held())?;
        canvas.present();
        max_render = max_render.max(render_now.elapsed());
        if render_now.duration_since(last_report) >= Duration::from_secs(5) {
            let (ticks, active) = frame.diagnostics();
            println!(
                "UI DIAG render {:.2} ms | music ticks {ticks} | active MIDI {active:?}",
                max_render.as_secs_f64() * 1000.0
            );
            max_render = Duration::ZERO;
            last_report = render_now;
        }
        thread::sleep(Duration::from_millis(1));
    }

    worker.shutdown()?;
    println!("Khaṇa exited cleanly.");
    Ok(())
}

/// Returns true for a clean application exit, regardless of input source.
fn dispatch_action(
    mut action: InputAction,
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
) -> Result<bool, String> {
    if action == InputAction::Quit {
        return Ok(true);
    }
    if action == InputAction::ToggleWorkspace && app.setting_focus.is_none() {
        if app.workspace_ui.take().is_none() {
            app.workspace_ui = Some(WorkspaceUi::default());
        }
        return Ok(false);
    }
    if app.workspace_ui.is_some() && action != InputAction::ToggleTransport {
        workspace::handle_action(action, app, transport, midi)?;
        return Ok(false);
    }
    if app.editor_modal.is_some() && action != InputAction::ToggleTransport {
        if let Some(section) = app.handle_editor_modal(action) {
            queue_editor_section(app, transport, section);
        }
        return Ok(false);
    }
    if action == InputAction::MenuTap {
        action = if app.setting_focus.is_some() {
            InputAction::ToggleSettings
        } else {
            InputAction::ToggleScreen
        };
    }
    if action == InputAction::ToggleTransport {
        toggle_transport(app, transport);
    } else if let Some(field) = app.setting_focus {
        handle_setting_action(action, field, app, transport, midi)?;
    } else if action == InputAction::ToggleSettings {
        let playing = transport.is_running() || transport.start_pending();
        app.setting_focus = if !playing {
            Some(SettingField::Tonic)
        } else if transport.mode() == SyncMode::Internal {
            Some(SettingField::Tempo)
        } else {
            None
        };
    } else {
        match app.screen {
            Screen::Performance => handle_performance_action(action, app, transport, midi)?,
            Screen::PhraseEditor => handle_editor_action(action, app, transport, midi)?,
        }
    }
    Ok(false)
}

fn toggle_transport(app: &mut App, transport: &mut SyncTransport) {
    app.setting_focus = None;
    app.panic_confirm = false;
    if transport.start_pending() {
        app.phrase_player.stop();
        app.recorder.cancel_pending_start();
        app.take_armed_at = None;
        app.playing_take = None;
    }
    if transport.is_running() || transport.start_pending() {
        let beat = transport.beat(app.settings.meter.beats_per_bar());
        app.finish_retrospective_pass(beat);
        app.pending_chord = None;
        app.pending_voicing = None;
        transport.stop();
    } else {
        let bar = app.settings.meter.beats_per_bar();
        transport.start(bar);
        if transport.is_running() || transport.start_pending() {
            let now = transport
                .pending_beat(bar)
                .unwrap_or_else(|| transport.beat(bar));
            app.phrase_player.arm_chain(now, bar);
        }
    }
}

fn handle_performance_action(
    action: InputAction,
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
) -> Result<(), String> {
    let now_beat = transport.beat(app.settings.meter.beats_per_bar());

    match action {
        InputAction::NavigateLeft => {
            let index = (app.selected_class.index() + RecommendationClass::ALL.len() - 1)
                % RecommendationClass::ALL.len();
            app.selected_class = RecommendationClass::from_index(index);
        }
        InputAction::NavigateRight => {
            app.selected_class = RecommendationClass::from_index(app.selected_class.index() + 1);
        }
        InputAction::NavigateUp => {
            app.selected_row = (app.selected_row + 2) % 3;
        }
        InputAction::NavigateDown => {
            app.selected_row = (app.selected_row + 1) % 3;
        }
        InputAction::Primary => {
            if app.playing_take.take().is_some() {
                app.stop_notes(midi)?;
                app.performer_engine.transport_started(now_beat);
            }
            let chord = app.chosen_chord();
            if transport.is_running()
                && let Some(quantum) = app.live_grid.quantum(app.settings.meter)
            {
                app.pending_chord = Some((chord, phrase::next_strict_boundary(now_beat, quantum)));
            } else {
                app.pending_chord = None;
                app.finish_retrospective_pass(now_beat);
                app.retrospective_recording = None;
                app.phrase_player.stop();
                app.set_harmony(chord, now_beat, transport.is_running(), true);
            }
        }
        InputAction::CycleLiveGrid => app.live_grid = app.live_grid.next(),
        InputAction::CyclePolyphony => {
            let meter = app.settings.meter;
            let next = transport.is_running().then(|| {
                phrase::next_strict_boundary(
                    now_beat,
                    EditGrid::Beat.beats(meter.numerator, meter.denominator),
                )
            });
            app.performer_engine.cycle_polyphony(app.scope, next);
        }
        InputAction::PatternHold => app.performer_engine.toggle_hold(app.scope),
        InputAction::ToggleMute => app.performer_engine.toggle_mute(app.scope),
        InputAction::ToggleSolo => app.performer_engine.toggle_solo(app.scope),
        InputAction::LeftClick if transport.is_running() => {
            app.performer_engine
                .toggle_sustain(app.scope, &app.voices, now_beat);
        }
        InputAction::RightClick => {
            let current = app.voice_engine.style;
            let next = app
                .pending_voicing
                .map_or(current, |(style, _)| style)
                .next(1);
            if transport.is_running() {
                let meter = app.settings.meter;
                let quantum = EditGrid::Beat.beats(meter.numerator, meter.denominator);
                app.pending_voicing = (next != current)
                    .then(|| (next, phrase::next_strict_boundary(now_beat, quantum)));
            } else {
                app.voice_engine.style = next;
                app.revoice_current();
            }
        }
        InputAction::Vary => app.performer_engine.request_variation(app.scope),
        InputAction::Secondary => {
            app.recommendation_engine.reroll();
            app.surprise = false;
            app.refresh_recommendations();
        }
        InputAction::Tertiary => {
            app.recommendation_engine.reroll();
            app.surprise = true;
            app.refresh_recommendations();
        }
        InputAction::BiasResolve => app.set_direction(HarmonicDirection::Resolve),
        InputAction::BiasNeutral => app.set_direction(HarmonicDirection::Neutral),
        InputAction::BiasTension => app.set_direction(HarmonicDirection::Tension),
        InputAction::DirectionStep(step) => {
            use HarmonicDirection::*;
            let direction = match (app.direction, step.signum()) {
                (Neutral, -1) => Resolve,
                (Neutral, 1) => Tension,
                (Resolve, 1) | (Tension, -1) => Neutral,
                (direction, _) => direction,
            };
            app.set_direction(direction);
        }
        InputAction::ToggleMacroMember => match app.scope {
            VoiceScope::All => app.macro_group = [false; 4],
            VoiceScope::One(index) => app.macro_group[index] = !app.macro_group[index],
        },
        InputAction::ScopePrevious => {
            app.scope = app.scope.previous();
        }
        InputAction::ScopeNext => {
            app.scope = app.scope.next();
        }
        InputAction::AdjustMacro { kind, delta } => {
            let targets = app.macro_targets();
            for (index, selected) in targets.into_iter().enumerate() {
                if selected {
                    adjust_macro(&mut app.voices, VoiceScope::One(index), kind, delta);
                }
            }
            if (kind == MacroKind::Spread && targets[1])
                || (kind == MacroKind::Density && targets[0])
            {
                app.revoice_current();
            }
        }
        InputAction::ToggleRecord if app.take_mode => {
            workspace::toggle_capture(app, transport);
        }
        InputAction::ToggleRecord => {
            if app.playing_take.is_some() {
                return Ok(());
            }
            if !transport.is_running() {
                if transport.mode() == SyncMode::MidiIn {
                    return Ok(());
                }
                transport.start(app.settings.meter.beats_per_bar());
            }

            let bar = app.settings.meter.beats_per_bar();
            let beat = transport
                .pending_beat(bar)
                .unwrap_or_else(|| transport.beat(bar));
            app.recorder.toggle(beat, bar);
        }
        InputAction::ArmNextRecording if !app.take_mode => {
            app.recorder.toggle_arm_next();
        }
        InputAction::ToggleScreen => {
            app.screen = Screen::PhraseEditor;
            app.clamp_editor_selection();
        }
        _ => {}
    }

    Ok(())
}

fn handle_setting_action(
    action: InputAction,
    field: SettingField,
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
) -> Result<(), String> {
    let running = transport.is_running() || transport.start_pending();
    if action == InputAction::Primary && !running && field == SettingField::MidiPanic {
        if std::mem::take(&mut app.panic_confirm) {
            midi.panic()?;
        } else {
            app.panic_confirm = true;
        }
        return Ok(());
    }
    if std::mem::take(&mut app.panic_confirm)
        && matches!(action, InputAction::ToggleSettings | InputAction::Secondary)
    {
        return Ok(());
    }
    match action {
        InputAction::ToggleSettings | InputAction::Secondary => app.setting_focus = None,
        InputAction::Tertiary if !running => {
            app.setting_focus = Some(if field.is_rig() {
                SettingField::Tonic
            } else {
                SettingField::MidiChannel(0)
            });
        }
        InputAction::NavigateLeft if !running => app.setting_focus = Some(field.move_cursor(0, -1)),
        InputAction::NavigateRight if !running => app.setting_focus = Some(field.move_cursor(0, 1)),
        InputAction::NavigateUp if !running => app.setting_focus = Some(field.move_cursor(-1, 0)),
        InputAction::NavigateDown if !running => app.setting_focus = Some(field.move_cursor(1, 0)),
        InputAction::ScopeNext | InputAction::ZoomIn
            if !running || live_tempo(field, transport) =>
        {
            adjust_setting(1, field, app, transport, midi)?;
        }
        InputAction::ScopePrevious | InputAction::ZoomOut
            if !running || live_tempo(field, transport) =>
        {
            adjust_setting(-1, field, app, transport, midi)?;
        }
        _ => {}
    }

    Ok(())
}

fn live_tempo(field: SettingField, transport: &SyncTransport) -> bool {
    field == SettingField::Tempo && transport.mode() == SyncMode::Internal
}

fn adjust_setting(
    direction: i32,
    field: SettingField,
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
) -> Result<(), String> {
    match field {
        SettingField::Tonic => {
            let current = i32::from(app.settings.tonal.tonic.value());
            let next = (current + direction).rem_euclid(12) as u8;
            app.settings.tonal.tonic = music::chord::PitchClass::from_value(next);
            app.reset_harmony_for_context(
                transport.beat(app.settings.meter.beats_per_bar()),
                midi,
            )?;
        }
        SettingField::Mode => {
            app.settings.tonal.mode = if direction > 0 {
                app.settings.tonal.mode.next()
            } else {
                app.settings.tonal.mode.previous()
            };
            app.reset_harmony_for_context(
                transport.beat(app.settings.meter.beats_per_bar()),
                midi,
            )?;
        }
        SettingField::Tempo if transport.mode() != SyncMode::MidiIn => {
            app.settings.tempo_bpm =
                (app.settings.tempo_bpm + f64::from(direction)).clamp(30.0, 300.0);
            transport.set_tempo(app.settings.tempo_bpm);
        }
        SettingField::Tempo => {}
        SettingField::Meter => {
            app.settings.meter = if direction > 0 {
                app.settings.meter.next()
            } else {
                app.settings.meter.previous()
            };
        }
        SettingField::Sync => {
            let modes: &[SyncMode] = if transport.link_available() {
                &[SyncMode::Internal, SyncMode::MidiIn, SyncMode::Link]
            } else {
                &[SyncMode::Internal, SyncMode::MidiIn]
            };
            let index = modes
                .iter()
                .position(|&mode| mode == transport.mode())
                .unwrap_or(0);
            let next = (index as i32 + direction).rem_euclid(modes.len() as i32) as usize;
            let next_mode = modes[next];

            if transport.set_mode(next_mode, app.settings.meter.beats_per_bar()) {
                midi.reset_clock_position();
            } else {
                eprintln!("Ableton Link is not compiled in. Rebuild with --features ableton-link");
            }

            app.settings.tempo_bpm = transport.tempo();
        }
        SettingField::StartStopSync => {
            let status = transport.status();
            transport.set_link_start_stop_sync(!status.link_start_stop_sync);
        }
        SettingField::MidiClockOut if transport.mode() != SyncMode::MidiIn => {
            midi.toggle_clock_out()
        }
        SettingField::MidiClockOut => {}
        SettingField::MidiChannel(voice) => midi.adjust_channel(voice, direction)?,
        SettingField::MidiTranspose(voice) => midi.adjust_transpose(voice, direction)?,
        SettingField::ChordHold => app.performer_engine.cycle_hold_length(direction),
        SettingField::BassArrival => app.performer_engine.toggle_bass_arrival(),
        SettingField::Groove => {
            app.performer_engine.groove = app.performer_engine.groove.next(direction)
        }
        SettingField::TextureStyle => {
            app.performer_engine.texture_style = app.performer_engine.texture_style.next(direction)
        }
        SettingField::MidiPanic => {}
        SettingField::MidiOutput => {
            if !transport.is_running() {
                midi.cycle_raw_destination(direction)?;
            }
        }
    }

    Ok(())
}

fn handle_editor_action(
    action: InputAction,
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
) -> Result<(), String> {
    app.editor_message = "";
    match action {
        InputAction::ToggleScreen => {
            app.screen = Screen::Performance;
        }
        InputAction::NavigateLeft => {
            app.editor_event_index = app.editor_event_index.saturating_sub(1);
        }
        InputAction::NavigateRight => {
            if let Some(phrase) = app.selected_phrase()
                && !phrase.events.is_empty()
            {
                app.editor_event_index = (app.editor_event_index + 1).min(phrase.events.len() - 1);
            }
        }
        InputAction::PhrasePrevious => {
            app.editor_phrase_index = app.editor_phrase_index.saturating_sub(1);
            app.editor_event_index = 0;
        }
        InputAction::PhraseNext => {
            if !app.phrases.is_empty() {
                app.editor_phrase_index = (app.editor_phrase_index + 1).min(app.phrases.len() - 1);
                app.editor_event_index = 0;
            }
        }
        InputAction::Primary => launch_editor_phrase(app, transport, midi)?,
        InputAction::LeftClick => {
            if let Some(phrase) = app.selected_phrase_mut() {
                phrase.looping = !phrase.looping;
                app.phrase_player.phrase_edited(app.editor_phrase_index);
            }
        }
        InputAction::RightClick => {
            app.phrase_player.clear_queue();
            app.editor_message = "QUEUE CLEARED";
        }
        InputAction::Secondary => app.open_chord_picker(ChordEdit::Replace),
        InputAction::Tertiary if app.selected_phrase().is_some() => {
            app.editor_modal = Some(EditorModal::Edit(0));
        }
        InputAction::EditDuplicate => app.duplicate_editor_event(),
        InputAction::EditDelete => app.delete_editor_event(),
        InputAction::EditShorten | InputAction::EditExtend => {
            let grid = app.duration_grid_beats();
            app.edit_phrase(|phrase, index| {
                phrase.snap_resize_event(index, grid, action == InputAction::EditExtend);
            });
        }
        InputAction::EditUndo => app.undo_phrase_edit(),
        InputAction::GridPrevious | InputAction::NavigateUp => {
            app.editor_grid = app.editor_grid.previous();
        }
        InputAction::GridNext | InputAction::NavigateDown => {
            app.editor_grid = app.editor_grid.next();
        }
        InputAction::ZoomOut => {
            app.editor_fit = false;
            app.editor_zoom = (app.editor_zoom - 12.0).max(24.0);
        }
        InputAction::ZoomIn => {
            app.editor_fit = false;
            app.editor_zoom = (app.editor_zoom + 12.0).min(180.0);
        }
        _ => {}
    }

    Ok(())
}

fn launch_editor_phrase(
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
) -> Result<(), String> {
    if app.selected_phrase().is_none() {
        return Ok(());
    }
    let bar = app.settings.meter.beats_per_bar();
    if !transport.is_running() && !transport.start_pending() {
        if transport.mode() == SyncMode::MidiIn {
            app.editor_message = "START EXTERNAL TRANSPORT";
            return Ok(());
        }
        transport.start(bar);
    }
    let now = transport
        .pending_beat(bar)
        .unwrap_or_else(|| transport.beat(bar));
    app.pending_chord = None;
    app.phrase_player.request(app.editor_phrase_index, now, bar);
    if app.playing_take.take().is_some() {
        app.stop_notes(midi)?;
    }
    Ok(())
}

fn queue_editor_section(app: &mut App, transport: &mut SyncTransport, section: ChainSection) {
    if !app.phrase_player.append_section(section) {
        app.editor_message = "CHAIN FULL";
        return;
    }
    let bar = app.settings.meter.beats_per_bar();
    if transport.is_running() || transport.start_pending() {
        let now = transport
            .pending_beat(bar)
            .unwrap_or_else(|| transport.beat(bar));
        app.phrase_player.arm_chain(now, bar);
        app.editor_message = "SECTION QUEUED";
    } else if transport.mode() == SyncMode::MidiIn {
        app.editor_message = "SECTION ADDED   START MIDI TRANSPORT";
    } else {
        app.editor_message = "SECTION ADDED   START TO PLAY CHAIN";
    }
}

fn print_controls() {
    println!("Khana v{}", env!("CARGO_PKG_VERSION"));
    println!("GLOBAL:");
    println!("  Space   transport start/stop");
    println!("  Tab     Performance / Phrase Editor; F2 opens Workspace; F3 cycles polyphony");
    println!("PERFORMANCE:");
    println!("  arrows  browse Safe / Colorful / Bold");
    println!("  Z       commit highlighted chord");
    println!("  F5/F6/F7  Sustain / voicing / Vary");
    println!("  F8/F9/F10/F11  Pattern Hold / live grid / Mute / Solo");
    println!("  X       reroll same vocabulary");
    println!("  S       surprise / broaden vocabulary");
    println!("  Q/E/W   Resolve / Neutral / Tension bias");
    println!("  C/V     previous / next performer; F4 macro group");
    println!("  T/G     density -/+");
    println!("  Y/H     dynamics -/+");
    println!("  U/J     spread -/+");
    println!("  I/K     color -/+ (role-specific note pool)");
    println!("  P       quantized record start/stop");
    println!("  N       arm next phrase");
    println!("  M       settings (stopped), or tempo (playing on internal clock)");
    println!("SETTINGS:");
    println!("  Arrows select value; -/= change; X switches tab; M exits");
    println!("  MIDI Rig: Channel, transpose, MIDI In/Out, Link");
    println!("  MIDI PANIC: Z reviews, Z again sends on assigned channels");
    println!("PERFORMERS:");
    println!("  Bass CHD | Chords BAR | Texture 2BAR | Melody PHR");
    println!("PHRASE EDITOR:");
    println!("  A/L        previous/next phrase");
    println!("  Z          play selected / queue at next bar");
    println!("  X          change selected chord; S toggles manual entry in the picker");
    println!("  S          edit menu (includes Fit Phrase and Queue Section)");
    println!("  Left/Right select chord event");
    println!("  D          duplicate event");
    println!("  Backspace  delete event");
    println!("  F/R        snap shorter/longer to edit grid");
    println!("  Up/Down    previous/next edit grid");
    println!("  [/]        previous/next edit grid");
    println!("  -/=        zoom");
    println!("  F5/F6/F12  Loop / clear queue / Undo");
    println!("BRICK PRO:");
    println!("  MENU tap: page, hold: exit; never SELECT+MENU");
    println!("  Performance: A commit B reroll X surprise L/R1 scope");
    println!("  L2/R2 step direction; Y toggles macro group membership");
    println!("  SELECT+A record, SELECT+B arm next, SELECT+X settings / live internal tempo");
    println!("  Sticks: left X/Y density/dynamics, right X/Y spread/color");
    println!("  Performance: L3 Sustain, R3 voicing, SELECT+L/R1 Mute/Solo");
    println!("  SELECT+L/R2 Pattern Hold/live grid, SELECT+R3 or SELECT+Y Vary");
    println!("  SELECT+L3 cycles polyphony on Texture or Melody; SELECT+START opens Workspace");
    println!("  Editor: A play B change X edit Y back; SELECT+Y delete");
    println!("  Editor: Queue Section builds a chain; START plays the chain while stopped");
    println!("  Workspace Takes: R3 / F6 recovers the last complete phrase pass");
    println!("  Editor: L3 Loop, R3 clear queue, SELECT+A Undo, SELECT+X Setup");
    println!("  L/R1 phrase, L/R2 duration, SELECT+L/R1 zoom, D-pad grid/event");
}

#[cfg(test)]
mod platform_tests {
    use super::ui_period;

    #[test]
    fn ui_period_accepts_only_supported_frame_rates() {
        assert_eq!(ui_period(None), ui_period(Some("30")));
        assert!(ui_period(Some("60")).unwrap() < ui_period(None).unwrap());
        assert!(ui_period(Some("0")).is_err());
        assert!(ui_period(Some("120")).is_err());
    }
}
