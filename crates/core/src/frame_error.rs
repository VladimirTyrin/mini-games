use std::fmt;

use crate::slot::Slot;
use crate::tick::Tick;

/// Any of these means the receiver is out of sync and must rebuild from the full log.
///
/// `E` is why an input was rejected: [`crate::Match`] gives the game's
/// `CommandError<G::Refusal>`; a host that erases the game type maps it with
/// [`FrameError::map_rejection`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError<E> {
    UnexpectedTick {
        expected: Tick,
        got: Tick,
    },
    Rejected {
        tick: Tick,
        input_index: usize,
        error: E,
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

impl<E> FrameError<E> {
    pub fn map_rejection<F>(self, map: impl FnOnce(E) -> F) -> FrameError<F> {
        match self {
            Self::UnexpectedTick { expected, got } => FrameError::UnexpectedTick { expected, got },
            Self::Rejected {
                tick,
                input_index,
                error,
            } => FrameError::Rejected {
                tick,
                input_index,
                error: map(error),
            },
            Self::HashMismatch {
                tick,
                expected,
                actual,
            } => FrameError::HashMismatch {
                tick,
                expected,
                actual,
            },
            Self::UnknownSlot { tick, slot } => FrameError::UnknownSlot { tick, slot },
            Self::MissingFrameAfterFinish { tick } => FrameError::MissingFrameAfterFinish { tick },
        }
    }
}

impl<E: fmt::Display> fmt::Display for FrameError<E> {
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

impl<E: fmt::Debug + fmt::Display> std::error::Error for FrameError<E> {}
