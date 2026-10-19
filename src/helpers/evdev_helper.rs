use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::time::Instant;

use crate::classes::{Event, Handler};

const EVIOCGRAB: libc::c_ulong = 0x40044590; // _IOW('E', 0x90, int)

/// Exclusive access to hardware inputs
fn grab_device(file: &File) -> io::Result<()> {
    let res = unsafe { libc::ioctl(file.as_raw_fd(), EVIOCGRAB as _, 1) };
    if res < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Finds /dev/input/eventN (not case-sensitive).
fn find_device(candidates: &[&str]) -> Option<(String, String)> {
    let mut found_devices = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/input") {
        for entry in entries.flatten() {
            let ev = entry.file_name().to_string_lossy().into_owned();
            if !ev.starts_with("event") {
                continue;
            }
            if let Ok(n) = fs::read_to_string(entry.path().join("device/name")) {
                let name = n.trim().to_string();
                let path = format!("/dev/input/{ev}");
                found_devices.push((name.clone(), path.clone()));
                if candidates.iter().any(|&c| c.eq_ignore_ascii_case(&name)) {
                    return Some((name, path));
                }
            }
        }
    }
    eprintln!("Available input devices in /sys/class/input: {:?}", found_devices);
    None
}

/// 24-byte 64-bit `input_event` - returns (type, code, value).
fn read_raw(mut dev: &File) -> io::Result<(u16, u16, i32)> {
    let mut buf = [0u8; 24];
    dev.read_exact(&mut buf)?;
    Ok((
        u16::from_ne_bytes([buf[16], buf[17]]),
        u16::from_ne_bytes([buf[18], buf[19]]),
        i32::from_ne_bytes([buf[20], buf[21], buf[22], buf[23]]),
    ))
}

/// Single-threaded loop: sleeps in poll() until an input event arrives or a
/// hold timer is due. Every `Event` is passed to `on_event`.
pub fn run_event_loop(mut on_event: impl FnMut(&Event)) -> Result<(), Box<dyn std::error::Error>> {
    let rotary_candidates = &["rotary", "rotary-encoder"];
    let (rotary_name, rotary_path) = find_device(rotary_candidates)
        .ok_or_else(|| format!("Rotary input device not found (tried {:?})", rotary_candidates))?;

    let buttons_candidates = &["gpio-keys", "gpio-keys-polled", "gpio_keys"];
    let (buttons_name, buttons_path) = find_device(buttons_candidates)
        .ok_or_else(|| format!("Buttons input device not found (tried {:?})", buttons_candidates))?;

    let rotary_dev = File::open(&rotary_path)?;
    let buttons_dev = File::open(&buttons_path)?;
    grab_device(&rotary_dev)?;
    grab_device(&buttons_dev)?;
    println!("input: grabbed '{}' ({}) and '{}' ({})", rotary_name, rotary_path, buttons_name, buttons_path);

    let media = OpenOptions::new().write(true).open("/dev/hidg0")?;
    let keyboard = OpenOptions::new().write(true).open("/dev/hidg1")?;
    println!("input: connected to /dev/hidg0 and /dev/hidg1");

    let mut handler = Handler::new(media, keyboard);

    let mut fds = [
        libc::pollfd { fd: rotary_dev.as_raw_fd(), events: libc::POLLIN, revents: 0 },
        libc::pollfd { fd: buttons_dev.as_raw_fd(), events: libc::POLLIN, revents: 0 },
    ];

    loop {
        let timeout_ms: i32 = match handler.next_timeout(Instant::now()) {
            Some(d) => (d.as_millis() as i32).saturating_add(1),
            None => -1,
        };
        let ret = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout_ms) };
        if ret < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err.into());
        }

        let now = Instant::now();
        let mut events: Vec<Event> = Vec::new();

        if (fds[0].revents & libc::POLLIN) != 0 {
            match read_raw(&rotary_dev).and_then(|(t, c, v)| handler.on_rotary(t, c, v)) {
                Ok(mut e) => events.append(&mut e),
                Err(err) => eprintln!("input: rotary error: {err}"),
            }
        }
        if (fds[1].revents & libc::POLLIN) != 0 {
            match read_raw(&buttons_dev).and_then(|(t, c, v)| handler.on_buttons(t, c, v, now)) {
                Ok(mut e) => events.append(&mut e),
                Err(err) => eprintln!("input: buttons error: {err}"),
            }
        }
        events.extend(handler.tick(now));

        for event in &events {
            on_event(event);
        }
    }
}
