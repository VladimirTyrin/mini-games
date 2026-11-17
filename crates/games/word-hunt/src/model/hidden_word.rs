use crate::model::Word;

/// The answer, known only to the authority until the game is over (see `mg_core::Game::Secret`).
#[derive(Clone, Debug, Default)]
pub struct HiddenWord {
    pub(crate) answer: Option<Word>,
}
