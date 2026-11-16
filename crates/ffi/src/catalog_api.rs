use mg_core::PlayerSpec;
use mg_protocol::proto;
use mg_registry::{GameKind, bot_kinds, create_mirror, default_settings};
use prost::Message;

use crate::boundary::{decode, guard, input_bytes, write_bytes};
use crate::buffer::MgBuffer;
use crate::ffi_error::FfiError;
use crate::status::MgStatus;

unsafe fn game_kind(game_id: *const u8, game_id_len: usize) -> Result<GameKind, FfiError> {
    let bytes = unsafe { input_bytes(game_id, game_id_len, "game_id") }?;
    let id = std::str::from_utf8(bytes).map_err(FfiError::GameIdEncoding)?;
    GameKind::ALL
        .iter()
        .copied()
        .find(|kind| kind.info().id == id)
        .ok_or_else(|| FfiError::UnknownGame(id.to_owned()))
}

/// `game_id`: UTF-8 game id (`tictactoe`, `snake`, ...). Writes the encoded default
/// `GameSettings`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_default_settings(
    game_id: *const u8,
    game_id_len: usize,
    out_settings: *mut MgBuffer,
) -> MgStatus {
    guard(|| {
        let kind = unsafe { game_kind(game_id, game_id_len) }?;
        let settings = default_settings(kind).encode_to_vec();
        unsafe { write_bytes(out_settings, settings, "out_settings") }
    })
}

/// The game's built-in `BotKind`s, each encoded length-delimited, concatenated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_bot_kinds(
    game_id: *const u8,
    game_id_len: usize,
    out_bot_kinds: *mut MgBuffer,
) -> MgStatus {
    guard(|| {
        let kind = unsafe { game_kind(game_id, game_id_len) }?;
        let mut bytes = Vec::new();
        for bot in bot_kinds(kind) {
            bytes.extend_from_slice(&bot.encode_length_delimited_to_vec());
        }
        unsafe { write_bytes(out_bot_kinds, bytes, "out_bot_kinds") }
    })
}

/// `MG_STATUS_OK` when the encoded `GameSettings` are valid for `seats` players, otherwise
/// `MG_STATUS_INVALID_ARGUMENT` with the reason as the last error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mg_validate_settings(
    settings: *const u8,
    settings_len: usize,
    seats: u32,
) -> MgStatus {
    guard(|| {
        let bytes = unsafe { input_bytes(settings, settings_len, "settings") }?;
        let settings: proto::GameSettings = decode(bytes, "settings")?;
        let seats = usize::from(u8::try_from(seats).unwrap_or(u8::MAX));
        create_mirror(&settings, &vec![PlayerSpec::HUMAN; seats])?;
        Ok(())
    })
}
