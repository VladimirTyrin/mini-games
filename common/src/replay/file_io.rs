use std::path::Path;
use std::io::{Read, Write};
use prost::Message;
use crate::{ReplayV1, ReplayV1Header, ReplayV1Metadata, ReplayGame};
use super::{REPLAY_VERSION, REPLAY_FILE_EXTENSION};

#[derive(Debug)]
pub enum ReplayError {
    IoError(std::io::Error),
    DecodeError(prost::DecodeError),
    UnsupportedVersion { found: u8, expected: u8 },
    EmptyFile,
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::IoError(e) => write!(f, "IO error: {}", e),
            ReplayError::DecodeError(e) => write!(f, "Decode error: {}", e),
            ReplayError::UnsupportedVersion { found, expected } => {
                write!(f, "Unsupported replay version: found {}, expected {}", found, expected)
            }
            ReplayError::EmptyFile => write!(f, "Empty replay file"),
        }
    }
}

impl std::error::Error for ReplayError {}

impl From<std::io::Error> for ReplayError {
    fn from(e: std::io::Error) -> Self {
        ReplayError::IoError(e)
    }
}

impl From<prost::DecodeError> for ReplayError {
    fn from(e: prost::DecodeError) -> Self {
        ReplayError::DecodeError(e)
    }
}

pub fn save_replay(path: &Path, replay: &ReplayV1) -> Result<(), ReplayError> {
    let mut file = std::fs::File::create(path)?;

    file.write_all(&[REPLAY_VERSION])?;

    let encoded = replay.encode_to_vec();
    file.write_all(&encoded)?;

    Ok(())
}

pub fn save_replay_to_bytes(replay: &ReplayV1) -> Vec<u8> {
    let mut result = vec![REPLAY_VERSION];
    result.extend(replay.encode_to_vec());
    result
}

pub fn load_replay(path: &Path) -> Result<ReplayV1, ReplayError> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    load_replay_from_bytes(&buffer)
}

pub fn load_replay_from_bytes(bytes: &[u8]) -> Result<ReplayV1, ReplayError> {
    if bytes.is_empty() {
        return Err(ReplayError::EmptyFile);
    }

    let version = bytes[0];
    if version != REPLAY_VERSION {
        return Err(ReplayError::UnsupportedVersion {
            found: version,
            expected: REPLAY_VERSION,
        });
    }

    let replay = ReplayV1::decode(&bytes[1..])?;
    Ok(replay)
}

pub fn load_replay_metadata(path: &Path) -> Result<ReplayV1Metadata, ReplayError> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    load_replay_metadata_from_bytes(&buffer)
}

pub fn load_replay_metadata_from_bytes(bytes: &[u8]) -> Result<ReplayV1Metadata, ReplayError> {
    if bytes.is_empty() {
        return Err(ReplayError::EmptyFile);
    }

    let version = bytes[0];
    if version != REPLAY_VERSION {
        return Err(ReplayError::UnsupportedVersion {
            found: version,
            expected: REPLAY_VERSION,
        });
    }

    let header = ReplayV1Header::decode(&bytes[1..])?;
    header.metadata.ok_or_else(|| {
        ReplayError::DecodeError(prost::DecodeError::new("Missing metadata in replay header"))
    })
}

pub fn generate_replay_filename(game: ReplayGame, version: &str) -> String {
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S");

    let game_name = match game {
        ReplayGame::Snake => "SNAKE",
        ReplayGame::Tictactoe => "TICTACTOE",
        ReplayGame::NumbersMatch => "NUMBERSMATCH",
        ReplayGame::Unspecified => "UNKNOWN",
    };

    let sanitized_version = version.replace('.', "_");

    format!("{}_{}_{}.{}", timestamp, game_name, sanitized_version, REPLAY_FILE_EXTENSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlayerIdentity;

    #[test]
    fn test_save_load_replay_bytes() {
        let replay = ReplayV1 {
            metadata: Some(ReplayV1Metadata {
                engine_version: "1.0.0".to_string(),
                game_started_timestamp_ms: 1234567890,
                game: ReplayGame::Snake.into(),
                seed: 42,
                lobby_settings: None,
                players: vec![
                    PlayerIdentity { player_id: "player1".to_string(), is_bot: false },
                    PlayerIdentity { player_id: "bot1".to_string(), is_bot: true },
                ],
            }),
            actions: vec![],
        };

        let bytes = save_replay_to_bytes(&replay);
        let loaded = load_replay_from_bytes(&bytes).unwrap();
        let loaded_meta = loaded.metadata.unwrap();
        let replay_meta = replay.metadata.unwrap();

        assert_eq!(loaded_meta.engine_version, replay_meta.engine_version);
        assert_eq!(loaded_meta.game_started_timestamp_ms, replay_meta.game_started_timestamp_ms);
        assert_eq!(loaded_meta.game, replay_meta.game);
        assert_eq!(loaded_meta.seed, replay_meta.seed);
        assert_eq!(loaded_meta.players.len(), 2);
    }

    #[test]
    fn test_load_replay_metadata() {
        let replay = ReplayV1 {
            metadata: Some(ReplayV1Metadata {
                engine_version: "2.0.0".to_string(),
                game_started_timestamp_ms: 9999999999,
                game: ReplayGame::Tictactoe.into(),
                seed: 123,
                lobby_settings: None,
                players: vec![
                    PlayerIdentity { player_id: "alice".to_string(), is_bot: false },
                ],
            }),
            actions: vec![],
        };

        let bytes = save_replay_to_bytes(&replay);
        let metadata = load_replay_metadata_from_bytes(&bytes).unwrap();

        assert_eq!(metadata.engine_version, "2.0.0");
        assert_eq!(metadata.game, ReplayGame::Tictactoe as i32);
        assert_eq!(metadata.seed, 123);
        assert_eq!(metadata.players.len(), 1);
    }

    #[test]
    fn test_generate_replay_filename() {
        let filename = generate_replay_filename(ReplayGame::Snake, "1.2.3");
        assert!(filename.ends_with(".minigamesreplay"));
        assert!(filename.contains("SNAKE"));
        assert!(filename.contains("1_2_3"));
    }

    #[test]
    fn test_load_empty_file_error() {
        let result = load_replay_from_bytes(&[]);
        assert!(matches!(result, Err(ReplayError::EmptyFile)));
    }

    #[test]
    fn test_load_unsupported_version_error() {
        let result = load_replay_from_bytes(&[99]);
        assert!(matches!(result, Err(ReplayError::UnsupportedVersion { found: 99, .. })));
    }
}
