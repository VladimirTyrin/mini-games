use std::time::Duration;

/// Abuse and resource limits. Fixed rather than configurable: they protect a 1 vCPU / 1 GB host.
pub(crate) struct Limits;

impl Limits {
    pub(crate) const MAX_CONNECTIONS: usize = 2_000;
    pub(crate) const MAX_LOBBIES: usize = 300;
    pub(crate) const MAX_MESSAGE_BYTES: usize = 64 * 1024;
    /// Replays are shared inside a JSON message (base64), so the frame limit is larger.
    pub(crate) const MAX_REPLAY_BYTES: usize = 4 * 1024 * 1024;
    pub(crate) const MAX_REPLAY_MESSAGE_BYTES: usize = Self::MAX_REPLAY_BYTES * 4 / 3 + 4096;
    pub(crate) const MAX_CHAT_CHARS: usize = 300;
    pub(crate) const MAX_LOBBY_NAME_CHARS: usize = 48;
    pub(crate) const MAX_OBSERVERS: usize = 32;

    /// Outbound queue per connection. A client that falls this far behind is disconnected
    /// (it can resume) instead of slowing anyone else down.
    pub(crate) const OUTBOUND_QUEUE: usize = 512;

    /// General messages: burst / sustained rate.
    pub(crate) const MESSAGE_BURST: u32 = 60;
    pub(crate) const MESSAGE_REFILL: Duration = Duration::from_millis(25);
    pub(crate) const CHAT_BURST: u32 = 5;
    pub(crate) const CHAT_REFILL: Duration = Duration::from_secs(1);

    /// Clients ping every 15 s; silence for longer than this closes the connection.
    pub(crate) const IDLE_TIMEOUT: Duration = Duration::from_secs(45);
    /// How long a disconnected session keeps its lobby seat.
    pub(crate) const RESUME_GRACE: Duration = Duration::from_secs(60);
    pub(crate) const OPEN_LOBBY_IDLE_TIMEOUT: Duration = Duration::from_secs(60 * 60);
    pub(crate) const SHUTDOWN_GRACE: Duration = Duration::from_secs(3);

    pub(crate) const DISCONNECT_BUDGET: Duration = Duration::from_secs(60);
    pub(crate) const DROP_VOTE_AFTER: Duration = Duration::from_secs(20);
    pub(crate) const RESUME_COUNTDOWN: Duration = Duration::from_secs(3);

    /// Finished matches whose replay can still be requested, per lobby.
    pub(crate) const REPLAYS_PER_LOBBY: usize = 3;
}
