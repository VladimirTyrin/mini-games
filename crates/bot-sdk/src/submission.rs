/// Where this bot's last command is. The strategy is not asked again until it shows up in a
/// frame (or is refused): the server acknowledges a command before it seals the frame with it,
/// so in between the mirror still shows the old position, and a second answer to it would be
/// refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Submission {
    Settled,
    AwaitingReply { request_id: u32 },
    AwaitingFrame,
}
