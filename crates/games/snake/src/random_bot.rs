use mg_core::{Bot, Budget, CoreRng, Slot};

use crate::bot_moves::{idle_seat, random_safe_turn};
use crate::command::Command;
use crate::snake::Snake;

#[derive(Debug)]
pub(crate) struct RandomBot;

impl Bot<Snake> for RandomBot {
    fn decide(
        &mut self,
        game: &Snake,
        me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Command> {
        let seat = idle_seat(game, me)?;
        random_safe_turn(game, me, seat, rng, budget)
    }
}
