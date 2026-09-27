//! Musical timing backend: standalone internal clock or Ableton Link.
//!
//! Link support is optional at compile time. Build with:
//! `cargo run --features ableton-link`

use std::time::Instant;

/// Clock source used by Khaṇa.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncMode {
    Internal,
    Link,
}

/// Snapshot of transport/synchronization state for UI rendering.
#[derive(Clone, Copy, Debug)]
pub struct SyncStatus {
    pub mode: SyncMode,
    pub running: bool,
    pub tempo_bpm: f64,
    pub link_peers: u64,
    pub link_start_stop_sync: bool,
    pub link_available: bool,
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

    #[cfg(feature = "ableton-link")]
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
        }
    }

    fn enable(&self, enabled: bool) {
        self.link.enable(enabled);
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

    fn set_start_stop_sync(&self, enabled: bool) {
        self.link.enable_start_stop_sync(enabled);
    }

    fn is_running(&mut self) -> bool {
        if self.start_stop_sync() {
            self.link.capture_app_session_state(&mut self.state);
            self.state.is_playing()
        } else {
            self.local_running
        }
    }

    fn set_running(&mut self, running: bool) {
        self.local_running = running;

        if self.start_stop_sync() {
            self.link.capture_app_session_state(&mut self.state);
            let now = self.link.clock_micros();
            self.state.set_is_playing(running, now);
            self.link.commit_app_session_state(&self.state);
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

        match mode {
            SyncMode::Internal => {
                #[cfg(feature = "ableton-link")]
                {
                    let beat = self.link.beat(quantum);
                    let tempo = self.link.tempo();
                    let running = self.link.is_running();
                    self.link.enable(false);
                    self.internal.set_position(beat);
                    self.internal.set_bpm(tempo);

                    if running {
                        self.internal.start();
                    } else {
                        self.internal.stop();
                    }
                }

                self.mode = SyncMode::Internal;
                true
            }
            SyncMode::Link => {
                #[cfg(feature = "ableton-link")]
                {
                    let bpm = self.internal.bpm;
                    let running = self.internal.running;

                    // Set our private Link timeline before networking is
                    // enabled. Once enabled, an existing Link session is free
                    // to supply its own session tempo/phase without Khaṇa
                    // pushing its standalone tempo into the jam.
                    self.link.set_tempo(bpm);
                    self.link
                        .set_start_stop_sync(self.link_start_stop_preference);
                    self.link.enable(true);
                    self.link.set_running(running);

                    self.mode = SyncMode::Link;
                    true
                }

                #[cfg(not(feature = "ableton-link"))]
                {
                    let _ = quantum;
                    false
                }
            }
        }
    }

    /// Returns current beat using the active source.
    pub fn beat(&mut self, quantum: f64) -> f64 {
        match self.mode {
            SyncMode::Internal => self.internal.beat(),
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
    pub fn start(&mut self) {
        match self.mode {
            SyncMode::Internal => self.internal.start(),
            SyncMode::Link => {
                #[cfg(feature = "ableton-link")]
                self.link.set_running(true);
            }
        }
    }

    /// Stops Khaṇa transport.
    pub fn stop(&mut self) {
        match self.mode {
            SyncMode::Internal => self.internal.stop(),
            SyncMode::Link => {
                #[cfg(feature = "ableton-link")]
                self.link.set_running(false);
            }
        }
    }

    /// Returns the current session tempo.
    pub fn tempo(&mut self) -> f64 {
        match self.mode {
            SyncMode::Internal => self.internal.bpm,
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
            tempo_bpm,
            link_peers: peers,
            link_start_stop_sync: self.link_start_stop_preference,
            link_available: self.link_available(),
        }
    }
}
