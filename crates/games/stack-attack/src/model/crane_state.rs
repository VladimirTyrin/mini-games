#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CraneState {
    /// Travelling to the target column, or waiting above it until the top cell is free (A2).
    Moving,
    /// Released its box this tick.
    Dropping,
    Leaving,
}
