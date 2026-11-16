/// Which empty cells count as candidate moves: those at most this far (Chebyshev) from a stone.
/// Cells that complete or block a line are always adjacent to a stone, so inner search nodes use
/// the narrower set; the root and the random bot keep the legacy two-cell reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Neighbourhood {
    Adjacent,
    WithinTwo,
}
