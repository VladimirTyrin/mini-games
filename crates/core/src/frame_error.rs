use std::fmt;

use crate::command_error::CommandError;
use crate::slot::Slot;
use crate::tick::Tick;

/// Any of these means the receiver is out of sync and must rebuild from the full log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    UnexpectedTick {
        expected: Tick,
        got: Tick,
    },
    Rejected {
        tick: Tick,
        input_index: usize,
        error: CommandError,
    },
    HashMismatch {
        tick: Tick,
        expected: u64,
        actual: u64,
    },
    UnknownSlot {
        tick: Tick,
        slot: Slot,
    },
    /// A compact log omits a frame after the game ended.
    MissingFrameAfterFinish {
        tick: Tick,
    },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedTick { expected, got } => {
                write!(f, "expected frame {expected}, got {got}")
            }
            Self::Rejected {
                tick,
                input_index,
                error,
            } => {
                write!(f, "frame {tick}: input #{input_index} rejected: {error}")
            }
            Self::HashMismatch {
                tick,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "frame {tick}: state hash {actual:016x} != authority {expected:016x}"
                )
            }
            Self::UnknownSlot { tick, slot } => write!(f, "frame {tick}: unknown seat {slot}"),
            Self::MissingFrameAfterFinish { tick } => {
                write!(f, "frame {tick} is missing after the game ended")
            }
        }
    }
}

impl std::error::Error for FrameError {}
