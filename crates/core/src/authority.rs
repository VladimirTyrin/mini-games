use crate::awaiting::Awaiting;
use crate::bot_seat::BotSeat;
use crate::budget::Budget;
use crate::command_error::CommandError;
use crate::frame::{Frame, GameFrame};
use crate::game::Game;
use crate::hash_policy::HashPolicy;
use crate::input::{GameInput, Input};
use crate::player_kind::PlayerKind;
use crate::player_spec::PlayerSpec;
use crate::rng::CoreRng;
use crate::seat_check::seat_may_command;
use crate::seated_bot::SeatedBot;
use crate::setup_error::SetupError;
use crate::slot::Slot;
use crate::tick::Tick;

/// Single source of truth for a match (the server, or the local host in the browser): validates
/// commands, runs bots, resolves randomness and seals frames. Every sealed frame must replay to
/// the identical state on a [`crate::Match`]; the conformance suite checks this for every game.
#[derive(Debug)]
pub struct Authority<G: Game> {
    game: G,
    players: Vec<PlayerSpec>,
    world_rng: CoreRng,
    bots: Vec<SeatedBot<G>>,
    dropped: Vec<bool>,
    pending_inputs: Vec<GameInput<G>>,
    next_tick: Tick,
    hash_policy: HashPolicy,
    /// See [`crate::CompactLog`]. `None` once the host keeps its own log.
    compact_log: Option<Vec<GameFrame<G>>>,
    rejected_bot_commands: u64,
}

impl<G: Game> Authority<G> {
    /// Every bot seat needs exactly one entry in `bots`. World and bot RNG streams are derived
    /// from `seed`, which never leaves the authority.
    pub fn new(
        settings: &G::Settings,
        players: &[PlayerSpec],
        seed: u64,
        bots: Vec<BotSeat<G>>,
    ) -> Result<Self, SetupError> {
        let game = G::new(settings, players)?;
        for (slot, spec) in Slot::all(players.len()).zip(players) {
            let seated = bots.iter().filter(|b| b.slot == slot).count();
            let expected = usize::from(spec.kind == PlayerKind::Bot);
            if seated != expected {
                return Err(SetupError::InvalidPlayers(format!(
                    "seat {slot}: {seated} bot(s) provided for a {:?} seat",
                    spec.kind
                )));
            }
        }
        let bots = bots
            .into_iter()
            .map(|seat| SeatedBot {
                slot: seat.slot,
                bot: seat.bot,
                rng: CoreRng::derive(seed, 1 + u64::from(seat.slot.get())),
                budget_nodes: seat.budget_nodes,
            })
            .collect();
        let hash_policy = HashPolicy::for_clock(game.clock());
        Ok(Self {
            game,
            players: players.to_vec(),
            world_rng: CoreRng::derive(seed, 0),
            bots,
            dropped: vec![false; players.len()],
            pending_inputs: Vec::new(),
            next_tick: Tick::ZERO,
            hash_policy,
            compact_log: Some(Vec::new()),
            rejected_bot_commands: 0,
        })
    }

    /// Must not hash less often than [`HashPolicy::for_clock`]: receivers bound the omitted
    /// runs of a compact log by it.
    pub fn set_hash_policy(&mut self, policy: HashPolicy) {
        self.hash_policy = policy;
    }

    pub fn hash_policy(&self) -> HashPolicy {
        self.hash_policy
    }

    /// For hosts that keep their own log (the registry keeps wire frames): sealing then stops
    /// cloning frames into [`Self::compact_log`], which stays empty.
    pub fn discard_compact_log(&mut self) {
        self.compact_log = None;
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

    /// Frames `0..next_tick` without the empty ones; see [`crate::CompactLog`].
    pub fn compact_log(&self) -> &[GameFrame<G>] {
        self.compact_log.as_deref().unwrap_or_default()
    }

    /// Bots should only produce legal commands; the conformance suite requires this to stay 0.
    pub fn rejected_bot_commands(&self) -> u64 {
        self.rejected_bot_commands
    }

    pub fn has_pending_inputs(&self) -> bool {
        !self.pending_inputs.is_empty()
    }

    pub fn is_waiting_for_bot(&self) -> bool {
        match self.game.awaiting() {
            Awaiting::Turn(slot) => self.is_live_bot(slot),
            Awaiting::Simultaneous => self.bots.iter().any(|b| !self.is_dropped(b.slot)),
            Awaiting::Nobody => false,
        }
    }

    fn is_dropped(&self, slot: Slot) -> bool {
        self.dropped.get(slot.index()).copied().unwrap_or(true)
    }

    fn is_live_bot(&self, slot: Slot) -> bool {
        !self.is_dropped(slot) && self.bots.iter().any(|b| b.slot == slot)
    }

    /// Applied to the authority state immediately: no `step` runs before the frame is sealed, so
    /// clients applying the same command at the start of the frame reach the same state.
    /// Turn-based hosts seal a frame after every accepted command: a move may need world events
    /// (e.g. a spawned tile) before the next move is legal.
    pub fn submit(&mut self, slot: Slot, command: G::Command) -> Result<(), CommandError> {
        seat_may_command(&self.game, &self.players, &self.dropped, slot)?;
        self.game.apply_command(slot, &command)?;
        self.pending_inputs.push(Input::Command { slot, command });
        Ok(())
    }

    /// What [`Self::submit`] would answer, without changing anything.
    pub fn check_command(&self, slot: Slot, command: &G::Command) -> Result<(), CommandError> {
        seat_may_command(&self.game, &self.players, &self.dropped, slot)?;
        self.game.check_command(slot, command)
    }

    pub fn drop_player(&mut self, slot: Slot) {
        let Some(dropped) = self.dropped.get_mut(slot.index()) else {
            return;
        };
        if *dropped {
            return;
        }
        *dropped = true;
        self.game.drop_player(slot);
        self.pending_inputs.push(Input::Dropped { slot });
    }

    /// Real-time: every live bot decides once. Turn-based: the bot on turn decides; one that
    /// produces no legal move forfeits, so a broken bot can never stall the match.
    pub fn run_bots(&mut self) {
        if self.game.status().is_finished() {
            return;
        }
        match self.game.awaiting() {
            Awaiting::Nobody => {}
            Awaiting::Turn(slot) => {
                if self.is_live_bot(slot) && !self.let_bot_act(slot) {
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
                        self.let_bot_act(slot);
                    }
                }
            }
        }
    }

    /// Returns whether the bot produced an accepted command.
    fn let_bot_act(&mut self, slot: Slot) -> bool {
        let Some(seat) = self.bots.iter_mut().find(|b| b.slot == slot) else {
            return false;
        };
        let mut budget = Budget::new(seat.budget_nodes);
        let Some(command) = seat
            .bot
            .decide(&self.game, slot, &mut seat.rng, &mut budget)
        else {
            return false;
        };
        if self.game.apply_command(slot, &command).is_err() {
            self.rejected_bot_commands += 1;
            return false;
        }
        self.pending_inputs.push(Input::Command { slot, command });
        true
    }

    pub fn seal_frame(&mut self) -> GameFrame<G> {
        let mut inputs = std::mem::take(&mut self.pending_inputs);
        if !self.game.status().is_finished() {
            for event in self.game.roll_world(&mut self.world_rng) {
                if self.game.apply_world(&event).is_ok() {
                    inputs.push(Input::World(event));
                } else {
                    debug_assert!(
                        false,
                        "apply_world rejected its own roll_world event: {event:?}"
                    );
                }
            }
        }
        // Must mirror Match::apply_frame exactly.
        if self.game.clock().is_realtime() && !self.game.status().is_finished() {
            self.game.step();
        }
        let tick = self.next_tick;
        self.next_tick = tick.next();
        let finished = self.game.status().is_finished();
        let hash_due = self
            .next_tick
            .get()
            .is_multiple_of(self.hash_policy.every_frames());
        let hash = (finished || hash_due).then(|| self.game.state_hash());
        let frame = Frame { tick, inputs, hash };
        // The domain log serves native replays and the conformance suite; the clone is the
        // price of returning an owned frame. Hosts with their own log discard this one.
        if frame.belongs_in_compact_log()
            && let Some(log) = &mut self.compact_log
        {
            log.push(frame.clone());
        }
        frame
    }
}
