//! [`Match`]: replays frames onto a game. Used by clients, replays, observers and tests.

use crate::error::{FrameError, SetupError};
use crate::frame::{Frame, Input};
use crate::game::{Clock, Game, PlayerSpec};

/// A game plus the lockstep bookkeeping needed to apply authority frames.
#[derive(Debug)]
pub struct Match<G: Game> {
    game: G,
    players: Vec<PlayerSpec>,
    next_tick: u64,
}

impl<G: Game> Match<G> {
    /// Creates the initial state.
    pub fn new(settings: &G::Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        let game = G::new(settings, players)?;
        Ok(Self {
            game,
            players: players.to_vec(),
            next_tick: 0,
        })
    }

    /// Current game state.
    pub fn game(&self) -> &G {
        &self.game
    }

    /// Seats.
    pub fn players(&self) -> &[PlayerSpec] {
        &self.players
    }

    /// Tick of the next frame to apply (= number of frames applied so far).
    pub fn next_tick(&self) -> u64 {
        self.next_tick
    }

    /// Applies one authority frame. On error the match is desynced and must be rebuilt from the
    /// full log (the state may be partially updated).
    pub fn apply_frame(
        &mut self,
        frame: &Frame<G::Command, G::WorldEvent>,
    ) -> Result<(), FrameError> {
        if frame.tick != self.next_tick {
            return Err(FrameError::UnexpectedTick {
                expected: self.next_tick,
                got: frame.tick,
            });
        }
        let tick = frame.tick;
        let seats = self.players.len();
        for (index, input) in frame.inputs.iter().enumerate() {
            let result = match input {
                Input::Command { slot, cmd } => {
                    if usize::from(*slot) >= seats {
                        return Err(FrameError::BadSlot { tick, slot: *slot });
                    }
                    self.game.apply_command(*slot, cmd)
                }
                Input::World(event) => self.game.apply_world(event),
                Input::Dropped { slot } => {
                    if usize::from(*slot) >= seats {
                        return Err(FrameError::BadSlot { tick, slot: *slot });
                    }
                    self.game.drop_player(*slot);
                    Ok(())
                }
            };
            result.map_err(|error| FrameError::Rejected { tick, index, error })?;
        }
        if matches!(self.game.clock(), Clock::Realtime { .. }) && !self.game.status().is_finished()
        {
            self.game.step();
        }
        self.next_tick += 1;
        if let Some(expected) = frame.hash {
            let actual = self.game.state_hash();
            if actual != expected {
                return Err(FrameError::HashMismatch {
                    tick,
                    expected,
                    actual,
                });
            }
        }
        Ok(())
    }

    /// Applies many frames.
    pub fn apply_all<'a, I>(&mut self, frames: I) -> Result<(), FrameError>
    where
        I: IntoIterator<Item = &'a Frame<G::Command, G::WorldEvent>>,
        G::Command: 'a,
        G::WorldEvent: 'a,
    {
        for frame in frames {
            self.apply_frame(frame)?;
        }
        Ok(())
    }
}
