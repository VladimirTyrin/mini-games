/// Which deal a cell came from: 0 for the opening deal, `n` for the `n`-th refill. Shown only
/// (new numbers get their own colour); the rules never look at it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Batch(u8);

impl Batch {
    pub const DEALT: Self = Self(0);

    /// Refills past 255 share the last batch; the board cannot hold that many anyway.
    pub fn refill(refills_used: u32) -> Self {
        Self(u8::try_from(refills_used).unwrap_or(u8::MAX))
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}
