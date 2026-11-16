use prost::Message;

use crate::proto;
use crate::replay_file_error::ReplayFileError;
use crate::wire_error::WireError;

/// `ReplayFile.format_version` written by this build; bumped on incompatible changes to rules or
/// the format.
pub const REPLAY_FORMAT_VERSION: u32 = 2;

/// Decodes a replay file and checks its envelope: format version, header and settings. Game
/// rules, seats and the frame log are checked by `mg-registry`.
pub fn decode_replay_file(bytes: &[u8]) -> Result<proto::ReplayFile, ReplayFileError> {
    let file = proto::ReplayFile::decode(bytes)?;
    if file.format_version != REPLAY_FORMAT_VERSION {
        return Err(ReplayFileError::UnsupportedFormat {
            found: file.format_version,
            supported: REPLAY_FORMAT_VERSION,
        });
    }
    let header = file.header.as_ref().ok_or(WireError::Missing("header"))?;
    header
        .settings
        .as_ref()
        .ok_or(WireError::Missing("header.settings"))?;
    Ok(file)
}
