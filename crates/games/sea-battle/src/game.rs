use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerSpec,
    ScoreKind, SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, SecretCommand, WorldEvent};
use crate::model::{Cell, CellMark, Fleet, HiddenFleets, LastShot, Phase, ShotOutcome, Waters};
use crate::rules::{Refusal, resolve_shot};
use crate::settings::Settings;
use crate::view::View;

const FIRST: Slot = Slot::new(0);
const SECOND: Slot = Slot::new(1);
const SEATS: [Slot; 2] = [FIRST, SECOND];

fn opponent(slot: Slot) -> Slot {
    if slot == FIRST { SECOND } else { FIRST }
}

/// Two seats each place a classic fleet on their own 10×10 sea, unseen, then take turns firing
/// at the other's; a hit fires again, and whoever sinks the whole other fleet wins.
///
/// The fleets are the authority's secret ([`HiddenFleets`]): a seat places its fleet with a
/// private command, every shot is resolved by the authority, and the fleets are revealed only
/// once the game is over.
#[derive(Clone, Debug)]
pub struct SeaBattle {
    phase: Phase,
    ready: [bool; 2],
    in_play: [bool; 2],
    /// By slot: the seat's own sea, as everyone sees it.
    waters: [Waters; 2],
    turn: Option<Slot>,
    pending_shot: Option<Cell>,
    winner: Option<Slot>,
    last_shot: Option<LastShot>,
    revealed: [Option<Fleet>; 2],
}

impl SeaBattle {
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// `slot`'s own sea as everyone sees it.
    pub fn waters(&self, slot: Slot) -> Option<&Waters> {
        self.waters.get(slot.index())
    }

    fn is_ready(&self, slot: Slot) -> bool {
        self.ready.get(slot.index()).copied().unwrap_or(false)
    }

    fn is_in_play(&self, slot: Slot) -> bool {
        self.in_play.get(slot.index()).copied().unwrap_or(false)
    }

    fn end(&mut self, phase: Phase, winner: Option<Slot>) {
        self.phase = phase;
        self.winner = winner;
        self.turn = None;
        self.pending_shot = None;
    }

    /// `loser` gave up or left: the other seat wins if it is still there.
    fn forfeit(&mut self, loser: Slot) {
        if let Some(in_play) = self.in_play.get_mut(loser.index()) {
            *in_play = false;
        }
        let other = opponent(loser);
        if self.is_in_play(other) {
            self.end(Phase::Forfeited, Some(other));
        } else {
            self.end(Phase::Abandoned, None);
        }
    }

    fn place(&mut self, slot: Slot) -> Result<(), CommandError<Refusal>> {
        if self.phase != Phase::Placing || !self.is_in_play(slot) || self.is_ready(slot) {
            return Err(CommandError::Illegal(Refusal::FleetCannotBePlacedNow));
        }
        if let Some(ready) = self.ready.get_mut(slot.index()) {
            *ready = true;
        }
        if SEATS.iter().all(|seat| self.is_ready(*seat)) {
            self.phase = Phase::Battle;
        }
        Ok(())
    }

    fn record_shot(&mut self, outcome: ShotOutcome) -> Result<(), CommandError<Refusal>> {
        let (Phase::Battle, Some(cell), Some(shooter)) = (self.phase, self.pending_shot, self.turn)
        else {
            return Err(CommandError::Illegal(Refusal::NoShotPending));
        };
        let target = opponent(shooter);
        let Some(waters) = self.waters.get_mut(target.index()) else {
            return Err(CommandError::Illegal(Refusal::NoSuchSea));
        };
        match outcome {
            ShotOutcome::Miss => {
                waters.set(cell, CellMark::Miss);
                self.turn = Some(target);
            }
            ShotOutcome::Hit => waters.set(cell, CellMark::Hit),
            ShotOutcome::Sunk(ship) => {
                let fits = ship.covers(cell)
                    && ship
                        .cells()
                        .filter(|own| *own != cell)
                        .all(|own| waters.mark(own) == CellMark::Hit);
                if !fits {
                    return Err(CommandError::Illegal(Refusal::SunkShipDoesNotFit));
                }
                waters.sink(ship);
            }
        }
        let fleet_gone = waters.is_all_sunk();
        self.pending_shot = None;
        self.last_shot = Some(LastShot {
            by: shooter,
            cell,
            outcome,
        });
        if fleet_gone {
            self.end(Phase::Won, Some(shooter));
        }
        Ok(())
    }
}

impl Game for SeaBattle {
    type Settings = Settings;
    type Command = Command;
    type WorldEvent = WorldEvent;
    type View = View;
    type Secret = HiddenFleets;
    type Refusal = Refusal;

    const INFO: GameInfo = GameInfo {
        id: "sea_battle",
        name: "Sea Battle",
        min_players: 2,
        max_players: 2,
        has_bots: true,
        cooperative: false,
        scores: ScoreKind::WinFlag,
    };

    fn new(_settings: &Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        SetupError::check_player_count(
            players.len(),
            Self::INFO.min_players,
            Self::INFO.max_players,
        )?;
        Ok(Self {
            phase: Phase::Placing,
            ready: [false; 2],
            in_play: [true; 2],
            waters: [Waters::default(), Waters::default()],
            turn: None,
            pending_shot: None,
            winner: None,
            last_shot: None,
            revealed: [None, None],
        })
    }

    fn clock(&self) -> Clock {
        Clock::TurnBased
    }

    fn apply_command(
        &mut self,
        slot: Slot,
        command: &Command,
    ) -> Result<(), CommandError<Refusal>> {
        if self.phase.is_over() {
            return Err(CommandError::GameOver);
        }
        let cell = match command {
            Command::GiveUp => {
                if !self.is_in_play(slot) {
                    return Err(CommandError::NotPlaying);
                }
                self.forfeit(slot);
                return Ok(());
            }
            Command::Fire(cell) => *cell,
        };
        if self.phase != Phase::Battle {
            return Err(CommandError::Illegal(Refusal::BattleNotStarted));
        }
        if self.pending_shot.is_some() {
            return Err(CommandError::Illegal(Refusal::LastShotNotResolved));
        }
        if self.turn != Some(slot) {
            return Err(CommandError::NotYourTurn);
        }
        let target = self.waters.get(opponent(slot).index());
        if !target.is_some_and(|waters| waters.mark(cell).is_open()) {
            return Err(CommandError::Illegal(Refusal::CellAlreadyKnown));
        }
        self.pending_shot = Some(cell);
        Ok(())
    }

    fn roll_secret(&self, secret: &mut HiddenFleets, _rng: &mut CoreRng) -> Vec<WorldEvent> {
        match self.phase {
            Phase::Placing => SEATS
                .into_iter()
                .filter(|slot| {
                    self.is_in_play(*slot) && !self.is_ready(*slot) && secret.fleet(*slot).is_some()
                })
                .map(WorldEvent::FleetPlaced)
                .collect(),
            Phase::Battle => {
                let (Some(cell), Some(shooter)) = (self.pending_shot, self.turn) else {
                    return Vec::new();
                };
                let target = opponent(shooter);
                match (secret.fleet(target), self.waters(target)) {
                    (Some(fleet), Some(waters)) => {
                        vec![WorldEvent::ShotResolved(resolve_shot(fleet, waters, cell))]
                    }
                    _ => Vec::new(),
                }
            }
            Phase::Won | Phase::Forfeited | Phase::Abandoned => Vec::new(),
        }
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<WorldEvent> {
        if self.phase != Phase::Battle || self.turn.is_some() {
            return Vec::new();
        }
        rng.pick(&SEATS)
            .map(|slot| WorldEvent::FirstShooterChosen(*slot))
            .into_iter()
            .collect()
    }

    fn reveal(&self, secret: &HiddenFleets) -> Vec<WorldEvent> {
        SEATS
            .into_iter()
            .filter_map(|slot| {
                secret.fleet(slot).map(|fleet| WorldEvent::FleetRevealed {
                    slot,
                    ships: fleet.ships().to_vec(),
                })
            })
            .collect()
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError<Refusal>> {
        match event {
            WorldEvent::FleetPlaced(slot) => self.place(*slot),
            WorldEvent::FirstShooterChosen(slot) => {
                if self.phase != Phase::Battle || self.turn.is_some() || !SEATS.contains(slot) {
                    return Err(CommandError::Illegal(Refusal::FirstShooterAlreadyChosen));
                }
                self.turn = Some(*slot);
                Ok(())
            }
            WorldEvent::ShotResolved(outcome) => self.record_shot(*outcome),
            WorldEvent::FleetRevealed { slot, ships } => {
                let already = self.revealed.get(slot.index()).is_none_or(Option::is_some);
                if !self.phase.is_over() || already {
                    return Err(CommandError::Illegal(Refusal::FleetRevealedEarly));
                }
                let fleet = Fleet::parse(ships)
                    .map_err(|_| CommandError::Illegal(Refusal::RevealedFleetInvalid))?;
                if let Some(revealed) = self.revealed.get_mut(slot.index()) {
                    *revealed = Some(fleet);
                }
                Ok(())
            }
        }
    }

    fn drop_player(&mut self, slot: Slot) {
        if self.phase.is_over() || !self.is_in_play(slot) {
            return;
        }
        self.forfeit(slot);
    }

    fn awaiting(&self) -> Awaiting {
        match (self.phase, self.turn, self.pending_shot) {
            (Phase::Battle, Some(slot), None) => Awaiting::Turn(slot),
            _ => Awaiting::Nobody,
        }
    }

    fn status(&self) -> Status {
        let finished = |kind, winners: Vec<Slot>, scores: Vec<i64>| {
            Status::Finished(Outcome {
                kind,
                winners,
                scores,
            })
        };
        match (self.phase, self.winner) {
            (Phase::Won | Phase::Forfeited, Some(winner)) => finished(
                OutcomeKind::Winner,
                vec![winner],
                SEATS
                    .iter()
                    .map(|slot| i64::from(*slot == winner))
                    .collect(),
            ),
            (Phase::Won | Phase::Forfeited | Phase::Abandoned, _) => {
                finished(OutcomeKind::Draw, Vec::new(), vec![0; SEATS.len()])
            }
            (Phase::Placing | Phase::Battle, _) => Status::InProgress,
        }
    }

    fn awaits_secret(&self, slot: Slot) -> bool {
        self.phase == Phase::Placing && self.is_in_play(slot) && !self.is_ready(slot)
    }

    fn apply_secret_command(
        &self,
        secret: &mut HiddenFleets,
        slot: Slot,
        command: &SecretCommand,
    ) -> Result<(), CommandError<Refusal>> {
        if !self.awaits_secret(slot) || secret.fleet(slot).is_some() {
            return Err(CommandError::Illegal(Refusal::FleetAlreadyPlaced));
        }
        let SecretCommand::PlaceFleet(ships) = command;
        let fleet = Fleet::parse(ships).map_err(|error| CommandError::Illegal(error.into()))?;
        secret.place(slot, fleet);
        Ok(())
    }

    fn default_secret_command(&self, _slot: Slot, rng: &mut CoreRng) -> Option<SecretCommand> {
        Fleet::random(rng).map(|fleet| SecretCommand::PlaceFleet(fleet.ships().to_vec()))
    }

    fn seat_secret(&self, secret: &HiddenFleets, slot: Slot) -> Option<Fleet> {
        secret.fleet(slot).cloned()
    }

    fn shows_seat_secrets_to_spectators(&self) -> bool {
        true
    }

    fn view(&self, _viewer: Viewer) -> View {
        View {
            phase: self.phase,
            ready: self.ready.to_vec(),
            waters: self.waters.to_vec(),
            turn: match self.awaiting() {
                Awaiting::Turn(slot) => Some(slot),
                Awaiting::Simultaneous | Awaiting::Nobody => None,
            },
            winner: self.winner,
            last_shot: self.last_shot,
            revealed: self
                .revealed
                .iter()
                .map(|fleet| fleet.as_ref().map(|fleet| fleet.ships().to_vec()))
                .collect(),
        }
    }

    fn hash_state(&self, hasher: &mut StateHasher) {
        hasher.u8(self.phase.hash_code());
        for slot in SEATS {
            hasher.bool(self.is_ready(slot));
            hasher.bool(self.is_in_play(slot));
        }
        for waters in &self.waters {
            waters.hash(hasher);
        }
        hasher.u8(self.turn.map_or(u8::MAX, Slot::get));
        hasher.bool(self.pending_shot.is_some());
        if let Some(cell) = self.pending_shot {
            cell.hash(hasher);
        }
        hasher.u8(self.winner.map_or(u8::MAX, Slot::get));
        hasher.bool(self.last_shot.is_some());
        if let Some(last) = self.last_shot {
            hasher.u8(last.by.get());
            last.cell.hash(hasher);
            match last.outcome {
                ShotOutcome::Miss => hasher.u8(0),
                ShotOutcome::Hit => hasher.u8(1),
                ShotOutcome::Sunk(ship) => {
                    hasher.u8(2);
                    ship.hash(hasher);
                }
            }
        }
        for fleet in &self.revealed {
            hasher.bool(fleet.is_some());
            if let Some(fleet) = fleet {
                fleet.hash(hasher);
            }
        }
    }
}
