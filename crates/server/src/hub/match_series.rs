use mg_protocol::proto;

/// The score of the matches the same players play in a row at one table; see
/// `proto::MatchSeries`. Seat order is slot order, so wins are kept by seat.
#[derive(Debug)]
pub(super) struct MatchSeries {
    wins: Vec<u32>,
    draws: u32,
}

impl MatchSeries {
    pub(super) fn new(seats: usize) -> Self {
        Self {
            wins: vec![0; seats],
            draws: 0,
        }
    }

    /// Counts a finished match of these seats. Solo and cooperative outcomes count for nobody.
    pub(super) fn record(&mut self, result: &proto::MatchResult) {
        match result.kind() {
            proto::OutcomeKind::Winner => {
                for &slot in &result.winner_slots {
                    if let Some(wins) = usize::try_from(slot)
                        .ok()
                        .and_then(|slot| self.wins.get_mut(slot))
                    {
                        *wins = wins.saturating_add(1);
                    }
                }
            }
            proto::OutcomeKind::Draw => self.draws = self.draws.saturating_add(1),
            proto::OutcomeKind::Unspecified
            | proto::OutcomeKind::Victory
            | proto::OutcomeKind::Defeat => {}
        }
    }

    /// `None` until a match counted for anyone.
    pub(super) fn to_wire(&self) -> Option<proto::MatchSeries> {
        let counted = self.draws > 0 || self.wins.iter().any(|&wins| wins > 0);
        counted.then(|| proto::MatchSeries {
            wins: self.wins.clone(),
            draws: self.draws,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(kind: proto::OutcomeKind, winner_slots: Vec<u32>) -> proto::MatchResult {
        proto::MatchResult {
            kind: kind as i32,
            winner_slots,
            scores: Vec::new(),
        }
    }

    #[test]
    fn wins_and_draws_add_up_over_matches() {
        let mut series = MatchSeries::new(2);
        series.record(&result(proto::OutcomeKind::Winner, vec![1]));
        series.record(&result(proto::OutcomeKind::Draw, Vec::new()));
        series.record(&result(proto::OutcomeKind::Winner, vec![1]));
        series.record(&result(proto::OutcomeKind::Winner, vec![0]));
        assert_eq!(
            series.to_wire(),
            Some(proto::MatchSeries {
                wins: vec![1, 2],
                draws: 1,
            })
        );
    }

    #[test]
    fn a_cooperative_outcome_counts_for_nobody() {
        let mut series = MatchSeries::new(2);
        series.record(&result(proto::OutcomeKind::Victory, Vec::new()));
        series.record(&result(proto::OutcomeKind::Defeat, Vec::new()));
        assert_eq!(series.to_wire(), None);
    }
}
