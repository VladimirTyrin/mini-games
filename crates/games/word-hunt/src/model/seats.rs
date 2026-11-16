use mg_core::{PlayerSpec, Slot, StateHasher};

/// Who sits at the table, who is a person, and who is still in play.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Seats {
    in_play: Vec<bool>,
    human: Vec<bool>,
}

impl Seats {
    pub(crate) fn new(players: &[PlayerSpec]) -> Self {
        Self {
            in_play: vec![true; players.len()],
            human: players.iter().map(|player| player.is_human()).collect(),
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.in_play.len()
    }

    /// More than one seat: turns, authors and a winner matter.
    pub(crate) fn is_group(&self) -> bool {
        self.count() > 1
    }

    pub(crate) fn in_play(&self) -> impl Iterator<Item = Slot> + '_ {
        Slot::all(self.in_play.len()).filter(|slot| self.is_in_play(*slot))
    }

    pub(crate) fn is_in_play(&self, slot: Slot) -> bool {
        self.in_play.get(slot.index()).copied().unwrap_or(false)
    }

    pub(crate) fn any_human_in_play(&self) -> bool {
        self.in_play
            .iter()
            .zip(&self.human)
            .any(|(in_play, human)| *in_play && *human)
    }

    /// A table of bots only (the arena) plays on until one is left.
    pub(crate) fn has_people(&self) -> bool {
        self.human.iter().any(|human| *human)
    }

    /// Next seat in play after `slot`, wrapping around.
    pub(crate) fn next_after(&self, slot: Slot) -> Option<Slot> {
        let seats = self.in_play.len();
        (1..=seats)
            .filter_map(|offset| Slot::try_from((slot.index() + offset) % seats).ok())
            .find(|candidate| self.is_in_play(*candidate))
    }

    pub(crate) fn remove(&mut self, slot: Slot) {
        if let Some(in_play) = self.in_play.get_mut(slot.index()) {
            *in_play = false;
        }
    }

    pub(crate) fn hash(&self, hasher: &mut StateHasher) {
        hasher.length(self.in_play.len());
        for (in_play, human) in self.in_play.iter().zip(&self.human) {
            hasher.bool(*in_play);
            hasher.bool(*human);
        }
    }
}
