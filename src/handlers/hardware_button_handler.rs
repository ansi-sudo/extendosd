use crate::classes::Event;

pub fn handle_hardware_event(event: &Event) {
    println!("hardware event: {event:?}");
}
