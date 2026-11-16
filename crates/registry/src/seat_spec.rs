use mg_core::PlayerSpec;
use mg_protocol::proto;

/// A seat as the lobby describes it.
#[derive(Clone, Debug, PartialEq)]
pub enum SeatSpec {
    Human,
    Bot(proto::BotKind),
}

impl SeatSpec {
    pub fn player_spec(&self) -> PlayerSpec {
        match self {
            Self::Human => PlayerSpec::HUMAN,
            Self::Bot(_) => PlayerSpec::BOT,
        }
    }
}
