use mg_core::CommandError;

use crate::proto;
use crate::wire_refusal::WireRefusal;

pub fn refusal_to_wire<R: WireRefusal>(error: &CommandError<R>) -> proto::Refusal {
    let common = match *error {
        CommandError::Illegal(refusal) => {
            return proto::Refusal {
                reason: Some(refusal.refusal_to_wire()),
            };
        }
        CommandError::GameOver => proto::CommonRefusal::GameOver,
        CommandError::NotYourTurn => proto::CommonRefusal::NotYourTurn,
        CommandError::NotPlaying => proto::CommonRefusal::NotPlaying,
        CommandError::Busy => proto::CommonRefusal::Busy,
        CommandError::Malformed(_) => proto::CommonRefusal::Malformed,
    };
    common_refusal(common)
}

/// For a game's refusals that only malformed input meets: the client shows a generic "not
/// allowed".
pub(crate) fn malformed_refusal() -> proto::refusal::Reason {
    proto::refusal::Reason::Common(proto::CommonRefusal::Malformed.into())
}

/// For refusals a host makes before the game is asked (an observer's command, a paused match).
pub fn common_refusal(reason: proto::CommonRefusal) -> proto::Refusal {
    proto::Refusal {
        reason: Some(proto::refusal::Reason::Common(reason.into())),
    }
}
