use mg_core::{GameSecret, Slot};

use crate::io::SecretCommand;
use crate::model::Fleet;

/// Both fleets, known only to the authority until the end (see `mg_core::Game::Secret`).
#[derive(Clone, Debug, Default)]
pub struct HiddenFleets {
    fleets: [Option<Fleet>; 2],
}

impl HiddenFleets {
    pub fn fleet(&self, slot: Slot) -> Option<&Fleet> {
        self.fleets.get(slot.index()).and_then(Option::as_ref)
    }

    pub(crate) fn place(&mut self, slot: Slot, fleet: Fleet) {
        if let Some(place) = self.fleets.get_mut(slot.index()) {
            *place = Some(fleet);
        }
    }
}

impl GameSecret for HiddenFleets {
    type Command = SecretCommand;
    /// The seat's own fleet.
    type SeatView = Fleet;
}
