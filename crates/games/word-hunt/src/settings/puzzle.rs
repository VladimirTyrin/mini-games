use crate::settings::DayNumber;

/// Which word the game asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Puzzle {
    /// A random answer.
    Unlimited,
    /// The answer of that day, the same for every player of the language (days since
    /// 1970-01-01 in the player's own time zone, supplied by the client: the game has no clock).
    Daily(DayNumber),
}
