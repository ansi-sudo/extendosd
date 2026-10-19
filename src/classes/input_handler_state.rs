use std::fs::File;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use crate::classes::hardware_button::{Button, HidAction, VOLUME_UP, VOLUME_DOWN};
use crate::classes::input_event::Event;

const EV_KEY: u16 = 1;
const EV_REL: u16 = 2;
const REL_HWHEEL: u16 = 6;

const LONG_PRESS: Duration = Duration::from_millis(500);
const REPEAT_EVERY: Duration = Duration::from_millis(100);
const INVERT: bool = false;
const MAX_TAPS_PER_EVENT: u32 = 5;

pub struct Pressed {
    pub button: Button,
    pub since: Instant,
    pub long_sent: bool,
    pub next_repeat: Instant,
}

pub struct Handler {
    media: File,    // /dev/hidg0
    keyboard: File, // /dev/hidg1
    pressed: Vec<Pressed>,
}

impl Handler {
    pub fn new(media: File, keyboard: File) -> Handler {
        Handler { media, keyboard, pressed: Vec::new() }
    }

    /// Send a press (down = true) or release (down = false) to the host.
    fn send_hid(&mut self, action: HidAction, down: bool) -> io::Result<()> {
        match action {
            HidAction::Media(bit) => self.media.write_all(&[if down { bit } else { 0 }]),
            HidAction::Key { modifier, usage } => {
                let (m, k) = if down { (modifier, usage) } else { (0, 0) };
                self.keyboard.write_all(&[m, 0, k, 0, 0, 0, 0, 0])
            }
        }
    }

    /// Dial event. `value` is +1 / -1 per click.
    pub fn on_rotary(&mut self, typ: u16, code: u16, value: i32) -> io::Result<Vec<Event>> {
        if typ != EV_REL || code != REL_HWHEEL || value == 0 {
            return Ok(Vec::new());
        }
        let clockwise = value > 0;
        let steps = value.unsigned_abs();

        let bit = if clockwise != INVERT { VOLUME_UP } else { VOLUME_DOWN };
        for _ in 0..steps.min(MAX_TAPS_PER_EVENT) {
            self.media.write_all(&[bit])?; // press
            self.media.write_all(&[0])?; // release
        }
        Ok(vec![Event::DialTurn { clockwise, steps }])
    }

    /// Button event. value: 1 = pressed, 0 = released.
    pub fn on_buttons(&mut self, typ: u16, code: u16, value: i32, now: Instant) -> io::Result<Vec<Event>> {
        let mut events = Vec::new();
        if typ != EV_KEY {
            return Ok(events);
        }
        let Some(button) = Button::from_code(code) else { return Ok(events) };
        let known = self.pressed.iter().position(|p| p.button == button);

        match (value, known) {
            // Pressed
            (1, None) => {
                self.pressed.push(Pressed {
                    button,
                    since: now,
                    long_sent: false,
                    next_repeat: now + LONG_PRESS + REPEAT_EVERY,
                });
                events.push(Event::ButtonDown(button));
                self.send_hid(button.hid_action(), true)?;
            }
            // Released
            (0, Some(i)) => {
                let p = self.pressed.remove(i);
                events.push(Event::ButtonUp { button, held_for: now - p.since });
                if !p.long_sent {
                    events.push(Event::Click(button));
                }
                self.send_hid(button.hid_action(), false)?;
            }
            _ => {}
        }
        Ok(events)
    }

    /// call whenver the clock may have advanced
    pub fn tick(&mut self, now: Instant) -> Vec<Event> {
        let mut events = Vec::new();
        for p in self.pressed.iter_mut() {
            if !p.long_sent {
                if now >= p.since + LONG_PRESS {
                    p.long_sent = true;
                    p.next_repeat = now + REPEAT_EVERY;
                    events.push(Event::LongPress(p.button));
                }
            } else if now >= p.next_repeat {
                p.next_repeat = now + REPEAT_EVERY;
                events.push(Event::Held { button: p.button, held_for: now - p.since });
            }
        }
        events
    }

    /// how long poll() may sleep before tick() has work
    pub fn next_timeout(&self, now: Instant) -> Option<Duration> {
        self.pressed
            .iter()
            .map(|p| {
                let deadline = if p.long_sent { p.next_repeat } else { p.since + LONG_PRESS };
                deadline.saturating_duration_since(now)
            })
            .min()
    }
}
