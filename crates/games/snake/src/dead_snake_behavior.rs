#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeadSnakeBehavior {
    /// The body stops being an obstacle (food may spawn there). It is still reported in the
    /// view so renderers can fade it out.
    Disappear,
    StayOnField,
}

impl DeadSnakeBehavior {
    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            DeadSnakeBehavior::Disappear => 0,
            DeadSnakeBehavior::StayOnField => 1,
        }
    }
}
