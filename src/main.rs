use drm::buffer::{Buffer, DrmFourcc};
use drm::control::Device as ControlDevice;
use drm::Device;
use std::fs::{File, OpenOptions};
use std::os::fd::{AsFd, BorrowedFd};
use std::{thread, time::Duration};

mod dial;

struct Card(File);
impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
impl Device for Card {}
impl ControlDevice for Card {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dial::spawn();
    let path = std::env::args().nth(1).unwrap_or("/dev/dri/card1".into());
    let card = Card(OpenOptions::new().read(true).write(true).open(&path)?);

    let res = card.resource_handles()?;
    let conn = res
        .connectors()
        .iter()
        .filter_map(|h| card.get_connector(*h, true).ok())
        .find(|c| !c.modes().is_empty())
        .ok_or("no connector with modes")?;
    let mode = conn.modes()[0];
    let (w, h) = mode.size();
    println!("extendosd: {} -> {}x{}", path, w, h);

    let crtc = conn
        .current_encoder()
        .and_then(|e| card.get_encoder(e).ok())
        .and_then(|e| e.crtc())
        .or_else(|| res.crtcs().first().copied())
        .ok_or("no crtc")?;

    let mut db = card.create_dumb_buffer((w as u32, h as u32), DrmFourcc::Xrgb8888, 32)?;
    let pitch = db.pitch() as usize;
    {
        let mut map = card.map_dumb_buffer(&mut db)?;
        let buf: &mut [u8] = map.as_mut();
        for y in 0..h as usize {
            for x in 0..w as usize {
                let (r, g, b) = if x < 8 || y < 8 || x >= w as usize - 8 || y >= h as usize - 8 {
                    (255, 255, 255) // white border
                } else if x < 60 && y < 60 {
                    (255, 255, 255) // white square = top-left marker
                } else if y < h as usize / 6 {
                    (220, 40, 40) // red band at top
                } else if y > h as usize * 5 / 6 {
                    (40, 200, 60) // green band at bottom
                } else {
                    (20, 30, 90)
                };
                let o = y * pitch + x * 4;
                let (r, g, b) = corner(y, w as usize - 1 - x, h as usize, w as usize);
                buf[o] = b;
                buf[o + 1] = g;
                buf[o + 2] = r;
                buf[o + 3] = 0;
            }
        }
    }

    let fb = card.add_framebuffer(&db, 24, 32)?;
    card.set_crtc(crtc, Some(fb), (0, 0), &[conn.handle()], Some(mode))?;
    println!("extendosd: modeset ok");

    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

fn corner(x: usize, y: usize, w: usize, h: usize) -> (u8, u8, u8) {
    let s = 100;
    match (y < s, y >= h - s, x < s, x >= w - s) {
        (true, _, true, _) => (255, 255, 255),
        (true, _, _, true) => (255, 0, 0),
        (_, true, true, _) => (0, 255, 0),
        (_, true, _, true) => (0, 0, 255),
        _ => (20, 30, 90),
    }
}
