#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HintMode {
    /// [`crate::LIMITED_INITIAL_HINTS`] at the start plus [`crate::LIMITED_HINTS_PER_REFILL`]
    /// per refill.
    #[default]
    Limited,
    Unlimited,
    Disabled,
}
