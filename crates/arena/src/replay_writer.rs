use std::path::Path;

use mg_core::{Clock, Outcome, Slot};
use mg_protocol::{REPLAY_FORMAT_VERSION, outcome_to_wire, proto, slot_to_wire};
use mg_registry::DynAuthority;
use prost::Message;

use crate::arena_config::ArenaConfig;
use crate::arena_error::ArenaError;

/// Writes finished game `index` as `<game>-<index>.mgreplay` into `dir`.
pub(crate) fn write_replay(
    dir: &Path,
    config: &ArenaConfig,
    index: u32,
    authority: &dyn DynAuthority,
    outcome: &Outcome,
) -> Result<(), ArenaError> {
    let game_id = authority.kind().info().id;
    let file = proto::ReplayFile {
        format_version: REPLAY_FORMAT_VERSION,
        header: Some(header(config, index, authority)),
        frames: authority.compact_log().to_vec(),
        end_tick: authority.next_tick().get(),
        result: Some(outcome_to_wire(outcome)),
    };
    let path = dir.join(format!("{game_id}-{index:05}.mgreplay"));
    std::fs::write(&path, file.encode_to_vec())
        .map_err(|source| ArenaError::WriteReplay { path, source })
}

fn header(config: &ArenaConfig, index: u32, authority: &dyn DynAuthority) -> proto::MatchHeader {
    let seats = Slot::all(config.bots.len())
        .zip(&config.bots)
        .map(|(slot, bot)| proto::MatchSeat {
            slot: slot_to_wire(slot),
            player: Some(proto::Identity {
                kind: proto::IdentityKind::BuiltInBot as i32,
                id: format!("bot-{}", slot.get()),
                display_name: bot.name.clone(),
            }),
            bot_kind: Some(bot.kind),
        })
        .collect();
    let tick_ms = match authority.clock() {
        Clock::Realtime { tick_ms } => tick_ms,
        Clock::TurnBased => 0,
    };
    proto::MatchHeader {
        match_id: format!("arena-{}-{index}", config.seed),
        settings: Some(config.settings),
        seats,
        tick_ms,
        engine_version: env!("CARGO_PKG_VERSION").to_owned(),
        started_at_unix_ms: 0,
    }
}
