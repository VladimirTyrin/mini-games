use tokio::sync::mpsc;

use super::match_control::MatchControl;
use super::match_request::MatchRequest;

/// Receiving side of a [`super::MatchHandle`].
#[derive(Debug)]
pub(crate) struct MatchInbox {
    pub control: mpsc::UnboundedReceiver<MatchControl>,
    pub requests: mpsc::Receiver<MatchRequest>,
}
