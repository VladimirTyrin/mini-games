use std::time::Duration;

/// How a session retries after its connection drops. The first retry is immediate; later ones
/// back off exponentially up to `max_delay`. During a match the seat's disconnect budget (60 s
/// on the default server) bounds how long retrying is useful.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReconnectPolicy {
    pub attempts: u32,
    pub first_delay: Duration,
    pub max_delay: Duration,
}

impl ReconnectPolicy {
    pub const NEVER: Self = Self {
        attempts: 0,
        first_delay: Duration::ZERO,
        max_delay: Duration::ZERO,
    };

    /// Delay before retry number `retry` (0-based).
    pub(crate) fn delay_before(&self, retry: u32) -> Duration {
        if retry == 0 {
            return Duration::ZERO;
        }
        let factor = 1_u32.checked_shl(retry - 1).unwrap_or(u32::MAX);
        self.first_delay
            .checked_mul(factor)
            .unwrap_or(self.max_delay)
            .min(self.max_delay)
    }
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            attempts: 8,
            first_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(8),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backs_off_exponentially_up_to_the_cap() {
        let policy = ReconnectPolicy::default();
        let delays: Vec<u64> = (0..8)
            .map(|retry| policy.delay_before(retry).as_millis() as u64)
            .collect();
        assert_eq!(delays, [0, 250, 500, 1000, 2000, 4000, 8000, 8000]);
        assert_eq!(policy.delay_before(u32::MAX), policy.max_delay);
    }
}
