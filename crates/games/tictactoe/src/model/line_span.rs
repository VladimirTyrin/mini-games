/// The on-board part of a line through a cell: the cell's index and how many steps fit before
/// leaving the board in each direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LineSpan {
    pub(crate) origin_index: usize,
    pub(crate) backward_steps: usize,
    pub(crate) forward_steps: usize,
    /// Index change of one forward step.
    pub(crate) stride: usize,
}
