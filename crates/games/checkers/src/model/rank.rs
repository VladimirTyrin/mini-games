#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rank {
    /// Moves one square forward; captures forward and backward.
    Man,
    /// Flies along a diagonal any number of empty squares, in all four directions.
    King,
}
