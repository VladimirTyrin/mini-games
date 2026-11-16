use mg_core::{Slot, Tick};
use mg_protocol::proto;

/// How a match went for this bot.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchSummary {
    pub match_id: String,
    /// `None` when the bot only watched.
    pub my_slot: Option<Slot>,
    /// `None` when the server aborted the match.
    pub result: Option<proto::MatchResult>,
    pub final_tick: Tick,
    /// The local mirror's state hash at the end; equals the server's when the bot stayed in
    /// sync.
    pub final_state_hash: u64,
    pub reconnects: u32,
    /// Commands the server refused (`ERROR_CODE_COMMAND_REJECTED`).
    pub rejected_commands: u32,
}

impl MatchSummary {
    pub fn won(&self) -> bool {
        let (Some(slot), Some(result)) = (self.my_slot, &self.result) else {
            return false;
        };
        result.kind == proto::OutcomeKind::Winner as i32
            && result.winner_slots.contains(&u32::from(slot.get()))
    }
}
