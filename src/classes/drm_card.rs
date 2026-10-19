use drm::control::Device as ControlDevice;
use drm::Device;
use std::fs::File;
use std::os::fd::{AsFd, BorrowedFd};

pub struct Card(File);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for Card {}
impl ControlDevice for Card {}

impl Card {
    pub fn new(file: File) -> Self {
        Self(file)
    }
}
