use mg_core::{Outcome, OutcomeKind, Slot};

use crate::arena_bot::ArenaBot;

/// Results of one seat over an arena run.
#[derive(Clone, Debug, PartialEq)]
pub struct SeatTally {
    pub slot: Slot,
    pub bot: ArenaBot,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub unfinished: u32,
}

impl SeatTally {
    pub(crate) fn new(slot: Slot, bot: ArenaBot) -> Self {
        Self {
            slot,
            bot,
            wins: 0,
            draws: 0,
            losses: 0,
            unfinished: 0,
        }
    }

    pub(crate) fn record(&mut self, outcome: Option<&Outcome>) {
        match outcome.map(|outcome| seat_result(outcome, self.slot)) {
            None => self.unfinished += 1,
            Some(SeatResult::Won) => self.wins += 1,
            Some(SeatResult::Drew) => self.draws += 1,
            Some(SeatResult::Lost) => self.losses += 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SeatResult {
    Won,
    Drew,
    Lost,
}

pub(crate) fn seat_result(outcome: &Outcome, slot: Slot) -> SeatResult {
    match outcome.kind {
        OutcomeKind::Winner if outcome.winners.contains(&slot) => SeatResult::Won,
        OutcomeKind::Winner | OutcomeKind::Defeat => SeatResult::Lost,
        OutcomeKind::Draw => SeatResult::Drew,
        OutcomeKind::Victory => SeatResult::Won,
    }
}
