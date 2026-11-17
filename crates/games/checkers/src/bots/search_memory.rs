/// Whether a search remembers the positions it has searched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchMemory {
    None,
    TranspositionTable,
}
