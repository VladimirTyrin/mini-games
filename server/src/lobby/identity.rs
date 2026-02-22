use crate::{BotId, PlayerId};
use super::BotType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerIdentity {
    Player(PlayerId),
    Bot { id: BotId, bot_type: BotType },
}

impl PlayerIdentity {
    pub fn client_id(&self) -> String {
        match self {
            PlayerIdentity::Player(id) => id.to_string(),
            PlayerIdentity::Bot { id, .. } => id.to_string(),
        }
    }

    pub fn to_proto(&self) -> crate::proto::game_service::PlayerIdentity {
        crate::proto::game_service::PlayerIdentity {
            player_id: self.client_id(),
            is_bot: matches!(self, PlayerIdentity::Bot { .. }),
        }
    }
}
