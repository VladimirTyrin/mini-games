use mg_core::conformance::{self, Conformance, Setup};
use mg_core::{BotSeat, CoreRng, HasBots, PlayerKind, PlayerSpec, Slot};

use super::support::field_settings;
use crate::model::{CellContent, Grid, SeatState};
use crate::{
    BotKind, Command, DeadSnakeBehavior, DeathReason, Direction, INPUT_BUFFER, RawSettings,
    START_LENGTH, Snake, WallCollisionMode,
};

use DeadSnakeBehavior::{Disappear, StayOnField};
use WallCollisionMode::{Death, WrapAround};

const HUMAN: PlayerSpec = PlayerSpec::HUMAN;
const BOT: PlayerSpec = PlayerSpec::BOT;

fn with_food(settings: RawSettings, max_food_count: u32, food_spawn_permille: u32) -> RawSettings {
    RawSettings {
        max_food_count,
        food_spawn_permille,
        ..settings
    }
}

fn alternating(humans: usize, bots: usize) -> Vec<PlayerSpec> {
    let mut players = Vec::new();
    for index in 0..humans.max(bots) {
        if index < humans {
            players.push(HUMAN);
        }
        if index < bots {
            players.push(BOT);
        }
    }
    players
}

impl Conformance for Snake {
    fn sample_setups() -> Vec<Setup<Self>> {
        vec![
            Setup::new(RawSettings::default(), vec![HUMAN]),
            Setup::new(field_settings(10, 10, Death, Disappear), vec![HUMAN]),
            Setup::new(
                with_food(field_settings(100, 100, WrapAround, StayOnField), 50, 1000),
                vec![HUMAN, HUMAN],
            ),
            Setup::new(
                field_settings(10, 10, WrapAround, StayOnField),
                alternating(5, 5),
            ),
            Setup::new(field_settings(15, 15, Death, Disappear), alternating(3, 2)),
            Setup::new(
                field_settings(20, 20, WrapAround, Disappear),
                vec![BOT, BOT],
            ),
            Setup::new(
                with_food(field_settings(10, 10, Death, StayOnField), 50, 1000),
                vec![BOT; 10],
            ),
            Setup::new(field_settings(100, 10, Death, Disappear), vec![BOT]),
            Setup::new(
                with_food(field_settings(10, 100, WrapAround, StayOnField), 50, 1),
                vec![HUMAN; 4],
            ),
            Setup::new(
                with_food(field_settings(30, 20, WrapAround, Disappear), 5, 300),
                vec![HUMAN, BOT, BOT],
            ),
            Setup::new(
                field_settings(100, 100, Death, StayOnField),
                vec![BOT, BOT, BOT],
            ),
            Setup::new(
                RawSettings {
                    tick_interval_ms: 50,
                    ..field_settings(10, 10, WrapAround, Disappear)
                },
                vec![HUMAN; 10],
            ),
        ]
    }

    fn invalid_setups() -> Vec<Setup<Self>> {
        let valid = RawSettings::default();
        let mut setups = Vec::new();
        for value in [0, 9, 101, u32::MAX] {
            setups.push(RawSettings {
                field_width: value,
                ..valid
            });
            setups.push(RawSettings {
                field_height: value,
                ..valid
            });
        }
        for value in [0, 51, u32::MAX] {
            setups.push(RawSettings {
                max_food_count: value,
                ..valid
            });
        }
        for value in [0, 1001, u32::MAX] {
            setups.push(RawSettings {
                food_spawn_permille: value,
                ..valid
            });
        }
        for value in [0, 49, 5001, u32::MAX] {
            setups.push(RawSettings {
                tick_interval_ms: value,
                ..valid
            });
        }
        let mut setups: Vec<Setup<Self>> = setups
            .into_iter()
            .map(|settings| Setup::new(settings, vec![HUMAN]))
            .collect();
        setups.push(Setup::new(valid, Vec::new()));
        setups.push(Setup::new(valid, vec![HUMAN; 11]));
        setups.push(Setup::new(valid, vec![BOT; 11]));
        setups
    }

    /// Mostly a perpendicular turn (legal unless the buffer is full), sometimes any direction.
    fn arbitrary_command(game: &Self, slot: Slot, rng: &mut CoreRng) -> Command {
        let intended = game
            .seat(slot)
            .map_or(Direction::Up, SeatState::intended_direction);
        let perpendicular: Vec<Direction> = Direction::ALL
            .into_iter()
            .filter(|&direction| direction != intended && !direction.is_opposite(intended))
            .collect();
        let choices: &[Direction] = if rng.chance(3, 4) {
            &perpendicular
        } else {
            &Direction::ALL
        };
        Command::Turn(rng.pick(choices).copied().unwrap_or(Direction::Up))
    }

    fn bots_for(players: &[PlayerSpec]) -> Vec<BotSeat<Self>> {
        Slot::all(players.len())
            .zip(players)
            .filter(|(_, player)| player.kind == PlayerKind::Bot)
            .map(|(slot, _)| {
                let kind = if slot.get() % 2 == 0 {
                    BotKind::Efficient
                } else {
                    BotKind::Random
                };
                BotSeat {
                    slot,
                    bot: Snake::make_bot(kind),
                    budget_nodes: Snake::default_bot_budget(kind),
                }
            })
            .collect()
    }

    fn check_invariants(&self) -> Result<(), String> {
        let field = self.field;
        if self.food.len() > usize::from(self.max_food_count) {
            return Err(format!("{} food over the cap", self.food.len()));
        }
        let mut expected = Grid::new(field);
        for &food in &self.food {
            if !expected.is_empty_cell(food) {
                return Err(format!("food {food:?} outside the field or duplicated"));
            }
            expected.set(food, CellContent::Food);
        }
        for (slot, seat) in Slot::all(self.seats.len()).zip(&self.seats) {
            if seat.body.len() < usize::from(START_LENGTH) {
                return Err(format!("seat {slot}: body shorter than the start length"));
            }
            if seat.queued_turns.len() > INPUT_BUFFER
                || (!seat.is_alive() && !seat.queued_turns.is_empty())
            {
                return Err(format!(
                    "seat {slot}: bad turn queue {:?}",
                    seat.queued_turns
                ));
            }
            let contiguous =
                seat.body
                    .iter()
                    .zip(seat.body.iter().skip(1))
                    .all(|(&closer_to_head, &next)| {
                        Direction::ALL.into_iter().any(|direction| {
                            field.neighbour(next, direction) == Some(closer_to_head)
                        })
                    });
            if !contiguous {
                return Err(format!(
                    "seat {slot}: body is not contiguous {:?}",
                    seat.body
                ));
            }
            let solid = seat.is_alive() || self.dead_snake_behavior == StayOnField;
            for &cell in &seat.body {
                if !field.contains(cell) {
                    return Err(format!("seat {slot}: {cell:?} outside the field"));
                }
                if !solid {
                    continue;
                }
                match expected.get(cell) {
                    Some(CellContent::Empty) => expected.set(cell, CellContent::Body(slot)),
                    other => return Err(format!("seat {slot}: {cell:?} already holds {other:?}")),
                }
            }
        }
        if expected != self.grid {
            return Err("occupancy grid out of sync with bodies and food".into());
        }
        let mut probe = self.clone();
        probe.check_end(DeathReason::WallCollision);
        if self.is_running() && !probe.is_running() {
            return Err("end condition met but the game is still running".into());
        }
        Ok(())
    }
}

#[test]
fn snake_passes_conformance() {
    conformance::run_all::<Snake>(30);
}

/// Pinned so accidental rule changes and native/wasm divergence are caught.
#[test]
fn golden_scripted_hash() {
    let setup = Setup::<Snake>::new(
        with_food(field_settings(12, 10, WrapAround, StayOnField), 3, 600),
        vec![HUMAN, BOT, BOT],
    );
    assert_eq!(conformance::scripted_hash(&setup, 2024), GOLDEN_HASH);
}

const GOLDEN_HASH: u64 = 0xA9C8_E5DA_80F7_804E;
