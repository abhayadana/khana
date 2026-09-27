//! Khaṇa v0.5 desktop prototype.
//!
//! This build establishes the intended MVP architecture:
//! chord recommendations -> smooth four-voice realization -> MIDI ->
//! quantized semantic phrase capture -> phrase chord editing.

mod clock;
mod input;
mod midi;
mod music;
mod phrase;
mod ui;
mod voice;

use std::thread;
use std::time::Duration;

use clock::BeatClock;
use input::{InputAction, map_key};
use midi::MidiEngine;
use music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};
use music::recommendation::{
    HarmonicDirection, RecommendationClass, RecommendationEngine, RecommendationSet,
};
use music::voicing::{Voicing, VoicingEngine};
use phrase::{Phrase, PhraseRecorder};
use sdl2::event::Event;
use ui::Ui;
use voice::{
    MacroKind, Voice, VoiceScope, adjust_macro, average_color, average_spread, default_voices,
};

const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 768;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Screen {
    Performance,
    PhraseEditor,
}

struct App {
    screen: Screen,
    current_chord: Chord,
    chord_started_beat: f64,
    current_voicing: Option<Voicing>,
    transition_counter: u64,
    recommendation_engine: RecommendationEngine,
    recommendations: RecommendationSet,
    selected_class: RecommendationClass,
    selected_row: usize,
    direction: HarmonicDirection,
    surprise: bool,
    voicing_engine: VoicingEngine,
    voices: [Voice; 4],
    scope: VoiceScope,
    recorder: PhraseRecorder,
    phrases: Vec<Phrase>,
    editor_phrase_index: usize,
    editor_event_index: usize,
    editor_zoom: f32,
}

impl App {
    fn new() -> Self {
        let current_chord = Chord::new(PitchClass::C, ChordQuality::Major, ChordExtension::Seventh);
        let recommendation_engine = RecommendationEngine::new();
        let direction = HarmonicDirection::Neutral;
        let recommendations = recommendation_engine.recommend(current_chord, direction, false);

        Self {
            screen: Screen::Performance,
            current_chord,
            chord_started_beat: 0.0,
            current_voicing: None,
            transition_counter: 0,
            recommendation_engine,
            recommendations,
            selected_class: RecommendationClass::Safe,
            selected_row: 0,
            direction,
            surprise: false,
            voicing_engine: VoicingEngine::new(),
            voices: default_voices(),
            scope: VoiceScope::All,
            recorder: PhraseRecorder::new(),
            phrases: Vec::new(),
            editor_phrase_index: 0,
            editor_event_index: 0,
            editor_zoom: 82.0,
        }
    }

    fn refresh_recommendations(&mut self) {
        self.recommendations =
            self.recommendation_engine
                .recommend(self.current_chord, self.direction, self.surprise);
        self.selected_row = 0;
    }

    fn chosen_chord(&self) -> Chord {
        self.recommendations
            .get(self.selected_class, self.selected_row)
    }

    fn commit_chord(
        &mut self,
        chord: Chord,
        now_beat: f64,
        midi: &mut MidiEngine,
    ) -> Result<(), String> {
        self.current_chord = chord;
        self.chord_started_beat = now_beat;
        self.transition_counter = self.transition_counter.wrapping_add(1);

        let color = average_color(&self.voices);
        let spread = average_spread(&self.voices);
        let realized_chord = chord.colored(color);
        let voicing = self
            .voicing_engine
            .realize(realized_chord, self.current_voicing, spread);

        let velocities = std::array::from_fn(|index| {
            velocity_from_dynamics(self.voices[index].parameters.dynamics)
        });
        let active = std::array::from_fn(|index| {
            density_gate(
                self.voices[index].parameters.density,
                self.transition_counter,
                index,
            )
        });

        midi.play_voicing(voicing.notes, velocities, active)?;
        self.current_voicing = Some(voicing);
        self.recorder.chord_changed(now_beat, chord);
        self.refresh_recommendations();

        println!(
            "CHORD {} -> MIDI {:?}",
            self.current_chord.symbol(),
            voicing.notes
        );

        Ok(())
    }

    fn revoice_current(&mut self, midi: &mut MidiEngine) -> Result<(), String> {
        let color = average_color(&self.voices);
        let spread = average_spread(&self.voices);
        let realized_chord = self.current_chord.colored(color);
        let voicing = self
            .voicing_engine
            .realize(realized_chord, self.current_voicing, spread);

        let velocities = std::array::from_fn(|index| {
            velocity_from_dynamics(self.voices[index].parameters.dynamics)
        });
        let active = std::array::from_fn(|index| {
            density_gate(
                self.voices[index].parameters.density,
                self.transition_counter,
                index,
            )
        });

        midi.play_voicing(voicing.notes, velocities, active)?;
        self.current_voicing = Some(voicing);
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
}

/// Runs Khaṇa v0.5.
///
/// # Errors
///
/// Returns an error if SDL or MIDI initialization/output fails.
fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let window = video
        .window(
            "Khaṇa v0.5 — chords, four voices, phrase capture",
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

    let ui = Ui::new(WINDOW_WIDTH, WINDOW_HEIGHT);
    let clock = BeatClock::new(120.0);
    let mut midi = MidiEngine::new_virtual("Khana MIDI")?;
    let mut app = App::new();

    app.commit_chord(app.current_chord, 0.0, &mut midi)?;
    print_controls();

    'running: loop {
        let now_beat = clock.beat();

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

                    match app.screen {
                        Screen::Performance => {
                            handle_performance_action(action, &mut app, now_beat, &mut midi)?;
                        }
                        Screen::PhraseEditor => {
                            handle_editor_action(action, &mut app);
                        }
                    }
                }
                _ => {}
            }
        }

        match app.screen {
            Screen::Performance => {
                ui.draw_performance(
                    &mut canvas,
                    app.current_chord,
                    now_beat - app.chord_started_beat,
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
                ui.draw_phrase_editor(
                    &mut canvas,
                    app.selected_phrase(),
                    app.editor_phrase_index,
                    app.phrases.len(),
                    app.editor_event_index,
                    app.editor_zoom,
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

fn handle_performance_action(
    action: InputAction,
    app: &mut App,
    now_beat: f64,
    midi: &mut MidiEngine,
) -> Result<(), String> {
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
            app.commit_chord(app.chosen_chord(), now_beat, midi)?;
        }
        InputAction::Reroll => {
            app.recommendation_engine.reroll();
            app.surprise = false;
            app.refresh_recommendations();
        }
        InputAction::Surprise => {
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

            // Spread and Color revoice the current harmony immediately.
            // This is a timbral/voicing gesture, not a new harmonic event, so
            // it must not reset chord duration or split a recorded phrase.
            if matches!(kind, MacroKind::Spread | MacroKind::Color) {
                app.revoice_current(midi)?;
            }
        }
        InputAction::ToggleRecord => {
            app.recorder.toggle(now_beat);
        }
        InputAction::ArmNextRecording => {
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

fn handle_editor_action(action: InputAction, app: &mut App) {
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
        _ => {}
    }
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

    let set = app
        .recommendation_engine
        .recommend(context_chord, HarmonicDirection::Neutral, false);
    let replacement = set.get(RecommendationClass::Safe, 0);

    if let Some(phrase) = app.phrases.get_mut(phrase_index) {
        phrase.replace_chord(event_index, replacement);
    }
}

fn velocity_from_dynamics(dynamics: f32) -> u8 {
    (35.0 + dynamics.clamp(0.0, 1.0) * 92.0).round() as u8
}

fn density_gate(density: f32, transition: u64, voice_index: usize) -> bool {
    let mixed = transition
        .wrapping_mul(1_103_515_245)
        .wrapping_add((voice_index as u64 + 1) * 12_345);
    let value = ((mixed >> 8) & 0xFFFF) as f32 / 65_535.0;

    value <= density.clamp(0.0, 1.0)
}

fn print_controls() {
    println!("Khaṇa v0.5.0");
    println!("PERFORMANCE:");
    println!("  arrows  browse Safe / Colorful / Bold");
    println!("  Z       commit highlighted chord");
    println!("  X       reroll");
    println!("  S       surprise");
    println!("  Q/E/W   Resolve / Neutral / Tension bias");
    println!("  C/V     previous / next voice scope");
    println!("  T/G     density -/+");
    println!("  Y/H     dynamics -/+");
    println!("  U/J     spread -/+");
    println!("  I/K     color -/+");
    println!("  P       quantized record start/stop");
    println!("  N       arm next phrase");
    println!("  Tab     Phrase Editor");
    println!("PHRASE EDITOR:");
    println!("  Left/Right select chord event");
    println!("  A/L        previous/next phrase");
    println!("  Z          replace selected chord with Safe suggestion");
    println!("  D          duplicate event");
    println!("  Backspace  delete event");
    println!("  F/R        shorter/longer by one beat");
    println!("  -/+        zoom");
    println!("  Tab        Performance");
}
