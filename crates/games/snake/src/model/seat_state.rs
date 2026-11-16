use std::collections::VecDeque;

use mg_core::{CommandError, PlayerKind};

use crate::io::INPUT_BUFFER;
use crate::model::{Direction, Point, Score};
use crate::rules::DeathReason;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SeatState {
    pub(crate) player: PlayerKind,
    /// Head first; never empty.
    pub(crate) body: VecDeque<Point>,
    pub(crate) direction: Direction,
    pub(crate) queued_turns: Vec<Direction>,
    pub(crate) death: Option<DeathReason>,
    pub(crate) score: Score,
}

impl SeatState {
    pub(crate) fn new(player: PlayerKind, body: VecDeque<Point>, direction: Direction) -> Self {
        Self {
            player,
            body,
            direction,
            queued_turns: Vec::with_capacity(INPUT_BUFFER),
            death: None,
            score: Score::ZERO,
        }
    }

    pub(crate) fn is_alive(&self) -> bool {
        self.death.is_none()
    }

    pub(crate) fn head(&self) -> Option<Point> {
        self.body.front().copied()
    }

    pub(crate) fn tail(&self) -> Option<Point> {
        self.body.back().copied()
    }

    /// Direction the snake will have once every queued turn is consumed.
    pub(crate) fn intended_direction(&self) -> Direction {
        self.queued_turns.last().copied().unwrap_or(self.direction)
    }

    pub(crate) fn queue_turn(&mut self, direction: Direction) -> Result<(), CommandError> {
        if !self.is_alive() {
            return Err(CommandError::NotPlaying);
        }
        if self.queued_turns.len() >= INPUT_BUFFER {
            return Err(CommandError::Busy);
        }
        let intended = self.intended_direction();
        if direction == intended {
            return Err(CommandError::Illegal("already heading that way"));
        }
        if direction.is_opposite(intended) {
            return Err(CommandError::Illegal("cannot reverse"));
        }
        self.queued_turns.push(direction);
        Ok(())
    }

    pub(crate) fn consume_queued_turn(&mut self) {
        if !self.queued_turns.is_empty() {
            self.direction = self.queued_turns.remove(0);
        }
    }
}
