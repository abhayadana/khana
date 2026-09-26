use sdl2::keyboard::Keycode;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputAction {
    SelectPrevious,
    SelectNext,
    Play,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEvent {
    Pressed(InputAction),
    Released(InputAction),
}

pub struct InputMapper;

impl InputMapper {
    pub const fn new() -> Self {
        Self
    }

    pub fn key_down(&self, keycode: Keycode) -> Option<InputEvent> {
        self.map_key(keycode).map(InputEvent::Pressed)
    }

    pub fn key_up(&self, keycode: Keycode) -> Option<InputEvent> {
        self.map_key(keycode).map(InputEvent::Released)
    }

    fn map_key(&self, keycode: Keycode) -> Option<InputAction> {
        match keycode {
            Keycode::Left => Some(InputAction::SelectPrevious),
            Keycode::Right => Some(InputAction::SelectNext),
            // Z temporarily stands in for the Brick's A button.
            Keycode::Z => Some(InputAction::Play),
            _ => None,
        }
    }
}
