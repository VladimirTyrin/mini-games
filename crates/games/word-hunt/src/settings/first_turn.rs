/// Group games: who makes the first guess. Turns then go round the table in seat order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirstTurn {
    Random,
    /// The first seat: the host, or the player in a local game.
    HostFirst,
    /// The seat after the host, so the host closes every round.
    HostLast,
}

impl FirstTurn {
    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            FirstTurn::Random => 0,
            FirstTurn::HostFirst => 1,
            FirstTurn::HostLast => 2,
        }
    }
}
