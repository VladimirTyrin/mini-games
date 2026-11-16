use crate::model::Direction;

/// Turns queued per snake at most; each tick consumes one, a full queue answers
/// [`mg_core::CommandError::Busy`].
pub const INPUT_BUFFER: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    /// Validated against the last queued direction (the current one when nothing is queued):
    /// reversing or repeating it is illegal.
    Turn(Direction),
}
