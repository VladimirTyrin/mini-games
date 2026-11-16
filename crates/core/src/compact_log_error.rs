use std::fmt;

use crate::tick::Tick;

/// A compact log (catch-up or replay file) that no authority could have produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompactLogError {
    TicksNotIncreasing {
        previous: Tick,
        next: Tick,
    },
    FrameAtOrAfterEnd {
        tick: Tick,
        end: Tick,
    },
    /// More omitted frames in a row than the hash policy allows.
    GapTooLong {
        from: Tick,
        to: Tick,
        max_omitted_frames: u64,
    },
}

impl fmt::Display for CompactLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TicksNotIncreasing { previous, next } => {
                write!(f, "frame {next} follows frame {previous}")
            }
            Self::FrameAtOrAfterEnd { tick, end } => {
                write!(f, "frame {tick} is not before the log end {end}")
            }
            Self::GapTooLong {
                from,
                to,
                max_omitted_frames,
            } => write!(
                f,
                "frames {from}..{to} are omitted, at most {max_omitted_frames} may be"
            ),
        }
    }
}

impl std::error::Error for CompactLogError {}
