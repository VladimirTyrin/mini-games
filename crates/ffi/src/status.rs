/// Result of every exported function. Values are part of the ABI: never renumber.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MgStatus {
    Ok = 0,
    NullArgument = 1,
    /// Undecodable bytes, invalid settings or header, unknown game, slot out of range.
    InvalidArgument = 2,
    /// The game refused the command (not your turn, illegal move, game over); nothing changed.
    Rejected = 3,
    /// A frame or catch-up does not apply: the mirror is out of sync; rebuild it from a catch-up.
    Desync = 4,
    /// A bug in the library (including a caught panic); free the handle involved.
    Internal = 5,
    /// The call does not fit the handle (`mg_mirror_advance_to` without a replay log).
    InvalidState = 6,
}
