use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::thread;
use std::time::Duration;

const EV_KEY: u16 = 1;
const EV_REL: u16 = 2;
const REL_HWHEEL: u16 = 6;

const VOL_UP: u8 = 1 << 0;
const VOL_DOWN: u8 = 1 << 1;
const MUTE: u8 = 1 << 2;
const PLAY_PAUSE: u8 = 1 << 3;
const NEXT: u8 = 1 << 4;
const PREV: u8 = 1 << 5;
const STOP: u8 = 1 << 6;
const PLAY: u8 = 1 << 7;

// Flip to true if clockwise lowers the volume (keep whatever you had in dial.rs).
const INVERT: bool = false;

// Evdev key code -> HID bit. Edit this table to remap; no reflash needed.
fn key_action(code: u16) -> Option<u8> {
    match code {
        2 => Some(PREV),       // button 1
        3 => Some(PLAY_PAUSE), // button 2
        4 => Some(NEXT),       // button 3
        5 => Some(MUTE),       // button 4
        28 => Some(PLAY_PAUSE), // dial press
        50 => Some(STOP),      // rightmost top button (KEY_M)
        1 => Some(PLAY),       // button under the dial (KEY_ESC)
        _ => None,
    }
}

fn find_device(name: &str) -> Option<String> {
    for entry in fs::read_dir("/sys/class/input").ok()? {
        let Ok(entry) = entry else { continue };
        let ev = entry.file_name().to_string_lossy().into_owned();
        if !ev.starts_with("event") {
            continue;
        }
        if let Ok(n) = fs::read_to_string(entry.path().join("device/name")) {
            if n.trim() == name {
                return Some(format!("/dev/input/{ev}"));
            }
        }
    }
    None
}

fn tap(hid: &mut File, bit: u8) -> io::Result<()> {
    hid.write_all(&[bit])?;
    hid.write_all(&[0])
}

fn on_rotary(hid: &mut File, typ: u16, code: u16, value: i32) -> io::Result<()> {
    if typ != EV_REL || code != REL_HWHEEL || value == 0 {
        return Ok(());
    }
    let bit = if (value > 0) != INVERT { VOL_UP } else { VOL_DOWN };
    for _ in 0..value.unsigned_abs().min(5) {
        tap(hid, bit)?;
    }
    Ok(())
}

fn on_buttons(hid: &mut File, typ: u16, code: u16, value: i32) -> io::Result<()> {
    if typ != EV_KEY || value == 2 {
        return Ok(());
    }
    let Some(bit) = key_action(code) else { return Ok(()) };
    hid.write_all(&[if value == 1 { bit } else { 0 }])
}

type Handler = fn(&mut File, u16, u16, i32) -> io::Result<()>;

fn run(name: &'static str, handler: Handler) {
    loop {
        let Some(path) = find_device(name) else {
            thread::sleep(Duration::from_secs(1));
            continue;
        };
        let Ok(mut dev) = File::open(&path) else {
            thread::sleep(Duration::from_secs(1));
            continue;
        };
        let Ok(mut hid) = OpenOptions::new().write(true).open("/dev/hidg0") else {
            thread::sleep(Duration::from_secs(1));
            continue;
        };
        eprintln!("input: {name} ({path}) -> /dev/hidg0");

        // 64-bit struct input_event: i64 sec, i64 usec, u16 type, u16 code, i32 value
        let mut buf = [0u8; 24];
        while dev.read_exact(&mut buf).is_ok() {
            let typ = u16::from_ne_bytes([buf[16], buf[17]]);
            let code = u16::from_ne_bytes([buf[18], buf[19]]);
            let value = i32::from_ne_bytes([buf[20], buf[21], buf[22], buf[23]]);
            if handler(&mut hid, typ, code, value).is_err() {
                break;
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
}

pub fn spawn() {
    thread::spawn(|| run("rotary", on_rotary));
    thread::spawn(|| run("gpio-keys-polled", on_buttons));
}
