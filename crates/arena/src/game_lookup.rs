use mg_protocol::proto;
use mg_registry::GameKind;

use crate::arena_error::ArenaError;

pub fn game_by_id(id: &str) -> Result<GameKind, ArenaError> {
    GameKind::ALL
        .iter()
        .copied()
        .find(|kind| kind.info().id.eq_ignore_ascii_case(id))
        .ok_or_else(|| ArenaError::UnknownGame {
            id: id.to_owned(),
            known: GameKind::ALL.iter().map(|kind| kind.info().id).collect(),
        })
}

/// Parses protobuf JSON of `GameSettings` and checks that it configures `game`.
pub fn settings_from_json(game: GameKind, json: &str) -> Result<proto::GameSettings, ArenaError> {
    let settings: proto::GameSettings =
        serde_json::from_str(json).map_err(ArenaError::SettingsJson)?;
    let settings_game = GameKind::of_settings(&settings).map_err(mg_registry::CreateError::from)?;
    if settings_game != game {
        return Err(ArenaError::SettingsForAnotherGame {
            game: game.info().id,
            settings: settings_game.info().id,
        });
    }
    Ok(settings)
}
