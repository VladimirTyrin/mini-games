#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameInfo {
    /// Stable wire/storage id (`snake`, `tictactoe`, ...). Never rename or reuse.
    pub id: &'static str,
    pub name: &'static str,
    /// Seat limits, humans and bots together.
    pub min_players: u8,
    pub max_players: u8,
    pub has_bots: bool,
    /// Players win or lose together; solo games count as cooperative.
    pub cooperative: bool,
}
