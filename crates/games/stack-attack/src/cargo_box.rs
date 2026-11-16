use crate::box_id::BoxId;
use crate::cell::Cell;
use crate::pattern_id::PatternId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CargoBox {
    pub id: BoxId,
    pub cell: Cell,
    pub pattern: PatternId,
    /// Moved down (or was dropped) this tick and has not landed yet. Falling boxes do not count
    /// for the ceiling check but do count for line clears (legacy).
    pub falling: bool,
}
