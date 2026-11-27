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
    phrase: Option<Phrase>,
    phrase_index: usize,
    selected_event: usize,
    pixels_per_beat: f32,
    playing_phrase: Option<usize>,
    queued_phrase: Option<usize>,
    edit_grid: EditGrid,
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
            midi_clock: midi.clock_label(),
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
                    settings: self.settings,
                    setting_focus: self.setting_focus,
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
                },
            ),
            Screen::PhraseEditor => ui.draw_phrase_editor(
                canvas,
                PhraseEditorView {
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
        Self::start_with(move || MidiEngine::configured(brick))
    }

    fn start_with<F>(make_midi: F) -> Result<Self, String>
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
                if ready_tx.send(()).is_err() {
                    return Ok(());
                }
                let result = run(&rx, &mut app, &mut transport, &mut midi);
                let stop_clock = midi.stop_clock();
                let cleanup = midi.stop_all();
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
) -> Result<(), String> {
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
        tick(app, transport, midi, &mut was_running)?;
        midi.service();
        max_work = max_work.max(poll.elapsed());
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

fn tick(
    app: &mut App,
    transport: &mut SyncTransport,
    midi: &mut MidiEngine,
    was_running: &mut bool,
) -> Result<(), String> {
    let bar_beats = app.settings.meter.beats_per_bar();
    let now_beat = transport.beat(bar_beats);
    let running_now = transport.is_running();
    let link_quantum = (transport.mode() == SyncMode::Link).then_some(bar_beats);
    midi.update_clock(now_beat, running_now, link_quantum)?;

    if running_now != *was_running {
        if running_now {
            app.setting_focus = None;
            app.chord_started_beat = now_beat;
            app.performer_engine.transport_started(now_beat);
        } else {
            app.phrase_player.stop();
            app.performer_engine.transport_stopped();
            midi.stop_all()?;
        }
        *was_running = running_now;
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
            app.set_harmony(chord, now_beat, true, false);
        }
        app.performer_engine
            .set_next_chord_target(playback.next_chord, playback.next_chord_beat);
        if playback.ended && app.phrase_player.queued_phrase().is_none() {
            app.performer_engine.set_enabled(false);
            midi.stop_all()?;
        }
        app.update_performers(now_beat, midi)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
