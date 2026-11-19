use mg_core::{CommandError, EnglishText};
use mg_protocol::{WireRefusal, common_refusal, proto, refusal_english, refusal_to_wire};

/// A refused command with the game type erased: what the wire carries, and the reason in English
/// for logs. For input no honest client sends, `explanation` keeps the detail the wire leaves out.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[error("{explanation}")]
pub struct Rejection {
    pub refusal: proto::Refusal,
    pub explanation: &'static str,
}

impl Rejection {
    /// A refusal a host makes before the game is asked (an observer's command, a paused match).
    pub fn common(reason: proto::CommonRefusal) -> Self {
        let refusal = common_refusal(reason);
        Self {
            explanation: refusal_english(&refusal),
            refusal,
        }
    }
}

impl<R: WireRefusal> From<CommandError<R>> for Rejection {
    fn from(error: CommandError<R>) -> Self {
        Self {
            refusal: refusal_to_wire(&error),
            explanation: error.english(),
        }
    }
}
