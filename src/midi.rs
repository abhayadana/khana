//! Four-channel MIDI output.

use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};

/// MIDI transport for the four Khaṇa performers.
pub struct MidiEngine {
    connection: MidiOutputConnection,
    active_notes: [Option<u8>; 4],
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
            active_notes: [None; 4],
        })
    }

    /// Plays one monophonic note per MIDI channel 1..=4.
    ///
    /// `active` allows Density to suppress individual voices on this chord
    /// attack while preserving the four-channel architecture.
    ///
    /// # Errors
    ///
    /// Returns an error if a MIDI message cannot be sent.
    pub fn play_voicing(
        &mut self,
        notes: [u8; 4],
        velocities: [u8; 4],
        active: [bool; 4],
    ) -> Result<(), String> {
        self.stop_all()?;

        for index in 0..4 {
            if !active[index] {
                continue;
            }

            let channel = index as u8;
            let status = 0x90 | channel;
            self.connection
                .send(&[status, notes[index], velocities[index]])
                .map_err(|error| error.to_string())?;
            self.active_notes[index] = Some(notes[index]);
        }

        Ok(())
    }

    /// Sends Note Off for every active Khaṇa voice.
    ///
    /// # Errors
    ///
    /// Returns an error if a MIDI message cannot be sent.
    pub fn stop_all(&mut self) -> Result<(), String> {
        for index in 0..4 {
            if let Some(note) = self.active_notes[index].take() {
                let channel = index as u8;
                let status = 0x80 | channel;
                self.connection
                    .send(&[status, note, 0])
                    .map_err(|error| error.to_string())?;
            }
        }

        Ok(())
    }
}
