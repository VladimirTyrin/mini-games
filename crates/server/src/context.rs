use std::sync::Arc;

use mg_protocol::proto;
use tokio::sync::{Semaphore, watch};

use crate::auth::{BotKeys, GoogleVerifier, PlayerIdKey, SessionTokens};
use crate::hub::HubHandle;
use crate::push::PushSender;
use crate::timing::Timing;

pub(crate) const PROTOCOL_VERSION: u32 = 2;

/// `2.0.0+<build>` when built with `MG_BUILD_NUMBER` (deploy.sh passes the commit count, which
/// is also the web bundle's build), else the plain workspace version.
pub(crate) fn server_version() -> String {
    let version = env!("CARGO_PKG_VERSION");
    match option_env!("MG_BUILD_NUMBER") {
        Some(build) if !build.is_empty() => format!("{version}+{build}"),
        _ => version.to_owned(),
    }
}

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
    pub push: Arc<PushSender>,
    pub timing: Timing,
    /// Turns `true` when the server stops; connections that never signed in close at once
    /// (signed-in ones are told by the hub).
    pub shutting_down: watch::Receiver<bool>,
}

impl ServerContext {
    pub(crate) fn server_info(&self) -> proto::ServerInfo {
        proto::ServerInfo {
            server_version: server_version(),
            min_protocol_version: PROTOCOL_VERSION,
            max_protocol_version: PROTOCOL_VERSION,
            guests_allowed: self.allow_guests,
            google_client_id: self
                .google
                .as_ref()
                .map(|g| g.client_id().to_owned())
                .unwrap_or_default(),
            web_push_public_key: self
                .push
                .web_push_public_key()
                .unwrap_or_default()
                .to_owned(),
        }
    }
}
