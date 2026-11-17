use mg_core::Slot;

use crate::model::{LastShot, Phase, Ship, Waters};

/// What everyone sees. A seat's own fleet is not here: it comes to that seat alone (the game's
/// seat secret), and every fleet is in `revealed` once the game is over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub phase: Phase,
    /// By slot: whether the seat has placed its fleet.
    pub ready: Vec<bool>,
    /// By slot: that seat's sea as everyone sees it.
    pub waters: Vec<Waters>,
    /// Who fires next; `None` while placing, while a shot is resolved and after the end.
    pub turn: Option<Slot>,
    pub winner: Option<Slot>,
    pub last_shot: Option<LastShot>,
    /// By slot: the fleet, once the game is over.
    pub revealed: Vec<Option<Vec<Ship>>>,
}
