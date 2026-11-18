use std::time::Duration;

/// Abuse and resource limits. Fixed rather than configurable: they protect a 1 vCPU / 1 GB host.
pub(crate) struct Limits;

impl Limits {
    pub(crate) const MAX_CONNECTIONS: usize = 2_000;
    pub(crate) const MAX_LOBBIES: usize = 300;
    pub(crate) const MAX_MESSAGE_BYTES: usize = 64 * 1024;
    /// What the socket reads at all. A message between this and `MAX_MESSAGE_BYTES` is still
    /// refused with an error; a bigger one closes the connection unread. Every socket, signed
    /// in or not, so 2000 connections hold at most ~190 MiB of incoming messages.
    pub(crate) const MAX_SOCKET_MESSAGE_BYTES: usize = 96 * 1024;
    /// A shared replay in all; it arrives in parts (`ShareReplay.more_parts`).
    pub(crate) const MAX_REPLAY_BYTES: usize = 4 * 1024 * 1024;
    /// Bytes of half-uploaded replays held across all connections.
    pub(crate) const REPLAY_UPLOAD_BUDGET_BYTES: usize = 32 * 1024 * 1024;
    pub(crate) const MAX_CHAT_CHARS: usize = 300;
    pub(crate) const MAX_LOBBY_NAME_CHARS: usize = 48;
    pub(crate) const MAX_OBSERVERS: usize = 32;

    /// Outbound queue per connection, by count and by bytes. A client that falls this far
    /// behind is disconnected (it can resume) instead of slowing anyone else down. One message
    /// larger than the byte cap (a replay) is accepted when the queue is empty.
    pub(crate) const OUTBOUND_QUEUE: usize = 512;
    pub(crate) const OUTBOUND_BYTES: usize = 8 * 1024 * 1024;
    /// A socket write that takes longer than this means the client stopped reading.
    pub(crate) const SEND_TIMEOUT: Duration = Duration::from_secs(10);

    /// General messages: burst / sustained rate.
    pub(crate) const MESSAGE_BURST: u32 = 60;
    pub(crate) const MESSAGE_REFILL: Duration = Duration::from_millis(25);
    /// A rate-limited frame up to this size is still decoded: a ping is answered, a request is
    /// refused with its id.
    pub(crate) const RATE_LIMITED_PEEK_BYTES: usize = 512;
    pub(crate) const CHAT_BURST: u32 = 5;
    pub(crate) const CHAT_REFILL: Duration = Duration::from_secs(1);

    /// Per-session cooldowns for requests whose answer is expensive (a whole match log or
    /// replay).
    pub(crate) const REPLAY_DOWNLOAD_COOLDOWN: Duration = Duration::from_secs(2);
    pub(crate) const CATCH_UP_COOLDOWN: Duration = Duration::from_secs(1);
    /// Each test push is an outgoing request to a push service.
    pub(crate) const TEST_PUSH_COOLDOWN: Duration = Duration::from_secs(10);

    /// Concurrent Google ID token verifications (each may block a pool thread on a key fetch).
    pub(crate) const GOOGLE_VERIFICATIONS: usize = 4;

    /// How long shutdown waits for the hub and then for connections to flush `ShuttingDown`.
    pub(crate) const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

    /// Finished matches whose replay can still be requested, per lobby.
    pub(crate) const REPLAYS_PER_LOBBY: usize = 3;

    /// Commands that may be shed (submits, catch-up, votes) queued per match actor; a
    /// turn-based actor busy with a bot search may fall behind. Lifecycle commands have their
    /// own unbounded queue.
    pub(crate) const MATCH_REQUEST_QUEUE: usize = 256;
}
