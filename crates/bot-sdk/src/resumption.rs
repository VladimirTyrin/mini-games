/// What a reconnect restored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resumption {
    /// The server gave back the lobby seat (and the match seat, if one was running).
    Resumed,
    /// Signed in again, but the earlier session was gone (grace period over, server restart).
    NewSession,
}
