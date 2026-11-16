use super::probe::Probe;
use crate::point::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub(crate) index: usize,
    pub(crate) point: Point,
    pub(crate) probe: Probe,
}
