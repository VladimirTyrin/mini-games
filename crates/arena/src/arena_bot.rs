use mg_protocol::proto;
use mg_registry::{GameKind, bot_kinds};

use crate::arena_error::ArenaError;

/// A built-in bot of one game, named as on the command line (`minimax`, `random`, ...).
#[derive(Clone, Debug, PartialEq)]
pub struct ArenaBot {
    pub name: String,
    pub kind: proto::BotKind,
}

impl ArenaBot {
    /// Built-in bots of `game`, in UI order.
    pub fn all(game: GameKind) -> Vec<ArenaBot> {
        bot_kinds(game)
            .into_iter()
            .filter_map(|kind| {
                let name = short_name(&kind)?;
                Some(ArenaBot { name, kind })
            })
            .collect()
    }

    pub fn find(game: GameKind, name: &str) -> Result<ArenaBot, ArenaError> {
        let mut bots = Self::all(game);
        let found = bots
            .iter()
            .position(|bot| bot.name.eq_ignore_ascii_case(name));
        match found {
            Some(index) => Ok(bots.swap_remove(index)),
            None => Err(ArenaError::UnknownBot {
                game: game.info().id,
                name: name.to_owned(),
                known: bots.into_iter().map(|bot| bot.name).collect(),
            }),
        }
    }
}

/// The JSON mapping names the enum value (`{"tictactoe":"TIC_TAC_TOE_BOT_KIND_MINIMAX"}`), so the
/// short name needs no per-game code: it is the part after `BOT_KIND_`, lowercased.
fn short_name(kind: &proto::BotKind) -> Option<String> {
    let json = serde_json::to_value(kind).ok()?;
    let value = json.as_object()?.values().next()?.as_str()?;
    let (_, name) = value.split_once("BOT_KIND_")?;
    Some(name.to_ascii_lowercase())
}
