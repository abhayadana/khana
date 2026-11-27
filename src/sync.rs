//! Musical timing backend: internal clock, MIDI Clock In, or Ableton Link.
//!
//! Link support is optional at compile time. Build with:
//! `cargo run --features ableton-link`

use std::time::Instant;

use crate::midi::{MidiFollower, RawClockInput};

/// Clock source used by Khaṇa.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncMode {
    Internal,
    Link,
    MidiIn,
}

#[cfg(all(test, feature = "ableton-link"))]
mod tests {
    use super::*;

    #[test]
    fn local_link_launch_is_quantized_and_can_be_cancelled() {
        for share_transport in [false, true] {
            check_quantized_launch(share_transport);
        }
    }

    fn check_quantized_launch(share_transport: bool) {
        let mut transport = LinkTransport::new(120.0);
        transport.join(120.0, share_transport);
        let quantum = 4.0;
        let time = transport.link.clock_micros();
        transport
            .link
            .capture_app_session_state(&mut transport.state);
        transport.state.force_beat_at_time(1.0, time, quantum);
        transport.link.commit_app_session_state(&transport.state);

        assert!(!transport.is_running());
        transport.start(quantum);
        assert_eq!(transport.pending_beat(quantum), Some(4.0));
        assert!(!transport.is_running());
        transport.stop();
        assert!(!transport.start_pending());
        assert!(!transport.is_running());

        transport.start(quantum);
        let time = transport.link.clock_micros();
        transport
            .link
            .capture_app_session_state(&mut transport.state);
        transport.state.force_beat_at_time(4.01, time, quantum);
        transport.link.commit_app_session_state(&transport.state);
        assert!(transport.is_running());
        assert!(transport.is_running());
        transport.stop();
        assert!(!transport.is_running());
    }
}

/// Snapshot of transport/synchronization state for UI rendering.
#[derive(Clone, Copy, Debug)]
pub struct SyncStatus {
    pub mode: SyncMode,
    pub running: bool,
    pub starting: bool,
    pub tempo_bpm: f64,
    pub link_peers: u64,
    pub link_start_stop_sync: bool,
    pub link_available: bool,
    pub midi_status: &'static str,
}

struct InternalTransport {
    running: bool,
    bpm: f64,
    stored_beat: f64,
    anchor: Instant,
}

impl InternalTransport {
    fn new(bpm: f64) -> Self {
        Self {
            running: false,
            bpm,
            stored_beat: 0.0,
            anchor: Instant::now(),
        }
    }

    fn beat(&self) -> f64 {
        if self.running {
            self.stored_beat + self.anchor.elapsed().as_secs_f64() * self.bpm / 60.0
        } else {
            self.stored_beat
        }
    }

    fn start(&mut self) {
        if self.running {
            return;
        }

        self.anchor = Instant::now();
        self.running = true;
    }

    fn stop(&mut self) {
        if !self.running {
            return;
        }

        self.stored_beat = self.beat();
        self.running = false;
    }

    fn set_bpm(&mut self, bpm: f64) {
        let beat = self.beat();
        self.stored_beat = beat;
        self.bpm = bpm.clamp(30.0, 300.0);
        self.anchor = Instant::now();
    }

    fn set_position(&mut self, beat: f64) {
        self.stored_beat = beat;
        self.anchor = Instant::now();
    }
}

#[cfg(feature = "ableton-link")]
struct LinkTransport {
    link: rusty_link::AblLink,
    state: rusty_link::SessionState,
    local_running: bool,
    local_start: Option<(f64, f64)>,
}

#[cfg(feature = "ableton-link")]
impl LinkTransport {
    fn new(bpm: f64) -> Self {
        let link = rusty_link::AblLink::new(bpm);
        link.enable(false);

        Self {
            link,
            state: rusty_link::SessionState::new(),
            local_running: false,
            local_start: None,
        }
    }

    fn enable(&self, enabled: bool) {
        self.link.enable(enabled);
    }

    fn join(&mut self, bpm: f64, share_transport: bool) {
        self.enable(false);
        self.local_running = false;
        self.local_start = None;
        self.link.enable_start_stop_sync(false);
        self.set_tempo(bpm);
        self.enable(true);
        // Joining is passive. Only a transport gesture publishes Start/Stop.
        self.link.enable_start_stop_sync(share_transport);
    }

    fn beat(&mut self, quantum: f64) -> f64 {
        self.link.capture_app_session_state(&mut self.state);
        let now = self.link.clock_micros();
        self.state.beat_at_time(now, quantum.max(0.25))
    }

    fn tempo(&mut self) -> f64 {
        self.link.capture_app_session_state(&mut self.state);
        self.state.tempo()
    }

    fn set_tempo(&mut self, bpm: f64) {
        self.link.capture_app_session_state(&mut self.state);
        let now = self.link.clock_micros();
        self.state.set_tempo(bpm.clamp(30.0, 300.0), now);
        self.link.commit_app_session_state(&self.state);
    }

    fn start_stop_sync(&self) -> bool {
        self.link.is_start_stop_sync_enabled()
    }

    fn set_start_stop_sync(&mut self, enabled: bool) {
        if enabled != self.start_stop_sync() {
            self.local_running = self.is_running();
            self.local_start = None;
        }
        self.link.enable_start_stop_sync(enabled);
    }

    fn is_running(&mut self) -> bool {
        let shared = self.start_stop_sync();
        if shared {
            self.link.capture_app_session_state(&mut self.state);
            if self.state.is_playing() {
                self.local_start = None;
                return self.state.time_for_is_playing() <= self.link.clock_micros();
            }
        }
        if let Some((target, quantum)) = self.local_start
            && self.beat(quantum) >= target
        {
            self.local_start = None;
            if shared {
                // Publish at launch. A future-dated shared Start would take
                // precedence over a Stop pressed before the bar arrives.
                let now = self.link.clock_micros();
                self.state.set_is_playing(true, now);
                self.link.commit_app_session_state(&self.state);
            } else {
                self.local_running = true;
            }
            return true;
        }
        !shared && self.local_running
    }

    fn start_pending(&mut self) -> bool {
        if self.local_start.is_some() {
            return true;
        }
        if self.start_stop_sync() {
            self.link.capture_app_session_state(&mut self.state);
            self.state.is_playing() && self.state.time_for_is_playing() > self.link.clock_micros()
        } else {
            false
        }
    }

    fn pending_beat(&mut self, quantum: f64) -> Option<f64> {
        if let Some((beat, _)) = self.local_start {
            return Some(beat);
        }
        if self.start_stop_sync() {
            self.link.capture_app_session_state(&mut self.state);
            let time = self.state.time_for_is_playing();
            (self.state.is_playing() && time > self.link.clock_micros())
                .then(|| self.state.beat_at_time(time, quantum))
        } else {
            None
        }
    }

    fn start(&mut self, quantum: f64) {
        if self.is_running() || self.start_pending() {
            return;
        }
        let now = self.link.clock_micros();
        self.link.capture_app_session_state(&mut self.state);
        let beat = self.state.beat_at_time(now, quantum);
        let target = ((beat / quantum).floor() + 1.0) * quantum;
        self.local_start = Some((target, quantum));
    }

    fn stop(&mut self) {
        self.local_running = false;
        self.local_start = None;
        if self.start_stop_sync() {
            self.link.capture_app_session_state(&mut self.state);
            if self.state.is_playing() {
                let now = self.link.clock_micros();
                let time = now.max(self.state.time_for_is_playing().saturating_add(1));
                self.state.set_is_playing(false, time);
                self.link.commit_app_session_state(&self.state);
            }
        }
    }

    fn peers(&self) -> u64 {
        self.link.num_peers()
    }
}

/// Unified transport used by the rest of Khaṇa.
pub struct SyncTransport {
    mode: SyncMode,
    internal: InternalTransport,
    #[cfg(feature = "ableton-link")]
    link: LinkTransport,
    link_start_stop_preference: bool,
    midi_port: Option<RawClockInput>,
    midi: MidiFollower,
}

impl SyncTransport {
    /// Creates a stopped transport using the internal clock.
    pub fn new(bpm: f64) -> Self {
        Self {
            mode: SyncMode::Internal,
            internal: InternalTransport::new(bpm),
            #[cfg(feature = "ableton-link")]
            link: LinkTransport::new(bpm),
            link_start_stop_preference: false,
            midi_port: None,
            midi: MidiFollower::new(bpm),
        }
    }

    /// Returns whether this binary contains the Ableton Link backend.
    pub const fn link_available(&self) -> bool {
        cfg!(feature = "ableton-link")
    }

    /// Returns the selected synchronization source.
    pub const fn mode(&self) -> SyncMode {
        self.mode
    }

    /// Attempts to switch synchronization source while preserving tempo/beat.
    ///
    /// Returns false if Link was requested but this binary was built without
    /// the `ableton-link` feature.
    pub fn set_mode(&mut self, mode: SyncMode, quantum: f64) -> bool {
        if mode == self.mode {
            return true;
        }
        if mode == SyncMode::Link && !self.link_available() {
            return false;
        }
        let beat = self.beat(quantum);
        let tempo = self.tempo();
        self.internal.stop();
        self.internal.set_bpm(tempo);
        self.internal.set_position(beat);
        self.midi_port = (mode == SyncMode::MidiIn).then(RawClockInput::new);
        self.midi = MidiFollower::new(tempo);

        #[cfg(feature = "ableton-link")]
        {
            if self.mode == SyncMode::Link {
                self.link.enable(false);
            }
            if mode == SyncMode::Link {
                self.link.join(tempo, self.link_start_stop_preference);
            }
        }
        self.mode = mode;
        true
    }

    pub fn poll_midi(&mut self, now: Instant) {
        if let Some(port) = &mut self.midi_port {
            self.midi.check_timeout(now);
            let connected = port.poll(now, |bytes| self.midi.receive(bytes, now));
            self.midi.update_port(connected, now);
        }
    }

    pub fn midi_input_label(&self) -> Option<&str> {
        self.midi_port.as_ref().map(RawClockInput::label)
    }

    pub fn take_midi_restart(&mut self) -> bool {
        self.mode == SyncMode::MidiIn && self.midi.take_restart()
    }

    /// Returns current beat using the active source.
    pub fn beat(&mut self, quantum: f64) -> f64 {
        match self.mode {
            SyncMode::Internal => self.internal.beat(),
            SyncMode::MidiIn => self.midi.beat(Instant::now()),
            SyncMode::Link => {
                #[cfg(feature = "ableton-link")]
                {
                    self.link.beat(quantum)
                }

                #[cfg(not(feature = "ableton-link"))]
                {
                    let _ = quantum;
                    self.internal.beat()
                }
            }
        }
    }

    /// Returns whether Khaṇa transport is running.
    pub fn is_running(&mut self) -> bool {
        match self.mode {
            SyncMode::Internal => self.internal.running,
            SyncMode::MidiIn => self.midi.running(),
            SyncMode::Link => {
                #[cfg(feature = "ableton-link")]
                {
                    self.link.is_running()
                }

                #[cfg(not(feature = "ableton-link"))]
                {
                    false
                }
            }
        }
    }

    /// Starts Khaṇa transport.
    pub fn start(&mut self, _quantum: f64) {
        match self.mode {
            SyncMode::Internal => self.internal.start(),
            SyncMode::MidiIn => self.midi.arm(),
            SyncMode::Link => {
                #[cfg(feature = "ableton-link")]
                self.link.start(_quantum);
            }
        }
    }

    /// Stops Khaṇa transport.
    pub fn stop(&mut self) {
        match self.mode {
            SyncMode::Internal => self.internal.stop(),
            SyncMode::MidiIn => self.midi.disarm(),
            SyncMode::Link => {
                #[cfg(feature = "ableton-link")]
                self.link.stop();
            }
        }
    }

    pub fn start_pending(&mut self) -> bool {
        #[cfg(feature = "ableton-link")]
        if self.mode == SyncMode::Link {
            return self.link.start_pending();
        }
        false
    }

    pub fn pending_beat(&mut self, _quantum: f64) -> Option<f64> {
        #[cfg(feature = "ableton-link")]
        if self.mode == SyncMode::Link {
            return self.link.pending_beat(_quantum);
        }
        None
    }

    /// Returns the current session tempo.
    pub fn tempo(&mut self) -> f64 {
        match self.mode {
            SyncMode::Internal => self.internal.bpm,
            SyncMode::MidiIn => self.midi.tempo(),
            SyncMode::Link => {
                #[cfg(feature = "ableton-link")]
                {
                    self.link.tempo()
                }

                #[cfg(not(feature = "ableton-link"))]
                {
                    self.internal.bpm
                }
            }
        }
    }

    /// Changes tempo locally or submits it to the Link session.
    pub fn set_tempo(&mut self, bpm: f64) {
        match self.mode {
            SyncMode::Internal => self.internal.set_bpm(bpm),
            SyncMode::MidiIn => {}
            SyncMode::Link => {
                #[cfg(feature = "ableton-link")]
                self.link.set_tempo(bpm);
            }
        }
    }

    /// Enables/disables Link start/stop synchronization.
    pub fn set_link_start_stop_sync(&mut self, enabled: bool) {
        self.link_start_stop_preference = enabled;

        #[cfg(feature = "ableton-link")]
        self.link.set_start_stop_sync(enabled);
    }

    /// Returns a UI-oriented synchronization snapshot.
    pub fn status(&mut self) -> SyncStatus {
        let running = self.is_running();
        let tempo_bpm = self.tempo();

        #[cfg(feature = "ableton-link")]
        let peers = if self.mode == SyncMode::Link {
            self.link.peers()
        } else {
            0
        };

        #[cfg(not(feature = "ableton-link"))]
        let peers = 0;

        SyncStatus {
            mode: self.mode,
            running,
            starting: self.start_pending(),
            tempo_bpm,
            link_peers: peers,
            link_start_stop_sync: self.link_start_stop_preference,
            link_available: self.link_available(),
            midi_status: self.midi.label(),
        }
    }
}
