use crate::clock::Clock;

/// How often sealed frames carry a state hash. The final frame always carries one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HashPolicy {
    every_frames: u64,
}

impl HashPolicy {
    pub fn every(frames: u64) -> Self {
        Self {
            every_frames: frames.max(1),
        }
    }

    /// Turn-based: every move (frames are rare). Real-time: every 10th tick (~2 s at 200 ms).
    pub fn for_clock(clock: Clock) -> Self {
        match clock {
            Clock::TurnBased => Self::every(1),
            Clock::Realtime { .. } => Self::every(10),
        }
    }

    pub fn every_frames(self) -> u64 {
        self.every_frames
    }
}
