use crate::awaiting::Awaiting;
use crate::bot_mind::BotMind;
use crate::bot_seat::BotSeat;
use crate::bot_turn::BotTurn;
use crate::budget::Budget;
use crate::command_error::CommandError;
use crate::decided_bot_turn::DecidedBotTurn;
use crate::frame::{Frame, GameFrame};
use crate::game::Game;
use crate::game_secret::{SeatSecret, SecretCommand};
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
    /// See [`Game::Secret`]: never leaves the authority, bots included.
    secret: G::Secret,
    is_secret_revealed: bool,
    /// A private command changed the secret since the last frame: the next one tells the others
    /// what they may learn of it.
    has_secret_changes: bool,
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
    /// Counts every change of the game or the log, so a [`BotTurn`] can tell it went stale.
    revision: u64,
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
                mind: Some(BotMind {
                    bot: seat.bot,
                    rng: CoreRng::derive(seed, 1 + u64::from(seat.slot.get())),
                }),
                budget_nodes: seat.budget_nodes,
            })
            .collect();
        let hash_policy = HashPolicy::for_clock(game.clock());
        Ok(Self {
            game,
            secret: G::Secret::default(),
            is_secret_revealed: false,
            has_secret_changes: false,
            players: players.to_vec(),
            world_rng: CoreRng::derive(seed, 0),
            bots,
            dropped: vec![false; players.len()],
            pending_inputs: Vec::new(),
            next_tick: Tick::ZERO,
            hash_policy,
            compact_log: Some(Vec::new()),
            rejected_bot_commands: 0,
            revision: 0,
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
        !self.pending_inputs.is_empty() || self.has_secret_changes
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
        self.revision += 1;
        Ok(())
    }

    /// A seat's private command (see [`crate::Game::apply_secret_command`]): applied to the
    /// secret, never put in a frame. The host seals a frame after it, as after a command.
    pub fn submit_secret(
        &mut self,
        slot: Slot,
        command: &SecretCommand<G>,
    ) -> Result<(), CommandError> {
        seat_may_command(&self.game, &self.players, &self.dropped, slot)?;
        self.game
            .apply_secret_command(&mut self.secret, slot, command)?;
        self.has_secret_changes = true;
        self.revision += 1;
        Ok(())
    }

    /// For a seat whose time to send its private command ran out: the game chooses one. Returns
    /// whether one was applied.
    pub fn submit_default_secret(&mut self, slot: Slot) -> bool {
        if seat_may_command(&self.game, &self.players, &self.dropped, slot).is_err()
            || !self.game.awaits_secret(slot)
        {
            return false;
        }
        let Some(command) = self.game.default_secret_command(slot, &mut self.world_rng) else {
            return false;
        };
        self.apply_secret_for(slot, &command)
    }

    /// What `slot` may see of the secret; only ever for that seat's own connection.
    pub fn seat_secret(&self, slot: Slot) -> Option<SeatSecret<G>> {
        self.game.seat_secret(&self.secret, slot)
    }

    fn apply_secret_for(&mut self, slot: Slot, command: &SecretCommand<G>) -> bool {
        if self
            .game
            .apply_secret_command(&mut self.secret, slot, command)
            .is_err()
        {
            return false;
        }
        self.has_secret_changes = true;
        self.revision += 1;
        true
    }

    /// Bots that owe a private command send it as the frame is sealed, so it is never a turn
    /// to wait for. A bot whose command is refused gets the game's default one; with none, it
    /// forfeits, so a broken bot can never stall the match.
    fn settle_bot_secrets(&mut self) {
        let slots: Vec<Slot> = self
            .bots
            .iter()
            .map(|seat| seat.slot)
            .filter(|slot| !self.is_dropped(*slot) && self.game.awaits_secret(*slot))
            .collect();
        for slot in slots {
            let decided = self
                .bots
                .iter_mut()
                .find(|seat| seat.slot == slot)
                .and_then(|seat| seat.mind.as_mut())
                .and_then(|mind| mind.bot.decide_secret(&self.game, slot, &mut mind.rng));
            if let Some(command) = decided {
                if self.apply_secret_for(slot, &command) {
                    continue;
                }
                self.rejected_bot_commands += 1;
            }
            let fallback = self.game.default_secret_command(slot, &mut self.world_rng);
            if !fallback.is_some_and(|command| self.apply_secret_for(slot, &command)) {
                self.drop_player(slot);
            }
        }
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
        self.revision += 1;
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
                if self.is_live_bot(slot) && !self.is_thinking(slot) && !self.let_bot_act(slot) {
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

    /// Turn-based: the bot on turn, taken out of its seat to think on a copy of the game while
    /// the host goes on. `None` when no bot is on turn or it is thinking already.
    pub fn begin_bot_turn(&mut self) -> Option<BotTurn<G>> {
        if self.game.status().is_finished() {
            return None;
        }
        let Awaiting::Turn(slot) = self.game.awaiting() else {
            return None;
        };
        if self.is_dropped(slot) {
            return None;
        }
        let seat = self.bots.iter_mut().find(|b| b.slot == slot)?;
        let mind = seat.mind.take()?;
        Some(BotTurn {
            slot,
            mind,
            budget_nodes: seat.budget_nodes,
            game: self.game.clone(),
            revision: self.revision,
        })
    }

    /// Seats the bot again and plays its move, unless the match moved on while it thought (a
    /// player left, say): then nothing is played, and [`Self::begin_bot_turn`] starts over. A
    /// bot that finds no legal move forfeits, as in [`Self::run_bots`].
    pub fn finish_bot_turn(&mut self, decided: DecidedBotTurn<G>) {
        let DecidedBotTurn {
            slot,
            mind,
            revision,
            command,
        } = decided;
        if let Some(seat) = self.bots.iter_mut().find(|b| b.slot == slot) {
            seat.mind = Some(mind);
        }
        let current = revision == self.revision
            && !self.game.status().is_finished()
            && self.game.awaiting() == Awaiting::Turn(slot)
            && !self.is_dropped(slot);
        if current && !self.play_bot_command(slot, command) {
            self.drop_player(slot);
        }
    }

    fn is_thinking(&self, slot: Slot) -> bool {
        self.bots.iter().any(|b| b.slot == slot && b.mind.is_none())
    }

    /// Returns whether the bot produced an accepted command.
    fn let_bot_act(&mut self, slot: Slot) -> bool {
        let Some(seat) = self.bots.iter_mut().find(|b| b.slot == slot) else {
            return false;
        };
        let Some(mind) = seat.mind.as_mut() else {
            return false;
        };
        let mut budget = Budget::new(seat.budget_nodes);
        let command = mind
            .bot
            .decide(&self.game, slot, &mut mind.rng, &mut budget);
        self.play_bot_command(slot, command)
    }

    /// Returns whether the command was accepted.
    fn play_bot_command(&mut self, slot: Slot, command: Option<G::Command>) -> bool {
        let Some(command) = command else {
            return false;
        };
        if self.game.apply_command(slot, &command).is_err() {
            self.rejected_bot_commands += 1;
            return false;
        }
        self.pending_inputs.push(Input::Command { slot, command });
        self.revision += 1;
        true
    }

    fn apply_own_events(&mut self, events: Vec<G::WorldEvent>, inputs: &mut Vec<GameInput<G>>) {
        for event in events {
            if self.game.apply_world(&event).is_ok() {
                inputs.push(Input::World(event));
            } else {
                debug_assert!(
                    false,
                    "apply_world rejected the game's own event: {event:?}"
                );
            }
        }
    }

    pub fn seal_frame(&mut self) -> GameFrame<G> {
        if !self.game.status().is_finished() {
            self.settle_bot_secrets();
        }
        self.revision += 1;
        self.has_secret_changes = false;
        let mut inputs = std::mem::take(&mut self.pending_inputs);
        if !self.game.status().is_finished() {
            let events = self.game.roll_secret(&mut self.secret, &mut self.world_rng);
            self.apply_own_events(events, &mut inputs);
        }
        if !self.game.status().is_finished() {
            let events = self.game.roll_world(&mut self.world_rng);
            self.apply_own_events(events, &mut inputs);
        }
        // Before the step, which Match runs after all inputs: a real-time game that a step ends
        // reveals in the next frame sealed.
        if self.game.status().is_finished() && !self.is_secret_revealed {
            self.is_secret_revealed = true;
            let events = self.game.reveal(&self.secret);
            self.apply_own_events(events, &mut inputs);
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
