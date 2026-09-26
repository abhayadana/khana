const MAJOR_SCALE_INTERVALS: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
const C4_MIDI_NOTE: u8 = 60;
const DEFAULT_VELOCITY: u8 = 100;

pub struct MusicState {
    root_midi_note: u8,
    selected_index: usize,
    velocity: u8,
}

impl MusicState {
    pub const fn new_c_major() -> Self {
        Self {
            root_midi_note: C4_MIDI_NOTE,
            selected_index: 0,
            velocity: DEFAULT_VELOCITY,
        }
    }

    pub fn select_previous_degree(&mut self) {
        if self.selected_index == 0 {
            self.selected_index = MAJOR_SCALE_INTERVALS.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    pub fn select_next_degree(&mut self) {
        self.selected_index =
            (self.selected_index + 1) % MAJOR_SCALE_INTERVALS.len();
    }

    pub const fn selected_degree(&self) -> usize {
        self.selected_index + 1
    }

    pub fn selected_midi_note(&self) -> u8 {
        self.root_midi_note + MAJOR_SCALE_INTERVALS[self.selected_index]
    }

    pub fn selected_note_name(&self) -> &'static str {
        match self.selected_index {
            0 => "C4",
            1 => "D4",
            2 => "E4",
            3 => "F4",
            4 => "G4",
            5 => "A4",
            6 => "B4",
            _ => unreachable!("selected_index is always normalized to 0..7"),
        }
    }

    pub const fn velocity(&self) -> u8 {
        self.velocity
    }
}

#[cfg(test)]
mod tests {
    use super::MusicState;

    #[test]
    fn c_major_maps_degrees_to_expected_midi_notes() {
        let mut state = MusicState::new_c_major();
        let expected = [60_u8, 62, 64, 65, 67, 69, 71];

        for note in expected {
            assert_eq!(state.selected_midi_note(), note);
            state.select_next_degree();
        }
    }

    #[test]
    fn degree_selection_wraps_in_both_directions() {
        let mut state = MusicState::new_c_major();

        state.select_previous_degree();
        assert_eq!(state.selected_degree(), 7);

        state.select_next_degree();
        assert_eq!(state.selected_degree(), 1);
    }
}
