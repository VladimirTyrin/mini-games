/// How a match actor ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchExit {
    /// The game finished; the hub got `MatchFinished`.
    Finished,
    /// The hub stopped it (lobby closed, shutdown).
    Stopped,
    /// The authority was lost (a bot task panicked): there is no result.
    Aborted,
}
