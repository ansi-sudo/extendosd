use drm::buffer::{Buffer, DrmFourcc};
use drm::control::Device as ControlDevice;
use std::fs::OpenOptions;
use crate::classes::Card;

/// initializes the DRM display device, creates a dumb buffer, draws a test pattern,
/// adds framebuffer
pub fn init_display(path: &str) -> Result<Card, Box<dyn std::error::Error>> {
    let card = Card::new(OpenOptions::new().read(true).write(true).open(path)?);

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

    Ok(card)
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
