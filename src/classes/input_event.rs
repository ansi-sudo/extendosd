use std::time::Duration;
use crate::classes::hardware_button::Button;

/// events for the app
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    ButtonDown(Button),
    ButtonUp { button: Button, held_for: Duration },
    Click(Button),
    LongPress(Button),
    Held { button: Button, held_for: Duration },
    DialTurn { clockwise: bool, steps: u32 },
}
