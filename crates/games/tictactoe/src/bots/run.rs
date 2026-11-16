#[derive(Clone, Copy, Debug)]
pub(crate) struct Run {
    pub(crate) length: usize,
    /// How many of the run's two ends are empty cells.
    pub(crate) open_ends: u8,
}
