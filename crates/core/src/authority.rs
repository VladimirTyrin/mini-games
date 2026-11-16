//! [`Authority`]: the single source of truth for a match (server, or the local host in WASM).
//!
//! It validates commands as they arrive, runs bots, resolves randomness and seals frames. Every
//! frame it emits replays to an identical state on a [`crate::Match`]; the conformance suite
//! checks that for every game.

use crate::bot::{Bot, Budget};
use crate::error::{CommandError, SetupError};
use crate::frame::{Frame, GameFrame, GameInput, Input};
use crate::game::{Awaiting, Clock, Game, PlayerKind, PlayerSpec, Slot};
use crate::rng::CoreRng;

/// How often sealed frames carry a state hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HashPolicy {
    /// Hash every N-th frame (N >= 1). The final frame always carries a hash.
    pub every: u64,
}

impl HashPolicy {
    /// Sensible default per clock: every 10th tick for real-time, every move for turn-based.
    pub fn for_clock(clock: Clock) -> Self {
        match clock {
            Clock::TurnBased => Self { every: 1 },
            Clock::Realtime { .. } => Self { every: 10 },
        }
    }
}

/// A seated bot.
pub struct BotSeat<G: Game> {
    /// Seat it plays.
    pub slot: Slot,
    /// The bot.
    pub bot: Box<dyn Bot<G>>,
    /// Work budget per decision.
    pub budget: u64,
}

impl<G: Game> std::fmt::Debug for BotSeat<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotSeat")
            .field("slot", &self.slot)
            .field("bot", &self.bot)
            .finish()
    }
}

#[derive(Debug)]
struct SeatedBot<G: Game> {
    slot: Slot,
    bot: Box<dyn Bot<G>>,
    rng: CoreRng,
    budget: u64,
}

/// Authoritative match state.
#[derive(Debug)]
pub struct Authority<G: Game> {
    game: G,
    players: Vec<PlayerSpec>,
    world_rng: CoreRng,
    bots: Vec<SeatedBot<G>>,
    dropped: Vec<bool>,
    pending: Vec<GameInput<G>>,
    next_tick: u64,
    hash_policy: HashPolicy,
    /// Non-empty (or hashed) frames only; empty frames are implied by tick gaps.
    log: Vec<GameFrame<G>>,
}

impl<G: Game> Authority<G> {
    /// Creates a match. Every `PlayerKind::Bot` seat needs exactly one entry in `bots`.
    pub fn new(
        settings: &G::Settings,
        players: &[PlayerSpec],
        seed: u64,
        bots: Vec<BotSeat<G>>,
    ) -> Result<Self, SetupError> {
        let game = G::new(settings, players)?;
        for (i, spec) in players.iter().enumerate() {
            let seated = bots.iter().filter(|b| usize::from(b.slot) == i).count();
            let expected = usize::from(spec.kind == PlayerKind::Bot);
            if seated != expected {
                return Err(SetupError::InvalidPlayers(format!(
                    "seat {i}: {} bot(s) provided for a {:?} seat",
                    seated, spec.kind
                )));
            }
        }
        let bots = bots
            .into_iter()
            .map(|b| SeatedBot {
                slot: b.slot,
                bot: b.bot,
                rng: CoreRng::derive(seed, 1 + u64::from(b.slot)),
                budget: b.budget,
            })
            .collect();
        let hash_policy = HashPolicy::for_clock(game.clock());
        Ok(Self {
            game,
            players: players.to_vec(),
            world_rng: CoreRng::derive(seed, 0),
            bots,
            dropped: vec![false; players.len()],
            pending: Vec::new(),
            next_tick: 0,
            hash_policy,
            log: Vec::new(),
        })
    }

    /// Overrides how often frames carry a hash.
    pub fn set_hash_policy(&mut self, policy: HashPolicy) {
        self.hash_policy = HashPolicy {
            every: policy.every.max(1),
        };
    }

    /// Current state.
    pub fn game(&self) -> &G {
        &self.game
    }

    /// Seats.
    pub fn players(&self) -> &[PlayerSpec] {
        &self.players
    }

    /// Tick the next sealed frame will carry.
    pub fn next_tick(&self) -> u64 {
        self.next_tick
    }

    /// Compact log of sealed frames (empty frames omitted).
    pub fn log(&self) -> &[GameFrame<G>] {
        &self.log
    }

    /// `true` if inputs are waiting to be sealed.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// `true` when a turn-based game waits for one of our bots.
    pub fn bot_to_move(&self) -> bool {
        match self.game.awaiting() {
            Awaiting::Turn(slot) => self.is_live_bot(slot),
            Awaiting::Simultaneous => self.bots.iter().any(|b| !self.is_dropped(b.slot)),
            Awaiting::Nobody => false,
        }
    }

    fn is_dropped(&self, slot: Slot) -> bool {
        self.dropped.get(usize::from(slot)).copied().unwrap_or(true)
    }

    fn is_live_bot(&self, slot: Slot) -> bool {
        !self.is_dropped(slot) && self.bots.iter().any(|b| b.slot == slot)
    }

    /// Submits a human command. Applied immediately; included in the next sealed frame.
    pub fn submit(&mut self, slot: Slot, cmd: G::Command) -> Result<(), CommandError> {
        match self.players.get(usize::from(slot)) {
            Some(spec) if spec.kind == PlayerKind::Human && !self.is_dropped(slot) => {}
            _ => return Err(CommandError::NotPlaying),
        }
        if self.game.status().is_finished() {
            return Err(CommandError::GameOver);
        }
        self.game.apply_command(slot, &cmd)?;
        self.pending.push(Input::Command { slot, cmd });
        Ok(())
    }

    /// Removes a seat from play for good (bots included).
    pub fn drop_player(&mut self, slot: Slot) {
        let Some(flag) = self.dropped.get_mut(usize::from(slot)) else {
            return;
        };
        if *flag {
            return;
        }
        *flag = true;
        self.game.drop_player(slot);
        self.pending.push(Input::Dropped { slot });
    }

    /// Lets bots act. Real-time: every live bot decides once. Turn-based: the bot whose turn it
    /// is decides; a bot that fails to produce a legal move forfeits (dropped) so the game can
    /// never stall.
    pub fn run_bots(&mut self) {
        if self.game.status().is_finished() {
            return;
        }
        match self.game.awaiting() {
            Awaiting::Nobody => {}
            Awaiting::Turn(slot) => {
                if self.is_live_bot(slot) && !self.bot_act(slot) {
                    self.drop_player(slot);
                }
            }
            Awaiting::Simultaneous => {
                let slots: Vec<Slot> = self.bots.iter().map(|b| b.slot).collect();
                for slot in slots {
                    if self.game.status().is_finished() {
                        break;
                    }
                    if !self.is_dropped(slot) {
                        self.bot_act(slot);
                    }
                }
            }
        }
    }

    /// Runs one bot; returns `true` if it produced an accepted command.
    fn bot_act(&mut self, slot: Slot) -> bool {
        let Some(seat) = self.bots.iter_mut().find(|b| b.slot == slot) else {
            return false;
        };
        let mut budget = Budget::new(seat.budget);
        let Some(cmd) = seat
            .bot
            .decide(&self.game, slot, &mut seat.rng, &mut budget)
        else {
            return false;
        };
        if self.game.apply_command(slot, &cmd).is_ok() {
            self.pending.push(Input::Command { slot, cmd });
            true
        } else {
            false
        }
    }

    /// Resolves randomness, advances the clock and emits the frame for this tick.
    pub fn seal(&mut self) -> GameFrame<G> {
        let mut inputs = std::mem::take(&mut self.pending);
        if !self.game.status().is_finished() {
            for event in self.game.roll_world(&mut self.world_rng) {
                if self.game.apply_world(&event).is_ok() {
                    inputs.push(Input::World(event));
                } else {
                    debug_assert!(
                        false,
                        "roll_world produced an event apply_world rejects: {event:?}"
                    );
                }
            }
        }
        if matches!(self.game.clock(), Clock::Realtime { .. }) && !self.game.status().is_finished()
        {
            self.game.step();
        }
        let tick = self.next_tick;
        self.next_tick += 1;
        let finished = self.game.status().is_finished();
        let hash = (finished || self.next_tick.is_multiple_of(self.hash_policy.every))
            .then(|| self.game.state_hash());
        let frame = Frame { tick, inputs, hash };
        if !frame.inputs.is_empty() || frame.hash.is_some() {
            self.log.push(frame.clone());
        }
        frame
    }
}

/// Expands a compact log (empty frames omitted) into a gap-free frame sequence up to
/// `end_tick` (exclusive).
pub fn expand_log<C: Clone, W: Clone>(compact: &[Frame<C, W>], end_tick: u64) -> Vec<Frame<C, W>> {
    let mut out = Vec::new();
    let mut tick = 0;
    for frame in compact {
        while tick < frame.tick && tick < end_tick {
            out.push(Frame {
                tick,
                inputs: Vec::new(),
                hash: None,
            });
            tick += 1;
        }
        if frame.tick >= end_tick {
            break;
        }
        out.push(frame.clone());
        tick = frame.tick + 1;
    }
    while tick < end_tick {
        out.push(Frame {
            tick,
            inputs: Vec::new(),
            hash: None,
        });
        tick += 1;
    }
    out
}
