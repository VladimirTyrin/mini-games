//! Driver tests on a toy game: two seats alternately add 1..=3, first to reach the target wins.

use crate::conformance::{self, Conformance, Setup};
use crate::*;

#[derive(Clone, Debug)]
struct Race {
    target: u32,
    totals: Vec<u32>,
    turn: Option<Slot>,
    winner: Option<Slot>,
}

#[derive(Clone, Debug)]
struct RaceSettings {
    target: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct Add(u32);

#[derive(Clone, Debug, PartialEq)]
struct FirstPlayer(Slot);

fn opponent(slot: Slot) -> Slot {
    Slot::new(1 - slot.get())
}

impl Game for Race {
    type Settings = RaceSettings;
    type Command = Add;
    type WorldEvent = FirstPlayer;
    type View = Vec<u32>;
    const INFO: GameInfo = GameInfo {
        id: "race",
        name: "Race",
        min_players: 2,
        max_players: 2,
        has_bots: true,
        cooperative: false,
    };

    fn new(settings: &RaceSettings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        SetupError::check_range("target", settings.target, 1, 1000)?;
        SetupError::check_player_count(players.len(), 2, 2)?;
        Ok(Self {
            target: settings.target,
            totals: vec![0; 2],
            turn: None,
            winner: None,
        })
    }

    fn clock(&self) -> Clock {
        Clock::TurnBased
    }

    fn apply_command(&mut self, slot: Slot, command: &Add) -> Result<(), CommandError> {
        if self.winner.is_some() {
            return Err(CommandError::GameOver);
        }
        if self.turn != Some(slot) {
            return Err(CommandError::NotYourTurn);
        }
        if !(1..=3).contains(&command.0) {
            return Err(CommandError::Illegal("add 1..=3"));
        }
        let total = &mut self.totals[slot.index()];
        *total += command.0;
        if *total >= self.target {
            self.winner = Some(slot);
            self.turn = None;
        } else {
            self.turn = Some(opponent(slot));
        }
        Ok(())
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<FirstPlayer> {
        if self.turn.is_none() && self.winner.is_none() {
            vec![FirstPlayer(Slot::new(rng.below(2) as u8))]
        } else {
            Vec::new()
        }
    }

    fn apply_world(&mut self, event: &FirstPlayer) -> Result<(), CommandError> {
        if self.turn.is_some() || event.0.get() > 1 {
            return Err(CommandError::Illegal("first player already chosen"));
        }
        self.turn = Some(event.0);
        Ok(())
    }

    fn drop_player(&mut self, slot: Slot) {
        if self.winner.is_none() {
            self.winner = Some(opponent(slot));
            self.turn = None;
        }
    }

    fn awaiting(&self) -> Awaiting {
        match (self.winner, self.turn) {
            (None, Some(slot)) => Awaiting::Turn(slot),
            _ => Awaiting::Nobody,
        }
    }

    fn status(&self) -> Status {
        match self.winner {
            None => Status::InProgress,
            Some(winner) => Status::Finished(Outcome {
                kind: OutcomeKind::Winner,
                winners: vec![winner],
                scores: self.totals.iter().map(|&total| i64::from(total)).collect(),
            }),
        }
    }

    fn view(&self, _viewer: Viewer) -> Vec<u32> {
        self.totals.clone()
    }

    fn hash_state(&self, hasher: &mut StateHasher) {
        hasher.u32(self.target);
        for &total in &self.totals {
            hasher.u32(total);
        }
        hasher.u8(self.turn.map_or(u8::MAX, Slot::get));
        hasher.u8(self.winner.map_or(u8::MAX, Slot::get));
    }
}

#[derive(Debug)]
struct RandomAdder;

impl Bot<Race> for RandomAdder {
    fn decide(
        &mut self,
        _game: &Race,
        _me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Add> {
        budget.spend(1);
        Some(Add(1 + rng.below(3)))
    }
}

impl Conformance for Race {
    fn sample_setups() -> Vec<Setup<Race>> {
        vec![
            Setup::new(
                RaceSettings { target: 10 },
                vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
            ),
            Setup::new(
                RaceSettings { target: 50 },
                vec![PlayerSpec::HUMAN, PlayerSpec::BOT],
            ),
            Setup::new(
                RaceSettings { target: 1000 },
                vec![PlayerSpec::BOT, PlayerSpec::BOT],
            ),
        ]
    }

    fn invalid_setups() -> Vec<Setup<Race>> {
        vec![
            Setup::new(
                RaceSettings { target: 0 },
                vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
            ),
            Setup::new(
                RaceSettings { target: u32::MAX },
                vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
            ),
            Setup::new(RaceSettings { target: 10 }, vec![PlayerSpec::HUMAN]),
        ]
    }

    fn arbitrary_command(_game: &Race, _slot: Slot, rng: &mut CoreRng) -> Add {
        Add(rng.below(5))
    }

    fn bots_for(players: &[PlayerSpec]) -> Vec<BotSeat<Race>> {
        Slot::all(players.len())
            .filter(|slot| !players[slot.index()].is_human())
            .map(|slot| BotSeat {
                slot,
                bot: Box::new(RandomAdder),
                budget_nodes: 10,
            })
            .collect()
    }

    fn check_invariants(&self) -> Result<(), String> {
        if self.winner.is_some() && self.turn.is_some() {
            return Err("turn set after the game ended".into());
        }
        Ok(())
    }
}

fn two_humans() -> (RaceSettings, [PlayerSpec; 2]) {
    (
        RaceSettings { target: 10 },
        [PlayerSpec::HUMAN, PlayerSpec::HUMAN],
    )
}

#[test]
fn race_passes_conformance() {
    conformance::run_all::<Race>(25);
}

#[test]
fn hash_mismatch_is_detected() {
    let (settings, players) = two_humans();
    let mut authority = Authority::<Race>::new(&settings, &players, 1, vec![]).unwrap();
    let mut mirror = Match::<Race>::new(&settings, &players).unwrap();
    let mut frame = authority.seal_frame();
    frame.hash = frame.hash.map(|hash| hash ^ 1);
    assert!(matches!(
        mirror.apply_frame(&frame),
        Err(FrameError::HashMismatch { .. })
    ));
}

#[test]
fn frames_must_be_sequential() {
    let (settings, players) = two_humans();
    let mut authority = Authority::<Race>::new(&settings, &players, 1, vec![]).unwrap();
    let mut mirror = Match::<Race>::new(&settings, &players).unwrap();
    authority.seal_frame();
    let second = authority.seal_frame();
    assert_eq!(
        mirror.apply_frame(&second),
        Err(FrameError::UnexpectedTick {
            expected: Tick::ZERO,
            got: Tick::new(1)
        })
    );
}

#[test]
fn bot_seat_without_bot_is_rejected() {
    let settings = RaceSettings { target: 10 };
    let error = Authority::<Race>::new(&settings, &[PlayerSpec::HUMAN, PlayerSpec::BOT], 1, vec![])
        .unwrap_err();
    assert!(matches!(error, SetupError::InvalidPlayers(_)));
}

#[test]
fn humans_cannot_command_bot_seats_or_after_drop() {
    let settings = RaceSettings { target: 10 };
    let players = [PlayerSpec::HUMAN, PlayerSpec::BOT];
    let mut authority =
        Authority::<Race>::new(&settings, &players, 3, Race::bots_for(&players)).unwrap();
    authority.seal_frame();
    assert_eq!(
        authority.submit(Slot::new(1), Add(1)),
        Err(CommandError::NotPlaying)
    );
    authority.drop_player(Slot::new(0));
    assert_eq!(
        authority.submit(Slot::new(0), Add(1)),
        Err(CommandError::NotPlaying)
    );
    assert!(authority.game().status().is_finished());
}

#[test]
fn expand_compact_log_fills_gaps() {
    let compact = vec![
        Frame::<u8, u8> {
            tick: Tick::new(2),
            inputs: vec![Input::World(1)],
            hash: None,
        },
        Frame::<u8, u8> {
            tick: Tick::new(5),
            inputs: vec![],
            hash: Some(7),
        },
    ];
    let frames = expand_compact_log(&compact, Tick::new(7));
    assert_eq!(frames.len(), 7);
    assert!(
        frames
            .iter()
            .enumerate()
            .all(|(i, frame)| frame.tick == Tick::new(i as u64))
    );
    assert_eq!(frames[2].inputs.len(), 1);
    assert_eq!(frames[5].hash, Some(7));
}
