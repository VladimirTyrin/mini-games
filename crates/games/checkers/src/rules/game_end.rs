use mg_core::Slot;

use super::{DrawReason, WinReason};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameEnd {
    Win { winner: Slot, reason: WinReason },
    Draw(DrawReason),
}

impl GameEnd {
    pub fn winner(self) -> Option<Slot> {
        match self {
            GameEnd::Win { winner, .. } => Some(winner),
            GameEnd::Draw(_) => None,
        }
    }

    pub(crate) fn code(self) -> u8 {
        match self {
            GameEnd::Win { reason, .. } => reason.code(),
            GameEnd::Draw(reason) => reason.code(),
        }
    }
}
