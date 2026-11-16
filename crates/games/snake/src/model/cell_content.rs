use mg_core::Slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellContent {
    Empty,
    Food,
    /// A solid body segment: a living snake, or a dead one with `StayOnField`.
    Body(Slot),
}
