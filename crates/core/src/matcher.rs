use crate::frame::{GameFrame, Input};
use crate::frame_error::FrameError;
use crate::game::Game;
use crate::player::PlayerSpec;
use crate::setup_error::SetupError;
use crate::tick::Tick;

/// Receiving side of lockstep: applies authority frames (clients, observers, replays).
#[derive(Debug)]
pub struct Match<G: Game> {
    game: G,
    players: Vec<PlayerSpec>,
    next_tick: Tick,
}

impl<G: Game> Match<G> {
    pub fn new(settings: &G::Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        let game = G::new(settings, players)?;
        Ok(Self {
            game,
            players: players.to_vec(),
            next_tick: Tick::ZERO,
        })
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    pub fn players(&self) -> &[PlayerSpec] {
        &self.players
    }

    pub fn next_tick(&self) -> Tick {
        self.next_tick
    }

    /// After an error the state may be partially updated; rebuild from the full log.
    pub fn apply_frame(&mut self, frame: &GameFrame<G>) -> Result<(), FrameError> {
        if frame.tick != self.next_tick {
            return Err(FrameError::UnexpectedTick {
                expected: self.next_tick,
                got: frame.tick,
            });
        }
        let tick = frame.tick;
        for (input_index, input) in frame.inputs.iter().enumerate() {
            let result = match input {
                Input::Command { slot, command } => {
                    if slot.index() >= self.players.len() {
                        return Err(FrameError::UnknownSlot { tick, slot: *slot });
                    }
                    self.game.apply_command(*slot, command)
                }
                Input::World(event) => self.game.apply_world(event),
                Input::Dropped { slot } => {
                    if slot.index() >= self.players.len() {
                        return Err(FrameError::UnknownSlot { tick, slot: *slot });
                    }
                    self.game.drop_player(*slot);
                    Ok(())
                }
            };
            result.map_err(|error| FrameError::Rejected {
                tick,
                input_index,
                error,
            })?;
        }
        // Must mirror Authority::seal_frame exactly.
        if self.game.clock().is_realtime() && !self.game.status().is_finished() {
            self.game.step();
        }
        self.next_tick = tick.next();
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

    pub fn apply_frames<'a>(
        &mut self,
        frames: impl IntoIterator<Item = &'a GameFrame<G>>,
    ) -> Result<(), FrameError> {
        for frame in frames {
            self.apply_frame(frame)?;
        }
        Ok(())
    }
}
