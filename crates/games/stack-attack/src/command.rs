use crate::direction::Direction;

/// Commands queued per worker at most; a full queue answers [`mg_core::CommandError::Busy`].
/// Each tick executes at most one queued command per worker (A3).
pub const MAX_QUEUED_COMMANDS: usize = 3;

/// Legality depends on the field when the command executes (in a later `step`), so a queued
/// command that turns out to be blocked is skipped without effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    /// One cell sideways; a box at feet level is pushed one cell further if that cell is free.
    Move(Direction),
    /// Only from the ground; lifts the worker one cell for one tick.
    Jump,
}
