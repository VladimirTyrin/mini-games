#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Both seats place their fleets, unseen by the other.
    Placing,
    Battle,
    /// A fleet was sunk entirely.
    Won,
    /// A seat gave up or left; the other wins.
    Forfeited,
    /// Both seats left.
    Abandoned,
}

impl Phase {
    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            Phase::Placing => 0,
            Phase::Battle => 1,
            Phase::Won => 2,
            Phase::Forfeited => 3,
            Phase::Abandoned => 4,
        }
    }

    pub const fn is_over(self) -> bool {
        matches!(self, Phase::Won | Phase::Forfeited | Phase::Abandoned)
    }
}
