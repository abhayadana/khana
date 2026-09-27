//! Four-channel MIDI output.
//!
//! The sustained-chord prototype guarantees one active note on each of MIDI
//! channels 1–4 whenever a render is sounding. Density is intentionally *not*
//! interpreted as a mute/probability gate here.

use crate::voice::VoiceRender;
use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};

const VOICE_COUNT: usize = 4;

/// MIDI transport for the four Khaṇa performers.
pub struct MidiEngine {
    connection: MidiOutputConnection,
    active_notes: [Option<u8>; VOICE_COUNT],
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
            active_notes: [None; VOICE_COUNT],
        })
    }

    /// Rearticulates a complete four-voice realization.
    ///
    /// Every currently tracked note is turned off first, then exactly one Note
    /// On is sent on each of channels 1–4. Rearticulation is deliberate: a new
    /// chord selection is a new harmonic attack even when one voice keeps the
    /// same pitch.
    ///
    /// # Errors
    ///
    /// Returns an error if a MIDI message cannot be sent.
    pub fn play_render(&mut self, render: VoiceRender) -> Result<(), String> {
        self.stop_all()?;

        for index in 0..VOICE_COUNT {
            let message =
                note_on_message(index, render.voicing.notes[index], render.velocities[index]);
            self.connection
                .send(&message)
                .map_err(|error| error.to_string())?;
            self.active_notes[index] = Some(render.voicing.notes[index]);
        }

        Ok(())
    }

    /// Sends Note Off for every note Khaṇa currently tracks as active.
    ///
    /// After any successful `play_render`, this means one Note Off on each of
    /// channels 1–4.
    ///
    /// # Errors
    ///
    /// Returns an error if a MIDI message cannot be sent.
    pub fn stop_all(&mut self) -> Result<(), String> {
        for index in 0..VOICE_COUNT {
            if let Some(note) = self.active_notes[index].take() {
                let message = note_off_message(index, note);
                self.connection
                    .send(&message)
                    .map_err(|error| error.to_string())?;
            }
        }

        Ok(())
    }
}

fn note_on_message(channel_index: usize, note: u8, velocity: u8) -> [u8; 3] {
    [
        0x90 | channel_nibble(channel_index),
        note,
        velocity.min(127),
    ]
}

fn note_off_message(channel_index: usize, note: u8) -> [u8; 3] {
    [0x80 | channel_nibble(channel_index), note, 0]
}

fn channel_nibble(channel_index: usize) -> u8 {
    debug_assert!(channel_index < VOICE_COUNT);
    channel_index as u8
}

#[cfg(test)]
mod tests {
    use super::{note_off_message, note_on_message};

    #[test]
    fn four_voice_note_on_messages_use_midi_channels_one_through_four() {
        let messages = [
            note_on_message(0, 48, 90),
            note_on_message(1, 55, 91),
            note_on_message(2, 60, 92),
            note_on_message(3, 67, 93),
        ];

        assert_eq!(messages[0], [0x90, 48, 90]);
        assert_eq!(messages[1], [0x91, 55, 91]);
        assert_eq!(messages[2], [0x92, 60, 92]);
        assert_eq!(messages[3], [0x93, 67, 93]);
    }

    #[test]
    fn four_voice_note_off_messages_use_midi_channels_one_through_four() {
        let messages = [
            note_off_message(0, 48),
            note_off_message(1, 55),
            note_off_message(2, 60),
            note_off_message(3, 67),
        ];

        assert_eq!(messages[0], [0x80, 48, 0]);
        assert_eq!(messages[1], [0x81, 55, 0]);
        assert_eq!(messages[2], [0x82, 60, 0]);
        assert_eq!(messages[3], [0x83, 67, 0]);
    }
}
