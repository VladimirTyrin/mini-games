use mg_core::Slot;
use mg_protocol::proto;

/// One call that changed a local authority; replaying them rebuilds it exactly.
#[derive(Clone, Debug)]
pub(crate) enum AuthorityStep {
    Submit {
        slot: Slot,
        command: proto::GameCommand,
    },
    DropPlayer(Slot),
    RunBots,
    SealFrame,
}
