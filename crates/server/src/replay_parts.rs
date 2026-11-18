use std::sync::Arc;

use mg_protocol::proto;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::limits::Limits;

/// A shared replay arriving in parts (`ShareReplay.more_parts`), so that no message, and no
/// socket, ever needs room for a whole replay. The bytes held are reserved from one server-wide
/// budget: many half-sent uploads cannot run the host out of memory.
#[derive(Debug, Default)]
pub(crate) struct ReplayParts {
    received: Vec<u8>,
    reserved: Option<OwnedSemaphorePermit>,
}

/// What a received part leads to.
#[derive(Debug)]
pub(crate) enum ReplayProgress {
    /// More parts are to come.
    Partial,
    /// The last part: the whole replay, to be checked and shared.
    Complete(proto::ShareReplay),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum ReplayPartsError {
    #[error("replay is too large")]
    TooLarge,
    #[error("too many replays are being uploaded; try again later")]
    ServerBusy,
}

impl ReplayParts {
    /// Adds a part. On an error the upload so far is dropped and the next part starts anew.
    pub(crate) fn add(
        &mut self,
        part: proto::ShareReplay,
        budget: &Arc<Semaphore>,
    ) -> Result<ReplayProgress, ReplayPartsError> {
        let received = std::mem::take(&mut self.received);
        let reserved = self.reserved.take();
        if received.is_empty() && !part.more_parts {
            // The whole replay in one message: nothing to hold between messages.
            return Ok(ReplayProgress::Complete(part));
        }
        let total = received.len() + part.replay_file.len();
        if total > Limits::MAX_REPLAY_BYTES {
            return Err(ReplayPartsError::TooLarge);
        }
        let more = u32::try_from(part.replay_file.len()).map_err(|_| ReplayPartsError::TooLarge)?;
        let Ok(more_reserved) = budget.clone().try_acquire_many_owned(more) else {
            return Err(ReplayPartsError::ServerBusy);
        };
        let reserved = match reserved {
            Some(mut reserved) => {
                reserved.merge(more_reserved);
                reserved
            }
            None => more_reserved,
        };
        let mut received = received;
        received.extend_from_slice(&part.replay_file);
        if part.more_parts {
            self.received = received;
            self.reserved = Some(reserved);
            return Ok(ReplayProgress::Partial);
        }
        Ok(ReplayProgress::Complete(proto::ShareReplay {
            replay_file: received,
            host_only_control: part.host_only_control,
            more_parts: false,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(bytes: &[u8], more_parts: bool) -> proto::ShareReplay {
        proto::ShareReplay {
            replay_file: bytes.to_vec(),
            host_only_control: true,
            more_parts,
        }
    }

    #[test]
    fn parts_are_joined_in_order() {
        let budget = Arc::new(Semaphore::new(100));
        let mut parts = ReplayParts::default();
        assert!(matches!(
            parts.add(part(b"ab", true), &budget),
            Ok(ReplayProgress::Partial)
        ));
        let Ok(ReplayProgress::Complete(replay)) = parts.add(part(b"cd", false), &budget) else {
            panic!("the last part completes the replay");
        };
        assert_eq!(replay.replay_file, b"abcd");
        assert_eq!(budget.available_permits(), 100, "the budget is given back");
    }

    #[test]
    fn half_sent_uploads_cannot_hold_more_than_the_budget() {
        let budget = Arc::new(Semaphore::new(5));
        let mut first = ReplayParts::default();
        let mut second = ReplayParts::default();
        assert!(first.add(part(b"abcd", true), &budget).is_ok());
        assert_eq!(
            second.add(part(b"ab", true), &budget).unwrap_err(),
            ReplayPartsError::ServerBusy
        );
        drop(first);
        assert!(second.add(part(b"ab", true), &budget).is_ok());
    }

    #[test]
    fn a_replay_over_the_limit_is_refused_and_forgotten() {
        let budget = Arc::new(Semaphore::new(Limits::MAX_REPLAY_BYTES * 2));
        let mut parts = ReplayParts::default();
        let half = vec![0; Limits::MAX_REPLAY_BYTES / 2 + 1];
        assert!(parts.add(part(&half, true), &budget).is_ok());
        assert_eq!(
            parts.add(part(&half, true), &budget).unwrap_err(),
            ReplayPartsError::TooLarge
        );
        assert_eq!(budget.available_permits(), Limits::MAX_REPLAY_BYTES * 2);
    }
}
