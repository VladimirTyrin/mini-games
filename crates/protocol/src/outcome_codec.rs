use mg_core::{Outcome, OutcomeKind};

use crate::frame_codec::slot_to_wire;
use crate::proto;

pub fn outcome_to_wire(outcome: &Outcome) -> proto::MatchResult {
    let kind = match outcome.kind {
        OutcomeKind::Winner => proto::OutcomeKind::Winner,
        OutcomeKind::Draw => proto::OutcomeKind::Draw,
        OutcomeKind::Victory => proto::OutcomeKind::Victory,
        OutcomeKind::Defeat => proto::OutcomeKind::Defeat,
    };
    proto::MatchResult {
        kind: kind as i32,
        winner_slots: outcome.winners.iter().copied().map(slot_to_wire).collect(),
        scores: outcome.scores.clone(),
    }
}
