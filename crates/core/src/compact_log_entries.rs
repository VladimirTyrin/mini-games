use crate::compact_log_entry::CompactLogEntry;
use crate::logged_frame::LoggedFrame;
use crate::tick::Tick;

/// Gap-free walk over a [`crate::CompactLog`]; omitted frames come out as
/// [`CompactLogEntry::Empty`] without being allocated.
#[derive(Debug)]
pub struct CompactLogEntries<'a, F> {
    /// Logged frames at or after `next`, in tick order.
    frames: &'a [F],
    next: Tick,
    end: Tick,
}

impl<'a, F> CompactLogEntries<'a, F> {
    pub(crate) fn new(frames: &'a [F], next: Tick, end: Tick) -> Self {
        Self { frames, next, end }
    }
}

impl<'a, F: LoggedFrame> Iterator for CompactLogEntries<'a, F> {
    type Item = CompactLogEntry<'a, F>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }
        let tick = self.next;
        self.next = tick.next();
        match self.frames.split_first() {
            Some((frame, rest)) if frame.tick() == tick => {
                self.frames = rest;
                Some(CompactLogEntry::Logged(frame))
            }
            _ => Some(CompactLogEntry::Empty(tick)),
        }
    }
}
