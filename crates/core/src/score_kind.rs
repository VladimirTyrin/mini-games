/// What the scores of a finished competitive match measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoreKind {
    /// Points of the game's own (a snake's length): worth showing beside the wins of a series.
    Points,
    /// One for the winner, zero for everyone else: the wins of a series say it all.
    WinFlag,
}
