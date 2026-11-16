use crate::logged_frame::LoggedFrame;
use crate::tick::Tick;

#[derive(Debug)]
pub enum CompactLogEntry<'a, F> {
    Logged(&'a F),
    /// The frame was omitted from the log: it had no inputs and no hash.
    Empty(Tick),
}

impl<F: LoggedFrame> CompactLogEntry<'_, F> {
    pub fn tick(&self) -> Tick {
        match self {
            Self::Logged(frame) => frame.tick(),
            Self::Empty(tick) => *tick,
        }
    }
}
