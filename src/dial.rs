use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

const EV_REL: u16 = 2;
const REL_HWHEEL: u16 = 6;
const VOL_UP: u8 = 0x01;
const VOL_DOWN: u8 = 0x02;
const INVERT: bool = false;

fn find_dial() -> Option<String> {
    for entry in fs::read_dir("/sys/class/input").ok()? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("event") {
            continue;
        }
        if let Ok(n) = fs::read_to_string(entry.path().join("device/name")) {
            if n.trim() == "rotary" {
                return Some(format!("/dev/input/{name}"));
            }
        }
    }
    None
}

fn tap(hid: &mut File, bit: u8) -> std::io::Result<()> {
    hid.write_all(&[bit])?; // press
    hid.write_all(&[0])?; // release
    Ok(())
}

fn run() {
    loop {
        let Some(path) = find_dial() else {
            thread::sleep(Duration::from_secs(1));
            continue;
        };
        let Ok(mut dial) = File::open(&path) else {
            thread::sleep(Duration::from_secs(1));
            continue;
        };
        let Ok(mut hid) = OpenOptions::new().write(true).open("/dev/hidg0") else {
            thread::sleep(Duration::from_secs(1));
            continue;
        };
        eprintln!("dial: {path} -> /dev/hidg0");

        let mut buf = [0u8; 24];
        loop {
            if dial.read_exact(&mut buf).is_err() {
                break;
            }
            let typ = u16::from_ne_bytes([buf[16], buf[17]]);
            let code = u16::from_ne_bytes([buf[18], buf[19]]);
            let value = i32::from_ne_bytes([buf[20], buf[21], buf[22], buf[23]]);
            if typ != EV_REL || code != REL_HWHEEL || value == 0 {
                continue;
            }
            let up = (value > 0) != INVERT;
            let bit = if up { VOL_UP } else { VOL_DOWN };
            let mut failed = false;
            for _ in 0..value.unsigned_abs().min(5) {
                if tap(&mut hid, bit).is_err() {
                    failed = true;
                    break;
                }
            }
            if failed {
                break; // reopen everything
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
}

pub fn spawn() {
    thread::spawn(run);
}
