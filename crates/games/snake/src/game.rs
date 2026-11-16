use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerKind,
    PlayerSpec, SettingRange, SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, WorldEvent};
use crate::model::{CellContent, Direction, Field, Grid, Point, SeatState, StepCount};
use crate::rules::{DeathReason, GameEnd, start_body};
use crate::settings::{DeadSnakeBehavior, MAX_FOOD_PERMILLE, MAX_SEATS, MIN_SEATS, Settings};
use crate::view::{SnakeView, View};

/// Multiplayer real-time Snake; all heads move at once every tick.
#[derive(Clone, Debug)]
pub struct Snake {
    pub(crate) field: Field,
    pub(crate) dead_snake_behavior: DeadSnakeBehavior,
    pub(crate) max_food_count: u8,
    pub(crate) food_spawn_permille: u32,
    pub(crate) tick_interval_ms: u32,
    pub(crate) seats: Vec<SeatState>,
    /// In spawn order; mirrored as `CellContent::Food` in `grid`.
    pub(crate) food: Vec<Point>,
    pub(crate) grid: Grid,
    pub(crate) steps: StepCount,
    pub(crate) end: Option<GameEnd>,
}

impl Snake {
    pub(crate) fn seat(&self, slot: Slot) -> Option<&SeatState> {
        self.seats.get(slot.index())
    }

    pub(crate) fn is_running(&self) -> bool {
        self.end.is_none()
    }

    pub(crate) fn kill(&mut self, slot: Slot, reason: DeathReason) {
        let disappears = self.dead_snake_behavior == DeadSnakeBehavior::Disappear;
        let Some(seat) = self.seats.get_mut(slot.index()) else {
            return;
        };
        seat.death = Some(reason);
        seat.queued_turns.clear();
        if disappears {
            for &cell in &seat.body {
                self.grid.set(cell, CellContent::Empty);
            }
        }
    }

    /// Single seat: over when it dies. Several seats: over when at most one snake is alive, or
    /// when the table had humans and none of them is alive (bots alone would play forever); the
    /// survivors win, so a human who dies before several bots sees a defeat, not a draw.
    pub(crate) fn check_end(&mut self, reason: DeathReason) {
        let alive: Vec<Slot> = Slot::all(self.seats.len())
            .zip(&self.seats)
            .filter(|(_, seat)| seat.is_alive())
            .map(|(slot, _)| slot)
            .collect();
        let had_humans = self
            .seats
            .iter()
            .any(|seat| seat.player == PlayerKind::Human);
        let humans_alive = self
            .seats
            .iter()
            .any(|seat| seat.player == PlayerKind::Human && seat.is_alive());
        let over = if self.seats.len() == 1 {
            alive.is_empty()
        } else {
            alive.len() <= 1 || (had_humans && !humans_alive)
        };
        if over {
            let winners = if self.seats.len() > 1 {
                alive
            } else {
                Vec::new()
            };
            self.end = Some(GameEnd { reason, winners });
        }
    }
}

impl Game for Snake {
    type Settings = Settings;
    type Command = Command;
    type WorldEvent = WorldEvent;
    type View = View;

    const INFO: GameInfo = GameInfo {
        id: "snake",
        name: "Snake",
        min_players: MIN_SEATS,
        max_players: MAX_SEATS,
        has_bots: true,
        cooperative: false,
    };

    fn new(settings: &Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        settings.validate()?;
        SetupError::check_player_count(players.len(), MIN_SEATS, MAX_SEATS)?;
        let width = u16::try_from(settings.field_width)
            .map_err(|error| SetupError::setting("field_width", error.to_string()))?;
        let height = u16::try_from(settings.field_height)
            .map_err(|error| SetupError::setting("field_height", error.to_string()))?;
        let max_food_count = u8::try_from(settings.max_food_count)
            .map_err(|error| SetupError::setting("max_food_count", error.to_string()))?;
        let field = Field::new(width, height, settings.wall_collision_mode);

        let mut grid = Grid::new(field);
        let mut seats = Vec::with_capacity(players.len());
        for (slot, player) in Slot::all(players.len()).zip(players) {
            let body = start_body(slot.index(), players.len(), field).ok_or_else(|| {
                SetupError::InvalidPlayers(format!("no start position for seat {slot}"))
            })?;
            for &cell in &body {
                if !grid.is_empty_cell(cell) {
                    return Err(SetupError::InvalidPlayers(format!(
                        "start position of seat {slot} overlaps another snake"
                    )));
                }
                grid.set(cell, CellContent::Body(slot));
            }
            seats.push(SeatState::new(player.kind, body, Direction::Up));
        }

        Ok(Self {
            field,
            dead_snake_behavior: settings.dead_snake_behavior,
            max_food_count,
            food_spawn_permille: settings.food_spawn_permille,
            tick_interval_ms: settings.tick_interval_ms,
            seats,
            food: Vec::with_capacity(usize::from(max_food_count)),
            grid,
            steps: StepCount::ZERO,
            end: None,
        })
    }

    fn setting_ranges(settings: &Settings) -> Vec<SettingRange> {
        settings.ranges()
    }

    fn clock(&self) -> Clock {
        Clock::Realtime {
            tick_ms: self.tick_interval_ms,
        }
    }

    fn apply_command(&mut self, slot: Slot, command: &Command) -> Result<(), CommandError> {
        if !self.is_running() {
            return Err(CommandError::GameOver);
        }
        let seat = self
            .seats
            .get_mut(slot.index())
            .ok_or(CommandError::NotPlaying)?;
        match *command {
            Command::Turn(direction) => seat.queue_turn(direction),
        }
    }

    /// At most one food item per tick, on a uniformly chosen empty cell: empty cells are
    /// counted and one is picked by index, so a crowded board needs no retries and a full board
    /// gets nothing.
    fn roll_world(&self, rng: &mut CoreRng) -> Vec<WorldEvent> {
        if !self.is_running() || self.food.len() >= usize::from(self.max_food_count) {
            return Vec::new();
        }
        if !rng.chance(self.food_spawn_permille, MAX_FOOD_PERMILLE) {
            return Vec::new();
        }
        rng.index(self.grid.empty_cell_count())
            .and_then(|nth| self.grid.nth_empty_cell(nth))
            .map(WorldEvent::SpawnFood)
            .into_iter()
            .collect()
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError> {
        if !self.is_running() {
            return Err(CommandError::GameOver);
        }
        match *event {
            WorldEvent::SpawnFood(point) => {
                if self.food.len() >= usize::from(self.max_food_count) {
                    return Err(CommandError::Illegal("food cap reached"));
                }
                if !self.grid.is_empty_cell(point) {
                    return Err(CommandError::Illegal("food needs an empty cell"));
                }
                self.food.push(point);
                self.grid.set(point, CellContent::Food);
                Ok(())
            }
        }
    }

    fn step(&mut self) {
        self.advance();
    }

    fn drop_player(&mut self, slot: Slot) {
        let alive = self.seat(slot).is_some_and(SeatState::is_alive);
        if self.is_running() && alive {
            self.kill(slot, DeathReason::PlayerDisconnected);
            self.check_end(DeathReason::PlayerDisconnected);
        }
    }

    fn awaiting(&self) -> Awaiting {
        if self.is_running() {
            Awaiting::Simultaneous
        } else {
            Awaiting::Nobody
        }
    }

    fn status(&self) -> Status {
        let Some(end) = &self.end else {
            return Status::InProgress;
        };
        let kind = if self.seats.len() == 1 {
            OutcomeKind::Defeat
        } else if end.winners.is_empty() {
            OutcomeKind::Draw
        } else {
            OutcomeKind::Winner
        };
        Status::Finished(Outcome {
            kind,
            winners: end.winners.clone(),
            scores: self
                .seats
                .iter()
                .map(|seat| i64::from(seat.score.get()))
                .collect(),
        })
    }

    fn view(&self, viewer: Viewer) -> View {
        View {
            steps: self.steps,
            field_width: self.field.width(),
            field_height: self.field.height(),
            tick_interval_ms: self.tick_interval_ms,
            wall_collision_mode: self.field.wall_mode(),
            dead_snake_behavior: self.dead_snake_behavior,
            max_food_count: self.max_food_count,
            snakes: Slot::all(self.seats.len())
                .zip(&self.seats)
                .map(|(slot, seat)| SnakeView {
                    slot,
                    player: seat.player,
                    body: seat.body.iter().copied().collect(),
                    direction: seat.direction,
                    queued_turns: seat.queued_turns.clone(),
                    death: seat.death,
                    score: seat.score,
                })
                .collect(),
            food: self.food.clone(),
            you: match viewer {
                Viewer::Player(slot) => self.seat(slot).map(|_| slot),
                Viewer::Spectator => None,
            },
            end: self.end.clone(),
        }
    }

    fn hash_state(&self, hasher: &mut StateHasher) {
        hasher.u16(self.field.width());
        hasher.u16(self.field.height());
        hasher.u8(self.field.wall_mode().hash_code());
        hasher.u8(self.dead_snake_behavior.hash_code());
        hasher.u8(self.max_food_count);
        hasher.u32(self.food_spawn_permille);
        hasher.u32(self.tick_interval_ms);
        hasher.u64(self.steps.get());
        hasher.length(self.seats.len());
        for seat in &self.seats {
            hasher.bool(seat.player == PlayerKind::Bot);
            hasher.u8(seat.direction.hash_code());
            hasher.length(seat.queued_turns.len());
            for turn in &seat.queued_turns {
                hasher.u8(turn.hash_code());
            }
            hasher.u8(seat.death.map_or(0, DeathReason::hash_code));
            hasher.u32(seat.score.get());
            hasher.length(seat.body.len());
            for &cell in &seat.body {
                hash_point(hasher, cell);
            }
        }
        hasher.length(self.food.len());
        for &food in &self.food {
            hash_point(hasher, food);
        }
        match &self.end {
            None => hasher.u8(0),
            Some(end) => {
                hasher.u8(end.reason.hash_code());
                hasher.length(end.winners.len());
                for winner in &end.winners {
                    hasher.u8(winner.get());
                }
            }
        }
    }
}

fn hash_point(hasher: &mut StateHasher, point: Point) {
    hasher.u16(point.x);
    hasher.u16(point.y);
}
