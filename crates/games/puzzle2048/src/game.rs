use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerSpec,
    ScoreKind, SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, WorldEvent};
use crate::model::{Board, Cell, Direction, Score, SpawnValue, Tile, TileId};
use crate::rules::{Phase, Refusal};
use crate::settings::{Settings, Target};
use crate::view::{TileOrigin, View};

const PLAYER: Slot = Slot::new(0);
const INITIAL_TILES: u8 = 2;

/// An accepted move leaves a spawn due; the authority delivers it in the same frame via
/// `roll_world`. Win and loss are evaluated only once the spawn has landed (as the legacy server
/// did), and further moves are rejected with [`CommandError::Busy`] until then.
#[derive(Clone, Debug)]
pub struct Puzzle2048 {
    board: Board,
    target: Target,
    next_tile_id: TileId,
    score: Score,
    moves: u32,
    pending_spawns: u8,
    last_move: Option<Direction>,
    phase: Phase,
}

impl Puzzle2048 {
    pub fn target(&self) -> Target {
        self.target
    }

    pub fn score(&self) -> Score {
        self.score
    }

    pub fn moves(&self) -> u32 {
        self.moves
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn tile_at(&self, cell: Cell) -> Option<&Tile> {
        self.board.tile(cell)
    }

    pub fn can_move(&self, direction: Direction) -> bool {
        self.phase == Phase::Playing
            && self.pending_spawns == 0
            && self.board.slide(direction, self.next_tile_id).changed
    }

    fn settle(&mut self) {
        let won = self
            .board
            .highest_tile()
            .is_some_and(|tile| self.target.is_reached_by(tile));
        if won {
            self.phase = Phase::Won;
        } else if !self.board.has_moves() {
            self.phase = Phase::Lost;
        }
    }

    /// Test fixture: a running game with the given row-major values (0 = empty) and no spawn due.
    #[cfg(test)]
    pub(crate) fn with_values(settings: &Settings, values: &[u32]) -> Self {
        let mut game = Self::new(settings, &[PlayerSpec::HUMAN]).expect("valid settings");
        let width = game.board.width();
        assert_eq!(
            values.len(),
            usize::from(width) * usize::from(game.board.height())
        );
        for (index, &value) in values.iter().enumerate() {
            if value == 0 {
                continue;
            }
            let row = u8::try_from(index / usize::from(width)).expect("small board");
            let col = u8::try_from(index % usize::from(width)).expect("small board");
            game.board.put(Tile {
                id: game.next_tile_id,
                value: crate::TileValue::new(value).expect("power of two"),
                cell: Cell::new(row, col),
                origin: TileOrigin::Spawned,
            });
            game.next_tile_id = game.next_tile_id.next();
        }
        game.pending_spawns = 0;
        game
    }
}

impl Game for Puzzle2048 {
    type Settings = Settings;
    type Command = Command;
    type WorldEvent = WorldEvent;
    type View = View;
    type Secret = ();
    type Refusal = Refusal;

    const INFO: GameInfo = GameInfo {
        id: "puzzle2048",
        name: "2048",
        min_players: 1,
        max_players: 1,
        has_bots: false,
        cooperative: true,
        scores: ScoreKind::Points,
    };

    fn new(settings: &Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        SetupError::check_player_count(
            players.len(),
            Self::INFO.min_players,
            Self::INFO.max_players,
        )?;
        if !players.iter().all(|player| player.is_human()) {
            return Err(SetupError::InvalidPlayers("2048 has no bots".into()));
        }
        Ok(Self {
            board: Board::empty(settings.width, settings.height),
            target: settings.target,
            next_tile_id: TileId::FIRST,
            score: Score::ZERO,
            moves: 0,
            pending_spawns: INITIAL_TILES,
            last_move: None,
            phase: Phase::Playing,
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
        if self.phase != Phase::Playing {
            return Err(CommandError::GameOver);
        }
        if slot != PLAYER {
            return Err(CommandError::NotPlaying);
        }
        if self.pending_spawns > 0 {
            return Err(CommandError::Busy);
        }
        let Command::Move(direction) = *command;
        let slide = self.board.slide(direction, self.next_tile_id);
        if !slide.changed {
            return Err(CommandError::Illegal(Refusal::MoveChangesNothing));
        }
        self.board = slide.board;
        self.next_tile_id = slide.next_tile_id;
        self.score = self.score.plus(slide.gained);
        self.moves = self.moves.saturating_add(1);
        self.last_move = Some(direction);
        // A move that changed the board always leaves an empty cell (sliding needs a gap, merging
        // frees one), so the spawn can always land.
        self.pending_spawns = 1;
        Ok(())
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<WorldEvent> {
        if self.phase != Phase::Playing {
            return Vec::new();
        }
        let mut empty = self.board.empty_cells();
        let mut events = Vec::new();
        for _ in 0..self.pending_spawns {
            let Some(index) = rng.index(empty.len()) else {
                break;
            };
            let cell = empty.remove(index);
            let (numerator, denominator) = SpawnValue::FOUR_CHANCE;
            let value = if rng.chance(numerator, denominator) {
                SpawnValue::Four
            } else {
                SpawnValue::Two
            };
            events.push(WorldEvent::TileSpawned { cell, value });
        }
        events
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError<Refusal>> {
        let WorldEvent::TileSpawned { cell, value } = *event;
        if self.phase != Phase::Playing {
            return Err(CommandError::GameOver);
        }
        if self.pending_spawns == 0 {
            return Err(CommandError::Illegal(Refusal::NoSpawnDue));
        }
        if !self.board.contains(cell) {
            return Err(CommandError::Illegal(Refusal::SpawnCellOutside));
        }
        if self.board.tile(cell).is_some() {
            return Err(CommandError::Illegal(Refusal::SpawnCellOccupied));
        }
        self.board.put(Tile {
            id: self.next_tile_id,
            value: value.value(),
            cell,
            origin: TileOrigin::Spawned,
        });
        self.next_tile_id = self.next_tile_id.next();
        self.pending_spawns -= 1;
        if self.pending_spawns == 0 {
            self.settle();
        }
        Ok(())
    }

    fn drop_player(&mut self, slot: Slot) {
        if slot == PLAYER && self.phase == Phase::Playing {
            self.phase = Phase::Abandoned;
            self.pending_spawns = 0;
        }
    }

    fn awaiting(&self) -> Awaiting {
        match self.phase {
            Phase::Playing => Awaiting::Turn(PLAYER),
            Phase::Won | Phase::Lost | Phase::Abandoned => Awaiting::Nobody,
        }
    }

    fn status(&self) -> Status {
        let kind = match self.phase {
            Phase::Playing => return Status::InProgress,
            Phase::Won => OutcomeKind::Victory,
            Phase::Lost | Phase::Abandoned => OutcomeKind::Defeat,
        };
        Status::Finished(Outcome {
            kind,
            winners: Vec::new(),
            scores: vec![i64::try_from(self.score.get()).unwrap_or(i64::MAX)],
        })
    }

    fn view(&self, _viewer: Viewer) -> View {
        View {
            width: self.board.width(),
            height: self.board.height(),
            target: self.target,
            tiles: self.board.tiles().copied().collect(),
            score: self.score,
            moves: self.moves,
            highest_tile: self.board.highest_tile(),
            last_move: self.last_move,
            phase: self.phase,
            movable_directions: Direction::ALL
                .into_iter()
                .filter(|&direction| self.can_move(direction))
                .collect(),
        }
    }

    fn hash_state(&self, hasher: &mut StateHasher) {
        self.board.hash(hasher);
        hasher.u32(self.target.get());
        hasher.u32(self.next_tile_id.get());
        hasher.u64(self.score.get());
        hasher.u32(self.moves);
        hasher.u8(self.pending_spawns);
        hasher.u8(self.last_move.map_or(u8::MAX, Direction::code));
        hasher.u8(self.phase.code());
    }
}
