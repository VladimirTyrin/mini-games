/// What placing a stone of one side on an empty cell would do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SideProbe {
    /// Change of the heuristic evaluation, from this side's point of view.
    pub(crate) gain: i64,
    pub(crate) completes_line: bool,
}
