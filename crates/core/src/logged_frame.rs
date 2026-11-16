use crate::frame::Frame;
use crate::tick::Tick;

/// A frame stored in a [`crate::CompactLog`]: domain frames here, wire frames in `mg-protocol`.
pub trait LoggedFrame {
    fn tick(&self) -> Tick;
}

impl<C, W> LoggedFrame for Frame<C, W> {
    fn tick(&self) -> Tick {
        self.tick
    }
}
