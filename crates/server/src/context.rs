use mg_protocol::proto;
use tokio::sync::{Semaphore, watch};

use crate::auth::{BotKeys, GoogleVerifier, PlayerIdKey, SessionTokens};
use crate::hub::HubHandle;
use crate::timing::Timing;

pub(crate) const PROTOCOL_VERSION: u32 = 2;
pub(crate) const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Shared, immutable server-wide state handed to every connection.
#[derive(Debug)]
pub(crate) struct ServerContext {
    pub allow_guests: bool,
    pub google: Option<GoogleVerifier>,
    pub google_verifications: Semaphore,
    pub session_tokens: SessionTokens,
    pub player_ids: PlayerIdKey,
    pub bot_keys: BotKeys,
    pub hub: HubHandle,
    pub timing: Timing,
    /// Turns `true` when the server stops; connections that never signed in close at once
    /// (signed-in ones are told by the hub).
    pub shutting_down: watch::Receiver<bool>,
}

impl ServerContext {
    pub(crate) fn server_info(&self) -> proto::ServerInfo {
        proto::ServerInfo {
            server_version: SERVER_VERSION.to_owned(),
            min_protocol_version: PROTOCOL_VERSION,
            max_protocol_version: PROTOCOL_VERSION,
            guests_allowed: self.allow_guests,
            google_client_id: self
                .google
                .as_ref()
                .map(|g| g.client_id().to_owned())
                .unwrap_or_default(),
        }
    }
}
