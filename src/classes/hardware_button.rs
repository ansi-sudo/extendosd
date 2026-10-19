const KEY_1: u16 = 2;
const KEY_2: u16 = 3;
const KEY_3: u16 = 4;
const KEY_4: u16 = 5;
const KEY_ENTER: u16 = 28; // dial press
const KEY_ESC: u16 = 1; // under dial
const KEY_M: u16 = 50; // rightmost top button

pub const VOLUME_UP: u8 = 1 << 0;
pub const VOLUME_DOWN: u8 = 1 << 1;
pub const MUTE: u8 = 1 << 2;
pub const PLAY_PAUSE: u8 = 1 << 3;
pub const NEXT: u8 = 1 << 4;
pub const PREVIOUS: u8 = 1 << 5;
pub const STOP: u8 = 1 << 6;

#[derive(Clone, Copy)]
pub enum HidAction {
    Media(u8),
    Key { modifier: u8, usage: u8 },
}

/// physical buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Preset1,
    Preset2,
    Preset3,
    Preset4,
    DialPress,
    TopRight,
    UnderDial,
}

impl Button {
    pub fn from_code(code: u16) -> Option<Button> {
        match code {
            KEY_1 => Some(Button::Preset1),
            KEY_2 => Some(Button::Preset2),
            KEY_3 => Some(Button::Preset3),
            KEY_4 => Some(Button::Preset4),
            KEY_ENTER => Some(Button::DialPress),
            KEY_M => Some(Button::TopRight),
            KEY_ESC => Some(Button::UnderDial),
            _ => None,
        }
    }

    /// What this button sends to the host
    pub fn hid_action(self) -> HidAction {
        match self {
            Button::Preset1 => HidAction::Media(PREVIOUS),
            Button::Preset2 => HidAction::Media(PLAY_PAUSE),
            Button::Preset3 => HidAction::Media(NEXT),
            Button::Preset4 => HidAction::Media(MUTE),
            Button::DialPress => HidAction::Media(PLAY_PAUSE),
            Button::TopRight => HidAction::Media(STOP),
            Button::UnderDial => HidAction::Key { modifier: 0, usage: 0x0E }, // 'k'
        }
    }
}
