mod classes;
mod handlers;
mod helpers;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let display_path = std::env::args().nth(1).unwrap_or_else(|| "/dev/dri/card1".into());
    let _card = helpers::init_display(&display_path)?;

    // Run EVIOCGRAB "interrupts"
    helpers::run_event_loop(handlers::handle_hardware_event)?;

    Ok(())
}
