//! Persist the last playable setup independently of the installed Pak.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};

use crate::App;
use crate::midi::MidiEngine;
use crate::music::chord::Chord;
use crate::music::settings::{MusicalSettings, TimeSignature};
use crate::music::voicing::VoicingStyle;
use crate::performer::{ChordHold, Groove, TextureStyle};
use crate::phrase::{EditGrid, LiveGrid, Phrase};
use crate::sync::{SyncMode, SyncTransport};
use crate::voice::VoiceParameters;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Session {
    version: u32,
    settings: MusicalSettings,
    current_chord: Chord,
    sync: SyncMode,
    shared_transport: bool,
    channels: [u8; 4],
    transpose: [i8; 4],
    clock_out: bool,
    voices: [VoiceParameters; 4],
    chord_hold: ChordHold,
    #[serde(default)]
    groove: Groove,
    #[serde(default)]
    texture_style: TextureStyle,
    voice_led_bass: bool,
    voicing: VoicingStyle,
    live_grid: LiveGrid,
    edit_grid: EditGrid,
    phrases: Vec<Phrase>,
}

impl Session {
    fn capture(app: &App, transport: &mut SyncTransport, midi: &MidiEngine) -> Self {
        let mut settings = app.settings;
        settings.tempo_bpm = transport.internal_tempo();
        Self {
            version: 1,
            settings,
            current_chord: app.current_chord,
            sync: transport.mode(),
            shared_transport: transport.status().link_start_stop_sync,
            channels: midi.channels(),
            transpose: midi.transpositions(),
            clock_out: midi.clock_enabled(),
            voices: app.voices.map(|voice| voice.parameters),
            chord_hold: app.performer_engine.hold_length,
            groove: app.performer_engine.groove,
            texture_style: app.performer_engine.texture_style,
            voice_led_bass: app.performer_engine.voice_led_bass,
            voicing: app.voice_engine.style,
            live_grid: app.live_grid,
            edit_grid: app.editor_grid,
            phrases: app.phrases.clone(),
        }
    }

    fn valid(&self) -> bool {
        self.version == 1
            && self.settings.tonal.tonic.value() < 12
            && self.current_chord.root.value() < 12
            && TimeSignature::PRESETS.contains(&self.settings.meter)
            && (30.0..=300.0).contains(&self.settings.tempo_bpm)
            && self.channels.iter().all(|x| (1..=16).contains(x))
            && self.transpose.iter().all(|x| (-24..=24).contains(x))
            && self.voices.iter().all(|voice| {
                [voice.density, voice.dynamics, voice.spread, voice.color]
                    .iter()
                    .all(|value| (0.0..=1.0).contains(value))
            })
            && self.phrases.iter().all(|phrase| {
                !phrase.events.is_empty()
                    && phrase.events.iter().all(|event| {
                        event.chord.root.value() < 12
                            && event.start_beat.is_finite()
                            && event.start_beat >= 0.0
                            && event.duration_beats.is_finite()
                            && event.duration_beats > 0.0
                    })
            })
    }

    fn restore(self, app: &mut App, transport: &mut SyncTransport, midi: &mut MidiEngine) {
        app.settings = self.settings;
        transport.set_tempo(self.settings.tempo_bpm);
        transport.restore_mode(
            self.sync,
            self.settings.meter.beats_per_bar(),
            self.shared_transport,
        );
        midi.restore_routing(self.channels, self.transpose, self.clock_out);
        for (voice, params) in app.voices.iter_mut().zip(self.voices) {
            voice.parameters = params;
        }
        app.performer_engine.hold_length = self.chord_hold;
        app.performer_engine.groove = self.groove;
        app.performer_engine.texture_style = self.texture_style;
        app.performer_engine.voice_led_bass = self.voice_led_bass;
        app.voice_engine.style = self.voicing;
        app.live_grid = self.live_grid;
        app.editor_grid = self.edit_grid;
        app.phrases = self.phrases;
        app.recorder.continue_numbering(&app.phrases);
        app.progression_history.clear();
        app.set_harmony(self.current_chord, 0.0, false, false);
    }
}

pub(crate) fn path(brick: bool) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("KHANA_SESSION_PATH").filter(|path| !path.is_empty()) {
        return Some(path.into());
    }
    if brick {
        return Some(PathBuf::from(
            "/mnt/SDCARD/.userdata/tg5040/khana/session.json",
        ));
    }
    std::env::var_os("XDG_STATE_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .map(|dir| dir.join("khana/session.json"))
}

fn write(path: &Path, session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut file = std::fs::File::create(&temporary)?;
        serde_json::to_writer(&mut file, session)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

pub(crate) struct Store {
    sender: SyncSender<Session>,
    thread: JoinHandle<()>,
    last: Session,
}

impl Store {
    pub fn open(
        path: PathBuf,
        app: &mut App,
        transport: &mut SyncTransport,
        midi: &mut MidiEngine,
    ) -> Option<Self> {
        match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Session>(&bytes) {
                Ok(session) if session.valid() => session.restore(app, transport, midi),
                _ => eprintln!(
                    "Session ignored: invalid data or unsupported version at {}",
                    path.display()
                ),
            },
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                eprintln!("Session read failed: {error}")
            }
            Err(_) => {}
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("khana-session".into())
            .spawn(move || {
                while let Ok(session) = receiver.recv() {
                    if let Err(error) = write(&path, &session) {
                        eprintln!("Session save failed at {}: {error}", path.display());
                    }
                }
            })
            .map_err(|error| eprintln!("Session writer unavailable: {error}"))
            .ok()?;
        Some(Self {
            sender,
            thread,
            last: Session::capture(app, transport, midi),
        })
    }

    /// Coalesce edits before copying the state. Disk writes happen on another thread.
    pub fn offer(&mut self, app: &App, transport: &mut SyncTransport, midi: &MidiEngine) -> bool {
        let session = Session::capture(app, transport, midi);
        if session == self.last {
            return true;
        }
        match self.sender.try_send(session.clone()) {
            Ok(()) => {
                self.last = session;
                true
            }
            Err(TrySendError::Full(_)) => false,
            Err(TrySendError::Disconnected(_)) => true,
        }
    }

    pub fn finish(self, app: &App, transport: &mut SyncTransport, midi: &MidiEngine) {
        let session = Session::capture(app, transport, midi);
        if session != self.last {
            let _ = self.sender.send(session);
        }
        drop(self.sender);
        let _ = self.thread.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phrase::ChordEvent;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn clean_exit_writes_session_and_restore_leaves_transport_stopped() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "khana-session-{}-{suffix}.json",
            std::process::id()
        ));
        let mut app = App::new();
        let mut transport = SyncTransport::new(app.settings.tempo_bpm);
        let mut midi = MidiEngine::dry_run();
        let store = Store::open(path.clone(), &mut app, &mut transport, &mut midi).unwrap();
        app.live_grid = LiveGrid::Bar;
        app.editor_grid = EditGrid::Eighth;
        app.voice_engine.style = VoicingStyle::Open;
        app.performer_engine.hold_length = ChordHold::Sixteen;
        app.performer_engine.groove = Groove::LaidBack;
        app.performer_engine.texture_style = TextureStyle::SparseEcho;
        app.voices[0].parameters.color = 0.7;
        midi.adjust_channel(0, 5).unwrap();
        midi.adjust_transpose(3, 12).unwrap();
        app.phrases.push(Phrase {
            looping: true,
            id: 8,
            events: vec![ChordEvent {
                chord: app.current_chord,
                start_beat: 0.0,
                duration_beats: 3.2,
            }],
        });
        transport.set_mode(SyncMode::MidiIn, 4.0);
        transport.start(4.0);
        store.finish(&app, &mut transport, &midi);

        let mut restored = App::new();
        let mut clock = SyncTransport::new(120.0);
        let mut routing = MidiEngine::dry_run();
        let reader = Store::open(path.clone(), &mut restored, &mut clock, &mut routing).unwrap();
        reader.finish(&restored, &mut clock, &routing);
        std::fs::remove_file(path).unwrap();
        assert_eq!(restored.live_grid, LiveGrid::Bar);
        assert_eq!(restored.editor_grid, EditGrid::Eighth);
        assert_eq!(restored.voice_engine.style, VoicingStyle::Open);
        assert_eq!(restored.performer_engine.hold_length, ChordHold::Sixteen);
        assert_eq!(restored.performer_engine.groove, Groove::LaidBack);
        assert_eq!(
            restored.performer_engine.texture_style,
            TextureStyle::SparseEcho
        );
        assert_eq!(restored.voices[0].parameters.color, 0.7);
        assert_eq!(restored.phrases, app.phrases);
        assert_eq!(restored.pending_chord, None);
        assert_eq!(restored.phrase_player.queued_phrase(), None);
        assert_eq!(clock.mode(), SyncMode::MidiIn);
        assert!(!clock.is_running());
    }
}
