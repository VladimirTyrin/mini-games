use tokio::sync::mpsc;

use super::match_busy::MatchBusy;
use super::match_control::MatchControl;
use super::match_inbox::MatchInbox;
use super::match_request::MatchRequest;
use crate::ids::MatchId;

/// Lifecycle commands go through an unbounded queue (bounded in practice by the number of
/// participants), so a backed-up actor never loses an attach, leave or stop; requests share a
/// bounded queue and are shed when it is full.
#[derive(Clone, Debug)]
pub(crate) struct MatchHandle {
    pub match_id: MatchId,
    control: mpsc::UnboundedSender<MatchControl>,
    requests: mpsc::Sender<MatchRequest>,
}

impl MatchHandle {
    pub(crate) fn channel(match_id: MatchId, request_capacity: usize) -> (Self, MatchInbox) {
        let (control, control_inbox) = mpsc::unbounded_channel();
        let (requests, request_inbox) = mpsc::channel(request_capacity);
        let handle = Self {
            match_id,
            control,
            requests,
        };
        let inbox = MatchInbox {
            control: control_inbox,
            requests: request_inbox,
        };
        (handle, inbox)
    }

    pub(crate) fn control(&self, command: MatchControl) {
        if self.control.send(command).is_err() {
            tracing::debug!(match_id = %self.match_id, "match actor already gone");
        }
    }

    /// Never blocks the hub. A gone actor is not an error: its end is reported separately.
    pub(crate) fn request(&self, request: MatchRequest) -> Result<(), MatchBusy> {
        match self.requests.try_send(request) {
            Ok(()) | Err(mpsc::error::TrySendError::Closed(_)) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => Err(MatchBusy),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::SessionId;

    #[test]
    fn lifecycle_commands_get_through_when_requests_are_shed() {
        let (handle, mut inbox) = MatchHandle::channel(MatchId::random(), 2);
        let session = SessionId::random();
        let catch_up = || MatchRequest::CatchUp {
            session: session.clone(),
        };
        assert_eq!(handle.request(catch_up()), Ok(()));
        assert_eq!(handle.request(catch_up()), Ok(()));
        assert_eq!(handle.request(catch_up()), Err(MatchBusy));
        handle.control(MatchControl::Leave {
            session: session.clone(),
        });
        handle.control(MatchControl::Stop);
        assert!(matches!(
            inbox.control.try_recv(),
            Ok(MatchControl::Leave { .. })
        ));
        assert!(matches!(inbox.control.try_recv(), Ok(MatchControl::Stop)));
    }
}
