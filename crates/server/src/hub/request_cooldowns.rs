use std::time::Duration;

use tokio::time::Instant;

use crate::limits::Limits;

/// Per-session cooldowns for requests whose answer is a whole match log or replay; they survive
/// reconnects because they live on the session.
#[derive(Debug, Default)]
pub(super) struct RequestCooldowns {
    last_replay_download: Option<Instant>,
    last_catch_up: Option<Instant>,
}

fn take(last: &mut Option<Instant>, cooldown: Duration, now: Instant) -> bool {
    if last.is_some_and(|at| now.saturating_duration_since(at) < cooldown) {
        return false;
    }
    *last = Some(now);
    true
}

impl RequestCooldowns {
    pub(super) fn try_replay_download(&mut self, now: Instant) -> bool {
        take(
            &mut self.last_replay_download,
            Limits::REPLAY_DOWNLOAD_COOLDOWN,
            now,
        )
    }

    /// Catch-up requests and desync reports (both answered with a catch-up).
    pub(super) fn try_catch_up(&mut self, now: Instant) -> bool {
        take(&mut self.last_catch_up, Limits::CATCH_UP_COOLDOWN, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_one_request_per_cooldown() {
        let start = Instant::now();
        let mut cooldowns = RequestCooldowns::default();
        assert!(cooldowns.try_catch_up(start));
        assert!(!cooldowns.try_catch_up(start + Limits::CATCH_UP_COOLDOWN / 2));
        assert!(cooldowns.try_replay_download(start));
        assert!(cooldowns.try_catch_up(start + Limits::CATCH_UP_COOLDOWN));
    }
}
