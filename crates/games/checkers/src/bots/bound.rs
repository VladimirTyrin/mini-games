/// What a remembered score says about the true score of its position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    Exact,
    /// The search was cut off: the true score is at least this.
    Lower,
    /// Nothing reached the window: the true score is at most this.
    Upper,
}
