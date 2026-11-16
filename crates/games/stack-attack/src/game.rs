use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerSpec,
    SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, MAX_QUEUED_COMMANDS, WorldEvent};
use crate::model::{
    BoxId, Cell, Column, Crane, CraneId, CraneState, Direction, Field, Level, PatternId, Score,
    Side, TickCount, Worker, WorkerState, WorkerStatus,
};
use crate::rules::{EndReason, TICK_MS};
use crate::settings::{MAX_SEATS, MIN_SEATS, Settings};
use crate::view::{GameEvent, View, WorkerView};

/// Cooperative real-time survival: cranes drop boxes, workers push them into full rows.
///
/// Frame order (fixed by `mg-core`): queued commands are only *enqueued* by `apply_command`;
/// crane spawns arrive via `apply_world`; `step` then executes at most one queued action per
/// worker and advances cranes, boxes and workers by one tick.
#[derive(Clone, Debug)]
pub struct StackAttack {
    pub(crate) field: Field,
    /// Indexed by seat.
    pub(crate) workers: Vec<Worker>,
    pub(crate) cranes: Vec<Crane>,
    pub(crate) score: Score,
    pub(crate) lines_cleared: u32,
    /// Ticks in which at least one row was cleared; with `lines_cleared` it determines the score
    /// (each clear of `n` rows scores `150 n - 50`).
    pub(crate) clearing_ticks: u32,
    pub(crate) boxes_pushed: u32,
    pub(crate) ticks_survived: TickCount,
    pub(crate) ticks_since_last_crane: u32,
    pub(crate) next_box_id: BoxId,
    pub(crate) next_crane_id: CraneId,
    pub(crate) end_reason: Option<EndReason>,
    /// Events from inputs applied before this frame's `step`; moved into `last_frame_events`
    /// by `step`.
    pub(crate) pending_events: Vec<GameEvent>,
    pub(crate) last_frame_events: Vec<GameEvent>,
}

impl StackAttack {
    pub(crate) fn level(&self) -> Level {
        Level::after_ticks(self.ticks_survived)
    }

    pub(crate) fn crane_spawn_due(&self) -> bool {
        let level = self.level();
        self.ticks_since_last_crane >= level.crane_spawn_interval()
            && u32::try_from(self.cranes.len()).unwrap_or(u32::MAX) < level.max_cranes()
    }

    pub(crate) fn any_worker_alive(&self) -> bool {
        self.workers.iter().any(Worker::is_alive)
    }
}

/// Legacy placement: seats split the floor into equal segments, rounded half up.
fn spawn_column(seat: usize, seats: usize, field_width: i32) -> i32 {
    let numerator = 2 * i64::from(field_width) * (seat as i64 + 1) + (seats as i64 + 1);
    let denominator = 2 * (seats as i64 + 1);
    i32::try_from(numerator / denominator).unwrap_or(0)
}

impl Game for StackAttack {
    type Settings = Settings;
    type Command = Command;
    type WorldEvent = WorldEvent;
    type View = View;

    const INFO: GameInfo = GameInfo {
        id: "stack_attack",
        name: "Stack Attack",
        min_players: MIN_SEATS,
        max_players: MAX_SEATS,
        has_bots: false,
        cooperative: true,
    };

    fn new(settings: &Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        SetupError::check_player_count(players.len(), MIN_SEATS, MAX_SEATS)?;
        if !players.iter().all(|p| p.is_human()) {
            return Err(SetupError::InvalidPlayers(
                "Stack Attack has no bots".to_string(),
            ));
        }
        let width = i32::from(settings.field_width.to_u8());
        let height = i32::from(settings.field_height.to_u8());
        let workers = (0..players.len())
            .map(|seat| Worker::new(Cell::new(spawn_column(seat, players.len(), width), 0)))
            .collect();
        Ok(Self {
            field: Field::new(width, height),
            workers,
            cranes: Vec::new(),
            score: Score::ZERO,
            lines_cleared: 0,
            clearing_ticks: 0,
            boxes_pushed: 0,
            ticks_survived: TickCount::ZERO,
            ticks_since_last_crane: 0,
            next_box_id: BoxId::FIRST,
            next_crane_id: CraneId::FIRST,
            end_reason: None,
            pending_events: Vec::new(),
            last_frame_events: Vec::new(),
        })
    }

    fn clock(&self) -> Clock {
        Clock::Realtime { tick_ms: TICK_MS }
    }

    fn apply_command(&mut self, slot: Slot, command: &Command) -> Result<(), CommandError> {
        if self.end_reason.is_some() {
            return Err(CommandError::GameOver);
        }
        let worker = self
            .workers
            .get_mut(slot.index())
            .filter(|w| w.is_alive())
            .ok_or(CommandError::NotPlaying)?;
        if worker.queue.len() >= MAX_QUEUED_COMMANDS {
            return Err(CommandError::Busy);
        }
        worker.queue.push_back(*command);
        Ok(())
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<WorldEvent> {
        if self.end_reason.is_some() || !self.crane_spawn_due() {
            return Vec::new();
        }
        let width = u32::try_from(self.field.width()).unwrap_or(1);
        let target_x = Column::new(i32::try_from(rng.below(width)).unwrap_or(0));
        let from = if rng.below(2) == 0 {
            Side::Left
        } else {
            Side::Right
        };
        let pattern_value = u8::try_from(rng.below(u32::from(PatternId::COUNT))).unwrap_or(0);
        let Some(pattern) = PatternId::new(pattern_value) else {
            return Vec::new();
        };
        vec![WorldEvent::CraneSpawned {
            target_x,
            from,
            pattern,
        }]
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError> {
        let WorldEvent::CraneSpawned {
            target_x,
            from,
            pattern,
        } = *event;
        if self.end_reason.is_some() {
            return Err(CommandError::GameOver);
        }
        if !self.crane_spawn_due() {
            return Err(CommandError::Illegal("no crane spawn is due"));
        }
        if !self.field.contains_column(target_x) {
            return Err(CommandError::Illegal("crane target outside the field"));
        }
        let crane_id = self.next_crane_id;
        let start_x = from.outside_column(self.field.width());
        self.cranes.push(Crane {
            id: crane_id,
            x: start_x,
            target_x,
            state: CraneState::Moving,
            pattern,
        });
        self.next_crane_id = crane_id.next();
        self.ticks_since_last_crane = 0;
        self.pending_events.push(GameEvent::CraneSpawned {
            crane_id,
            start_x,
            target_x,
        });
        Ok(())
    }

    fn step(&mut self) {
        if self.end_reason.is_none() {
            self.run_tick();
        }
    }

    /// Only this seat's worker leaves (A11); the game ends once no worker is alive.
    fn drop_player(&mut self, slot: Slot) {
        if self.end_reason.is_some() {
            return;
        }
        let Some(worker) = self.workers.get_mut(slot.index()) else {
            return;
        };
        if !worker.is_alive() {
            return;
        }
        worker.leave_play(WorkerStatus::Left);
        self.pending_events.push(GameEvent::WorkerLeft { slot });
        if !self.any_worker_alive() {
            self.end_reason = Some(EndReason::AllWorkersLeft);
            // No `step` follows a finished game, so publish the events now.
            self.last_frame_events = std::mem::take(&mut self.pending_events);
        }
    }

    fn awaiting(&self) -> Awaiting {
        if self.end_reason.is_some() {
            Awaiting::Nobody
        } else {
            Awaiting::Simultaneous
        }
    }

    fn status(&self) -> Status {
        if self.end_reason.is_none() {
            return Status::InProgress;
        }
        Status::Finished(Outcome {
            kind: OutcomeKind::Defeat,
            winners: Vec::new(),
            scores: vec![i64::from(self.score.get()); self.workers.len()],
        })
    }

    fn view(&self, viewer: Viewer) -> View {
        let workers = self
            .workers
            .iter()
            .zip(Slot::all(self.workers.len()))
            .map(|(worker, slot)| WorkerView {
                slot,
                feet: worker.feet,
                state: worker.state,
                status: worker.status,
                queued_commands: worker.queue.len(),
            })
            .collect();
        View {
            you: match viewer {
                Viewer::Player(slot) => Some(slot),
                Viewer::Spectator => None,
            },
            field_width: u32::try_from(self.field.width()).unwrap_or(0),
            field_height: u32::try_from(self.field.height()).unwrap_or(0),
            tick_ms: TICK_MS,
            workers,
            boxes: self.field.boxes().to_vec(),
            cranes: self.cranes.clone(),
            score: self.score,
            lines_cleared: self.lines_cleared,
            boxes_pushed: self.boxes_pushed,
            level: self.level(),
            ticks_survived: self.ticks_survived,
            events: self.last_frame_events.clone(),
            end_reason: self.end_reason,
        }
    }

    fn hash_state(&self, hasher: &mut StateHasher) {
        let hash_cell = |hasher: &mut StateHasher, cell: Cell| {
            hasher.i32(cell.x);
            hasher.i32(cell.y);
        };
        hasher.i32(self.field.width());
        hasher.i32(self.field.height());
        hasher.length(self.field.boxes().len());
        for cargo in self.field.boxes() {
            hasher.u32(cargo.id.get());
            hash_cell(hasher, cargo.cell);
            hasher.u8(cargo.pattern.get());
            hasher.bool(cargo.falling);
        }
        hasher.length(self.workers.len());
        for worker in &self.workers {
            hash_cell(hasher, worker.feet);
            hasher.u8(match worker.state {
                WorkerState::Grounded => 0,
                WorkerState::Jumping => 1,
                WorkerState::Falling => 2,
            });
            hasher.u8(match worker.status {
                WorkerStatus::Alive => 0,
                WorkerStatus::Crushed => 1,
                WorkerStatus::Left => 2,
            });
            hasher.length(worker.queue.len());
            for command in &worker.queue {
                hasher.u8(match command {
                    Command::Move(Direction::Left) => 0,
                    Command::Move(Direction::Right) => 1,
                    Command::Jump => 2,
                });
            }
        }
        hasher.length(self.cranes.len());
        for crane in &self.cranes {
            hasher.u32(crane.id.get());
            hasher.i32(crane.x.get());
            hasher.i32(crane.target_x.get());
            hasher.u8(match crane.state {
                CraneState::Moving => 0,
                CraneState::Dropping => 1,
                CraneState::Leaving => 2,
            });
            hasher.u8(crane.pattern.get());
        }
        hasher.u32(self.score.get());
        hasher.u32(self.lines_cleared);
        hasher.u32(self.clearing_ticks);
        hasher.u32(self.boxes_pushed);
        hasher.u64(self.ticks_survived.get());
        hasher.u32(self.ticks_since_last_crane);
        hasher.u32(self.next_box_id.get());
        hasher.u32(self.next_crane_id.get());
        hasher.u8(match self.end_reason {
            None => 0,
            Some(EndReason::WorkersCrushed) => 1,
            Some(EndReason::CeilingReached) => 2,
            Some(EndReason::AllWorkersLeft) => 3,
            Some(EndReason::WorkersTrapped) => 4,
        });
    }
}
