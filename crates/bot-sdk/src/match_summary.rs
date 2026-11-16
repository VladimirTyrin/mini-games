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
    /// Won a competitive match, or completed a solo or cooperative one.
    pub fn won(&self) -> bool {
        let (Some(slot), Some(result)) = (self.my_slot, &self.result) else {
            return false;
        };
        match proto::OutcomeKind::try_from(result.kind) {
            Ok(proto::OutcomeKind::Winner) => result.winner_slots.contains(&u32::from(slot.get())),
            Ok(proto::OutcomeKind::Victory) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(kind: proto::OutcomeKind, winners: Vec<u32>) -> MatchSummary {
        MatchSummary {
            match_id: "m".into(),
            my_slot: Some(Slot::new(0)),
            result: Some(proto::MatchResult {
                kind: kind as i32,
                winner_slots: winners,
                ..proto::MatchResult::default()
            }),
            final_tick: Tick::default(),
            final_state_hash: 0,
            reconnects: 0,
            rejected_commands: 0,
        }
    }

    #[test]
    fn a_solo_victory_is_a_win() {
        assert!(summary(proto::OutcomeKind::Victory, vec![]).won());
        assert!(!summary(proto::OutcomeKind::Defeat, vec![]).won());
        assert!(summary(proto::OutcomeKind::Winner, vec![0]).won());
        assert!(!summary(proto::OutcomeKind::Winner, vec![1]).won());
    }
}
