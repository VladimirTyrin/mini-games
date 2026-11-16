use std::time::Duration;

/// Server timeouts and grace periods. Production uses [`Timing::default`]; tests shorten them
/// so a scenario like "the budget runs out" finishes in milliseconds instead of a minute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timing {
    /// A connection that has not signed in within this time is closed.
    pub hello_timeout: Duration,
    /// Clients ping every 15 s; silence for longer than this closes the connection.
    pub idle_timeout: Duration,
    /// How long a disconnected session keeps its lobby seat.
    pub resume_grace: Duration,
    /// Per seat and match: how long the match waits for a disconnected human in total.
    pub disconnect_budget: Duration,
    /// Other players may vote to drop a disconnected seat after this long.
    pub drop_vote_after: Duration,
    pub resume_countdown: Duration,
    /// Lobby list pushes are coalesced over this window.
    pub lobby_list_coalesce: Duration,
    /// Lobbies outside a match close after this long without activity.
    pub idle_lobby_timeout: Duration,
    pub housekeeping_every: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            hello_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(45),
            resume_grace: Duration::from_secs(60),
            disconnect_budget: Duration::from_secs(60),
            drop_vote_after: Duration::from_secs(20),
            resume_countdown: Duration::from_secs(3),
            lobby_list_coalesce: Duration::from_millis(250),
            idle_lobby_timeout: Duration::from_secs(60 * 60),
            housekeeping_every: Duration::from_secs(60),
        }
    }
}
