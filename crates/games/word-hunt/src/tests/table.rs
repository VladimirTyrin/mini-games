use std::ops::Deref;

use mg_core::{CommandError, CoreRng, Game, Slot};

use crate::{Command, HiddenWord, Word, WordHunt, WorldEvent};

/// A game run the way the authority runs it, with the word set by the test instead of drawn:
/// after every command it gets what the authority would send (the marks, the word at the end).
pub(crate) struct Table {
    game: WordHunt,
    secret: HiddenWord,
    rng: CoreRng,
    is_revealed: bool,
}

impl Table {
    /// `game` must not have its word yet. Nobody is on turn in a group until [`Self::roll`] or
    /// a `FirstPlayerChosen` from the test.
    pub(crate) fn new(mut game: WordHunt, answer: &str) -> Self {
        let settings = *game.settings();
        let answer = Word::parse(answer, settings.language, settings.word_length()).unwrap();
        game.apply_world(&WorldEvent::WordChosen).unwrap();
        Self {
            game,
            secret: HiddenWord {
                answer: Some(answer),
            },
            rng: CoreRng::new(1),
            is_revealed: false,
        }
    }

    pub(crate) fn command(&mut self, slot: Slot, command: &Command) -> Result<(), CommandError> {
        self.game.apply_command(slot, command)?;
        self.roll();
        Ok(())
    }

    pub(crate) fn world(&mut self, event: &WorldEvent) -> Result<(), CommandError> {
        self.game.apply_world(event)
    }

    pub(crate) fn drop_player(&mut self, slot: Slot) {
        self.game.drop_player(slot);
        self.roll();
    }

    /// What the authority does when it seals a frame.
    pub(crate) fn roll(&mut self) {
        if !self.game.status().is_finished() {
            for event in self.game.roll_secret(&mut self.secret, &mut self.rng) {
                self.game.apply_world(&event).unwrap();
            }
        }
        if !self.game.status().is_finished() {
            for event in self.game.roll_world(&mut self.rng) {
                self.game.apply_world(&event).unwrap();
            }
        }
        if self.game.status().is_finished() && !self.is_revealed {
            self.is_revealed = true;
            for event in self.game.reveal(&self.secret) {
                self.game.apply_world(&event).unwrap();
            }
        }
    }
}

impl Deref for Table {
    type Target = WordHunt;

    fn deref(&self) -> &WordHunt {
        &self.game
    }
}
