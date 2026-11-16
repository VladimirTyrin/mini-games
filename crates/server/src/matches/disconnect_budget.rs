use std::collections::BTreeMap;
use std::time::Duration;

use mg_core::Slot;
use tokio::time::Instant;

/// SC2-style disconnect handling: while any seated human is disconnected the match is frozen
/// and each disconnected seat spends its own per-match budget.
#[derive(Debug)]
pub(super) struct DisconnectBudgets {
    budget_per_seat: Duration,
    vote_after: Duration,
    spent: BTreeMap<Slot, Duration>,
    disconnected_since: BTreeMap<Slot, Instant>,
}

impl DisconnectBudgets {
    pub(super) fn new(budget_per_seat: Duration, vote_after: Duration) -> Self {
        Self {
            budget_per_seat,
            vote_after,
            spent: BTreeMap::new(),
            disconnected_since: BTreeMap::new(),
        }
    }

    pub(super) fn disconnect(&mut self, slot: Slot, now: Instant) {
        self.disconnected_since.entry(slot).or_insert(now);
    }

    pub(super) fn reconnect(&mut self, slot: Slot, now: Instant) {
        if let Some(since) = self.disconnected_since.remove(&slot) {
            *self.spent.entry(slot).or_default() += now.saturating_duration_since(since);
        }
    }

    /// A seat that left play (dropped) stops being waited for; the time it was gone stays spent.
    pub(super) fn forget(&mut self, slot: Slot, now: Instant) {
        self.reconnect(slot, now);
    }

    pub(super) fn is_disconnected(&self, slot: Slot) -> bool {
        self.disconnected_since.contains_key(&slot)
    }

    pub(super) fn disconnected(&self) -> impl Iterator<Item = Slot> + '_ {
        self.disconnected_since.keys().copied()
    }

    pub(super) fn budget_left(&self, slot: Slot, now: Instant) -> Duration {
        let spent = self.spent.get(&slot).copied().unwrap_or_default();
        let current = self
            .disconnected_since
            .get(&slot)
            .map_or(Duration::ZERO, |since| {
                now.saturating_duration_since(*since)
            });
        self.budget_per_seat.saturating_sub(spent + current)
    }

    pub(super) fn drop_vote_in(&self, slot: Slot, now: Instant) -> Duration {
        self.disconnected_since
            .get(&slot)
            .map_or(self.vote_after, |since| {
                self.vote_after
                    .saturating_sub(now.saturating_duration_since(*since))
            })
    }

    pub(super) fn exhausted(&self, now: Instant) -> Vec<Slot> {
        self.disconnected()
            .filter(|&slot| self.budget_left(slot, now).is_zero())
            .collect()
    }

    pub(super) fn next_exhaustion(&self, now: Instant) -> Option<Instant> {
        self.disconnected()
            .map(|slot| now + self.budget_left(slot, now))
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_is_spent_across_disconnects_and_exhausts() {
        let start = Instant::now();
        let slot = Slot::new(1);
        let mut budgets = DisconnectBudgets::new(Duration::from_secs(60), Duration::from_secs(20));
        budgets.disconnect(slot, start);
        assert_eq!(
            budgets.drop_vote_in(slot, start + Duration::from_secs(5)),
            Duration::from_secs(15)
        );
        budgets.reconnect(slot, start + Duration::from_secs(40));
        assert_eq!(
            budgets.budget_left(slot, start + Duration::from_secs(100)),
            Duration::from_secs(20)
        );
        budgets.disconnect(slot, start + Duration::from_secs(100));
        assert!(
            budgets
                .exhausted(start + Duration::from_secs(119))
                .is_empty()
        );
        assert_eq!(
            budgets.exhausted(start + Duration::from_secs(120)),
            vec![slot]
        );
        assert_eq!(
            budgets.next_exhaustion(start + Duration::from_secs(100)),
            Some(start + Duration::from_secs(120))
        );
    }

    #[test]
    fn forgetting_a_seat_keeps_the_time_it_spent() {
        let start = Instant::now();
        let slot = Slot::new(0);
        let mut budgets = DisconnectBudgets::new(Duration::from_secs(60), Duration::from_secs(20));
        budgets.disconnect(slot, start);
        budgets.forget(slot, start + Duration::from_secs(25));
        assert!(!budgets.any_disconnected());
        assert_eq!(
            budgets.budget_left(slot, start + Duration::from_secs(90)),
            Duration::from_secs(35)
        );
    }
}
