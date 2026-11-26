//! Four-channel MIDI output with polyphonic note ownership.
//!
//! Bass, Texture, and Melody currently behave monophonically at the performer
//! layer. Chords is polyphonic on MIDI channel 2. The MIDI layer therefore
//! tracks a set of active notes for every channel so stale Note Off messages
//! cannot terminate unrelated notes.

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

/// MIDI transport for the four Khaṇa performers.
pub struct MidiEngine {
    connection: MidiOutputConnection,
    ownership: NoteOwnership,
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
            connection,
            ownership: NoteOwnership::new(),
        })
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
        for voice_index in 0..VOICE_COUNT {
            for note in 0_u8..=127 {
                if self.ownership.contains(voice_index, note) {
                    self.send(note_off_message(voice_index, note))?;
                    self.ownership.remove(voice_index, note);
                }
            }
        }

        Ok(())
    }

    fn send(&mut self, message: [u8; 3]) -> Result<(), String> {
        self.connection
            .send(&message)
            .map_err(|error| error.to_string())
    }
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
}
