use mg_core::{PlayerSpec, ScoreKind};
use mg_protocol::proto;
use mg_registry::{GameKind, bot_kinds, create_mirror};
use prost::Message;
use wasm_bindgen::prelude::*;

use crate::settings_problem::SettingsProblem;

fn kind_by_id(game_id: &str) -> Result<GameKind, JsError> {
    GameKind::ALL
        .iter()
        .copied()
        .find(|kind| kind.info().id == game_id)
        .ok_or_else(|| JsError::new(&format!("unknown game `{game_id}`")))
}

/// JSON array describing every game (id, name, seat limits, bots). Built by hand: the catalog is
/// tiny and pulling serde into the WASM bundle would cost far more than it saves.
#[wasm_bindgen(js_name = gameCatalogJson)]
pub fn game_catalog_json() -> String {
    let entries: Vec<String> = GameKind::ALL
        .iter()
        .copied()
        .map(|kind| {
            let info = kind.info();
            format!(
                r#"{{"id":"{}","name":"{}","minPlayers":{},"maxPlayers":{},"hasBots":{},"cooperative":{},"scores":"{}","botKindCount":{}}}"#,
                info.id,
                info.name,
                info.min_players,
                info.max_players,
                info.has_bots,
                info.cooperative,
                match info.scores {
                    ScoreKind::Points => "points",
                    ScoreKind::WinFlag => "winFlag",
                },
                bot_kinds(kind).len()
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}

/// Encoded default `GameSettings` for a game id.
#[wasm_bindgen(js_name = defaultSettings)]
pub fn default_settings(game_id: &str) -> Result<Vec<u8>, JsError> {
    Ok(mg_registry::default_settings(kind_by_id(game_id)?).encode_to_vec())
}

/// Encoded `BotKind`s a game offers, concatenated as length-delimited messages.
#[wasm_bindgen(js_name = botKinds)]
pub fn bot_kinds_delimited(game_id: &str) -> Result<Vec<u8>, JsError> {
    let mut out = Vec::new();
    for kind in bot_kinds(kind_by_id(game_id)?) {
        kind.encode_length_delimited(&mut out)
            .map_err(|e| JsError::new(&e.to_string()))?;
    }
    Ok(out)
}

/// JSON array of `{"field","min","max"}` bounds of the numeric settings, as the game enforces
/// them for `settings` (a bound may depend on other fields). Field names match
/// [`SettingsProblem::field`].
#[wasm_bindgen(js_name = settingRangesJson)]
pub fn setting_ranges_json(settings: &[u8]) -> Result<String, JsError> {
    let settings =
        proto::GameSettings::decode(settings).map_err(|error| JsError::new(&error.to_string()))?;
    let ranges =
        mg_registry::setting_ranges(&settings).map_err(|error| JsError::new(&error.to_string()))?;
    let entries: Vec<String> = ranges
        .iter()
        .map(|range| {
            format!(
                r#"{{"field":"{}","min":{},"max":{}}}"#,
                range.field, range.min, range.max
            )
        })
        .collect();
    Ok(format!("[{}]", entries.join(",")))
}

/// `None` when the settings are valid for `seats` human players, otherwise the reason. Uses the
/// game's own rules, so the UI never duplicates limits.
#[wasm_bindgen(js_name = validateSettings)]
pub fn validate_settings(settings: &[u8], seats: u8) -> Option<SettingsProblem> {
    let settings = match proto::GameSettings::decode(settings) {
        Ok(settings) => settings,
        Err(error) => return Some(SettingsProblem::general(error.to_string())),
    };
    let players = vec![PlayerSpec::HUMAN; usize::from(seats)];
    create_mirror(&settings, &players)
        .err()
        .map(SettingsProblem::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tictactoe_settings(field_width: u32) -> Vec<u8> {
        let mut settings = mg_registry::default_settings(GameKind::TicTacToe);
        if let Some(proto::game_settings::Game::Tictactoe(tictactoe)) = settings.game.as_mut() {
            tictactoe.field_width = field_width;
        }
        settings.encode_to_vec()
    }

    #[test]
    fn a_rejected_setting_names_its_field() {
        assert_eq!(validate_settings(&tictactoe_settings(15), 2), None);
        assert_eq!(
            validate_settings(&tictactoe_settings(2), 2),
            Some(SettingsProblem {
                field: Some("field_width".to_owned()),
                message: "must be between 3 and 20, got 2".to_owned(),
            })
        );
        let wrong_seats = validate_settings(&tictactoe_settings(15), 3).unwrap();
        assert_eq!(wrong_seats.field, None);
    }

    #[test]
    fn ranges_follow_the_current_settings() {
        assert_eq!(
            setting_ranges_json(&tictactoe_settings(9)).unwrap(),
            r#"[{"field":"field_width","min":3,"max":20},{"field":"field_height","min":3,"max":20},{"field":"win_count","min":3,"max":9}]"#
        );
    }
}
