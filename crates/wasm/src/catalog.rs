use mg_core::PlayerSpec;
use mg_protocol::proto;
use mg_registry::{GameKind, bot_kinds, create_mirror};
use prost::Message;
use wasm_bindgen::prelude::*;

fn kind_by_id(game_id: &str) -> Result<GameKind, JsError> {
    GameKind::ALL
        .into_iter()
        .find(|kind| kind.info().id == game_id)
        .ok_or_else(|| JsError::new(&format!("unknown game `{game_id}`")))
}

/// JSON array describing every game (id, name, seat limits, bots). Built by hand: the catalog is
/// tiny and pulling serde into the WASM bundle would cost far more than it saves.
#[wasm_bindgen(js_name = gameCatalogJson)]
pub fn game_catalog_json() -> String {
    let entries: Vec<String> = GameKind::ALL
        .into_iter()
        .map(|kind| {
            let info = kind.info();
            format!(
                r#"{{"id":"{}","name":"{}","minPlayers":{},"maxPlayers":{},"hasBots":{},"cooperative":{},"botKindCount":{}}}"#,
                info.id,
                info.name,
                info.min_players,
                info.max_players,
                info.has_bots,
                info.cooperative,
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

/// `None` when the settings are valid for `seats` human players, otherwise the reason. Uses the
/// game's own rules, so the UI never duplicates limits.
#[wasm_bindgen(js_name = validateSettings)]
pub fn validate_settings(settings: &[u8], seats: u8) -> Option<String> {
    let settings = match proto::GameSettings::decode(settings) {
        Ok(settings) => settings,
        Err(error) => return Some(error.to_string()),
    };
    let players = vec![PlayerSpec::HUMAN; usize::from(seats)];
    create_mirror(&settings, &players)
        .err()
        .map(|error| error.to_string())
}
