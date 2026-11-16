use std::time::Duration;

use tokio::time::Instant;

/// Classic token bucket; time is passed in so tests control the clock.
#[derive(Clone, Debug)]
pub(crate) struct TokenBucket {
    capacity: u32,
    refill_every: Duration,
    tokens: u32,
    last_refill: Instant,
}

impl TokenBucket {
    /// Allows bursts of `capacity` and a sustained rate of one token per `refill_every`.
    pub(crate) fn new(capacity: u32, refill_every: Duration, now: Instant) -> Self {
        Self {
            capacity,
            refill_every,
            tokens: capacity,
            last_refill: now,
        }
    }

    pub(crate) fn try_take(&mut self, now: Instant) -> bool {
        self.refill(now);
        if self.tokens == 0 {
            return false;
        }
        self.tokens -= 1;
        true
    }

    fn refill(&mut self, now: Instant) {
        if self.refill_every.is_zero() {
            self.tokens = self.capacity;
            return;
        }
        let elapsed = now.saturating_duration_since(self.last_refill);
        let earned = elapsed.as_nanos() / self.refill_every.as_nanos();
        if earned == 0 {
            return;
        }
        let earned = u32::try_from(earned).unwrap_or(u32::MAX);
        self.tokens = self.tokens.saturating_add(earned).min(self.capacity);
        // Advance by whole tokens only, so fractional progress is not lost.
        self.last_refill += self.refill_every * earned.min(self.capacity.saturating_add(1));
        if self.tokens == self.capacity {
            self.last_refill = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_burst_then_limits_then_refills() {
        let start = Instant::now();
        let mut bucket = TokenBucket::new(3, Duration::from_millis(100), start);
        assert!((0..3).all(|_| bucket.try_take(start)));
        assert!(!bucket.try_take(start));
        assert!(!bucket.try_take(start + Duration::from_millis(99)));
        assert!(bucket.try_take(start + Duration::from_millis(100)));
        assert!(!bucket.try_take(start + Duration::from_millis(150)));
        let later = start + Duration::from_secs(10);
        assert!((0..3).all(|_| bucket.try_take(later)));
        assert!(!bucket.try_take(later));
    }
}
