//! Khaṇa v0.6 desktop prototype.
//!
//! v0.6 adds transport, editable tonal/metric settings, visible voicing,
//! semantic phrase playback/queueing, and an explicit chord -> voice-engine
//! boundary while retaining the two-screen MVP.

mod input;
mod midi;
mod music;
mod phrase;
mod sync;
mod ui;
mod voice;

use std::thread;
use std::time::Duration;

use input::{InputAction, map_key};
use midi::MidiEngine;
use music::chord::Chord;
use music::recommendation::{
    HarmonicDirection, RecommendationClass, RecommendationEngine, RecommendationSet,
};
use music::settings::{MusicalSettings, SettingField};
use phrase::{Phrase, PhrasePlayer, PhraseRecorder};
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
    recommendation_engine: RecommendationEngine,
    recommendations: RecommendationSet,
    selected_class: RecommendationClass,
    selected_row: usize,
    direction: HarmonicDirection,
    surprise: bool,
    voice_engine: VoiceEngine,
    voices: [Voice; 4],
    scope: VoiceScope,
    recorder: PhraseRecorder,
    phrase_player: PhrasePlayer,
    phrases: Vec<Phrase>,
    editor_phrase_index: usize,
    editor_event_index: usize,
    editor_zoom: f32,
}

impl App {
    fn new() -> Self {
        let settings = MusicalSettings::default();
        let current_chord = settings.tonal.tonic_chord();
        let recommendation_engine = RecommendationEngine::new();
        let direction = HarmonicDirection::Neutral;
        let recommendations =
            recommendation_engine.recommend(current_chord, settings.tonal, direction, false);
        let voices = default_voices();
        let mut voice_engine = VoiceEngine::new();
        let current_render = voice_engine.harmonic_change(current_chord, &voices);

        Self {
            screen: Screen::Performance,
            settings,
            setting_focus: None,
            current_chord,
            chord_started_beat: 0.0,
            current_render,
            recommendation_engine,
            recommendations,
            selected_class: RecommendationClass::Safe,
            selected_row: 0,
            direction,
            surprise: false,
            voice_engine,
            voices,
            scope: VoiceScope::All,
            recorder: PhraseRecorder::new(),
            phrase_player: PhrasePlayer::new(),
            phrases: Vec::new(),
            editor_phrase_index: 0,
            editor_event_index: 0,
            editor_zoom: 82.0,
        }
    }

    fn refresh_recommendations(&mut self) {
        self.recommendations = self.recommendation_engine.recommend(
            self.current_chord,
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

    fn set_harmony(
        &mut self,
        chord: Chord,
        now_beat: f64,
        midi: &mut MidiEngine,
        transport_running: bool,
        record: bool,
    ) -> Result<(), String> {
        self.current_chord = chord;
        self.chord_started_beat = now_beat;
        self.current_render = self.voice_engine.harmonic_change(chord, &self.voices);

        if transport_running {
            midi.play_render(self.current_render)?;
        }

        if record {
            self.recorder.chord_changed(now_beat, chord);
        }

        self.refresh_recommendations();

        println!(
            "CHORD {} / {} -> {:?}",
            self.current_chord.symbol(),
            self.current_render.realized_chord.symbol(),
            self.current_render.voicing.notes
        );

        Ok(())
    }

    fn revoice_current(
        &mut self,
        midi: &mut MidiEngine,
        transport_running: bool,
    ) -> Result<(), String> {
        self.current_render = self.voice_engine.revoice(self.current_chord, &self.voices);

        if transport_running {
            midi.play_render(self.current_render)?;
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
        midi.stop_all()?;
        self.refresh_recommendations();
        Ok(())
    }
}

/// Runs Khaṇa v0.6.
///
/// # Errors
///
/// Returns an error if SDL or MIDI initialization/output fails.
fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let window = video
        .window(
            "Khaṇa v0.6.5 — corrected harmony and voicing",
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
        )
        .position_centered()
        .build()
        .map_err(|error| error.to_string())?;
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .map_err(|error| error.to_string())?;
    let mut event_pump = sdl_context.event_pump()?;

    let ui = Ui::new(WINDOW_WIDTH);
    let mut app = App::new();
    let mut transport = SyncTransport::new(app.settings.tempo_bpm);
    let mut midi = MidiEngine::new_virtual("Khana MIDI")?;
    let mut was_running = false;

    print_controls();

    'running: loop {
        let now_beat = transport.beat(app.settings.meter.beats_per_bar());
        let running_now = transport.is_running();

        if running_now != was_running {
            if running_now {
                app.chord_started_beat = now_beat;
                midi.play_render(app.current_render)?;
            } else {
                app.phrase_player.stop();
                midi.stop_all()?;
            }
            was_running = running_now;
        }

        if running_now {
            if let Some(phrase) = app.recorder.update(now_beat, app.current_chord) {
                println!(
                    "CAPTURED P{:02}: {} events, {:.1} beats",
                    phrase.id,
                    phrase.events.len(),
                    phrase.duration_beats()
                );
                app.phrases.push(phrase);
                app.editor_phrase_index = app.phrases.len().saturating_sub(1);
                app.editor_event_index = 0;
            }

            let playback = app.phrase_player.update(&app.phrases, now_beat);

            if let Some(chord) = playback.chord {
                app.set_harmony(chord, now_beat, &mut midi, true, false)?;
            }

            if playback.ended && app.phrase_player.queued_phrase().is_none() {
                midi.stop_all()?;
            }
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    repeat: false,
                    ..
                } => {
                    let Some(action) = map_key(keycode) else {
                        continue;
                    };

                    if action == InputAction::Quit {
                        break 'running;
                    }

                    if action == InputAction::ToggleTransport {
                        toggle_transport(&mut app, &mut transport);
                        continue;
                    }

                    match app.screen {
                        Screen::Performance => {
                            handle_performance_action(action, &mut app, &mut transport, &mut midi)?;
                        }
                        Screen::PhraseEditor => {
                            handle_editor_action(action, &mut app, &mut transport)?;
                        }
                    }
                }
                _ => {}
            }
        }

        let now_beat = transport.beat(app.settings.meter.beats_per_bar());

        match app.screen {
            Screen::Performance => {
                let sync_status = transport.status();
                app.settings.tempo_bpm = sync_status.tempo_bpm;

                ui.draw_performance(
                    &mut canvas,
                    app.settings,
                    app.setting_focus,
                    sync_status,
                    app.current_chord,
                    now_beat - app.chord_started_beat,
                    app.current_render,
                    &app.recommendations,
                    app.selected_class,
                    app.selected_row,
                    app.direction,
                    &app.voices,
                    app.scope,
                    app.recorder.status(),
                    app.recorder.next_is_armed(),
                    app.recorder.quantize_beats(),
                    app.phrases.len(),
                )?;
            }
            Screen::PhraseEditor => {
                let sync_status = transport.status();
                app.settings.tempo_bpm = sync_status.tempo_bpm;

                ui.draw_phrase_editor(
                    &mut canvas,
                    app.settings,
                    sync_status,
                    app.selected_phrase(),
                    app.editor_phrase_index,
                    app.phrases.len(),
                    app.editor_event_index,
                    app.editor_zoom,
                    app.phrase_player.playing_phrase(),
                    app.phrase_player.queued_phrase(),
                )?;
            }
        }

        canvas.present();
        thread::sleep(Duration::from_millis(1));
    }

    midi.stop_all()?;
    println!("Khaṇa exited cleanly.");
    Ok(())
}

fn toggle_transport(app: &mut App, transport: &mut SyncTransport) {
    if transport.is_running() {
        transport.stop();
    } else {
        app.setting_focus = None;
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
            app.set_harmony(
                app.chosen_chord(),
                now_beat,
                midi,
                transport.is_running(),
                true,
            )?;
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
            adjust_macro(&mut app.voices, app.scope, kind, delta);

            if matches!(kind, MacroKind::Spread | MacroKind::Color) {
                app.revoice_current(midi, transport.is_running())?;
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
    match action {
        InputAction::ToggleSettings | InputAction::Primary => {
            app.setting_focus = None;
        }
        InputAction::NavigateLeft => {
            app.setting_focus = Some(field.previous());
        }
        InputAction::NavigateRight => {
            app.setting_focus = Some(field.next());
        }
        InputAction::NavigateUp => {
            adjust_setting(1, field, app, transport, midi)?;
        }
        InputAction::NavigateDown => {
            adjust_setting(-1, field, app, transport, midi)?;
        }
        _ => {}
    }

    Ok(())
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

            if !transport.set_mode(next_mode, app.settings.meter.beats_per_bar()) {
                eprintln!("Ableton Link is not compiled in. Rebuild with --features ableton-link");
            }

            app.settings.tempo_bpm = transport.tempo();
        }
        SettingField::StartStopSync => {
            let status = transport.status();
            transport.set_link_start_stop_sync(!status.link_start_stop_sync);
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
            if let Some(phrase) = app.selected_phrase_mut() {
                phrase.resize_event(index, -1.0);
            }
        }
        InputAction::EditExtend => {
            let index = app.editor_event_index;
            if let Some(phrase) = app.selected_phrase_mut() {
                phrase.resize_event(index, 1.0);
            }
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
    let Some(event) = phrase.events.get(event_index) else {
        return;
    };

    let context_chord = if event_index > 0 {
        phrase.events[event_index - 1].chord
    } else {
        event.chord
    };

    let set = app.recommendation_engine.recommend(
        context_chord,
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
    println!("Khaṇa v0.6.5");
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
    println!("  I/K     color -/+");
    println!("  P       quantized record start/stop");
    println!("  N       arm next phrase");
    println!("  M       edit tonic/mode/tempo/meter/sync (stopped)");
    println!("SETTINGS:");
    println!("  Left/Right field   Up/Down value   Z/M exit");
    println!("  SYNC INTERNAL/LINK; STARTSYNC OFF/ON");
    println!("PHRASE EDITOR:");
    println!("  A/L        previous/next phrase");
    println!("  Z          play selected / queue at next bar");
    println!("  X          change selected chord");
    println!("  Left/Right select chord event");
    println!("  D          duplicate event");
    println!("  Backspace  delete event");
    println!("  F/R        shorter/longer by one beat");
    println!("  -/+        zoom");
}
