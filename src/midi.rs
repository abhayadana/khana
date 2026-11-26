//! Four-channel MIDI output with polyphonic note ownership.
//!
//! Bass, Texture, and Melody currently behave monophonically at the performer
//! layer. Chords is polyphonic on MIDI channel 2. The MIDI layer therefore
//! tracks a set of active notes for every channel so stale Note Off messages
//! cannot terminate unrelated notes.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::Path;
use std::time::Duration;

use crate::performer::VoiceEvent;
use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};

const VOICE_COUNT: usize = 4;

#[derive(Clone, Copy, Debug)]
struct NoteOwnership {
    active: [u128; VOICE_COUNT],
}

impl NoteOwnership {
    const fn new() -> Self {
        Self {
            active: [0; VOICE_COUNT],
        }
    }

    fn contains(&self, voice_index: usize, note: u8) -> bool {
        self.active
            .get(voice_index)
            .is_some_and(|bits| bits & note_bit(note) != 0)
    }

    fn insert(&mut self, voice_index: usize, note: u8) {
        if let Some(bits) = self.active.get_mut(voice_index) {
            *bits |= note_bit(note);
        }
    }

    fn remove(&mut self, voice_index: usize, note: u8) {
        if let Some(bits) = self.active.get_mut(voice_index) {
            *bits &= !note_bit(note);
        }
    }
}

/// The selected output backend. Raw MIDI is explicit, never guessed.
enum MidiSink {
    Alsa(MidiOutputConnection),
    Raw(File),
    DryRun,
}

/// MIDI transport for the four Khaṇa performers.
pub struct MidiEngine {
    connection: MidiSink,
    ownership: NoteOwnership,
    destination_name: String,
    trace: bool,
}

impl MidiEngine {
    /// Creates a virtual ALSA MIDI output.
    ///
    /// # Errors
    ///
    /// Returns an error if the MIDI subsystem or virtual port cannot be opened.
    pub fn new_virtual(port_name: &str) -> Result<Self, String> {
        let midi_output = MidiOutput::new("Khana").map_err(|error| error.to_string())?;
        let connection = midi_output
            .create_virtual(port_name)
            .map_err(|error| error.to_string())?;

        Ok(Self {
            connection: MidiSink::Alsa(connection),
            ownership: NoteOwnership::new(),
            destination_name: format!("VIRTUAL: {port_name}"),
            trace: trace_enabled(),
        })
    }

    /// Connects to an ALSA MIDI destination when available, otherwise creates
    /// the desktop-compatible virtual ALSA port.
    ///
    /// `KHANA_MIDI_PORT` can contain a case-insensitive substring used to
    /// select among multiple physical destinations.
    pub fn new_auto(virtual_port_name: &str) -> Result<Self, String> {
        let midi_output = MidiOutput::new("Khana").map_err(|error| error.to_string())?;
        let ports = midi_output.ports();
        let requested = std::env::var("KHANA_MIDI_PORT")
            .ok()
            .filter(|value| !value.trim().is_empty());

        let names: Vec<String> = ports
            .iter()
            .enumerate()
            .map(|(index, port)| {
                midi_output
                    .port_name(port)
                    .unwrap_or_else(|_| format!("MIDI PORT {}", index + 1))
            })
            .collect();
        for (index, name) in names.iter().enumerate() {
            println!("MIDI port {index}: {name}");
        }
        let selected_index = if let Some(needle) = requested {
            let needle = needle.to_lowercase();
            let mut matches = names
                .iter()
                .enumerate()
                .filter(|(_, name)| name.to_lowercase().contains(&needle));
            let index = matches.next().map(|(index, _)| index).ok_or_else(|| {
                format!("KHANA_MIDI_PORT did not match any destination: {needle}")
            })?;
            if matches.next().is_some() {
                return Err(format!(
                    "KHANA_MIDI_PORT matches multiple destinations: {needle}"
                ));
            }
            Some(index)
        } else {
            // ALSA's Midi Through is not evidence of an attached USB synth.
            names
                .iter()
                .position(|name| !name.to_lowercase().contains("midi through"))
        };

        if let Some(index) = selected_index {
            let port = &ports[index];
            let name = names[index].clone();
            let connection = midi_output
                .connect(port, "Khana")
                .map_err(|error| error.to_string())?;

            println!("MIDI: connected to destination: {name}");
            return Ok(Self {
                connection: MidiSink::Alsa(connection),
                ownership: NoteOwnership::new(),
                destination_name: name,
                trace: trace_enabled(),
            });
        }

        println!(
            "MIDI: no non-through destination found; creating virtual port {virtual_port_name}"
        );
        Self::new_virtual(virtual_port_name)
    }

    /// UI/control bring-up without an ALSA sequencer or attached synth.
    /// Still exercises note ownership but never sends external MIDI.
    pub fn dry_run() -> Self {
        println!("MIDI: DRY RUN - no output");
        Self {
            connection: MidiSink::DryRun,
            ownership: NoteOwnership::new(),
            destination_name: "DRY RUN - NO OUTPUT".to_owned(),
            trace: trace_enabled(),
        }
    }

    /// Open the requested Linux raw MIDI character device without blocking.
    ///
    /// This bypasses ALSA sequencer discovery, not the kernel USB MIDI driver.
    fn new_raw(path: &Path) -> Result<Self, String> {
        // Linux O_NONBLOCK on x86_64 and aarch64. No new libc dependency.
        const O_NONBLOCK: i32 = 0x800;
        let metadata = std::fs::metadata(path).map_err(|error| error.to_string())?;
        if !metadata.file_type().is_char_device() {
            return Err(format!("Not a MIDI character device: {}", path.display()));
        }
        let file = OpenOptions::new()
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(path)
            .map_err(|error| format!("Raw MIDI {}: {error}", path.display()))?;
        let destination_name = format!("RAW: {}", path.display());
        println!("MIDI: {destination_name}");
        Ok(Self {
            connection: MidiSink::Raw(file),
            ownership: NoteOwnership::new(),
            destination_name,
            trace: trace_enabled(),
        })
    }

    /// Resolve explicit output requests before attempting automatic discovery.
    /// Only an unrequested auto connection may fall back to Brick dry-run.
    pub fn configured(brick: bool) -> Result<Self, String> {
        if std::env::var("KHANA_MIDI_DRY_RUN").as_deref() == Ok("1") {
            return Ok(Self::dry_run());
        }

        let raw = std::env::var_os("KHANA_MIDI_RAW_DEVICE").filter(|path| !path.is_empty());
        let requested = explicit_port();
        if let Some(path) = raw {
            if requested {
                return Err("Set only one of KHANA_MIDI_PORT and KHANA_MIDI_RAW_DEVICE".to_owned());
            }
            return Self::new_raw(Path::new(&path));
        }

        match Self::new_auto("Khana MIDI") {
            Err(error) if brick && !requested => {
                eprintln!("MIDI unavailable: {error}; starting UI-only dry run");
                Ok(Self::dry_run())
            }
            result => result,
        }
    }

    pub fn active_counts(&self) -> [u32; VOICE_COUNT] {
        self.ownership.active.map(u128::count_ones)
    }

    /// Human-readable connected MIDI destination.
    pub fn destination_name(&self) -> &str {
        &self.destination_name
    }

    /// Applies one performer event while preserving per-note ownership.
    ///
    /// # Errors
    ///
    /// Returns an error if a MIDI message cannot be sent.
    pub fn apply_event(&mut self, event: VoiceEvent) -> Result<(), String> {
        match event {
            VoiceEvent::NoteOn {
                voice_index,
                note,
                velocity,
            } => self.note_on(voice_index, note, velocity),
            VoiceEvent::NoteOff { voice_index, note } => self.note_off_if_owned(voice_index, note),
        }
    }

    fn note_on(&mut self, voice_index: usize, note: u8, velocity: u8) -> Result<(), String> {
        if voice_index >= VOICE_COUNT {
            return Ok(());
        }

        // Rearticulating the same pitch first releases only that pitch. Other
        // Chords notes on the channel remain owned and sounding.
        if self.ownership.contains(voice_index, note) {
            self.send(note_off_message(voice_index, note))?;
            self.ownership.remove(voice_index, note);
        }

        self.send(note_on_message(voice_index, note, velocity))?;
        self.ownership.insert(voice_index, note);
        Ok(())
    }

    fn note_off_if_owned(&mut self, voice_index: usize, note: u8) -> Result<(), String> {
        if voice_index >= VOICE_COUNT {
            return Ok(());
        }

        if self.ownership.contains(voice_index, note) {
            self.send(note_off_message(voice_index, note))?;
            self.ownership.remove(voice_index, note);
        }

        Ok(())
    }

    /// Sends Note Off for every note currently owned by Khaṇa.
    ///
    /// # Errors
    ///
    /// Returns an error if a MIDI message cannot be sent.
    pub fn stop_all(&mut self) -> Result<(), String> {
        let mut first_error = None;
        for voice_index in 0..VOICE_COUNT {
            for note in 0_u8..=127 {
                if self.ownership.contains(voice_index, note) {
                    match self.send(note_off_message(voice_index, note)) {
                        Ok(()) => self.ownership.remove(voice_index, note),
                        Err(error) => {
                            if first_error.is_none() {
                                first_error = Some(error);
                            }
                        }
                    }
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    fn send(&mut self, message: [u8; 3]) -> Result<(), String> {
        if self.trace {
            println!(
                "MIDI {:02X} {:02X} {:02X}",
                message[0], message[1], message[2]
            );
        }
        match &mut self.connection {
            MidiSink::Alsa(connection) => {
                connection.send(&message).map_err(|error| error.to_string())
            }
            MidiSink::Raw(file) => file
                .write_all(&message)
                .map_err(|error| format!("Raw MIDI write: {error}")),
            MidiSink::DryRun => Ok(()),
        }
    }
}

// Error returns from the event/render loop must also attempt owned Note Offs.
impl Drop for MidiEngine {
    fn drop(&mut self) {
        if let Err(error) = self.stop_all() {
            eprintln!("MIDI cleanup: {error}");
        }
    }
}

/// Print both discovery paths, even when the sequencer cannot initialize.
pub fn list_destinations() {
    match MidiOutput::new("Khana probe") {
        Ok(output) => {
            let ports = output.ports();
            println!("ALSA sequencer destinations: {}", ports.len());
            for (index, port) in ports.iter().enumerate() {
                let name = output.port_name(port).unwrap_or_else(|error| error.to_string());
                println!("  {index}: {name}");
            }
        }
        Err(error) => eprintln!("ALSA sequencer unavailable: {error}"),
    }
    match std::fs::read_dir("/dev/snd") {
        Ok(entries) => {
            let mut paths: Vec<_> = entries
                .filter_map(Result::ok)
                .filter(|entry| raw_device_name(&entry.file_name().to_string_lossy()))
                .map(|entry| entry.path())
                .collect();
            paths.sort();
            println!("Raw MIDI device nodes: {}", paths.len());
            for path in paths {
                println!("  {}", path.display());
            }
        }
        Err(error) => eprintln!("Cannot enumerate /dev/snd: {error}"),
    }
}

fn raw_device_name(name: &str) -> bool {
    let Some((card, device)) = name.strip_prefix("midiC").and_then(|rest| rest.split_once('D'))
    else {
        return false;
    };
    !card.is_empty()
        && !device.is_empty()
        && card.bytes().all(|byte| byte.is_ascii_digit())
        && device.bytes().all(|byte| byte.is_ascii_digit())
}

fn explicit_port() -> bool {
    std::env::var("KHANA_MIDI_PORT").is_ok_and(|value| !value.trim().is_empty())
}

/// Exercise the four channels with a short, explicit-destination audition.
pub fn smoke_test() -> Result<(), String> {
    let dry = std::env::var("KHANA_MIDI_DRY_RUN").as_deref() == Ok("1");
    let raw = std::env::var_os("KHANA_MIDI_RAW_DEVICE").is_some_and(|path| !path.is_empty());
    if !dry && !raw && !explicit_port() {
        return Err("MIDI test requires KHANA_MIDI_PORT, KHANA_MIDI_RAW_DEVICE or dry-run".to_owned());
    }
    let mut midi = MidiEngine::configured(false)?;
    let groups: [&[u8]; VOICE_COUNT] = [&[48], &[60, 64, 67], &[72], &[76]];
    for (voice_index, notes) in groups.into_iter().enumerate() {
        println!("Testing channel {}: {notes:?}", voice_index + 1);
        for &note in notes {
            midi.apply_event(VoiceEvent::NoteOn {
                voice_index,
                note,
                velocity: 72,
            })?;
        }
        println!("Owned notes: {:?}", midi.active_counts());
        std::thread::sleep(Duration::from_millis(350));
        midi.stop_all()?;
    }
    println!("MIDI test complete; owned notes: {:?}", midi.active_counts());
    Ok(())
}

fn trace_enabled() -> bool {
    std::env::var("KHANA_MIDI_TRACE").as_deref() == Ok("1")
}

fn note_bit(note: u8) -> u128 {
    1_u128 << note.min(127)
}

fn note_on_message(channel_index: usize, note: u8, velocity: u8) -> [u8; 3] {
    [
        0x90 | channel_nibble(channel_index),
        note.min(127),
        velocity.min(127),
    ]
}

fn note_off_message(channel_index: usize, note: u8) -> [u8; 3] {
    [0x80 | channel_nibble(channel_index), note.min(127), 0]
}

fn channel_nibble(channel_index: usize) -> u8 {
    debug_assert!(channel_index < VOICE_COUNT);
    channel_index as u8
}

#[cfg(test)]
mod tests {
    use super::{NoteOwnership, note_off_message, note_on_message};

    #[test]
    fn four_voice_note_on_messages_use_midi_channels_one_through_four() {
        assert_eq!(note_on_message(0, 48, 90), [0x90, 48, 90]);
        assert_eq!(note_on_message(1, 55, 91), [0x91, 55, 91]);
        assert_eq!(note_on_message(2, 60, 92), [0x92, 60, 92]);
        assert_eq!(note_on_message(3, 67, 93), [0x93, 67, 93]);
    }

    #[test]
    fn four_voice_note_off_messages_use_midi_channels_one_through_four() {
        assert_eq!(note_off_message(0, 48), [0x80, 48, 0]);
        assert_eq!(note_off_message(1, 55), [0x81, 55, 0]);
        assert_eq!(note_off_message(2, 60), [0x82, 60, 0]);
        assert_eq!(note_off_message(3, 67), [0x83, 67, 0]);
    }

    #[test]
    fn one_channel_can_own_multiple_chord_notes() {
        let mut ownership = NoteOwnership::new();
        ownership.insert(1, 52);
        ownership.insert(1, 55);
        ownership.insert(1, 59);

        assert!(ownership.contains(1, 52));
        assert!(ownership.contains(1, 55));
        assert!(ownership.contains(1, 59));
    }

    #[test]
    fn removing_one_chord_note_keeps_the_others_owned() {
        let mut ownership = NoteOwnership::new();
        ownership.insert(1, 52);
        ownership.insert(1, 55);
        ownership.insert(1, 59);
        ownership.remove(1, 55);

        assert!(ownership.contains(1, 52));
        assert!(!ownership.contains(1, 55));
        assert!(ownership.contains(1, 59));
    }

    #[test]
    fn stale_note_off_cannot_release_an_unrelated_note() {
        let mut ownership = NoteOwnership::new();
        ownership.insert(2, 64);

        assert!(!ownership.contains(2, 60));
        assert!(ownership.contains(2, 64));
    }

    #[test]
    fn raw_device_names_exclude_pcm_and_control_nodes() {
        assert!(super::raw_device_name("midiC1D0"));
        assert!(super::raw_device_name("midiC12D3"));
        for name in ["seq", "controlC1", "pcmC1D0p", "midiCD0", "midiC1D0x"] {
            assert!(!super::raw_device_name(name));
        }
    }

    #[test]
    fn dry_run_tracks_polyphony_and_cleans_up() {
        use crate::performer::VoiceEvent;
        let mut midi = super::MidiEngine::dry_run();
        for note in [52, 55, 59, 62, 65] {
            midi.apply_event(VoiceEvent::NoteOn {
                voice_index: 1,
                note,
                velocity: 90,
            })
            .unwrap();
        }
        assert_eq!(midi.active_counts(), [0, 5, 0, 0]);
        midi.stop_all().unwrap();
        assert_eq!(midi.active_counts(), [0; 4]);
    }
}
