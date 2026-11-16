use mg_core::{Outcome, OutcomeKind};
use mg_protocol::{proto, slot_to_wire};

pub(crate) fn match_result(outcome: Outcome) -> proto::MatchResult {
    let kind = match outcome.kind {
        OutcomeKind::Winner => proto::OutcomeKind::Winner,
        OutcomeKind::Draw => proto::OutcomeKind::Draw,
        OutcomeKind::Victory => proto::OutcomeKind::Victory,
        OutcomeKind::Defeat => proto::OutcomeKind::Defeat,
    };
    proto::MatchResult {
        kind: kind as i32,
        winner_slots: outcome.winners.into_iter().map(slot_to_wire).collect(),
        scores: outcome.scores,
    }
}
