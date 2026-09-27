//! Desktop keyboard mapping to semantic Khaṇa actions.

use crate::voice::MacroKind;
use sdl2::keyboard::Keycode;

/// Application-level action independent of physical keyboard layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputAction {
    NavigateLeft,
    NavigateRight,
    NavigateUp,
    NavigateDown,
    Primary,
    Reroll,
    Surprise,
    BiasResolve,
    BiasNeutral,
    BiasTension,
    ScopePrevious,
    ScopeNext,
    AdjustMacro { kind: MacroKind, delta: f32 },
    ToggleRecord,
    ArmNextRecording,
    ToggleScreen,
    EditDuplicate,
    EditDelete,
    EditShorten,
    EditExtend,
    PhrasePrevious,
    PhraseNext,
    ZoomOut,
    ZoomIn,
    Quit,
}

/// Maps Xubuntu keyboard input to semantic actions.
pub fn map_key(keycode: Keycode) -> Option<InputAction> {
    use InputAction::*;

    match keycode {
        Keycode::Left => Some(NavigateLeft),
        Keycode::Right => Some(NavigateRight),
        Keycode::Up => Some(NavigateUp),
        Keycode::Down => Some(NavigateDown),
        Keycode::Z => Some(Primary),
        Keycode::X => Some(Reroll),
        Keycode::S => Some(Surprise),
        Keycode::Q => Some(BiasResolve),
        Keycode::E => Some(BiasNeutral),
        Keycode::W => Some(BiasTension),
        Keycode::C => Some(ScopePrevious),
        Keycode::V => Some(ScopeNext),
        Keycode::T => Some(AdjustMacro {
            kind: MacroKind::Density,
            delta: -0.05,
        }),
        Keycode::G => Some(AdjustMacro {
            kind: MacroKind::Density,
            delta: 0.05,
        }),
        Keycode::Y => Some(AdjustMacro {
            kind: MacroKind::Dynamics,
            delta: -0.05,
        }),
        Keycode::H => Some(AdjustMacro {
            kind: MacroKind::Dynamics,
            delta: 0.05,
        }),
        Keycode::U => Some(AdjustMacro {
            kind: MacroKind::Spread,
            delta: -0.05,
        }),
        Keycode::J => Some(AdjustMacro {
            kind: MacroKind::Spread,
            delta: 0.05,
        }),
        Keycode::I => Some(AdjustMacro {
            kind: MacroKind::Color,
            delta: -0.05,
        }),
        Keycode::K => Some(AdjustMacro {
            kind: MacroKind::Color,
            delta: 0.05,
        }),
        Keycode::P => Some(ToggleRecord),
        Keycode::N => Some(ArmNextRecording),
        Keycode::Tab => Some(ToggleScreen),
        Keycode::D => Some(EditDuplicate),
        Keycode::Backspace => Some(EditDelete),
        Keycode::F => Some(EditShorten),
        Keycode::R => Some(EditExtend),
        Keycode::A => Some(PhrasePrevious),
        Keycode::L => Some(PhraseNext),
        Keycode::Minus => Some(ZoomOut),
        Keycode::Equals => Some(ZoomIn),
        Keycode::Escape => Some(Quit),
        _ => None,
    }
}
