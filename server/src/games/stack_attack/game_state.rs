use std::collections::HashMap;

use crate::PlayerId;
use crate::games::SessionRng;
use crate::proto::stack_attack::{
    self as proto, BoxDroppedEvent, BoxLandedEvent, BoxPushedEvent, CraneSpawnedEvent,
    GameEvent, LineClearedEvent, WorkerCrushedEvent, WorkerJumpedEvent, WorkerLandedEvent,
    game_event,
};

use super::crane::{Crane, CraneAction};
use super::field::Field;
use super::settings::{FIELD_HEIGHT, FIELD_WIDTH, TICK_INTERVAL_MS};
use super::types::{GameOverReason, HorizontalDirection, MoveResult, Point};
use super::worker::Worker;

pub const POINTS_PER_LINE: u32 = 100;
pub const POINTS_PER_MULTI_LINE_BONUS: u32 = 50;

pub const INITIAL_CRANE_SPAWN_INTERVAL_TICKS: u32 = 15;
pub const INITIAL_MAX_SIMULTANEOUS_CRANES: u32 = 1;
pub const TICKS_PER_DIFFICULTY_INCREASE: u32 = 150;
pub const MIN_CRANE_SPAWN_INTERVAL_TICKS: u32 = 4;
pub const MAX_SIMULTANEOUS_CRANES_CAP: u32 = 5;

pub const PATTERN_COUNT: u32 = 8;

pub struct StackAttackGameState {
    pub field: Field,
    pub workers: HashMap<PlayerId, Worker>,
    pub cranes: Vec<Crane>,
    pub score: u32,
    pub lines_cleared: u32,
    pub boxes_pushed: u32,
    pub game_over: bool,
    pub game_over_reason: Option<GameOverReason>,
    pub difficulty_level: u32,
    next_box_id: u32,
    next_crane_id: u32,
    ticks_since_last_crane: u32,
    total_ticks: u64,
}

impl StackAttackGameState {
    pub fn new(players: &[PlayerId]) -> Self {
        let field = Field::new(FIELD_WIDTH, FIELD_HEIGHT);

        let mut workers = HashMap::new();
        let total = players.len();
        for (idx, player_id) in players.iter().enumerate() {
            let x = calculate_spawn_x(idx, total, FIELD_WIDTH);
            let worker = Worker::new(player_id.clone(), Point::new(x, 0), idx as u32);
            workers.insert(player_id.clone(), worker);
        }

        Self {
            field,
            workers,
            cranes: Vec::new(),
            score: 0,
            lines_cleared: 0,
            boxes_pushed: 0,
            game_over: false,
            game_over_reason: None,
            difficulty_level: 1,
            next_box_id: 1,
            next_crane_id: 1,
            ticks_since_last_crane: 0,
            total_ticks: 0,
        }
    }

    pub fn update(&mut self, rng: &mut SessionRng) -> Vec<GameEvent> {
        if self.game_over {
            return Vec::new();
        }

        let mut events = Vec::new();

        self.total_ticks += 1;
        self.update_difficulty();

        if let Some(event) = self.maybe_spawn_crane(rng) {
            events.push(event);
        }

        events.extend(self.update_cranes());
        events.extend(self.update_boxes());
        events.extend(self.update_workers());
        events.extend(self.check_worker_crushed());

        if self.check_ceiling_reached() {
            self.game_over = true;
            self.game_over_reason = Some(GameOverReason::BoxesReachedCeiling);
        }

        events.extend(self.check_and_clear_lines());

        events
    }

    pub fn handle_move(
        &mut self,
        player_id: &PlayerId,
        direction: HorizontalDirection,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if let Some(worker) = self.workers.get_mut(player_id)
            && worker.alive
        {
            let result = worker.try_move(direction, &mut self.field);
            if let MoveResult::PushedBox(box_id) = result {
                self.boxes_pushed += 1;
                let to_x = worker.position.x + direction.dx();
                events.push(GameEvent {
                    event: Some(game_event::Event::BoxPushed(BoxPushedEvent {
                        box_id,
                        from_x: worker.position.x,
                        to_x,
                        pushed_by: player_id.to_string(),
                    })),
                });
            }
        }

        events
    }

    pub fn handle_jump(&mut self, player_id: &PlayerId) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if let Some(worker) = self.workers.get_mut(player_id)
            && worker.alive
            && worker.jump(&self.field)
        {
            events.push(GameEvent {
                event: Some(game_event::Event::WorkerJumped(WorkerJumpedEvent {
                    player_id: player_id.to_string(),
                })),
            });
        }

        events
    }

    pub fn handle_player_disconnect(&mut self) {
        self.game_over = true;
        self.game_over_reason = Some(GameOverReason::PlayerDisconnected);
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn to_proto(&self, tick: u64, bots: &HashMap<crate::BotId, crate::games::BotType>) -> proto::StackAttackGameState {
        let workers: Vec<proto::Worker> = self
            .workers
            .values()
            .map(|w| {
                let is_bot = bots.keys().any(|bot_id| bot_id.to_player_id() == w.player_id);
                w.to_proto(is_bot)
            })
            .collect();

        let boxes: Vec<proto::Box> = self.field.boxes().map(|b| b.to_proto()).collect();
        let cranes: Vec<proto::Crane> = self.cranes.iter().map(|c| c.to_proto()).collect();

        let status = if self.game_over {
            proto::GameStatus::GameOver
        } else {
            proto::GameStatus::InProgress
        };

        proto::StackAttackGameState {
            tick,
            field_width: self.field.width(),
            field_height: self.field.height(),
            tick_interval_ms: TICK_INTERVAL_MS,
            workers,
            boxes,
            cranes,
            score: self.score,
            lines_cleared: self.lines_cleared,
            current_difficulty_level: self.difficulty_level,
            status: status as i32,
            events: Vec::new(),
        }
    }

    fn update_difficulty(&mut self) {
        let new_level = 1 + (self.total_ticks as u32 / TICKS_PER_DIFFICULTY_INCREASE);
        self.difficulty_level = new_level;
    }

    fn get_current_crane_spawn_interval(&self) -> u32 {
        let reduction = (self.difficulty_level - 1) * 2;
        INITIAL_CRANE_SPAWN_INTERVAL_TICKS
            .saturating_sub(reduction)
            .max(MIN_CRANE_SPAWN_INTERVAL_TICKS)
    }

    fn get_current_max_cranes(&self) -> u32 {
        let increase = (self.difficulty_level - 1) / 2;
        (INITIAL_MAX_SIMULTANEOUS_CRANES + increase).min(MAX_SIMULTANEOUS_CRANES_CAP)
    }

    fn maybe_spawn_crane(&mut self, rng: &mut SessionRng) -> Option<GameEvent> {
        self.ticks_since_last_crane += 1;

        let spawn_interval = self.get_current_crane_spawn_interval();
        let max_cranes = self.get_current_max_cranes();

        if self.ticks_since_last_crane < spawn_interval {
            return None;
        }

        if self.cranes.len() >= max_cranes as usize {
            return None;
        }

        self.ticks_since_last_crane = 0;

        let width = self.field.width() as i32;
        let target_x = rng.random_range(0..self.field.width() as i32);
        let from_left: bool = rng.random_bool();
        let start_x = if from_left { -1 } else { width };

        let pattern_id = rng.random_range(0..PATTERN_COUNT);

        let crane_id = self.next_crane_id;
        self.next_crane_id += 1;

        let crane = Crane::new(crane_id, start_x, target_x, pattern_id, self.field.width());
        self.cranes.push(crane);

        Some(GameEvent {
            event: Some(game_event::Event::CraneSpawned(CraneSpawnedEvent {
                crane_id,
                start_x,
                target_x,
            })),
        })
    }

    fn update_cranes(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut finished_cranes = Vec::new();

        for crane in &mut self.cranes {
            if let Some(action) = crane.update() {
                match action {
                    CraneAction::DropBox => {
                        let box_id = self.next_box_id;
                        self.next_box_id += 1;

                        self.field.spawn_box(box_id, crane.x, crane.box_pattern_id);

                        events.push(GameEvent {
                            event: Some(game_event::Event::BoxDropped(BoxDroppedEvent {
                                crane_id: crane.id,
                                box_id,
                                x: crane.x,
                            })),
                        });
                    }
                    CraneAction::Finished => {
                        finished_cranes.push(crane.id);
                    }
                }
            }
        }

        self.cranes.retain(|c| !finished_cranes.contains(&c.id));

        events
    }

    fn update_boxes(&mut self) -> Vec<GameEvent> {
        let landed_boxes = self.field.update_falling_boxes();

        landed_boxes
            .into_iter()
            .map(|info| GameEvent {
                event: Some(game_event::Event::BoxLanded(BoxLandedEvent {
                    box_id: info.box_id,
                    x: info.x,
                    y: info.y,
                })),
            })
            .collect()
    }

    fn update_workers(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        for worker in self.workers.values_mut() {
            if !worker.alive {
                continue;
            }
            let landed = worker.apply_gravity(&self.field);
            if landed {
                events.push(GameEvent {
                    event: Some(game_event::Event::WorkerLanded(WorkerLandedEvent {
                        player_id: worker.player_id.to_string(),
                        x: worker.position.x,
                        y: worker.position.y,
                    })),
                });
            }
        }

        events
    }

    fn check_worker_crushed(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut crushed_players = Vec::new();

        for worker in self.workers.values() {
            if !worker.alive {
                continue;
            }
            if let Some(box_id) = self.field.get_box_id_at(worker.position.x, worker.position.y) {
                crushed_players.push((worker.player_id.clone(), box_id));
            } else if let Some(box_id) = self.field.get_box_id_at(worker.position.x, worker.head_y())
            {
                crushed_players.push((worker.player_id.clone(), box_id));
            }
        }

        for (player_id, box_id) in crushed_players {
            if let Some(worker) = self.workers.get_mut(&player_id) {
                worker.alive = false;
            }
            events.push(GameEvent {
                event: Some(game_event::Event::WorkerCrushed(WorkerCrushedEvent {
                    player_id: player_id.to_string(),
                    box_id,
                })),
            });
        }

        if !events.is_empty() && self.workers.values().all(|w| !w.alive) {
            self.game_over = true;
            self.game_over_reason = Some(GameOverReason::WorkerCrushed);
        }

        events
    }

    fn check_ceiling_reached(&self) -> bool {
        self.field.has_box_at_ceiling()
    }

    fn check_and_clear_lines(&mut self) -> Vec<GameEvent> {
        let cleared = self.field.check_and_clear_lines();

        if cleared.is_empty() {
            return Vec::new();
        }

        let line_count = cleared.len() as u32;
        let base_points = line_count * POINTS_PER_LINE;
        let bonus = if line_count > 1 {
            (line_count - 1) * POINTS_PER_MULTI_LINE_BONUS
        } else {
            0
        };
        let total_points = base_points + bonus;

        self.score += total_points;
        self.lines_cleared += line_count;

        cleared
            .into_iter()
            .map(|y| {
                let points = POINTS_PER_LINE
                    + if line_count > 1 {
                        POINTS_PER_MULTI_LINE_BONUS
                    } else {
                        0
                    };
                GameEvent {
                    event: Some(game_event::Event::LineCleared(LineClearedEvent {
                        y,
                        points_earned: points,
                    })),
                }
            })
            .collect()
    }
}

fn calculate_spawn_x(index: usize, total: usize, field_width: u32) -> i32 {
    let segment_count = total + 1;
    let segment_width = field_width as f32 / segment_count as f32;
    (segment_width * (index + 1) as f32).round() as i32
}
