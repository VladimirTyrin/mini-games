/// The match actor is too far behind to queue another request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the match is busy, try again")]
pub(crate) struct MatchBusy;
