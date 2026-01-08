use crate::{add_bot_request, SnakeBotType, TicTacToeBotType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotType {
    Snake(SnakeBotType),
    TicTacToe(TicTacToeBotType),
}

impl BotType {
    pub fn from_proto(bot_type: Option<add_bot_request::BotType>) -> Result<Self, String> {
        match bot_type {
            Some(add_bot_request::BotType::SnakeBot(t)) => Ok(BotType::Snake(
                SnakeBotType::try_from(t).map_err(|_| "Invalid snake bot type")?
            )),
            Some(add_bot_request::BotType::TictactoeBot(t)) => Ok(BotType::TicTacToe(
                TicTacToeBotType::try_from(t).map_err(|_| "Invalid tictactoe bot type")?
            )),
            None => Err("No bot type provided".to_string()),
        }
    }
}
