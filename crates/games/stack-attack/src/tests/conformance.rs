use mg_core::conformance::{self, Conformance, Setup};
use mg_core::{CoreRng, PlayerSpec, Slot};

use crate::rules::{MULTI_LINE_BONUS, POINTS_PER_LINE};
use crate::{
    Column, Command, CraneState, Direction, FieldHeight, FieldWidth, MAX_QUEUED_COMMANDS,
    RawSettings, StackAttack,
};

fn humans(seats: usize) -> Vec<PlayerSpec> {
    vec![PlayerSpec::HUMAN; seats]
}

fn sized(field_width: u32, field_height: u32) -> RawSettings {
    RawSettings {
        field_width,
        field_height,
    }
}

impl Conformance for StackAttack {
    fn sample_setups() -> Vec<Setup<Self>> {
        let mut setups: Vec<Setup<Self>> = (1..=4)
            .map(|seats| Setup::new(RawSettings::default(), humans(seats)))
            .collect();
        setups.push(Setup::new(
            sized(FieldWidth::MIN, FieldHeight::MIN),
            humans(4),
        ));
        setups.push(Setup::new(
            sized(FieldWidth::MAX, FieldHeight::MAX),
            humans(1),
        ));
        setups
    }

    fn invalid_setups() -> Vec<Setup<Self>> {
        let default = RawSettings::default();
        vec![
            Setup::new(default, humans(0)),
            Setup::new(default, humans(5)),
            Setup::new(default, vec![PlayerSpec::BOT]),
            Setup::new(default, vec![PlayerSpec::HUMAN, PlayerSpec::BOT]),
            Setup::new(sized(FieldWidth::MIN - 1, 10), humans(1)),
            Setup::new(sized(FieldWidth::MAX + 1, 10), humans(1)),
            Setup::new(sized(15, FieldHeight::MIN - 1), humans(1)),
            Setup::new(sized(15, FieldHeight::MAX + 1), humans(1)),
            Setup::new(sized(0, 0), humans(1)),
            Setup::new(sized(u32::MAX, u32::MAX), humans(1)),
        ]
    }

    /// Mostly random, but dodges threatened columns so matches last long enough to reach line
    /// clears, higher levels and the ceiling instead of ending with an early crush.
    fn arbitrary_command(game: &Self, slot: Slot, rng: &mut CoreRng) -> Command {
        let random = match rng.below(5) {
            0 | 1 => Command::Move(Direction::Left),
            2 | 3 => Command::Move(Direction::Right),
            _ => Command::Jump,
        };
        let Some(worker) = game.workers.get(slot.index()) else {
            return random;
        };
        let threatened = |column: Column| {
            game.field
                .boxes()
                .iter()
                .any(|b| b.falling && b.cell.column() == column && b.cell.y > worker.feet.y)
                || game
                    .cranes
                    .iter()
                    .any(|c| c.state == CraneState::Moving && c.target_x == column)
        };
        if !threatened(worker.feet.column()) || rng.chance(1, 4) {
            return random;
        }
        if threatened(Column::new(worker.feet.x - 1)) {
            Command::Move(Direction::Right)
        } else {
            Command::Move(Direction::Left)
        }
    }

    fn check_invariants(&self) -> Result<(), String> {
        self.field.check_consistency()?;
        for (seat, worker) in self.workers.iter().enumerate() {
            if !self.field.contains(worker.feet) || !self.field.contains(worker.head()) {
                return Err(format!("worker {seat} outside the field: {worker:?}"));
            }
            if worker.queue.len() > MAX_QUEUED_COMMANDS {
                return Err(format!("worker {seat} queued too much"));
            }
            if !worker.is_alive() && !worker.queue.is_empty() {
                return Err(format!("worker {seat} is out of play with queued commands"));
            }
            if worker.is_alive()
                && (self.field.is_occupied(worker.feet) || self.field.is_occupied(worker.head()))
            {
                return Err(format!("box overlaps living worker {seat}"));
            }
        }
        let expected_score = u64::from(self.lines_cleared)
            * u64::from(POINTS_PER_LINE + MULTI_LINE_BONUS)
            - u64::from(self.clearing_ticks) * u64::from(MULTI_LINE_BONUS);
        if u64::from(self.score.get()) != expected_score {
            return Err(format!(
                "score {} does not match {} lines in {} clears",
                self.score.get(),
                self.lines_cleared,
                self.clearing_ticks
            ));
        }
        let width = self.field.width();
        if let Some(crane) = self
            .cranes
            .iter()
            .find(|c| !(-1..=width).contains(&c.x.get()))
        {
            return Err(format!("crane outside its track: {crane:?}"));
        }
        if self.end_reason.is_none() && !self.any_worker_alive() {
            return Err("no worker alive but the game goes on".into());
        }
        Ok(())
    }

    fn max_frames() -> u64 {
        4_000
    }
}

#[test]
fn passes_conformance() {
    conformance::run_all::<StackAttack>(40);
}

#[test]
fn scripted_match_hash_is_pinned() {
    let setup = Setup::<StackAttack>::new(RawSettings::default(), humans(2));
    assert_eq!(
        conformance::scripted_hash(&setup, 42),
        0x74D5_FE84_B620_B19C
    );
}
