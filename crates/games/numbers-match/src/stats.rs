/// End-of-game counters (legacy `NumbersMatchGameEndInfo`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub pairs_removed: u32,
    pub refills_used: u32,
    pub hints_used: u32,
}
