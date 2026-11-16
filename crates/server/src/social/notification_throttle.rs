use std::collections::HashMap;
use std::time::Duration;

use tokio::time::Instant;

use super::AccountId;

/// What a notification is about, for [`NotificationThrottle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NotificationKind {
    FriendRequest,
    NewFriend,
    Invitation,
}

impl NotificationKind {
    /// How long one account is spared a second notification of this kind from the same sender:
    /// requesting, withdrawing and requesting again must not turn into a stream of pushes.
    fn quiet_period(self) -> Duration {
        match self {
            Self::FriendRequest | Self::NewFriend => Duration::from_secs(24 * 3600),
            // Friends asked each other in; a repeat after a cancel is fine, a burst is not.
            Self::Invitation => Duration::from_secs(10),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NotificationKey {
    kind: NotificationKind,
    from: AccountId,
    to: AccountId,
}

/// Remembers recent notifications per sender and recipient. Bounded: entries go once their quiet
/// period ends ([`Self::forget_expired`]), and past `MAX_ENTRIES` the oldest go first.
#[derive(Debug, Default)]
pub(crate) struct NotificationThrottle {
    sent: HashMap<NotificationKey, Instant>,
}

impl NotificationThrottle {
    const MAX_ENTRIES: usize = 50_000;

    /// Whether a notification may go out now; if so, it counts as sent.
    pub(crate) fn allow(
        &mut self,
        kind: NotificationKind,
        from: &AccountId,
        to: &AccountId,
        now: Instant,
    ) -> bool {
        let key = NotificationKey {
            kind,
            from: from.clone(),
            to: to.clone(),
        };
        if self
            .sent
            .get(&key)
            .is_some_and(|sent| now.duration_since(*sent) < kind.quiet_period())
        {
            return false;
        }
        if self.sent.len() >= Self::MAX_ENTRIES {
            self.forget_expired(now);
        }
        if self.sent.len() >= Self::MAX_ENTRIES
            && let Some(oldest) = self
                .sent
                .iter()
                .min_by_key(|(_, sent)| **sent)
                .map(|(key, _)| key.clone())
        {
            self.sent.remove(&oldest);
        }
        self.sent.insert(key, now);
        true
    }

    pub(crate) fn forget_expired(&mut self, now: Instant) {
        self.sent
            .retain(|key, sent| now.duration_since(*sent) < key.kind.quiet_period());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(digit: char) -> AccountId {
        AccountId::parse(&format!("g_{}", digit.to_string().repeat(32))).unwrap()
    }

    #[test]
    fn a_repeated_friend_request_notifies_once_a_day() {
        let mut throttle = NotificationThrottle::default();
        let (ada, bob, start) = (account('a'), account('b'), Instant::now());

        let first = throttle.allow(NotificationKind::FriendRequest, &ada, &bob, start);
        let soon = throttle.allow(
            NotificationKind::FriendRequest,
            &ada,
            &bob,
            start + Duration::from_secs(60),
        );
        let next_day = throttle.allow(
            NotificationKind::FriendRequest,
            &ada,
            &bob,
            start + Duration::from_secs(24 * 3600),
        );

        assert!(first);
        assert!(!soon);
        assert!(next_day);
    }

    #[test]
    fn other_senders_and_kinds_are_not_held_back() {
        let mut throttle = NotificationThrottle::default();
        let (ada, bob, cid, now) = (account('a'), account('b'), account('c'), Instant::now());

        throttle.allow(NotificationKind::FriendRequest, &ada, &bob, now);

        assert!(throttle.allow(NotificationKind::FriendRequest, &cid, &bob, now));
        assert!(throttle.allow(NotificationKind::Invitation, &ada, &bob, now));
    }
}
