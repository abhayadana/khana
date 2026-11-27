//! Desktop keyboard and handheld-controller mapping to semantic Khaṇa actions.

use crate::voice::MacroKind;
use sdl2::Sdl;
use sdl2::controller::{Button, GameController};
use sdl2::event::Event;
use sdl2::joystick::{HatState, Joystick};
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

/// Application-level action independent of screen-specific meaning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputAction {
    NavigateLeft,
    NavigateRight,
    NavigateUp,
    NavigateDown,
    Primary,
    Secondary,
    Tertiary,
    BiasResolve,
    BiasNeutral,
    BiasTension,
    DirectionStep(i32),
    ToggleMacroMember,
    CyclePolyphony,
    ToggleWorkspace,
    WorkspaceRecall,
    ScopePrevious,
    ScopeNext,
    AdjustMacro { kind: MacroKind, delta: f32 },
    ToggleRecord,
    ArmNextRecording,
    ToggleScreen,
    ToggleTransport,
    LeftClick,
    RightClick,
    PatternHold,
    CycleLiveGrid,
    ToggleMute,
    ToggleSolo,
    Vary,
    ToggleSettings,
    EditDuplicate,
    EditDelete,
    EditUndo,
    EditShorten,
    EditExtend,
    PhrasePrevious,
    PhraseNext,
    ZoomOut,
    ZoomIn,
    GridPrevious,
    GridNext,
    MenuTap,
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
        Keycode::X => Some(Secondary),
        Keycode::S => Some(Tertiary),
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
        Keycode::F4 => Some(ToggleMacroMember),
        Keycode::F3 => Some(CyclePolyphony),
        Keycode::F2 => Some(ToggleWorkspace),
        Keycode::F1 => Some(WorkspaceRecall),
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
        Keycode::Space => Some(ToggleTransport),
        Keycode::M => Some(ToggleSettings),
        Keycode::D => Some(EditDuplicate),
        Keycode::Backspace => Some(EditDelete),
        Keycode::F => Some(EditShorten),
        Keycode::R => Some(EditExtend),
        Keycode::A => Some(PhrasePrevious),
        Keycode::L => Some(PhraseNext),
        Keycode::Minus => Some(ZoomOut),
        Keycode::Equals => Some(ZoomIn),
        Keycode::LeftBracket => Some(GridPrevious),
        Keycode::RightBracket => Some(GridNext),
        Keycode::Escape => Some(Quit),
        Keycode::F5 => Some(LeftClick),
        Keycode::F6 => Some(RightClick),
        Keycode::F7 => Some(Vary),
        Keycode::F8 => Some(PatternHold),
        Keycode::F9 => Some(CycleLiveGrid),
        Keycode::F10 => Some(ToggleMute),
        Keycode::F11 => Some(ToggleSolo),
        Keycode::F12 => Some(EditUndo),
        _ => None,
    }
}

/// Keeps SDL controller/joystick devices open for the lifetime of the app.
///
/// The Brick/NextUI environment can expose controls either through SDL's game
/// controller API or as a generic joystick, so Khaṇa supports both paths.
pub fn open_input_devices(
    sdl: &Sdl,
    brick: bool,
) -> Result<(Vec<GameController>, Vec<Joystick>), String> {
    let controller_subsystem = sdl.game_controller()?;
    let joystick_subsystem = sdl.joystick()?;
    let count = joystick_subsystem
        .num_joysticks()
        .map_err(|error| error.to_string())?;

    let mut controllers = Vec::new();
    let mut joysticks = Vec::new();

    for index in 0..count {
        if !brick && controller_subsystem.is_game_controller(index) {
            match controller_subsystem.open(index) {
                Ok(controller) => controllers.push(controller),
                Err(error) => eprintln!("INPUT: controller {index} could not open: {error}"),
            }
        } else {
            match joystick_subsystem.open(index) {
                Ok(joystick) => {
                    println!(
                        "INPUT: joystick {} id={} axes={} buttons={} hats={}",
                        joystick.name(),
                        joystick.instance_id(),
                        joystick.num_axes(),
                        joystick.num_buttons(),
                        joystick.num_hats()
                    );
                    joysticks.push(joystick);
                }
                Err(error) => eprintln!("INPUT: joystick {index} could not open: {error}"),
            }
        }
    }

    println!(
        "INPUT: {} controller(s), {} joystick(s)",
        controllers.len(),
        joysticks.len()
    );

    Ok((controllers, joysticks))
}

/// Converts one SDL event into a semantic Khaṇa action.
fn map_event(event: &Event, page: InputPage) -> Option<InputAction> {
    match event {
        Event::KeyDown {
            keycode: Some(Keycode::X),
            repeat: false,
            ..
        } if matches!(page, InputPage::Settings | InputPage::Workspace) => {
            Some(InputAction::Tertiary)
        }
        Event::KeyDown {
            keycode: Some(keycode),
            repeat: false,
            ..
        } => map_key(*keycode),
        Event::ControllerButtonDown { button, .. } => map_controller_button(*button, page),
        Event::JoyButtonDown { button_idx, .. } => map_joystick_button(*button_idx),
        Event::JoyHatMotion { state, .. } => map_hat(*state),
        _ => None,
    }
}

/// SDL GameController mapping for desktop controllers.
fn map_controller_button(button: Button, page: InputPage) -> Option<InputAction> {
    use InputAction::*;

    match button {
        Button::DPadLeft => Some(NavigateLeft),
        Button::DPadRight => Some(NavigateRight),
        Button::DPadUp => Some(NavigateUp),
        Button::DPadDown => Some(NavigateDown),
        Button::A => Some(Primary),
        Button::B => Some(Secondary),
        Button::X => Some(Tertiary),
        Button::Y if page == InputPage::Performance => Some(ToggleMacroMember),
        Button::Y if page == InputPage::Editor => Some(ToggleScreen),
        Button::Y if page == InputPage::Workspace => Some(WorkspaceRecall),
        Button::Back => Some(ToggleSettings),
        Button::LeftStick => Some(LeftClick),
        Button::RightStick => Some(RightClick),
        Button::Start => Some(ToggleTransport),
        Button::LeftShoulder => Some(ScopePrevious),
        Button::RightShoulder => Some(ScopeNext),
        _ => None,
    }
}

/// Generic joystick fallback for handheld Linux devices that do not expose an
/// SDL GameController mapping.
fn map_joystick_button(button: u8) -> Option<InputAction> {
    use InputAction::*;

    match button {
        0 => Some(Primary),
        1 => Some(Secondary),
        2 => Some(Tertiary),
        3 => Some(ToggleScreen),
        4 => Some(ScopePrevious),
        5 => Some(ScopeNext),
        6 => Some(ToggleSettings),
        7 => Some(ToggleTransport),
        _ => None,
    }
}

fn map_hat(state: HatState) -> Option<InputAction> {
    use InputAction::*;

    match state {
        HatState::Left => Some(NavigateLeft),
        HatState::Right => Some(NavigateRight),
        HatState::Up => Some(NavigateUp),
        HatState::Down => Some(NavigateDown),
        _ => None,
    }
}

/// Screen context for handheld actions, independent of rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputPage {
    Performance,
    Editor,
    Settings,
    Workspace,
}

const MENU_HOLD: Duration = Duration::from_millis(700);
const MACRO_TICK: Duration = Duration::from_millis(50);
const AXIS_DEAD_ZONE: f32 = 0.12;

/// Raw SDL state for one Brick controller. Do not consume its duplicate
/// GameController events. Triggers are digital despite their axis encoding.
pub struct HandheldInput {
    instance: Option<u32>,
    select: bool,
    left_click: u8,
    right_click: Option<u8>,
    menu_down: Option<Instant>,
    menu_consumed: bool,
    axes: [i16; 6],
    triggers: [bool; 2],
    next_macro: Instant,
    hat: HatState,
}

impl HandheldInput {
    pub fn new(instance: Option<u32>, now: Instant) -> Self {
        let left_click = click_button("KHANA_L3_BUTTON").unwrap_or(11);
        let right_click = click_button("KHANA_R3_BUTTON").unwrap_or(12);
        Self {
            instance,
            select: false,
            left_click,
            right_click: (left_click != right_click).then_some(right_click),
            menu_down: None,
            menu_consumed: false,
            axes: [0, 0, i16::MIN, 0, 0, i16::MIN],
            triggers: [false; 2],
            next_macro: now + MACRO_TICK,
            hat: HatState::Centered,
        }
    }

    fn clear(&mut self, now: Instant) {
        *self = Self::new(self.instance, now);
    }

    /// Processes the selected raw joystick and keyboard. Menu gestures never
    /// combine with Select (NextUI may intercept that combination).
    pub fn event(&mut self, event: &Event, page: InputPage, now: Instant) -> Option<InputAction> {
        match event {
            Event::KeyDown {
                keycode: Some(key),
                repeat: false,
                ..
            } if *key == Keycode::X
                && matches!(page, InputPage::Settings | InputPage::Workspace) =>
            {
                Some(InputAction::Tertiary)
            }
            Event::KeyDown {
                keycode: Some(key),
                repeat: false,
                ..
            } => map_key(*key),
            Event::JoyButtonDown {
                which, button_idx, ..
            } if Some(*which) == self.instance => match *button_idx {
                6 => {
                    self.select = true;
                    if self.menu_down.is_some() {
                        self.menu_consumed = true;
                    }
                    None
                }
                8 => {
                    if self.menu_down.is_none() {
                        self.menu_down = Some(now);
                        self.menu_consumed = self.select;
                    }
                    None
                }
                button if button == self.left_click => {
                    Some(if self.select && page == InputPage::Performance {
                        InputAction::CyclePolyphony
                    } else {
                        InputAction::LeftClick
                    })
                }
                button if Some(button) == self.right_click => {
                    Some(if self.select && page == InputPage::Performance {
                        InputAction::Vary
                    } else {
                        InputAction::RightClick
                    })
                }
                button => {
                    if button > 8 {
                        println!("INPUT: unassigned Brick button index {button}");
                    }
                    brick_button(page, button, self.select)
                }
            },
            Event::JoyButtonUp {
                which, button_idx, ..
            } if Some(*which) == self.instance => {
                match *button_idx {
                    6 => self.select = false,
                    8 => {
                        let pressed = self.menu_down.take()?;
                        if self.menu_consumed {
                            self.menu_consumed = false;
                            return None;
                        }
                        return Some(if now.duration_since(pressed) >= MENU_HOLD {
                            InputAction::Quit
                        } else {
                            InputAction::MenuTap
                        });
                    }
                    _ => {}
                }
                None
            }
            Event::JoyAxisMotion {
                which,
                axis_idx,
                value,
                ..
            } if Some(*which) == self.instance => {
                let index = usize::from(*axis_idx);
                if let Some(axis) = self.axes.get_mut(index) {
                    *axis = *value;
                }
                let trigger = match index {
                    2 => 0,
                    5 => 1,
                    _ => return None,
                };
                self.trigger(page, trigger, *value)
            }
            Event::JoyHatMotion { which, state, .. } if Some(*which) == self.instance => {
                if *state == self.hat {
                    return None;
                }
                self.hat = *state;
                map_hat(*state)
            }
            Event::JoyDeviceRemoved { which, .. } if Some(*which) == self.instance => {
                self.instance = None;
                self.clear(now);
                eprintln!("INPUT: Brick joystick removed; restart to reconnect");
                None
            }
            Event::Window {
                win_event: sdl2::event::WindowEvent::FocusLost,
                ..
            } => {
                self.clear(now);
                None
            }
            _ => None,
        }
    }

    fn trigger(&mut self, page: InputPage, trigger: usize, value: i16) -> Option<InputAction> {
        use InputAction::*;
        // Digital press edges with hysteresis.
        if value <= -16_384 {
            self.triggers[trigger] = false;
        }
        if value < 16_384 || self.triggers[trigger] {
            return None;
        }
        self.triggers[trigger] = true;
        match (page, self.select, trigger) {
            (InputPage::Performance, true, 0) => Some(PatternHold),
            (InputPage::Performance, true, _) => Some(CycleLiveGrid),
            (InputPage::Editor, true, _) => None,
            _ => trigger_action(page, trigger),
        }
    }

    pub fn select_held(&self) -> bool {
        self.select
    }

    /// Held Menu exits once, independent of frame rate. Relative stick changes
    /// use a time-based tick; returning to centre leaves macro values intact.
    pub fn tick(&mut self, page: InputPage, now: Instant) -> [Option<InputAction>; 5] {
        let mut actions = [None; 5];
        if !self.menu_consumed
            && self
                .menu_down
                .is_some_and(|start| now.duration_since(start) >= MENU_HOLD)
        {
            self.menu_consumed = true;
            actions[0] = Some(InputAction::Quit);
            return actions;
        }
        if now < self.next_macro {
            return actions;
        }
        self.next_macro = now + MACRO_TICK;
        if page != InputPage::Performance || self.select || self.menu_down.is_some() {
            return actions;
        }
        for (slot, axis, kind, direction) in [
            (1, 0, MacroKind::Density, 1.0),
            (2, 1, MacroKind::Dynamics, -1.0),
            (3, 3, MacroKind::Spread, 1.0),
            (4, 4, MacroKind::Color, -1.0),
        ] {
            let delta = stick_deflection(self.axes[axis]) * direction * 0.05;
            if delta != 0.0 {
                actions[slot] = Some(InputAction::AdjustMacro { kind, delta });
            }
        }
        actions
    }
}

/// Desktop controller events are preferred over duplicate raw events from the
/// same SDL instance. Generic desktop joysticks retain the old mapping.
pub fn desktop_event(
    event: &Event,
    controllers: &[GameController],
    page: InputPage,
) -> Option<InputAction> {
    if let Event::JoyButtonDown { which, .. } | Event::JoyHatMotion { which, .. } = event
        && controllers
            .iter()
            .any(|controller| controller.instance_id() == *which)
    {
        return None;
    }
    map_event(event, page)
}

fn stick_deflection(value: i16) -> f32 {
    let value = (f32::from(value) / 32_767.0).clamp(-1.0, 1.0);
    let magnitude = ((value.abs() - AXIS_DEAD_ZONE) / (1.0 - AXIS_DEAD_ZONE)).max(0.0);
    magnitude * value.signum()
}

fn trigger_action(page: InputPage, trigger: usize) -> Option<InputAction> {
    use InputAction::*;
    match (page, trigger) {
        (InputPage::Performance, 0) => Some(DirectionStep(-1)),
        (InputPage::Performance, 1) => Some(DirectionStep(1)),
        (InputPage::Editor, 0) => Some(EditShorten),
        (InputPage::Editor, 1) => Some(EditExtend),
        _ => None,
    }
}

fn click_button(variable: &str) -> Option<u8> {
    std::env::var(variable)
        .ok()?
        .parse::<u8>()
        .ok()
        .filter(|&index| index > 8)
}

fn brick_button(page: InputPage, button: u8, select: bool) -> Option<InputAction> {
    use InputAction::*;
    if button == 7 && !select {
        return Some(ToggleTransport);
    }
    match (page, select, button) {
        (InputPage::Performance, false, 1) => Some(Primary), // A
        (InputPage::Performance, false, 0) => Some(Secondary), // B
        (InputPage::Performance, false, 3) => Some(Tertiary), // X
        (InputPage::Performance, false, 2) => Some(ToggleMacroMember), // Y
        (InputPage::Performance, false, 4) => Some(ScopePrevious),
        (InputPage::Performance, false, 5) => Some(ScopeNext),
        (InputPage::Performance, true, 1) => Some(ToggleRecord),
        (InputPage::Performance, true, 0) => Some(ArmNextRecording),
        (InputPage::Performance, true, 3) => Some(ToggleSettings),
        (InputPage::Performance, true, 4) => Some(ToggleMute),
        (InputPage::Performance, true, 5) => Some(ToggleSolo),
        (InputPage::Performance, true, 2) => Some(Vary),
        (InputPage::Editor, true, 1) => Some(EditUndo),
        (InputPage::Editor, true, 3) => Some(ToggleSettings),
        (InputPage::Editor, true, 0) => Some(RightClick),
        (InputPage::Editor, false, 1) => Some(Primary),
        (InputPage::Editor, false, 0) => Some(Secondary),
        (InputPage::Editor, false, 3) => Some(EditDuplicate),
        (InputPage::Editor, false, 2) => Some(ToggleScreen),
        (InputPage::Editor, false, 4) => Some(PhrasePrevious),
        (InputPage::Editor, false, 5) => Some(PhraseNext),
        (InputPage::Editor, true, 2) => Some(EditDelete),
        (InputPage::Editor, true, 4) => Some(ZoomOut),
        (InputPage::Editor, true, 5) => Some(ZoomIn),
        (InputPage::Performance | InputPage::Editor | InputPage::Workspace, true, 7) => {
            Some(ToggleWorkspace)
        }
        (InputPage::Workspace, false, 1) => Some(Primary),
        (InputPage::Workspace, false, 0) => Some(Secondary),
        (InputPage::Workspace, false, 3) => Some(Tertiary),
        (InputPage::Workspace, false, 2) => Some(WorkspaceRecall),
        (InputPage::Workspace, false, 4) => Some(ScopePrevious),
        (InputPage::Workspace, false, 5) => Some(ScopeNext),
        (InputPage::Settings, false, 0) => Some(ToggleSettings),
        (InputPage::Settings, false, 1) => Some(Primary),
        (InputPage::Settings, false, 3) => Some(Tertiary),
        (InputPage::Settings, false, 4) => Some(ScopePrevious),
        (InputPage::Settings, false, 5) => Some(ScopeNext),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_brick_labels_and_contexts() {
        assert_eq!(
            brick_button(InputPage::Performance, 1, false),
            Some(InputAction::Primary)
        );
        assert_eq!(
            brick_button(InputPage::Performance, 0, false),
            Some(InputAction::Secondary)
        );
        assert_eq!(
            brick_button(InputPage::Performance, 1, true),
            Some(InputAction::ToggleRecord)
        );
        assert_eq!(
            brick_button(InputPage::Editor, 2, true),
            Some(InputAction::EditDelete)
        );
        assert_eq!(
            trigger_action(InputPage::Editor, 1),
            Some(InputAction::EditExtend)
        );
    }

    #[test]
    fn menu_hold_and_modifier_do_not_emit_a_second_action() {
        let start = Instant::now();
        let mut input = HandheldInput::new(Some(1), start);
        let down = |button_idx| Event::JoyButtonDown {
            timestamp: 0,
            which: 1,
            button_idx,
        };
        let up = |button_idx| Event::JoyButtonUp {
            timestamp: 0,
            which: 1,
            button_idx,
        };
        let page = InputPage::Performance;
        assert_eq!(input.event(&down(8), page, start), None);
        assert_eq!(
            input.tick(page, start + MENU_HOLD)[0],
            Some(InputAction::Quit)
        );
        assert_eq!(input.event(&up(8), page, start + MENU_HOLD), None);
        input.event(&down(6), page, start + MENU_HOLD);
        input.event(&down(8), page, start + MENU_HOLD);
        input.event(&up(6), page, start + MENU_HOLD);
        assert_eq!(input.event(&up(8), page, start + MENU_HOLD), None);
    }

    #[test]
    fn digital_trigger_only_fires_on_press_edge() {
        let now = Instant::now();
        let mut input = HandheldInput::new(Some(1), now);
        let event = |value| Event::JoyAxisMotion {
            timestamp: 0,
            which: 1,
            axis_idx: 2,
            value,
        };
        let page = InputPage::Editor;
        assert_eq!(
            input.event(&event(i16::MAX), page, now),
            Some(InputAction::EditShorten)
        );
        assert_eq!(input.event(&event(i16::MAX), page, now), None);
        assert_eq!(input.event(&event(i16::MIN), page, now), None);
        assert_eq!(
            input.event(&event(i16::MAX), page, now),
            Some(InputAction::EditShorten)
        );
    }

    #[test]
    fn performance_trigger_edges_step_direction_or_control_pattern_and_grid() {
        use InputAction::*;
        let start = Instant::now();
        let mut input = HandheldInput::new(Some(1), start);
        let page = InputPage::Performance;
        assert_eq!(input.trigger(page, 0, i16::MAX), Some(DirectionStep(-1)));
        assert_eq!(input.trigger(page, 0, i16::MAX), None);
        assert_eq!(input.trigger(page, 0, i16::MIN), None);
        assert_eq!(input.trigger(page, 1, i16::MAX), Some(DirectionStep(1)));
        input.trigger(page, 1, i16::MIN);
        input.select = true;
        assert_eq!(input.trigger(page, 0, i16::MAX), Some(PatternHold));
        assert_eq!(input.trigger(page, 1, i16::MAX), Some(CycleLiveGrid));
        assert_eq!(brick_button(page, 4, true), Some(ToggleMute));
        assert_eq!(brick_button(page, 5, true), Some(ToggleSolo));
    }

    #[test]
    fn brick_stick_clicks_and_macro_group_are_page_aware() {
        let now = Instant::now();
        let mut input = HandheldInput {
            left_click: 11,
            right_click: Some(12),
            ..HandheldInput::new(Some(1), now)
        };
        let down = |button_idx| Event::JoyButtonDown {
            timestamp: 0,
            which: 1,
            button_idx,
        };
        assert_eq!(
            input.event(&down(11), InputPage::Performance, now),
            Some(InputAction::LeftClick)
        );
        assert_eq!(
            input.event(&down(12), InputPage::Editor, now),
            Some(InputAction::RightClick)
        );
        assert_eq!(
            input.event(&down(2), InputPage::Performance, now),
            Some(InputAction::ToggleMacroMember)
        );
        input.event(&down(6), InputPage::Performance, now);
        assert_eq!(
            input.event(&down(12), InputPage::Performance, now),
            Some(InputAction::Vary)
        );
        assert_eq!(
            input.event(&down(1), InputPage::Editor, now),
            Some(InputAction::EditUndo)
        );
        assert_eq!(
            map_controller_button(Button::Y, InputPage::Performance),
            Some(InputAction::ToggleMacroMember)
        );
        assert_eq!(
            map_controller_button(Button::Y, InputPage::Editor),
            Some(InputAction::ToggleScreen)
        );
    }

    #[test]
    fn sticks_are_relative_and_have_a_dead_zone() {
        assert_eq!(stick_deflection(0), 0.0);
        assert_eq!(stick_deflection(1_000), 0.0);
        assert_eq!(stick_deflection(i16::MIN), -1.0);
        assert_eq!(stick_deflection(i16::MAX), 1.0);
    }

    #[test]
    fn menu_tap_and_hat_events_are_discrete() {
        let now = Instant::now();
        let mut input = HandheldInput::new(Some(2), now);
        let page = InputPage::Performance;
        let down = Event::JoyButtonDown {
            timestamp: 0,
            which: 2,
            button_idx: 8,
        };
        let up = Event::JoyButtonUp {
            timestamp: 0,
            which: 2,
            button_idx: 8,
        };
        assert_eq!(input.event(&down, page, now), None);
        assert_eq!(
            input.event(&up, page, now + Duration::from_millis(100)),
            Some(InputAction::MenuTap)
        );
        let hat = Event::JoyHatMotion {
            timestamp: 0,
            which: 2,
            hat_idx: 0,
            state: HatState::Right,
        };
        assert_eq!(
            input.event(&hat, page, now),
            Some(InputAction::NavigateRight)
        );
        assert_eq!(input.event(&hat, page, now), None);
    }
}
