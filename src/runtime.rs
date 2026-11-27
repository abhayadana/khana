//! Musical runtime: SDL never owns or locks the clock, performer, or MIDI sink.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::editor::EditorModal;
use crate::input::{InputAction, InputPage};
use crate::midi::MidiEngine;
use crate::music::chord::Chord;
use crate::music::recommendation::{HarmonicDirection, RecommendationClass, RecommendationSet};
use crate::music::settings::{MusicalSettings, SettingField};
use crate::music::voicing::VoicingStyle;
use crate::performer::PerformerSnapshot;
use crate::phrase::{EditGrid, Phrase, RecorderStatus};
use crate::sync::{SyncMode, SyncStatus, SyncTransport};
use crate::ui::{PerformanceView, PhraseEditorView, Ui, WorkspaceView};
use crate::voice::{Voice, VoiceRender, VoiceScope};
use crate::workspace::Tab;
use crate::{App, Screen, dispatch_action};

const POLL_PERIOD: Duration = Duration::from_millis(1);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);

/// Owned render data; drawing and present never hold a musical-state lock.
pub(crate) struct Frame {
    screen: Screen,
    workspace: Option<WorkspaceView>,
    take_status: Option<&'static str>,
    take_count: usize,
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
    phrase_count: usize,
    midi_destination: String,
    midi_clock: &'static str,
    midi_channels: [u8; 4],
    midi_transpose: [i8; 4],
    midi_input: Option<String>,
    phrase: Option<Phrase>,
    selected_event: usize,
    pixels_per_beat: f32,
    previous_phrase: Option<u32>,
    following_phrase: Option<u32>,
    playing_phrase: Option<u32>,
    queued_phrase: Option<u32>,
    playhead: Option<f64>,
    next_sections: Vec<(u32, u8, crate::phrase::Transition)>,
    section_passes: Option<u8>,
    editor_fit: bool,
    editor_modal: Option<EditorModal>,
    editor_clipboard: bool,
    editor_message: &'static str,
    edit_pending: bool,
    edit_grid: EditGrid,
    timing: String,
    beat_flash: Option<bool>,
    chord_gate: &'static str,
    bass_arrival: &'static str,
    groove: &'static str,
    texture_role: &'static str,
    panic_confirm: bool,
    macro_targets: [bool; 4],
    has_macro_group: bool,
    live_grid: &'static str,
    voicing: VoicingStyle,
    pending_voicing: Option<VoicingStyle>,
    can_undo_phrase: bool,
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
        let take_status = if let Some(playback) = &app.playing_take {
            Some(if playback.started {
                "PLAYING TAKE"
            } else {
                "TAKE QUEUED"
            })
        } else if app.recording_take.is_some() {
            Some("RECORDING")
        } else if app.take_armed_at.is_some() {
            Some("ARMED")
        } else {
            None
        };
        Self {
            screen: app.screen,
            workspace: app.workspace_ui.as_ref().map(|ui| {
                let first = ui.row.saturating_sub(7);
                let items = match ui.tab {
                    Tab::Takes => app
                        .takes
                        .iter()
                        .skip(first)
                        .take(8)
                        .map(|take| format!("{}   {:.1} BEATS", take.name, take.duration))
                        .collect(),
                    Tab::Projects => app
                        .collection
                        .projects
                        .iter()
                        .skip(first)
                        .take(8)
                        .map(|entry| entry.name.clone())
                        .collect(),
                    Tab::Presets => app
                        .collection
                        .presets
                        .iter()
                        .skip(first)
                        .take(8)
                        .map(|entry| entry.name.clone())
                        .collect(),
                };
                WorkspaceView {
                    tab: ui.tab,
                    items,
                    row: ui.row - first,
                    name: ui.name.as_ref().map(|edit| (edit.bytes, edit.cursor)),
                    message: if ui.message.is_empty() {
                        take_status.unwrap_or("").into()
                    } else {
                        ui.message.clone()
                    },
                    take_mode: app.take_mode,
                    project: app.project_name.clone(),
                    retrospective_ready: app.retrospective_take.is_some(),
                }
            }),
            take_status: take_status.or_else(|| {
                app.take_mode
                    .then_some(if app.takes.len() >= crate::workspace::MAX_LIBRARY_ITEMS {
                        "LIST FULL"
                    } else {
                        "READY"
                    })
            }),
            take_count: app.takes.len(),
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
            selected_event: app.editor_event_index,
            pixels_per_beat: app.editor_zoom,
            previous_phrase: app
                .editor_phrase_index
                .checked_sub(1)
                .and_then(|index| app.phrases.get(index))
                .map(|phrase| phrase.id),
            following_phrase: app
                .phrases
                .get(app.editor_phrase_index + 1)
                .map(|phrase| phrase.id),
            playing_phrase: app.phrase_player.pass().map(|pass| pass.phrase_id),
            queued_phrase: app
                .phrase_player
                .queued_phrase()
                .and_then(|index| app.phrases.get(index))
                .map(|phrase| phrase.id),
            playhead: app
                .phrase_player
                .pass()
                .filter(|pass| pass.phrase_index == app.editor_phrase_index)
                .map(|pass| (now_beat - pass.start).clamp(0.0, pass.end - pass.start)),
            next_sections: if app.screen == Screen::PhraseEditor {
                app.phrase_player
                    .chain_sections()
                    .filter_map(|section| {
                        let id = app.phrases.get(section.phrase_index)?.id;
                        Some((id, section.passes, section.transition))
                    })
                    .collect()
            } else {
                Vec::new()
            },
            section_passes: app.phrase_player.section_passes_left(),
            editor_fit: app.editor_fit,
            editor_modal: app.editor_modal.clone(),
            editor_clipboard: app.editor_clipboard.is_some(),
            editor_message: app.editor_message,
            edit_pending: app.phrase_player.edit_pending(app.editor_phrase_index),
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
                format!("BEAT {pulse}{pending}")
            },
            beat_flash: (sync_status.running && bar_phase.rem_euclid(beat) < beat * 0.22)
                .then_some(beat_index == 0),
            voicing: app.voice_engine.style,
            pending_voicing: app.pending_voicing.map(|(style, _)| style),
            can_undo_phrase: app.can_undo_phrase(),
            chord_gate: app.performer_engine.hold_label(),
            bass_arrival: app.performer_engine.bass_label(),
            groove: app.performer_engine.groove.label(),
            texture_role: app.performer_engine.texture_style.label(),
            panic_confirm: app.panic_confirm,
            macro_targets: app.macro_targets(),
            has_macro_group: app.macro_group.iter().any(|&member| member),
            live_grid: app.live_grid.label(),
        }
    }

    pub(crate) fn diagnostics(&self) -> (u64, [u32; 4]) {
        (self.worker_ticks, self.active_midi)
    }

    pub(crate) fn draw(
        &self,
        ui: &Ui,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        select_held: bool,
    ) -> Result<(), String> {
        if let Some(workspace) = &self.workspace {
            return ui.draw_workspace(canvas, workspace);
        }
        match if self.setting_focus.is_some() {
            Screen::Performance
        } else {
            self.screen
        } {
            Screen::Performance => ui.draw_performance(
                canvas,
                PerformanceView {
                    timing: &self.timing,
                    beat_flash: self.beat_flash,
                    settings: self.settings,
                    setting_focus: self.setting_focus,
                    chord_gate: self.chord_gate,
                    bass_arrival: self.bass_arrival,
                    groove: self.groove,
                    texture_role: self.texture_role,
                    panic_confirm: self.panic_confirm,
                    macro_targets: self.macro_targets,
                    has_macro_group: self.has_macro_group,
                    live_grid: self.live_grid,
                    select_held,
                    voicing: self.voicing,
                    pending_voicing: self.pending_voicing,
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
                    take_status: self.take_status,
                    take_count: self.take_count,
                    next_recording_armed: self.next_recording_armed,
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
                    previous_phrase: self.previous_phrase,
                    following_phrase: self.following_phrase,
                    selected_event: self.selected_event,
                    pixels_per_beat: self.pixels_per_beat,
                    playing_phrase: self.playing_phrase,
                    queued_phrase: self.queued_phrase,
                    playhead: self.playhead,
                    next_sections: &self.next_sections,
                    section_passes: self.section_passes,
                    fit: self.editor_fit,
                    modal: self.editor_modal.as_ref(),
                    has_clipboard: self.editor_clipboard,
                    message: self.editor_message,
                    edit_pending: self.edit_pending,
                    edit_grid: self.edit_grid,
                    can_undo: self.can_undo_phrase,
                    select_held,
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
    let mut counts = (app.phrases.len(), app.takes.len());
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
        let current_counts = (app.phrases.len(), app.takes.len());
        if live_commit || counts != current_counts {
            counts = current_counts;
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
        app.pending_voicing = None;
        app.recorder.reset_timeline();
        app.take_armed_at = None;
        app.recording_take = None;
        app.retrospective_recording = None;
        app.playing_take = None;
        if *was_running {
            app.phrase_player.stop();
            app.performer_engine.transport_stopped();
            app.stop_notes(midi)?;
            *was_running = false;
        }
    }

    app.finish_retrospective_pass(now_beat);
    if running_now != *was_running {
        if running_now {
            app.setting_focus = None;
            app.panic_confirm = false;
            app.chord_started_beat = now_beat;
            app.performer_engine.transport_started(now_beat);
        } else {
            app.finish_take(now_beat);
            app.retrospective_recording = None;
            app.take_armed_at = None;
            app.playing_take = None;
            app.pending_chord = None;
            app.pending_voicing = None;
            app.phrase_player.stop();
            app.performer_engine.transport_stopped();
            app.stop_notes(midi)?;
        }
        *was_running = running_now;
    }

    let mut live_commit = false;
    if running_now {
        if app
            .playing_take
            .as_ref()
            .is_some_and(|playback| now_beat >= playback.start)
        {
            return tick_take_playback(app, midi, now_beat);
        }
        if let Some(stop) = app
            .recording_take
            .as_ref()
            .and_then(|take| {
                if take.full() {
                    Some(now_beat)
                } else {
                    take.stop
                }
            })
            .filter(|&stop| now_beat >= stop)
        {
            live_commit |= app.finish_take(stop);
        }
        crate::workspace::begin_take(app, now_beat);
        let voicing_due = app.pending_voicing.filter(|&(_, at)| now_beat >= at);
        if let Some((style, _)) = voicing_due {
            app.pending_voicing = None;
            app.voice_engine.style = style;
        }
        let mut harmony_committed = false;
        if let Some((chord, at)) = app.pending_chord
            && now_beat >= at
        {
            advance_recorder(app, at);
            live_commit = true;
            app.pending_chord = None;
            app.phrase_player.stop();
            app.retrospective_recording = None;
            app.set_harmony(chord, at, true, true);
            harmony_committed = true;
        }
        advance_recorder(app, now_beat);
        if transport.mode() == SyncMode::MidiIn {
            // A staged chain starts on the incoming clock. Use the active meter
            // even when the project was prepared before the external Start.
            app.phrase_player.arm_chain(now_beat, bar_beats);
        }
        let playback = app.phrase_player.update(&app.phrases, now_beat);
        if let Some(at) = playback.chord_beat
            && !app.performer_engine.is_enabled()
        {
            app.performer_engine.transport_started(at);
        }
        if let Some((chord, at)) = playback.chord.zip(playback.chord_beat)
            && chord != app.current_chord
        {
            app.set_harmony(chord, at, true, false);
            harmony_committed = true;
        }
        if voicing_due.is_some() {
            if !harmony_committed {
                app.revoice_current();
            }
            app.performer_engine.request_live_revoice();
            live_commit = true;
        }
        app.performer_engine.set_phrase_position(playback.position);
        let phrase_next = playback.next_chord.zip(playback.next_chord_beat);
        let next = match (app.pending_chord, phrase_next) {
            (Some(a), Some(b)) => Some(if a.1 <= b.1 { a } else { b }),
            (a, b) => a.or(b),
        };
        app.performer_engine
            .set_next_chord_target(next.map(|(chord, _)| chord), next.map(|(_, beat)| beat));
        if playback.position.is_none() && playback.ended {
            app.performer_engine.set_enabled(false);
            app.stop_notes(midi)?;
        }
        begin_retrospective_pass(app, now_beat);
        app.update_performers(now_beat, midi)?;
    }
    Ok(live_commit)
}

fn begin_retrospective_pass(app: &mut App, now: f64) {
    let Some(pass) = app.phrase_player.pass() else {
        app.retrospective_recording = None;
        return;
    };
    if app
        .retrospective_recording
        .as_ref()
        .is_some_and(|take| take.start == pass.start)
    {
        return;
    }
    app.retrospective_recording = None;
    // If the external clock jumped well into a pass, its start was not played.
    let launch_window = (pass.end - pass.start).min(1.0 / 24.0);
    if now - pass.start > launch_window + 1.0e-6 {
        return;
    }
    let mut take = crate::workspace::TakeRecording::new(
        app,
        pass.start,
        Some(pass.phrase_id),
        format!("P{:02} PASS", pass.phrase_id),
    );
    take.stop = Some(pass.end);
    take.seed_notes(&app.active_notes);
    app.retrospective_recording = Some(take);
}

fn tick_take_playback(app: &mut App, midi: &mut MidiEngine, now: f64) -> Result<bool, String> {
    let Some(mut playback) = app.playing_take.take() else {
        return Ok(false);
    };
    if !playback.started {
        app.retrospective_recording = None;
        app.stop_notes(midi)?;
        app.performer_engine.set_enabled(false);
        app.phrase_player.stop();
        app.pending_chord = None;
        app.pending_voicing = None;
        playback.started = true;
    }
    let elapsed = now - playback.start;
    while let Some(entry) = playback
        .take
        .events
        .get(playback.event)
        .filter(|event| event.beat <= elapsed)
    {
        app.active_notes.apply(entry.event);
        midi.apply_event(entry.event)?;
        playback.event += 1;
    }
    let mut chord = None;
    while let Some(event) = playback
        .take
        .harmony
        .get(playback.harmony)
        .filter(|event| event.start_beat <= elapsed)
    {
        chord = Some(*event);
        playback.harmony += 1;
    }
    if let Some(event) = chord.filter(|event| event.chord != app.current_chord) {
        app.set_harmony(event.chord, playback.start + event.start_beat, false, false);
    }
    if elapsed >= playback.take.duration {
        app.stop_notes(midi)?;
        Ok(true)
    } else {
        app.playing_take = Some(playback);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle_performance_action;

    #[test]
    fn take_playback_uses_recorded_velocity_and_beat_and_releases_at_end() {
        use crate::performer::VoiceEvent;
        let mut app = App::new();
        let mut midi = MidiEngine::dry_run();
        let note = VoiceEvent::NoteOn {
            voice_index: 2,
            note: 64,
            velocity: 79,
        };
        let seed = VoiceEvent::NoteOn {
            voice_index: 3,
            note: 72,
            velocity: 92,
        };
        let mut draft = crate::workspace::TakeRecording::new(&app, 4.0, None, "TAKE 01".into());
        app.active_notes.apply(seed);
        draft.seed_notes(&app.active_notes);
        draft.event(4.6, note);
        let recorded = draft.finish(5.8).unwrap();
        assert_eq!(recorded.events[0].event, seed);
        assert_eq!(recorded.events[1].event, note);
        assert!(recorded.valid());
        app.playing_take = Some(crate::workspace::TakePlayback::new(recorded, 8.0));
        tick_take_playback(&mut app, &mut midi, 8.0).unwrap();
        assert_eq!(midi.active_counts(), [0, 0, 0, 1]);
        tick_take_playback(&mut app, &mut midi, 8.7).unwrap();
        assert_eq!(midi.active_counts(), [0, 0, 1, 1]);
        assert!(tick_take_playback(&mut app, &mut midi, 10.0).unwrap());
        assert_eq!(midi.active_counts(), [0; 4]);
        assert!(app.playing_take.is_none());
    }

    #[test]
    fn workspace_actions_save_and_restore_project_while_keeping_the_rig() {
        let mut app = App::new();
        let mut midi = MidiEngine::dry_run();
        let mut clock = SyncTransport::new(120.0);
        for action in [
            InputAction::ToggleWorkspace,
            InputAction::NavigateRight,
            InputAction::Tertiary,
            InputAction::Primary,
        ] {
            dispatch_action(action, &mut app, &mut clock, &mut midi).unwrap();
        }
        assert_eq!(app.project_name.as_deref(), Some("PROJECT 01"));
        let saved = app.voices[3].parameters.color;
        app.voices[3].parameters.color = 0.1;
        midi.adjust_channel(3, 3).unwrap();
        for _ in 0..2 {
            dispatch_action(InputAction::Primary, &mut app, &mut clock, &mut midi).unwrap();
        }
        assert_eq!(app.voices[3].parameters.color, saved);
        assert_eq!(midi.channels(), [1, 2, 3, 7]);
    }

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
    fn queued_voicing_and_chord_land_in_one_worker_update() {
        let mut app = App::new();
        let mut midi = MidiEngine::dry_run();
        let mut transport = SyncTransport::new(app.settings.tempo_bpm);
        transport.start(4.0);
        let mut running = false;
        let chord = app
            .settings
            .tonal
            .diatonic_chord(4, crate::music::chord::ChordExtension::Seventh);
        for _ in 0..2 {
            handle_performance_action(InputAction::RightClick, &mut app, &mut transport, &mut midi)
                .unwrap();
        }
        assert_eq!(app.pending_voicing, Some((VoicingStyle::Open, 1.0)));
        assert_eq!(app.voice_engine.style, VoicingStyle::Smooth);
        app.pending_chord = Some((chord, 0.0));
        app.pending_voicing = Some((VoicingStyle::Open, 0.0));

        assert!(tick(&mut app, &mut transport, &mut midi, &mut running).unwrap());
        assert_eq!(app.current_chord, chord);
        assert_eq!(app.voice_engine.style, VoicingStyle::Open);
        assert!(app.pending_chord.is_none());
        assert!(app.pending_voicing.is_none());
    }

    #[test]
    fn phrase_undo_restores_edits_and_selection_without_affecting_other_phrases() {
        use crate::music::chord::ChordExtension;
        use crate::phrase::ChordEvent;

        let mut app = App::new();
        let mut midi = MidiEngine::dry_run();
        let mut transport = SyncTransport::new(app.settings.tempo_bpm);
        let original = vec![
            ChordEvent {
                chord: app.current_chord,
                start_beat: 0.0,
                duration_beats: 1.3,
            },
            ChordEvent {
                chord: app
                    .settings
                    .tonal
                    .diatonic_chord(1, ChordExtension::Seventh),
                start_beat: 1.3,
                duration_beats: 1.0,
            },
        ];
        for id in [1, 2] {
            app.phrases.push(Phrase {
                id,
                looping: true,
                events: original.clone(),
            });
        }
        app.screen = Screen::PhraseEditor;
        app.editor_event_index = 1;
        dispatch_action(InputAction::EditExtend, &mut app, &mut transport, &mut midi).unwrap();
        let after_size = app.phrases[0].events.clone();
        assert_eq!(after_size[1].duration_beats, 2.0);
        dispatch_action(
            InputAction::EditDuplicate,
            &mut app,
            &mut transport,
            &mut midi,
        )
        .unwrap();
        let after_duplicate = app.phrases[0].events.clone();
        dispatch_action(InputAction::EditDelete, &mut app, &mut transport, &mut midi).unwrap();
        assert_eq!(app.phrases[0].events, after_size);
        dispatch_action(InputAction::PhraseNext, &mut app, &mut transport, &mut midi).unwrap();
        dispatch_action(InputAction::EditUndo, &mut app, &mut transport, &mut midi).unwrap();
        assert_eq!(app.phrases[1].events, original);
        dispatch_action(
            InputAction::PhrasePrevious,
            &mut app,
            &mut transport,
            &mut midi,
        )
        .unwrap();
        dispatch_action(InputAction::EditUndo, &mut app, &mut transport, &mut midi).unwrap();
        assert_eq!(app.phrases[0].events, after_duplicate);
        assert_eq!(app.editor_event_index, 2);
        dispatch_action(InputAction::EditUndo, &mut app, &mut transport, &mut midi).unwrap();
        assert_eq!(app.phrases[0].events, after_size);
        assert_eq!(app.editor_event_index, 1);
        assert!(app.edit_phrase(|phrase, index| phrase.replace_chord(index, original[0].chord)));
        dispatch_action(InputAction::EditUndo, &mut app, &mut transport, &mut midi).unwrap();
        assert_eq!(app.phrases[0].events, after_size);
        dispatch_action(InputAction::EditUndo, &mut app, &mut transport, &mut midi).unwrap();
        assert_eq!(app.phrases[0].events, original);
        assert!(!app.can_undo_phrase());
        assert!(app.phrases[0].looping);
    }

    #[test]
    fn editor_picker_and_clipboard_preserve_off_grid_timing() {
        use crate::editor::{EditItem, EditorModal};
        use crate::phrase::ChordEvent;

        let mut app = App::new();
        let mut midi = MidiEngine::dry_run();
        let mut transport = SyncTransport::new(app.settings.tempo_bpm);
        let mut act = |app: &mut App, action| {
            dispatch_action(action, app, &mut transport, &mut midi).unwrap()
        };
        let original = ChordEvent {
            chord: app.current_chord,
            start_beat: 0.0,
            duration_beats: 1.3,
        };
        for id in [1, 2] {
            app.phrases.push(Phrase {
                id,
                looping: true,
                events: vec![original],
            });
        }
        app.screen = Screen::PhraseEditor;
        act(&mut app, InputAction::Secondary);
        assert_eq!(app.input_page(), InputPage::EditorModal);
        act(&mut app, InputAction::Secondary);
        assert!(!app.can_undo_phrase());

        act(&mut app, InputAction::Secondary);
        act(&mut app, InputAction::Tertiary);
        act(&mut app, InputAction::NavigateUp);
        act(&mut app, InputAction::Primary);
        assert_eq!(
            app.phrases[0].events[0].chord.root,
            original.chord.root.transpose(1)
        );
        assert_eq!(app.phrases[0].events[0].duration_beats, 1.3);

        let edit = |app: &mut App, item: EditItem| {
            app.editor_modal = Some(EditorModal::Edit(
                EditItem::ALL
                    .iter()
                    .position(|candidate| *candidate == item)
                    .unwrap(),
            ));
        };
        edit(&mut app, EditItem::InsertBefore);
        act(&mut app, InputAction::Primary);
        act(&mut app, InputAction::Primary);
        assert_eq!(app.phrases[0].events[0].duration_beats, 1.0);
        assert_eq!(app.phrases[0].events[1].start_beat, 1.0);
        act(&mut app, InputAction::NavigateRight);
        let cut = app.phrases[0].events[1];
        edit(&mut app, EditItem::Cut);
        act(&mut app, InputAction::Primary);
        assert_eq!(app.phrases[0].events.len(), 1);
        assert_eq!(app.editor_clipboard, Some(cut));
        act(&mut app, InputAction::PhraseNext);
        edit(&mut app, EditItem::Paste);
        act(&mut app, InputAction::Primary);
        assert_eq!(
            app.phrases[1].events[1],
            ChordEvent {
                start_beat: 1.3,
                ..cut
            }
        );
        act(&mut app, InputAction::EditUndo);
        assert_eq!(app.phrases[1].events, vec![original]);
        act(&mut app, InputAction::EditDelete);
        assert_eq!(app.phrases[1].events, vec![original]);
    }

    #[test]
    fn recovery_promotes_only_a_complete_phrase_pass_to_takes() {
        use crate::performer::VoiceEvent;
        use crate::phrase::ChordEvent;

        let mut app = App::new();
        app.phrases.push(Phrase {
            id: 3,
            looping: true,
            events: vec![ChordEvent {
                chord: app.current_chord,
                start_beat: 0.0,
                duration_beats: 2.5,
            }],
        });
        app.phrase_player.request(0, 4.0, 4.0);
        app.phrase_player.update(&app.phrases, 4.0);
        let note = VoiceEvent::NoteOn {
            voice_index: 0,
            note: 47,
            velocity: 92,
        };
        app.active_notes.apply(note);
        begin_retrospective_pass(&mut app, 4.0);
        app.finish_retrospective_pass(6.0);
        assert!(app.retrospective_take.is_none());
        app.finish_retrospective_pass(6.5);
        assert!(app.takes.is_empty());
        let take = app.retrospective_take.as_ref().unwrap();
        assert!(take.valid());
        assert_eq!(take.duration, 2.5);
        assert_eq!(take.source_phrase, Some(3));
        assert_eq!(take.events[0].beat, 0.0);
        assert_eq!(take.events[0].event, note);
        assert_eq!(take.events.last().unwrap().beat, 2.5);
        assert!(matches!(
            take.events.last().unwrap().event,
            VoiceEvent::NoteOff { .. }
        ));

        let mut midi = MidiEngine::dry_run();
        let mut transport = SyncTransport::new(app.settings.tempo_bpm);
        app.workspace_ui = Some(crate::workspace::WorkspaceUi::default());
        dispatch_action(InputAction::RightClick, &mut app, &mut transport, &mut midi).unwrap();
        assert_eq!(app.takes.len(), 1);
        assert!(app.retrospective_take.is_none());
        dispatch_action(InputAction::RightClick, &mut app, &mut transport, &mut midi).unwrap();
        assert_eq!(app.takes.len(), 1);
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
        assert_eq!(worker.input_page(), InputPage::Editor);
        assert!(worker.action(InputAction::Quit).unwrap());
        worker.shutdown().unwrap();
    }

    #[test]
    fn internal_live_tempo_cannot_navigate_to_stopped_settings() {
        let mut worker = Worker::start_with(|| Ok(MidiEngine::dry_run())).unwrap();
        worker.action(InputAction::ToggleTransport).unwrap();
        worker.action(InputAction::ToggleSettings).unwrap();
        worker.action(InputAction::NavigateRight).unwrap();
        worker.action(InputAction::ScopeNext).unwrap();
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
        worker.action(InputAction::Tertiary).unwrap();
        for _ in 0..5 {
            worker.action(InputAction::NavigateDown).unwrap();
        }
        worker.action(InputAction::ScopeNext).unwrap();
        assert_eq!(worker.snapshot().unwrap().midi_clock, "CLOCK OUT READY");
        for _ in 0..3 {
            worker.action(InputAction::NavigateDown).unwrap();
        }
        assert_eq!(
            worker.snapshot().unwrap().setting_focus,
            Some(SettingField::MidiPanic)
        );
        worker.action(InputAction::Primary).unwrap();
        assert!(worker.snapshot().unwrap().panic_confirm);
        worker.action(InputAction::Primary).unwrap();
        assert!(!worker.snapshot().unwrap().panic_confirm);
        worker.action(InputAction::Secondary).unwrap();
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
    fn macro_group_stays_pinned_and_buttons_use_the_highlight() {
        use crate::voice::MacroKind;

        let mut worker = Worker::start_with(|| Ok(MidiEngine::dry_run())).unwrap();
        let initial = worker.snapshot().unwrap().voices;
        for action in [
            InputAction::ScopeNext,
            InputAction::ToggleMacroMember,
            InputAction::ScopeNext,
            InputAction::ScopeNext,
            InputAction::ToggleMacroMember,
            InputAction::ScopeNext,
            InputAction::AdjustMacro {
                kind: MacroKind::Dynamics,
                delta: 0.1,
            },
            InputAction::ToggleMute,
        ] {
            worker.action(action).unwrap();
        }
        let frame = worker.snapshot().unwrap();
        assert_eq!(frame.macro_targets, [true, false, true, false]);
        for (index, previous) in initial.into_iter().enumerate() {
            let delta = if frame.macro_targets[index] { 0.1 } else { 0.0 };
            assert!(
                (frame.voices[index].parameters.dynamics - previous.parameters.dynamics - delta)
                    .abs()
                    < 0.0001
            );
            assert_eq!(frame.performer_snapshots[index].muted, index == 3);
        }
        worker.action(InputAction::ScopeNext).unwrap();
        worker.action(InputAction::ToggleMacroMember).unwrap();
        assert_eq!(worker.snapshot().unwrap().macro_targets, [true; 4]);
        worker.shutdown().unwrap();
    }

    #[test]
    fn trigger_direction_stops_at_edges_and_can_return_to_neutral() {
        let mut worker = Worker::start_with(|| Ok(MidiEngine::dry_run())).unwrap();
        for delta in [-1, -1, 1, 1, 1, -1] {
            worker.action(InputAction::DirectionStep(delta)).unwrap();
        }
        assert_eq!(
            worker.snapshot().unwrap().direction,
            HarmonicDirection::Neutral
        );
        worker.shutdown().unwrap();
    }

    #[test]
    fn stopped_settings_assign_channels_and_midi_input_blocks_clock_out() {
        let mut worker = Worker::start_with(|| Ok(MidiEngine::dry_run())).unwrap();
        worker.action(InputAction::ToggleSettings).unwrap();
        worker.action(InputAction::Tertiary).unwrap();
        for _ in 0..3 {
            worker.action(InputAction::NavigateDown).unwrap();
        }
        assert_eq!(
            worker.snapshot().unwrap().setting_focus,
            Some(SettingField::MidiChannel(3))
        );
        for _ in 0..3 {
            worker.action(InputAction::ScopeNext).unwrap();
        }
        assert_eq!(worker.snapshot().unwrap().midi_channels, [1, 2, 3, 7]);
        worker.action(InputAction::NavigateRight).unwrap();
        for _ in 0..12 {
            worker.action(InputAction::ScopeNext).unwrap();
        }
        let transpose = worker.snapshot().unwrap();
        assert_eq!(
            transpose.setting_focus,
            Some(SettingField::MidiTranspose(3))
        );
        assert_eq!(transpose.midi_transpose, [0, 0, 0, 12]);
        worker.action(InputAction::NavigateDown).unwrap();
        worker.action(InputAction::ScopeNext).unwrap();
        let settings = worker.snapshot().unwrap();
        assert_eq!(settings.sync_status.mode, SyncMode::MidiIn);
        assert_eq!(settings.midi_clock, "CLOCK OUT BLOCKED");
        worker.action(InputAction::ToggleTransport).unwrap();
        assert!(!worker.snapshot().unwrap().sync_status.running);
        worker.shutdown().unwrap();
    }
}
