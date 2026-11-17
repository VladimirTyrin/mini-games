use mg_core::Slot;

use crate::model::Word;

/// A guess made and not yet marked: only the authority, which knows the word, can mark it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingGuess {
    pub word: Word,
    pub by: Slot,
}
