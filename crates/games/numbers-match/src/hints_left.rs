#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintsLeft {
    Unlimited,
    /// Always `Count(0)` when hints are disabled.
    Count(u32),
}
