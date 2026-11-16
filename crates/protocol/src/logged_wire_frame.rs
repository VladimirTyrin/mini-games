use mg_core::{LoggedFrame, Tick};

use crate::proto;

impl LoggedFrame for proto::TickFrame {
    fn tick(&self) -> Tick {
        Tick::new(self.tick)
    }
}
