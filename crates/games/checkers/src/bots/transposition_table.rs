use super::TableEntry;

/// 2^18 entries, a few megabytes: allocated per decision, so nothing is held between moves.
const TABLE_BITS: u32 = 18;

/// Positions already searched, so a position reached by another move order is not searched
/// again and its best move is tried first. Always-replace: the newest result wins a slot.
pub(crate) struct TranspositionTable {
    entries: Vec<Option<TableEntry>>,
}

impl TranspositionTable {
    pub(crate) fn new() -> Self {
        Self {
            entries: vec![None; 1 << TABLE_BITS],
        }
    }

    pub(crate) fn probe(&self, key: u64) -> Option<TableEntry> {
        self.entries
            .get(Self::slot(key))
            .copied()
            .flatten()
            .filter(|entry| entry.key == key)
    }

    pub(crate) fn store(&mut self, entry: TableEntry) {
        if let Some(slot) = self.entries.get_mut(Self::slot(entry.key)) {
            *slot = Some(entry);
        }
    }

    fn slot(key: u64) -> usize {
        // The low bits of an FNV-1a hash are well mixed; the full key is kept to tell positions
        // sharing a slot apart.
        (key & ((1 << TABLE_BITS) - 1)) as usize
    }
}
