use super::candidate::Candidate;
use super::score::Score;

#[derive(Clone, Copy, Debug)]
pub(crate) struct RootMove {
    pub(crate) candidate: Candidate,
    pub(crate) score: Score,
}
