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

type Target = Capped<1, 1000>;

#[derive(Clone, Debug)]
struct RaceSettings {
    target: Target,
}

#[derive(Clone, Debug, PartialEq)]
struct RawRaceSettings {
    target: u32,
}

impl GameSettings for RaceSettings {
    type Raw = RawRaceSettings;

    fn parse(raw: RawRaceSettings) -> Result<Self, SettingsError> {
        Ok(Self {
            target: Target::new(raw.target).map_err(SettingsError::at("target"))?,
        })
    }

    fn to_raw(&self) -> RawRaceSettings {
        RawRaceSettings {
            target: self.target.get(),
        }
    }

    fn ranges(_raw: &RawRaceSettings) -> Vec<SettingRange> {
        vec![Target::range("target")]
    }
}

fn race(target: u32) -> RaceSettings {
    RaceSettings::parse(RawRaceSettings { target }).unwrap()
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
    type Secret = ();
    const INFO: GameInfo = GameInfo {
        id: "race",
        name: "Race",
        min_players: 2,
        max_players: 2,
        has_bots: true,
        cooperative: false,
    };

    fn new(settings: &RaceSettings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        SetupError::check_player_count(players.len(), 2, 2)?;
        Ok(Self {
            target: settings.target.get(),
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
                RawRaceSettings { target: 10 },
                vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
            ),
            Setup::new(
                RawRaceSettings { target: 50 },
                vec![PlayerSpec::HUMAN, PlayerSpec::BOT],
            ),
            Setup::new(
                RawRaceSettings { target: 1000 },
                vec![PlayerSpec::BOT, PlayerSpec::BOT],
            ),
        ]
    }

    fn invalid_setups() -> Vec<Setup<Race>> {
        vec![
            Setup::new(
                RawRaceSettings { target: 0 },
                vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
            ),
            Setup::new(
                RawRaceSettings { target: u32::MAX },
                vec![PlayerSpec::HUMAN, PlayerSpec::HUMAN],
            ),
            Setup::new(RawRaceSettings { target: 10 }, vec![PlayerSpec::HUMAN]),
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
    (race(10), [PlayerSpec::HUMAN, PlayerSpec::HUMAN])
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
    let settings = race(10);
    let error = Authority::<Race>::new(&settings, &[PlayerSpec::HUMAN, PlayerSpec::BOT], 1, vec![])
        .unwrap_err();
    assert!(matches!(error, SetupError::InvalidPlayers(_)));
}

#[test]
fn humans_cannot_command_bot_seats_or_after_drop() {
    let settings = race(10);
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

/// Two humans add 1 in turn, with `idle_frames` empty frames sealed between moves.
fn finished_race_with_idle_frames(policy: HashPolicy, idle_frames: usize) -> Authority<Race> {
    let (settings, players) = two_humans();
    let mut authority = Authority::<Race>::new(&settings, &players, 9, vec![]).unwrap();
    authority.set_hash_policy(policy);
    authority.seal_frame();
    while let Awaiting::Turn(slot) = authority.game().awaiting() {
        for _ in 0..idle_frames {
            authority.seal_frame();
        }
        authority.submit(slot, Add(1)).unwrap();
        authority.seal_frame();
    }
    authority
}

fn log_of(authority: &Authority<Race>) -> CompactLog<GameFrame<Race>> {
    CompactLog::new(
        authority.compact_log().to_vec(),
        authority.next_tick(),
        authority.hash_policy(),
    )
    .unwrap()
}

fn frame_at(tick: u64) -> Frame<u8, u8> {
    Frame {
        tick: Tick::new(tick),
        inputs: vec![Input::World(1)],
        hash: None,
    }
}

#[test]
fn compact_log_with_gaps_replays_to_the_authority_state() {
    let authority = finished_race_with_idle_frames(HashPolicy::every(4), 2);
    let log = log_of(&authority);
    assert!(log.frames().len() < usize::try_from(log.end().get()).unwrap());
    let (settings, players) = two_humans();
    let mut mirror = Match::<Race>::new(&settings, &players).unwrap();
    mirror.apply_compact_log(&log, log.end()).unwrap();
    assert_eq!(mirror.next_tick(), authority.next_tick());
    assert_eq!(mirror.game().state_hash(), authority.game().state_hash());
    assert!(mirror.game().status().is_finished());
}

#[test]
fn compact_log_can_be_applied_tick_by_tick() {
    let authority = finished_race_with_idle_frames(HashPolicy::every(3), 1);
    let log = log_of(&authority);
    let (settings, players) = two_humans();
    let mut mirror = Match::<Race>::new(&settings, &players).unwrap();
    for stop in 1..=log.end().get() {
        mirror.apply_compact_log(&log, Tick::new(stop)).unwrap();
        assert_eq!(mirror.next_tick(), Tick::new(stop));
    }
    assert_eq!(mirror.game().state_hash(), authority.game().state_hash());
}

#[test]
fn compact_log_entries_fill_gaps_without_frames() {
    let log = CompactLog::new(
        vec![frame_at(2), frame_at(5)],
        Tick::new(7),
        HashPolicy::every(3),
    )
    .unwrap();
    let logged: Vec<bool> = log
        .entries_from(Tick::new(1))
        .map(|entry| matches!(entry, CompactLogEntry::Logged(_)))
        .collect();
    assert_eq!(logged, [false, true, false, false, true, false]);
}

#[test]
fn compact_log_with_a_huge_end_is_rejected_before_expansion() {
    let huge = Tick::new(1_000_000_000_000);
    assert!(matches!(
        CompactLog::new(vec![frame_at(3)], huge, HashPolicy::every(10)),
        Err(CompactLogError::GapTooLong { .. })
    ));
    assert!(matches!(
        CompactLog::<Frame<u8, u8>>::new(Vec::new(), huge, HashPolicy::every(10)),
        Err(CompactLogError::GapTooLong { .. })
    ));
    assert!(matches!(
        CompactLog::new(
            vec![frame_at(u64::MAX - 1)],
            Tick::new(u64::MAX),
            HashPolicy::every(10)
        ),
        Err(CompactLogError::GapTooLong { .. })
    ));
}

#[test]
fn compact_log_tail_is_bounded_by_the_hash_policy() {
    let policy = HashPolicy::every(10);
    assert!(CompactLog::new(vec![frame_at(9)], Tick::new(19), policy).is_ok());
    assert!(CompactLog::new(vec![frame_at(9)], Tick::new(20), policy).is_err());
    assert!(CompactLog::<Frame<u8, u8>>::new(Vec::new(), Tick::new(9), policy).is_ok());
    assert!(CompactLog::<Frame<u8, u8>>::new(Vec::new(), Tick::new(10), policy).is_err());
}

#[test]
fn compact_log_ticks_must_increase_and_stay_before_the_end() {
    let policy = HashPolicy::every(10);
    assert_eq!(
        CompactLog::new(vec![frame_at(3), frame_at(3)], Tick::new(5), policy).unwrap_err(),
        CompactLogError::TicksNotIncreasing {
            previous: Tick::new(3),
            next: Tick::new(3)
        }
    );
    assert!(matches!(
        CompactLog::new(vec![frame_at(4), frame_at(2)], Tick::new(5), policy),
        Err(CompactLogError::TicksNotIncreasing { .. })
    ));
    assert_eq!(
        CompactLog::new(vec![frame_at(5)], Tick::new(5), policy).unwrap_err(),
        CompactLogError::FrameAtOrAfterEnd {
            tick: Tick::new(5),
            end: Tick::new(5)
        }
    );
}

#[test]
fn omitted_frame_after_the_end_is_rejected() {
    let authority = finished_race_with_idle_frames(HashPolicy::every(1), 0);
    let end = authority.next_tick();
    let padded = CompactLog::new(
        authority.compact_log().to_vec(),
        Tick::new(end.get() + 2),
        HashPolicy::every(10),
    )
    .unwrap();
    let (settings, players) = two_humans();
    let mut mirror = Match::<Race>::new(&settings, &players).unwrap();
    assert_eq!(
        mirror.apply_compact_log(&padded, padded.end()),
        Err(FrameError::MissingFrameAfterFinish { tick: end })
    );
}

/// Seals until a seat is on turn; returns it.
fn seal_until_turn(authority: &mut Authority<Race>) -> Slot {
    loop {
        authority.seal_frame();
        if let Awaiting::Turn(slot) = authority.game().awaiting() {
            return slot;
        }
    }
}

#[test]
fn bot_turns_thought_away_play_as_run_bots_does() {
    let settings = race(30);
    let players = [PlayerSpec::BOT, PlayerSpec::BOT];
    let mut inline =
        Authority::<Race>::new(&settings, &players, 9, Race::bots_for(&players)).unwrap();
    let mut away =
        Authority::<Race>::new(&settings, &players, 9, Race::bots_for(&players)).unwrap();
    seal_until_turn(&mut inline);
    seal_until_turn(&mut away);
    while !inline.game().status().is_finished() {
        inline.run_bots();
        inline.seal_frame();
        let turn = away.begin_bot_turn().expect("a bot is on turn");
        away.finish_bot_turn(turn.think(&CancelToken::new()));
        away.seal_frame();
    }
    assert!(away.game().status().is_finished());
    assert_eq!(away.game().state_hash(), inline.game().state_hash());
}

#[test]
fn a_bot_move_for_a_match_that_moved_on_is_not_played() {
    let settings = race(10);
    let players = [PlayerSpec::HUMAN, PlayerSpec::BOT];
    let mut authority =
        Authority::<Race>::new(&settings, &players, 3, Race::bots_for(&players)).unwrap();
    if seal_until_turn(&mut authority) == Slot::new(0) {
        authority.submit(Slot::new(0), Add(1)).unwrap();
        authority.seal_frame();
    }
    let turn = authority.begin_bot_turn().expect("the bot is on turn");
    // Out thinking: neither a second turn nor an inline run (which would forfeit it) starts.
    assert!(authority.begin_bot_turn().is_none());
    authority.run_bots();
    assert!(!authority.has_pending_inputs());

    let decided = turn.think(&CancelToken::new());
    authority.seal_frame();
    authority.finish_bot_turn(decided);
    assert!(!authority.has_pending_inputs(), "a stale move is dropped");

    let turn = authority.begin_bot_turn().expect("the bot thinks again");
    authority.finish_bot_turn(turn.think(&CancelToken::new()));
    assert!(authority.has_pending_inputs());
    assert_eq!(authority.rejected_bot_commands(), 0);
    assert_eq!(authority.game().awaiting(), Awaiting::Turn(Slot::new(0)));
}

#[test]
fn a_cancelled_budget_runs_out_at_once() {
    let cancel = CancelToken::new();
    let mut budget = Budget::cancellable(1_000, cancel.clone());
    assert!(budget.spend(1));
    cancel.cancel();
    assert!(!budget.spend(1));
    assert!(budget.is_exhausted());
}
