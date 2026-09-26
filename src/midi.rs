use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};

const MIDI_CHANNEL_1_NOTE_ON: u8 = 0x90;
const MIDI_CHANNEL_1_NOTE_OFF: u8 = 0x80;

pub struct MidiEngine {
    connection: MidiOutputConnection,
}

impl MidiEngine {
    pub fn new_virtual(port_name: &str) -> Result<Self, String> {
        let midi_output =
            MidiOutput::new("Khana").map_err(|error| error.to_string())?;

        let connection = midi_output
            .create_virtual(port_name)
            .map_err(|error| error.to_string())?;

        Ok(Self { connection })
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) -> Result<(), String> {
        self.connection
            .send(&[MIDI_CHANNEL_1_NOTE_ON, note, velocity])
            .map_err(|error| error.to_string())
    }

    pub fn note_off(&mut self, note: u8) -> Result<(), String> {
        self.connection
            .send(&[MIDI_CHANNEL_1_NOTE_OFF, note, 0])
            .map_err(|error| error.to_string())
    }
}
