use crate::frame::Frame;
use crate::tick::Tick;

/// Restores the gap-free frame sequence `0..end` from a log that omits empty frames.
pub fn expand_compact_log<C: Clone, W: Clone>(
    compact: &[Frame<C, W>],
    end: Tick,
) -> Vec<Frame<C, W>> {
    let mut frames = Vec::new();
    let mut tick = Tick::ZERO;
    for frame in compact {
        while tick < frame.tick && tick < end {
            frames.push(Frame::empty(tick));
            tick = tick.next();
        }
        if frame.tick >= end {
            break;
        }
        frames.push(frame.clone());
        tick = frame.tick.next();
    }
    while tick < end {
        frames.push(Frame::empty(tick));
        tick = tick.next();
    }
    frames
}
