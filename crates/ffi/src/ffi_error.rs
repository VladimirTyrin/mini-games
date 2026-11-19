use mg_protocol::{WireError, proto};
use mg_registry::{ApplyError, CreateError, ReplayError, SubmitError};

use crate::status::MgStatus;

#[derive(Debug, thiserror::Error)]
pub(crate) enum FfiError {
    #[error("`{0}` is null")]
    NullArgument(&'static str),
    #[error("`{0}` is longer than isize::MAX bytes")]
    TooLong(&'static str),
    #[error("`{what}` is not a valid protobuf message: {source}")]
    Decode {
        what: &'static str,
        #[source]
        source: prost::DecodeError,
    },
    #[error("game id is not UTF-8: {0}")]
    GameIdEncoding(#[source] std::str::Utf8Error),
    #[error("unknown game `{0}`")]
    UnknownGame(String),
    #[error("invalid argument: {0}")]
    Wire(#[from] WireError),
    #[error("cannot set up the match: {0}")]
    Create(#[from] CreateError),
    #[error("invalid replay file: {0}")]
    Replay(#[from] ReplayError),
    #[error("command rejected: {0}")]
    Submit(#[from] SubmitError),
    #[error("out of sync: {0}")]
    Apply(#[from] ApplyError),
    #[error("viewer slot {0} is out of range")]
    ViewerSlot(i32),
    #[error("this mirror has no replay log")]
    NoReplayLog,
    #[error("internal error: the authority's own frame did not apply to its mirror: {0}")]
    OwnFrameRejected(#[source] ApplyError),
    #[error("internal error: replaying the authority history for a clone failed: {0}")]
    CloneDiverged(#[source] SubmitError),
    #[error("internal error: panic: {0}")]
    Panic(String),
}

impl FfiError {
    /// Why the game refused a command, for `mg_last_refusal`.
    pub(crate) fn refusal(&self) -> Option<&proto::Refusal> {
        match self {
            Self::Submit(SubmitError::Rejected(rejection)) => Some(&rejection.refusal),
            _ => None,
        }
    }

    pub(crate) fn status(&self) -> MgStatus {
        match self {
            Self::NullArgument(_) => MgStatus::NullArgument,
            Self::TooLong(_)
            | Self::Decode { .. }
            | Self::GameIdEncoding(_)
            | Self::UnknownGame(_)
            | Self::Wire(_)
            | Self::Create(_)
            | Self::Replay(_)
            | Self::ViewerSlot(_)
            | Self::Submit(SubmitError::Wire(_)) => MgStatus::InvalidArgument,
            Self::Submit(SubmitError::Rejected(_)) => MgStatus::Rejected,
            Self::Apply(_) => MgStatus::Desync,
            Self::NoReplayLog => MgStatus::InvalidState,
            Self::OwnFrameRejected(_) | Self::CloneDiverged(_) | Self::Panic(_) => {
                MgStatus::Internal
            }
        }
    }
}
