use mg_core::Tick;
use mg_protocol::{ReplayFileError, WireError, decode_replay_file};

use crate::create_mirror_from_header;
use crate::loaded_replay::LoadedReplay;
use crate::replay_error::ReplayError;

/// Decodes an untrusted replay file and checks everything short of replaying it: format,
/// header, seats, settings against the game rules, and the shape of the frame log.
pub fn load_replay_file(bytes: &[u8]) -> Result<LoadedReplay, ReplayError> {
    let file = decode_replay_file(bytes)?;
    let header = file
        .header
        .as_ref()
        .ok_or(ReplayFileError::from(WireError::Missing("header")))?;
    let mirror = create_mirror_from_header(header)?;
    let log = mirror.compact_log(file.frames, Tick::new(file.end_tick))?;
    Ok(LoadedReplay { mirror, log })
}
