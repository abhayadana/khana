//! Musical runtime: SDL never owns or locks the clock, performer, or MIDI sink.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::input::{InputAction, InputPage};
use crate::midi::MidiEngine;
use crate::music::chord::Chord;
use crate::music::recommendation::{HarmonicDirection, RecommendationClass, RecommendationSet};
use crate::music::settings::{MusicalSettings, SettingField};
use crate::performer::PerformerSnapshot;
use crate::phrase::{EditGrid, Phrase, RecorderStatus};
use crate::sync::{SyncMode, SyncStatus, SyncTransport};
use crate::ui::{PerformanceView, PhraseEditorView, Ui};
use crate::voice::{Voice, VoiceRender, VoiceScope};
use crate::{App, Screen, dispatch_action};

const POLL_PERIOD: Duration = Duration::from_millis(1);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);

/// Owned render data; drawing and present never hold a musical-state lock.
pub(crate) struct Frame {
    screen: Screen,
    worker_ticks: u64,
    active_midi: [u32; 4],
    settings: MusicalSettings,
    setting_focus: Option<SettingField>,
    sync_status: SyncStatus,
    current_chord: Chord,
    chord_elapsed_beats: f64,
    current_render: VoiceRender,
    recommendations: RecommendationSet,
    selected_class: RecommendationClass,
    selected_row: usize,
    direction: HarmonicDirection,
    voices: [Voice; 4],
    performer_snapshots: [PerformerSnapshot; 4],
    scope: VoiceScope,
    recorder_status: RecorderStatus,
    next_recording_armed: bool,
    quantize_beats: f64,
    phrase_count: usize,
    midi_destination: String,
    midi_clock: &'static str,
    midi_channels: [u8; 4],
    midi_transpose: [i8; 4],
    midi_input: Option<String>,
    phrase: Option<Phrase>,
    phrase_index: usize,
    selected_event: usize,
    pixels_per_beat: f32,
    playing_phrase: Option<usize>,
    queued_phrase: Option<usize>,
    edit_grid: EditGrid,
    timing: String,
    beat_flash: Option<bool>,
    options: String,
}

impl Frame {
    fn capture(
        app: &mut App,
        transport: &mut SyncTransport,
        midi: &MidiEngine,
        worker_ticks: u64,
    ) -> Self {
        let now_beat = transport.beat(app.settings.meter.beats_per_bar());
        let sync_status = transport.status();
        app.settings.tempo_bpm = sync_status.tempo_bpm;
        let bar = app.settings.meter.beats_per_bar();
        let beat =
            EditGrid::Beat.beats(app.settings.meter.numerator, app.settings.meter.denominator);
        let bar_phase = now_beat.rem_euclid(bar);
        let beat_index = (bar_phase / beat).floor() as u32;
        Self {
            screen: app.screen,
            worker_ticks,
            active_midi: midi.active_counts(),
            settings: app.settings,
            setting_focus: app.setting_focus,
            sync_status,
            current_chord: app.current_chord,
            chord_elapsed_beats: now_beat - app.chord_started_beat,
            current_render: app.current_render,
            recommendations: app.recommendations.clone(),
            selected_class: app.selected_class,
            selected_row: app.selected_row,
            direction: app.direction,
            voices: app.voices,
            performer_snapshots: app.performer_engine.snapshots(),
            scope: app.scope,
            recorder_status: app.recorder.status(),
            next_recording_armed: app.recorder.next_is_armed(),
            quantize_beats: app.recorder.quantize_beats(),
            phrase_count: app.phrases.len(),
            midi_destination: midi.destination_name().to_owned(),
            midi_clock: if sync_status.mode == SyncMode::MidiIn {
                "CLOCK OUT BLOCKED"
            } else {
                midi.clock_label()
            },
            midi_channels: midi.channels(),
            midi_transpose: midi.transpositions(),
            midi_input: transport.midi_input_label().map(str::to_owned),
            phrase: if app.screen == Screen::PhraseEditor {
                app.selected_phrase().cloned()
            } else {
                None
            },
            phrase_index: app.editor_phrase_index,
            selected_event: app.editor_event_index,
            pixels_per_beat: app.editor_zoom,
            playing_phrase: app.phrase_player.playing_phrase(),
            queued_phrase: app.phrase_player.queued_phrase(),
            edit_grid: app.editor_grid,
            timing: {
                let pulse = if sync_status.running {
                    (beat_index + 1).to_string()
                } else {
                    "-".into()
                };
                let pending = app.pending_chord.map_or(String::new(), |(chord, at)| {
                    format!(
                        "  {} NEXT {:.1}B",
                        app.settings.tonal.chord_symbol(chord),
                        (at - now_beat).max(0.0)
                    )
                });
                format!("BEAT {pulse}  GRID {}{pending}", app.live_grid.label())
            },
            beat_flash: (sync_status.running && bar_phase.rem_euclid(beat) < beat * 0.22)
                .then_some(beat_index == 0),
            options: match app.setting_focus {
                Some(SettingField::ChordHold) => {
                    format!("CHORD HOLD {}", app.performer_engine.hold_label())
                }
                Some(SettingField::BassArrival) => {
                    format!("BASS ARRIVAL {}", app.performer_engine.bass_label())
                }
                Some(SettingField::Voicing) => {
                    format!("VOICING {}", app.voice_engine.style.label())
                }
                _ => String::new(),
            },
        }
    }

    pub(crate) fn diagnostics(&self) -> (u64, [u32; 4]) {
        (self.worker_ticks, self.active_midi)
    }

    pub(crate) fn draw(
        &self,
        ui: &Ui,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    ) -> Result<(), String> {
        match self.screen {
            Screen::Performance => ui.draw_performance(
                canvas,
                PerformanceView {
                    timing: &self.timing,
                    beat_flash: self.beat_flash,
                    settings: self.settings,
                    setting_focus: self.setting_focus,
                    options: &self.options,
                    sync_status: self.sync_status,
                    current_chord: self.current_chord,
                    chord_elapsed_beats: self.chord_elapsed_beats,
                    current_render: self.current_render,
                    recommendations: &self.recommendations,
                    selected_class: self.selected_class,
                    selected_row: self.selected_row,
                    direction: self.direction,
                    voices: &self.voices,
                    performer_snapshots: self.performer_snapshots,
                    scope: self.scope,
                    recorder_status: self.recorder_status,
                    next_recording_armed: self.next_recording_armed,
                    quantize_beats: self.quantize_beats,
                    phrase_count: self.phrase_count,
                    midi_destination: &self.midi_destination,
                    midi_clock: self.midi_clock,
                    midi_channels: self.midi_channels,
                    midi_transpose: self.midi_transpose,
                    midi_input: self.midi_input.as_deref(),
                },
            ),
            Screen::PhraseEditor => ui.draw_phrase_editor(
                canvas,
                PhraseEditorView {
                    timing: &self.timing,
                    beat_flash: self.beat_flash,
                    settings: self.settings,
                    sync_status: self.sync_status,
                    phrase: self.phrase.as_ref(),
                    phrase_index: self.phrase_index,
                    phrase_count: self.phrase_count,
                    selected_event: self.selected_event,
                    pixels_per_beat: self.pixels_per_beat,
                    playing_phrase: self.playing_phrase,
                    queued_phrase: self.queued_phrase,
                    edit_grid: self.edit_grid,
                },
            ),
        }
    }
}

enum Command {
    Action(InputAction, SyncSender<(InputPage, bool)>),
    Snapshot(SyncSender<Frame>),
}

/// The UI submits one request at a time. Action acknowledgments preserve
/// screen-dependent input semantics without allowing unbounded command queues.
pub(crate) struct Worker {
    tx: Option<Sender<Command>>,
    handle: Option<JoinHandle<Result<(), String>>>,
    page: InputPage,
}

impl Worker {
    pub(crate) fn spawn(brick: bool) -> Result<Self, String> {
        Self::start_with_session(
            move || MidiEngine::configured(brick),
            crate::session::path(brick),
        )
    }

    #[cfg(test)]
    fn start_with<F>(make_midi: F) -> Result<Self, String>
    where
        F: FnOnce() -> Result<MidiEngine, String> + Send + 'static,
    {
        Self::start_with_session(make_midi, None)
    }

    fn start_with_session<F>(make_midi: F, path: Option<std::path::PathBuf>) -> Result<Self, String>
    where
        F: FnOnce() -> Result<MidiEngine, String> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let handle = thread::Builder::new()
            .name("khana-music".to_owned())
            .spawn(move || {
                // Construct platform objects here; no requirement for Link or
                // midir handles themselves to implement Send.
                let mut app = App::new();
                let mut transport = SyncTransport::new(app.settings.tempo_bpm);
                let mut midi = make_midi()?;
                let mut store = path.and_then(|path| {
                    crate::session::Store::open(path, &mut app, &mut transport, &mut midi)
                });
                if ready_tx.send(()).is_err() {
                    return Ok(());
                }
                let result = run(&rx, &mut app, &mut transport, &mut midi, store.as_mut());
                let stop_clock = midi.stop_clock();
                let cleanup = midi.stop_all();
                if let Some(store) = store {
                    store.finish(&app, &mut transport, &midi);
                }
                result.and(stop_clock).and(cleanup)
            })
            .map_err(|error| format!("Cannot start music worker: {error}"))?;
        let mut worker = Self {
            tx: Some(tx),
            handle: Some(handle),
            page: InputPage::Performance,
        };
        if ready_rx.recv_timeout(RESPONSE_TIMEOUT).is_err() {
            worker.shutdown()?;
            return Err("Music worker did not initialize".to_owned());
        }
        Ok(worker)
    }

    pub(crate) fn input_page(&self) -> InputPage {
        self.page
    }

    fn request<T>(&self, make: impl FnOnce(SyncSender<T>) -> Command) -> Result<T, String> {
        let (reply, response) = mpsc::sync_channel(1);
        self.tx
            .as_ref()
            .ok_or_else(|| "Music worker closed".to_owned())?
            .send(make(reply))
            .map_err(|_| "Music worker disconnected".to_owned())?;
        response
            .recv_timeout(RESPONSE_TIMEOUT)
            .map_err(|error| format!("Music worker response: {error}"))
    }

    pub(crate) fn action(&mut self, action: InputAction) -> Result<bool, String> {
        let (page, quit) = self.request(|reply| Command::Action(action, reply))?;
        self.page = page;
        Ok(quit)
    }

    pub(crate) fn snapshot(&self) -> Result<Frame, String> {
        self.request(Command::Snapshot)
    }

    pub(crate) fn shutdown(&mut self) -> Result<(), String> {
        // Dropping the last sender wakes recv_timeout even when stopped.
        drop(self.tx.take());
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| "Music worker panicked".to_owned())??;
        }
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            eprintln!("Music worker shutdown: {error}");
        }
    }
}

fn run(
    rx: &Receiver<Command>,
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
    mut store: Option<&mut crate::session::Store>,
) -> Result<(), String> {
    let mut save_at = None;
    let mut phrases = app.phrases.len();
    let mut was_running = false;
    let mut last_poll = Instant::now();
    let mut last_report = last_poll;
    let mut max_gap = Duration::ZERO;
    let mut max_work = Duration::ZERO;
    let mut worker_ticks = 0_u64;

    loop {
        let poll = Instant::now();
        worker_ticks = worker_ticks.saturating_add(1);
        max_gap = max_gap.max(poll.duration_since(last_poll));
        last_poll = poll;
        midi.service();
        let live_commit = tick(app, transport, midi, &mut was_running)?;
        midi.service();
        max_work = max_work.max(poll.elapsed());
        if live_commit || phrases != app.phrases.len() {
            phrases = app.phrases.len();
            save_at = Some(poll + Duration::from_millis(650));
        }
        if save_at.is_some_and(|at| poll >= at) {
            save_at = store.as_deref_mut().and_then(|store| {
                (!store.offer(app, transport, midi)).then(|| poll + Duration::from_millis(650))
            });
        }
        if poll.duration_since(last_report) >= Duration::from_secs(5) {
            let (expired, late_beats) = app.performer_engine.take_timing_diagnostics();
            println!(
                "MUSIC DIAG poll {:.2} ms | work {:.2} ms | expired attacks {} | max attack lateness {:.4} beats | active MIDI {:?}",
                max_gap.as_secs_f64() * 1000.0,
                max_work.as_secs_f64() * 1000.0,
                expired,
                late_beats,
                midi.active_counts()
            );
            midi.report();
            max_gap = Duration::ZERO;
            max_work = Duration::ZERO;
            last_report = poll;
        }
        // Service musical time between every command, including rapid stick
        // adjustments. Sleep only the unused part of this poll period.
        match rx.recv_timeout(POLL_PERIOD.saturating_sub(poll.elapsed())) {
            Ok(Command::Action(action, reply)) => {
                let quit = dispatch_action(action, app, transport, midi)?;
                save_at = Some(Instant::now() + Duration::from_millis(650));
                // Apply transport transitions before acknowledging the action.
                if !quit {
                    tick(app, transport, midi, &mut was_running)?;
                    midi.service();
                }
                if reply.send((app.input_page(), quit)).is_err() || quit {
                    break;
                }
            }
            Ok(Command::Snapshot(reply)) => {
                if reply
                    .send(Frame::capture(app, transport, midi, worker_ticks))
                    .is_err()
                {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn advance_recorder(app: &mut App, beat: f64) {
    if let Some(phrase) = app.recorder.update(beat, app.current_chord) {
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
}

fn tick(
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
    was_running: &mut bool,
) -> Result<bool, String> {
    transport.poll_midi(Instant::now());
    let bar_beats = app.settings.meter.beats_per_bar();
    let now_beat = transport.beat(bar_beats);
    let running_now = transport.is_running();
    let link_target = transport.pending_beat(bar_beats);
    let link_launch = (transport.mode() == SyncMode::Link).then_some((bar_beats, link_target));
    let clock_running =
        transport.mode() != SyncMode::MidiIn && (running_now || link_target.is_some());
    midi.update_clock(now_beat, clock_running, link_launch)?;

    if transport.take_midi_restart() {
        app.pending_chord = None;
        app.recorder.reset_timeline();
        if *was_running {
            app.phrase_player.stop();
            app.performer_engine.transport_stopped();
            midi.stop_all()?;
            *was_running = false;
        }
    }

    if running_now != *was_running {
        if running_now {
            app.setting_focus = None;
            app.chord_started_beat = now_beat;
            app.performer_engine.transport_started(now_beat);
        } else {
            app.pending_chord = None;
            app.phrase_player.stop();
            app.performer_engine.transport_stopped();
            midi.stop_all()?;
        }
        *was_running = running_now;
    }

    let mut live_commit = false;
    if running_now {
        if let Some((chord, at)) = app.pending_chord
            && now_beat >= at
        {
            advance_recorder(app, at);
            live_commit = true;
            app.pending_chord = None;
            app.phrase_player.stop();
            app.set_harmony(chord, at, true, true);
        }
        advance_recorder(app, now_beat);
        let playback = app.phrase_player.update(&app.phrases, now_beat);
        if let Some((chord, at)) = playback.chord.zip(playback.chord_beat)
            && chord != app.current_chord
        {
            app.set_harmony(chord, at, true, false);
        }
        let phrase_next = playback.next_chord.zip(playback.next_chord_beat);
        let next = match (app.pending_chord, phrase_next) {
            (Some(a), Some(b)) => Some(if a.1 <= b.1 { a } else { b }),
            (a, b) => a.or(b),
        };
        app.performer_engine
            .set_next_chord_target(next.map(|(chord, _)| chord), next.map(|(_, beat)| beat));
        if playback.ended && app.phrase_player.queued_phrase().is_none() {
            app.performer_engine.set_enabled(false);
            midi.stop_all()?;
        }
        app.update_performers(now_beat, midi)?;
    }
    Ok(live_commit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle_performance_action;

    #[test]
    fn live_commit_replaces_pending_chord_and_records_its_boundary() {
        let mut app = App::new();
        let mut midi = MidiEngine::dry_run();
        let mut transport = SyncTransport::new(app.settings.tempo_bpm);
        transport.start(4.0);
        let mut running = false;
        tick(&mut app, &mut transport, &mut midi, &mut running).unwrap();
        app.recorder.toggle(0.0, 0.25);
        app.live_grid = crate::phrase::LiveGrid::Bar;
        let initial = app.current_chord;
        handle_performance_action(InputAction::Primary, &mut app, &mut transport, &mut midi)
            .unwrap();
        let first = app.pending_chord.unwrap();
        assert_eq!(first.1, 4.0);
        handle_performance_action(
            InputAction::NavigateDown,
            &mut app,
            &mut transport,
            &mut midi,
        )
        .unwrap();
        handle_performance_action(InputAction::Primary, &mut app, &mut transport, &mut midi)
            .unwrap();
        let replacement = app.pending_chord.unwrap();
        assert_ne!(replacement.0, first.0);
        assert_eq!(replacement.1, 4.0);
        assert_eq!(app.current_chord, initial);

        // Supply a boundary the clock has already crossed to exercise the worker
        // without waiting an entire bar in the test.
        app.pending_chord = Some((replacement.0, 0.0));
        assert!(tick(&mut app, &mut transport, &mut midi, &mut running).unwrap());
        assert_eq!(app.current_chord, replacement.0);
        assert_eq!(app.pending_chord, None);
        app.recorder.toggle(0.1, 0.25);
        let recorded = app.recorder.update(0.25, app.current_chord).unwrap();
        assert_eq!(recorded.events.len(), 1);
        assert_eq!(recorded.events[0].chord, replacement.0);
        assert_eq!(recorded.events[0].start_beat, 0.0);
    }

    #[test]
    fn worker_advances_without_ui_requests_and_acknowledges_stop() {
        let mut worker = Worker::start_with(|| Ok(MidiEngine::dry_run())).unwrap();
        assert!(!worker.snapshot().unwrap().sync_status.running);
        assert!(!worker.action(InputAction::ToggleTransport).unwrap());
        let first = worker.snapshot().unwrap();
        thread::sleep(Duration::from_millis(80));
        let second = worker.snapshot().unwrap();
        assert!(second.chord_elapsed_beats > first.chord_elapsed_beats);
        assert!(second.worker_ticks > first.worker_ticks + 1);
        assert!(!worker.action(InputAction::ToggleTransport).unwrap());
        let stopped = worker.snapshot().unwrap();
        assert!(!stopped.sync_status.running);
        assert_eq!(stopped.active_midi, [0; 4]);
        worker.shutdown().unwrap();
    }

    #[test]
    fn screen_actions_acknowledge_new_input_context() {
        let mut worker = Worker::start_with(|| Ok(MidiEngine::dry_run())).unwrap();
        assert_eq!(worker.input_page(), InputPage::Performance);
        worker.action(InputAction::ToggleScreen).unwrap();
        assert_eq!(worker.input_page(), InputPage::Editor);
        worker.action(InputAction::ToggleSettings).unwrap();
        assert_eq!(worker.input_page(), InputPage::Settings);
        worker.action(InputAction::MenuTap).unwrap();
        assert_eq!(worker.input_page(), InputPage::Performance);
        assert!(worker.action(InputAction::Quit).unwrap());
        worker.shutdown().unwrap();
    }

    #[test]
    fn internal_live_tempo_cannot_navigate_to_stopped_settings() {
        let mut worker = Worker::start_with(|| Ok(MidiEngine::dry_run())).unwrap();
        worker.action(InputAction::ToggleTransport).unwrap();
        worker.action(InputAction::ToggleSettings).unwrap();
        worker.action(InputAction::NavigateRight).unwrap();
        worker.action(InputAction::NavigateUp).unwrap();
        let live = worker.snapshot().unwrap();
        assert_eq!(live.setting_focus, Some(SettingField::Tempo));
        assert_eq!(live.sync_status.tempo_bpm, 121.0);
        assert!(live.sync_status.running);
        worker.action(InputAction::ToggleTransport).unwrap();
        assert_eq!(worker.input_page(), InputPage::Performance);
        worker.action(InputAction::ToggleSettings).unwrap();
        assert_eq!(
            worker.snapshot().unwrap().setting_focus,
            Some(SettingField::Tonic)
        );
        worker.shutdown().unwrap();
    }

    #[test]
    fn clock_out_and_panic_are_accessible_from_stopped_settings() {
        let mut worker = Worker::start_with(|| Ok(MidiEngine::dry_run())).unwrap();
        assert_eq!(worker.snapshot().unwrap().midi_clock, "CLOCK OUT OFF");
        worker.action(InputAction::ToggleSettings).unwrap();
        for _ in 0..3 {
            worker.action(InputAction::NavigateLeft).unwrap();
        }
        worker.action(InputAction::NavigateUp).unwrap();
        assert_eq!(worker.snapshot().unwrap().midi_clock, "CLOCK OUT READY");
        worker.action(InputAction::NavigateRight).unwrap();
        assert_eq!(
            worker.snapshot().unwrap().setting_focus,
            Some(SettingField::MidiPanic)
        );
        worker.action(InputAction::Primary).unwrap();
        assert_eq!(worker.input_page(), InputPage::Performance);
        worker.action(InputAction::ToggleTransport).unwrap();
        thread::sleep(Duration::from_millis(50));
        assert_eq!(worker.snapshot().unwrap().midi_clock, "CLOCK OUT SENDING");
        worker.action(InputAction::ToggleTransport).unwrap();
        let stopped = worker.snapshot().unwrap();
        assert_eq!(stopped.midi_clock, "CLOCK OUT READY");
        assert_eq!(stopped.active_midi, [0; 4]);
        worker.shutdown().unwrap();
    }

    #[test]
    fn stopped_settings_assign_channels_and_midi_input_blocks_clock_out() {
        let mut worker = Worker::start_with(|| Ok(MidiEngine::dry_run())).unwrap();
        worker.action(InputAction::ToggleSettings).unwrap();
        for _ in 0..11 {
            worker.action(InputAction::NavigateLeft).unwrap();
        }
        let settings = worker.snapshot().unwrap();
        assert_eq!(settings.setting_focus, Some(SettingField::MidiChannel(3)));
        worker.action(InputAction::NavigateUp).unwrap();
        assert_eq!(worker.snapshot().unwrap().midi_channels, [1, 2, 3, 5]);
        worker.action(InputAction::NavigateRight).unwrap();
        worker.action(InputAction::NavigateUp).unwrap();
        let transpose = worker.snapshot().unwrap();
        assert_eq!(
            transpose.setting_focus,
            Some(SettingField::MidiTranspose(0))
        );
        assert_eq!(transpose.midi_transpose, [1, 0, 0, 0]);
        worker.action(InputAction::MenuTap).unwrap();
        worker.action(InputAction::ToggleSettings).unwrap();
        for _ in 0..4 {
            worker.action(InputAction::NavigateRight).unwrap();
        }
        worker.action(InputAction::NavigateUp).unwrap();
        let settings = worker.snapshot().unwrap();
        assert_eq!(settings.sync_status.mode, SyncMode::MidiIn);
        assert_eq!(settings.midi_clock, "CLOCK OUT BLOCKED");
        worker.action(InputAction::ToggleTransport).unwrap();
        assert!(!worker.snapshot().unwrap().sync_status.running);
        worker.shutdown().unwrap();
    }
}
