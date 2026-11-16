use super::replay_check_error::ReplayCheckError;
use super::replay_upload::ReplayUpload;
use crate::ids::{LobbyId, SessionId};

/// Result of checking an uploaded replay on the blocking pool.
#[derive(Debug)]
pub(crate) struct ValidatedReplay {
    pub session: SessionId,
    pub lobby: LobbyId,
    pub request_id: u32,
    pub host_only_control: bool,
    pub outcome: Result<ReplayUpload, ReplayCheckError>,
}
