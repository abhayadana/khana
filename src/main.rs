//! Khaṇa v0.11.5 MIDI Clock Out and live tempo.
//!
//! v0.11 adds complete semantic chord voicings, rhythmic/gated Melody phrases,
//! persistent cyclic Texture, and lightweight performer-to-performer awareness.

mod input;
mod midi;
mod music;
mod performer;
mod phrase;
mod runtime;
mod sync;
mod ui;
mod voice;

use std::thread;
use std::time::{Duration, Instant};

use input::{HandheldInput, InputAction, InputPage, desktop_event, open_input_devices};
use midi::MidiEngine;
use music::chord::Chord;
use music::recommendation::{
    HarmonicDirection, RecommendationClass, RecommendationEngine, RecommendationSet,
};
use music::settings::{MusicalSettings, SettingField};
use performer::PerformerEngine;
use phrase::{EditGrid, Phrase, PhrasePlayer, PhraseRecorder};
use sdl2::event::Event;
use sync::{SyncMode, SyncTransport};
use ui::Ui;
use voice::{MacroKind, Voice, VoiceEngine, VoiceRender, VoiceScope, adjust_macro, default_voices};

const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 768;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Screen {
    Performance,
    PhraseEditor,
}

struct App {
    screen: Screen,
    settings: MusicalSettings,
    setting_focus: Option<SettingField>,
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
    recorder: PhraseRecorder,
    phrase_player: PhrasePlayer,
    phrases: Vec<Phrase>,
    editor_phrase_index: usize,
    editor_event_index: usize,
    editor_zoom: f32,
    editor_grid: EditGrid,
}

impl App {
    fn input_page(&self) -> InputPage {
        if self.setting_focus.is_some() {
            InputPage::Settings
        } else if self.screen == Screen::Performance {
            InputPage::Performance
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
            recorder: PhraseRecorder::new(),
            phrase_player: PhrasePlayer::new(),
            phrases: Vec::new(),
            editor_phrase_index: 0,
            editor_event_index: 0,
            editor_zoom: 82.0,
            editor_grid: EditGrid::Quarter,
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
                desktop_event(&event, &controllers)
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
        frame.draw(&ui, &mut canvas)?;
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
    if action == InputAction::MenuTap {
        action = if app.setting_focus.is_some() {
            InputAction::ToggleSettings
        } else {
            InputAction::ToggleScreen
        };
    }
    if action == InputAction::ToggleTransport {
        toggle_transport(app, transport);
    } else {
        match app.screen {
            Screen::Performance => handle_performance_action(action, app, transport, midi)?,
            Screen::PhraseEditor => handle_editor_action(action, app, transport)?,
        }
    }
    Ok(false)
}

fn toggle_transport(app: &mut App, transport: &mut SyncTransport) {
    app.setting_focus = None;
    if transport.is_running() {
        transport.stop();
    } else {
        transport.start();
    }
}

fn handle_performance_action(
    action: InputAction,
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
) -> Result<(), String> {
    if let Some(field) = app.setting_focus {
        return handle_setting_action(action, field, app, transport, midi);
    }

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
            app.phrase_player.stop();
            app.set_harmony(app.chosen_chord(), now_beat, transport.is_running(), true);
        }
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
        InputAction::BiasResolve => {
            app.direction = HarmonicDirection::Resolve;
            app.refresh_recommendations();
        }
        InputAction::BiasNeutral => {
            app.direction = HarmonicDirection::Neutral;
            app.refresh_recommendations();
        }
        InputAction::BiasTension => {
            app.direction = HarmonicDirection::Tension;
            app.refresh_recommendations();
        }
        InputAction::ScopePrevious => {
            app.scope = app.scope.previous();
        }
        InputAction::ScopeNext => {
            app.scope = app.scope.next();
        }
        InputAction::AdjustMacro { kind, delta } => {
            let bass_density_changed = kind == MacroKind::Density
                && matches!(app.scope, VoiceScope::All | VoiceScope::One(0));
            adjust_macro(&mut app.voices, app.scope, kind, delta);

            if kind == MacroKind::Spread || bass_density_changed {
                app.revoice_current();
            }
        }
        InputAction::ToggleRecord => {
            if !transport.is_running() {
                transport.start();
            }

            app.recorder.toggle(
                transport.beat(app.settings.meter.beats_per_bar()),
                app.settings.meter.beats_per_bar(),
            );
        }
        InputAction::ArmNextRecording => {
            app.recorder.toggle_arm_next();
        }
        InputAction::ToggleScreen => {
            app.screen = Screen::PhraseEditor;
            app.clamp_editor_selection();
        }
        InputAction::ToggleSettings if !transport.is_running() => {
            app.setting_focus = Some(SettingField::Tonic);
        }
        InputAction::ToggleSettings if transport.mode() == SyncMode::Internal => {
            app.setting_focus = Some(SettingField::Tempo);
        }
        InputAction::ToggleSettings => {}
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
    let running = transport.is_running();
    match action {
        InputAction::Primary if !running && field == SettingField::MidiPanic => {
            midi.panic()?;
            app.setting_focus = None;
        }
        InputAction::ToggleSettings | InputAction::Primary => {
            app.setting_focus = None;
        }
        InputAction::NavigateLeft if !running => {
            app.setting_focus = Some(field.previous());
        }
        InputAction::NavigateRight if !running => {
            app.setting_focus = Some(field.next());
        }
        InputAction::NavigateUp if !running || live_tempo(field, transport) => {
            adjust_setting(1, field, app, transport, midi)?;
        }
        InputAction::NavigateDown if !running || live_tempo(field, transport) => {
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
        SettingField::Tempo => {
            app.settings.tempo_bpm =
                (app.settings.tempo_bpm + f64::from(direction)).clamp(30.0, 300.0);
            transport.set_tempo(app.settings.tempo_bpm);
        }
        SettingField::Meter => {
            app.settings.meter = if direction > 0 {
                app.settings.meter.next()
            } else {
                app.settings.meter.previous()
            };
        }
        SettingField::Sync => {
            let next_mode = match transport.mode() {
                SyncMode::Internal => SyncMode::Link,
                SyncMode::Link => SyncMode::Internal,
            };

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
        SettingField::MidiClockOut => midi.toggle_clock_out(),
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
) -> Result<(), String> {
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
        InputAction::Primary => {
            if app.selected_phrase().is_some() {
                if !transport.is_running() {
                    transport.start();
                }

                app.performer_engine.set_enabled(true);
                app.phrase_player.request(
                    app.editor_phrase_index,
                    transport.beat(app.settings.meter.beats_per_bar()),
                    app.settings.meter.beats_per_bar(),
                );
            }
        }
        InputAction::Secondary => {
            replace_selected_with_safe_choice(app);
        }
        InputAction::EditDuplicate => {
            let index = app.editor_event_index;
            if let Some(phrase) = app.selected_phrase_mut() {
                phrase.duplicate_event(index);
            }
            app.editor_event_index = app.editor_event_index.saturating_add(1);
            app.clamp_editor_selection();
        }
        InputAction::EditDelete => {
            let index = app.editor_event_index;
            if let Some(phrase) = app.selected_phrase_mut() {
                phrase.delete_event(index);
            }
            app.clamp_editor_selection();
        }
        InputAction::EditShorten => {
            let index = app.editor_event_index;
            let grid = app
                .editor_grid
                .beats(app.settings.meter.numerator, app.settings.meter.denominator);
            if let Some(phrase) = app.selected_phrase_mut() {
                phrase.snap_resize_event(index, grid, false);
            }
        }
        InputAction::EditExtend => {
            let index = app.editor_event_index;
            let grid = app
                .editor_grid
                .beats(app.settings.meter.numerator, app.settings.meter.denominator);
            if let Some(phrase) = app.selected_phrase_mut() {
                phrase.snap_resize_event(index, grid, true);
            }
        }
        InputAction::GridPrevious | InputAction::NavigateUp => {
            app.editor_grid = app.editor_grid.previous();
        }
        InputAction::GridNext | InputAction::NavigateDown => {
            app.editor_grid = app.editor_grid.next();
        }
        InputAction::ZoomOut => {
            app.editor_zoom = (app.editor_zoom - 12.0).max(24.0);
        }
        InputAction::ZoomIn => {
            app.editor_zoom = (app.editor_zoom + 12.0).min(180.0);
        }
        InputAction::ToggleSettings if !transport.is_running() => {
            app.screen = Screen::Performance;
            app.setting_focus = Some(SettingField::Tonic);
        }
        InputAction::ToggleSettings => {}
        _ => {}
    }

    Ok(())
}

fn replace_selected_with_safe_choice(app: &mut App) {
    let phrase_index = app.editor_phrase_index;
    let event_index = app.editor_event_index;

    let Some(phrase) = app.phrases.get(phrase_index) else {
        return;
    };
    if phrase.events.get(event_index).is_none() {
        return;
    }

    let mut history = phrase.events[..event_index]
        .iter()
        .map(|event| event.chord)
        .collect::<Vec<_>>();

    if history.is_empty() {
        history.push(app.settings.tonal.tonic_chord());
    }

    let set = app.recommendation_engine.recommend(
        &history,
        app.settings.tonal,
        HarmonicDirection::Neutral,
        false,
    );
    let replacement = set.get(RecommendationClass::Safe, 0);

    if let Some(phrase) = app.phrases.get_mut(phrase_index) {
        phrase.replace_chord(event_index, replacement);
    }
}

fn print_controls() {
    println!("Khana v{}", env!("CARGO_PKG_VERSION"));
    println!("GLOBAL:");
    println!("  Space   transport start/stop");
    println!("  Tab     Performance / Phrase Editor");
    println!("PERFORMANCE:");
    println!("  arrows  browse Safe / Colorful / Bold");
    println!("  Z       commit highlighted chord");
    println!("  X       reroll same vocabulary");
    println!("  S       surprise / broaden vocabulary");
    println!("  Q/E/W   Resolve / Neutral / Tension bias");
    println!("  C/V     previous / next voice scope");
    println!("  T/G     density -/+");
    println!("  Y/H     dynamics -/+");
    println!("  U/J     spread -/+");
    println!("  I/K     color -/+ (role-specific note pool)");
    println!("  P       quantized record start/stop");
    println!("  N       arm next phrase");
    println!("  M       settings (stopped), or tempo (playing on internal clock)");
    println!("SETTINGS:");
    println!("  Left/Right field   Up/Down value   Z/M exit");
    println!("  SYNC INTERNAL/LINK; STARTSYNC OFF/ON");
    println!("  CLOCK OUT: Up/Down toggle; MIDI PANIC: Z send on channels 1-4");
    println!("PERFORMERS:");
    println!("  Bass CHD | Chords BAR | Texture 2BAR | Melody PHR");
    println!("PHRASE EDITOR:");
    println!("  A/L        previous/next phrase");
    println!("  Z          play selected / queue at next bar");
    println!("  X          change selected chord");
    println!("  Left/Right select chord event");
    println!("  D          duplicate event");
    println!("  Backspace  delete event");
    println!("  F/R        snap shorter/longer to edit grid");
    println!("  Up/Down    previous/next edit grid");
    println!("  [/]        previous/next edit grid");
    println!("  -/+        zoom");
    println!("BRICK BRING-UP:");
    println!("  MENU tap: page, hold: exit; never SELECT+MENU");
    println!("  Performance: A commit B reroll X surprise Y neutral L/R1 scope L/R2 bias");
    println!("  SELECT+A record, SELECT+B arm next, SELECT+X settings / live internal tempo");
    println!("  Sticks: left X/Y density/dynamics, right X/Y spread/color");
    println!("  Editor: A play B change X duplicate Y back; SELECT+Y delete");
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
