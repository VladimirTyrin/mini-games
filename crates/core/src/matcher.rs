use std::borrow::Cow;

use crate::command_error::CommandError;
use crate::compact_log::CompactLog;
use crate::compact_log_entry::CompactLogEntry;
use crate::frame::{Frame, GameFrame};
use crate::frame_error::FrameError;
use crate::game::Game;
use crate::game_secret::SecretCommand;
use crate::input::Input;
use crate::logged_frame::LoggedFrame;
use crate::player_spec::PlayerSpec;
use crate::rng::CoreRng;
use crate::seat_check::seat_may_command;
use crate::setup_error::SetupError;
use crate::slot::Slot;
use crate::tick::Tick;

/// Why a [`Match`] could not apply a frame.
pub type MatchFrameError<G> = FrameError<CommandError<<G as Game>::Refusal>>;

/// Receiving side of lockstep: applies authority frames (clients, observers, replays).
#[derive(Debug)]
pub struct Match<G: Game> {
    game: G,
    players: Vec<PlayerSpec>,
    /// Seats the frames dropped, so [`Self::check_command`] refuses them as the authority does.
    dropped: Vec<bool>,
    next_tick: Tick,
}

impl<G: Game> Match<G> {
    pub fn new(settings: &G::Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        let game = G::new(settings, players)?;
        Ok(Self {
            game,
            dropped: vec![false; players.len()],
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

    /// What the authority would answer to `command` from `slot` on this state, without changing
    /// anything: its seat checks, then [`Game::check_command`].
    pub fn check_command(
        &self,
        slot: Slot,
        command: &G::Command,
    ) -> Result<(), CommandError<G::Refusal>> {
        seat_may_command(&self.game, &self.players, &self.dropped, slot)?;
        self.game.check_command(slot, command)
    }

    /// The game right after `command`, if the authority would accept it; see
    /// [`Game::preview_command`].
    pub fn preview_command(
        &self,
        slot: Slot,
        command: &G::Command,
    ) -> Result<G, CommandError<G::Refusal>> {
        seat_may_command(&self.game, &self.players, &self.dropped, slot)?;
        self.game.preview_command(slot, command)
    }

    /// What the authority would answer to a private command, as far as the public state tells:
    /// the game checks it against an empty secret. A client asks before sending its fleet.
    pub fn check_secret_command(
        &self,
        slot: Slot,
        command: &SecretCommand<G>,
    ) -> Result<(), CommandError<G::Refusal>> {
        seat_may_command(&self.game, &self.players, &self.dropped, slot)?;
        self.game
            .apply_secret_command(&mut G::Secret::default(), slot, command)
    }

    /// The private command the game would choose for `slot` (a random fleet), for a client that
    /// offers it as a start.
    pub fn default_secret_command(
        &self,
        slot: Slot,
        rng: &mut CoreRng,
    ) -> Option<SecretCommand<G>> {
        self.game.default_secret_command(slot, rng)
    }

    /// After an error the state may be partially updated; rebuild from the full log.
    pub fn apply_frame(&mut self, frame: &GameFrame<G>) -> Result<(), MatchFrameError<G>> {
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
                    if let Some(dropped) = self.dropped.get_mut(slot.index()) {
                        *dropped = true;
                    }
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
    ) -> Result<(), MatchFrameError<G>> {
        for frame in frames {
            self.apply_frame(frame)?;
        }
        Ok(())
    }

    /// Applies the log's ticks from [`Self::next_tick`] up to `stop` (exclusive, capped at the
    /// log end), so a replay can step through one log in several calls.
    pub fn apply_compact_log(
        &mut self,
        log: &CompactLog<GameFrame<G>>,
        stop: Tick,
    ) -> Result<(), MatchFrameError<G>> {
        self.apply_compact_log_with(log, stop, |frame| {
            Ok::<_, MatchFrameError<G>>(Cow::Borrowed(frame))
        })
    }

    /// [`Self::apply_compact_log`] for logs of another frame type: `decode` turns each logged
    /// frame into a domain frame.
    pub fn apply_compact_log_with<'a, F, E>(
        &mut self,
        log: &'a CompactLog<F>,
        stop: Tick,
        mut decode: impl FnMut(&'a F) -> Result<Cow<'a, GameFrame<G>>, E>,
    ) -> Result<(), E>
    where
        F: LoggedFrame,
        E: From<MatchFrameError<G>>,
    {
        let entries = log
            .entries_from(self.next_tick)
            .take_while(|entry| entry.tick() < stop);
        for entry in entries {
            match entry {
                CompactLogEntry::Logged(frame) => self.apply_frame(decode(frame)?.as_ref())?,
                // Once finished, the authority hashes, and therefore logs, every frame.
                CompactLogEntry::Empty(tick) if self.game.status().is_finished() => {
                    return Err(FrameError::MissingFrameAfterFinish { tick }.into());
                }
                CompactLogEntry::Empty(tick) => self.apply_frame(&Frame::empty(tick))?,
            }
        }
        Ok(())
    }
}
