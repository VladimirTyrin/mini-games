//! Deterministic, dependency-free PRNG.
//!
//! The algorithm is pinned (xoshiro256** seeded through splitmix64) so the same seed yields the
//! same stream on every platform and every release. Only the match authority owns a [`CoreRng`];
//! game state never sees one outside [`crate::Game::roll_world`] and bots.

/// xoshiro256** generator with unbiased bounded sampling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreRng {
    s: [u64; 4],
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl CoreRng {
    /// Creates a generator from a 64-bit seed.
    pub fn new(seed: u64) -> Self {
        let mut sm = seed;
        let s = [
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
        ];
        Self { s }
    }

    /// Derives an independent stream (e.g. one per bot) from this generator's seed material.
    pub fn derive(seed: u64, stream: u64) -> Self {
        let mut sm = seed ^ stream.wrapping_mul(0xD1B5_4A32_D192_ED03);
        Self::new(splitmix64(&mut sm))
    }

    /// Next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Next raw 32-bit value (upper bits of [`Self::next_u64`]).
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform value in `0..bound` (Lemire's nearly-divisionless method). Returns 0 for `bound == 0`.
    pub fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        let mut m = u64::from(self.next_u32()) * u64::from(bound);
        let mut low = m as u32;
        if low < bound {
            let threshold = bound.wrapping_neg() % bound;
            while low < threshold {
                m = u64::from(self.next_u32()) * u64::from(bound);
                low = m as u32;
            }
        }
        (m >> 32) as u32
    }

    /// Uniform value in `lo..=hi`. Returns `lo` if `hi < lo`.
    pub fn range_inclusive(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        match (hi - lo).checked_add(1) {
            Some(span) => lo + self.below(span),
            None => self.next_u32(),
        }
    }

    /// Uniform index into a slice of length `len`; `None` when empty.
    pub fn index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        let bound = u32::try_from(len).unwrap_or(u32::MAX);
        Some(self.below(bound) as usize)
    }

    /// Picks a uniformly random element.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        self.index(items.len()).and_then(|i| items.get(i))
    }

    /// `true` with probability `numerator / denominator` (exact, integer-only).
    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        if denominator == 0 || numerator == 0 {
            return false;
        }
        if numerator >= denominator {
            return true;
        }
        self.below(denominator) < numerator
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let bound = u32::try_from(i + 1).unwrap_or(u32::MAX);
            let j = self.below(bound) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_is_pinned() {
        // Golden values: changing them breaks determinism guarantees for everyone.
        let mut rng = CoreRng::new(42);
        let got: Vec<u64> = (0..4).map(|_| rng.next_u64()).collect();
        let mut again = CoreRng::new(42);
        let expected: Vec<u64> = (0..4).map(|_| again.next_u64()).collect();
        assert_eq!(got, expected);
        assert_eq!(got[0], GOLDEN_FIRST);
    }

    const GOLDEN_FIRST: u64 = 0x1578_0B2E_0C2E_C716;

    #[test]
    fn below_is_in_range_and_covers_values() {
        let mut rng = CoreRng::new(7);
        let mut seen = [false; 7];
        for _ in 0..1000 {
            let v = rng.below(7) as usize;
            assert!(v < 7);
            seen[v] = true;
        }
        assert!(seen.iter().all(|&s| s));
        assert_eq!(rng.below(0), 0);
        assert_eq!(rng.range_inclusive(5, 5), 5);
        assert_eq!(rng.range_inclusive(9, 3), 9);
        let v = rng.range_inclusive(0, u32::MAX);
        let _ = v;
    }

    #[test]
    fn chance_edges() {
        let mut rng = CoreRng::new(1);
        assert!(!rng.chance(0, 10));
        assert!(rng.chance(10, 10));
        assert!(!rng.chance(5, 0));
        let hits = (0..10_000).filter(|_| rng.chance(1, 4)).count();
        assert!((2200..2800).contains(&hits), "{hits}");
    }

    #[test]
    fn derived_streams_differ() {
        let mut a = CoreRng::derive(99, 1);
        let mut b = CoreRng::derive(99, 2);
        assert_ne!(a.next_u64(), b.next_u64());
    }
}
