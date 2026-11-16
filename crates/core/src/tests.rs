//! Driver tests on a toy game ("race to 10").

use crate::*;

/// Two seats alternately add 1..=3; first to reach the target wins. Who starts is random.
#[derive(Clone, Debug)]
struct Race {
    target: u32,
    totals: Vec<u32>,
    turn: Option<Slot>,
    dropped: Vec<bool>,
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
        if players.len() != 2 {
            return Err(SetupError::PlayerCount {
                got: players.len(),
                min: 2,
                max: 2,
            });
        }
        Ok(Self {
            target: settings.target,
            totals: vec![0; 2],
            turn: None,
            dropped: vec![false; 2],
            winner: None,
        })
    }

    fn clock(&self) -> Clock {
        Clock::TurnBased
    }

    fn apply_command(&mut self, slot: Slot, cmd: &Add) -> Result<(), CommandError> {
        if self.winner.is_some() {
            return Err(CommandError::GameOver);
        }
        if self.turn != Some(slot) {
            return Err(CommandError::NotYourTurn);
        }
        if !(1..=3).contains(&cmd.0) {
            return Err(CommandError::Illegal("add 1..=3"));
        }
        let i = usize::from(slot);
        self.totals[i] += cmd.0;
        if self.totals[i] >= self.target {
            self.winner = Some(slot);
            self.turn = None;
        } else {
            self.turn = Some(1 - slot);
        }
        Ok(())
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<FirstPlayer> {
        if self.turn.is_none() && self.winner.is_none() {
            vec![FirstPlayer(rng.below(2) as Slot)]
        } else {
            Vec::new()
        }
    }

    fn apply_world(&mut self, event: &FirstPlayer) -> Result<(), CommandError> {
        if self.turn.is_some() || event.0 > 1 {
            return Err(CommandError::Illegal("first player already chosen"));
        }
        self.turn = Some(event.0);
        Ok(())
    }

    fn drop_player(&mut self, slot: Slot) {
        if self.winner.is_none() {
            self.dropped[usize::from(slot)] = true;
            self.winner = Some(1 - slot);
            self.turn = None;
        }
    }

    fn awaiting(&self) -> Awaiting {
        match (self.winner, self.turn) {
            (None, Some(s)) => Awaiting::Turn(s),
            _ => Awaiting::Nobody,
        }
    }

    fn status(&self) -> Status {
        match self.winner {
            None => Status::InProgress,
            Some(w) => Status::Finished(Outcome {
                kind: OutcomeKind::Winner,
                winners: vec![w],
                scores: self.totals.iter().map(|&t| i64::from(t)).collect(),
            }),
        }
    }

    fn view(&self, _viewer: Viewer) -> Vec<u32> {
        self.totals.clone()
    }

    fn hash_state(&self, h: &mut StateHasher) {
        h.u32(self.target);
        for &t in &self.totals {
            h.u32(t);
        }
        h.u8(self.turn.unwrap_or(255));
        h.u8(self.winner.unwrap_or(255));
    }
}

#[derive(Debug)]
struct GreedyBot;

impl Bot<Race> for GreedyBot {
    fn decide(
        &mut self,
        _g: &Race,
        _me: Slot,
        rng: &mut CoreRng,
        budget: &mut Budget,
    ) -> Option<Add> {
        budget.spend(1);
        Some(Add(1 + rng.below(3)))
    }
}

impl conformance::Conformance for Race {
    fn sample_setups() -> Vec<(RaceSettings, Vec<PlayerSpec>)> {
        vec![
            (
                RaceSettings { target: 10 },
                vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
            ),
            (
                RaceSettings { target: 50 },
                vec![PlayerSpec::HUMAN, PlayerSpec::BOT],
            ),
            (
                RaceSettings { target: 1000 },
                vec![PlayerSpec::BOT, PlayerSpec::BOT],
            ),
        ]
    }

    fn invalid_setups() -> Vec<(RaceSettings, Vec<PlayerSpec>)> {
        vec![
            (
                RaceSettings { target: 0 },
                vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
            ),
            (
                RaceSettings { target: u32::MAX },
                vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
            ),
            (RaceSettings { target: 10 }, vec![PlayerSpec::HUMAN]),
        ]
    }

    fn arbitrary_command(_g: &Race, _slot: Slot, rng: &mut CoreRng) -> Add {
        Add(rng.below(5))
    }

    fn bots_for(players: &[PlayerSpec]) -> Vec<BotSeat<Race>> {
        players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.kind == PlayerKind::Bot)
            .map(|(i, _)| BotSeat {
                slot: i as Slot,
                bot: Box::new(GreedyBot),
                budget: 10,
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

#[test]
fn race_passes_conformance() {
    conformance::run_all::<Race>(25);
}

#[test]
fn hash_mismatch_is_detected() {
    let settings = RaceSettings { target: 10 };
    let players = [PlayerSpec::HUMAN, PlayerSpec::HUMAN];
    let mut auth = Authority::<Race>::new(&settings, &players, 1, vec![]).unwrap();
    let mut mirror = Match::<Race>::new(&settings, &players).unwrap();
    let mut frame = auth.seal();
    frame.hash = Some(frame.hash.unwrap() ^ 1);
    assert!(matches!(
        mirror.apply_frame(&frame),
        Err(FrameError::HashMismatch { .. })
    ));
}

#[test]
fn frames_must_be_sequential() {
    let settings = RaceSettings { target: 10 };
    let players = [PlayerSpec::HUMAN, PlayerSpec::HUMAN];
    let mut auth = Authority::<Race>::new(&settings, &players, 1, vec![]).unwrap();
    let mut mirror = Match::<Race>::new(&settings, &players).unwrap();
    let _skipped = auth.seal();
    let second = auth.seal();
    assert_eq!(
        mirror.apply_frame(&second),
        Err(FrameError::UnexpectedTick {
            expected: 0,
            got: 1
        })
    );
}

#[test]
fn bot_seat_without_bot_is_rejected() {
    let settings = RaceSettings { target: 10 };
    let err = Authority::<Race>::new(&settings, &[PlayerSpec::HUMAN, PlayerSpec::BOT], 1, vec![])
        .unwrap_err();
    assert!(matches!(err, SetupError::InvalidPlayers(_)));
}

#[test]
fn humans_cannot_command_bot_seats_or_after_drop() {
    let settings = RaceSettings { target: 10 };
    let players = [PlayerSpec::HUMAN, PlayerSpec::BOT];
    let mut auth =
        Authority::<Race>::new(&settings, &players, 3, Race::bots_for(&players)).unwrap();
    auth.seal();
    assert_eq!(auth.submit(1, Add(1)), Err(CommandError::NotPlaying));
    auth.drop_player(0);
    assert_eq!(auth.submit(0, Add(1)), Err(CommandError::NotPlaying));
    assert!(auth.game().status().is_finished());
}

#[test]
fn expand_log_fills_gaps() {
    let compact = vec![
        Frame::<u8, u8> {
            tick: 2,
            inputs: vec![Input::World(1)],
            hash: None,
        },
        Frame::<u8, u8> {
            tick: 5,
            inputs: vec![],
            hash: Some(7),
        },
    ];
    let full = expand_log(&compact, 7);
    assert_eq!(full.len(), 7);
    assert!(full.iter().enumerate().all(|(i, f)| f.tick == i as u64));
    assert_eq!(full[2].inputs.len(), 1);
    assert_eq!(full[5].hash, Some(7));
}

use conformance::Conformance as _;
