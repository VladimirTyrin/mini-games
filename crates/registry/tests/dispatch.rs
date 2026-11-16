use mg_core::{Clock, Game, HasBots, PlayerSpec, Slot, Viewer};
use mg_numbers_match::NumbersMatch;
use mg_protocol::{WireBots, WireGame, proto};
use mg_puzzle2048::Puzzle2048;
use mg_registry::{CreateError, GameKind, SeatSpec, create_authority, create_mirror};
use mg_snake::Snake;
use mg_stack_attack::StackAttack;
use mg_tictactoe::TicTacToe;

fn default_settings<G: WireGame>() -> proto::GameSettings
where
    G::Settings: Default,
{
    G::settings_to_wire(&G::Settings::default())
}

/// Runs a few frames through the type-erased authority and mirror and checks they agree.
fn run_frames(settings: &proto::GameSettings, seats: &[SeatSpec], frames: usize) {
    let mut authority = create_authority(settings, seats, 7).unwrap();
    let players: Vec<PlayerSpec> = seats.iter().map(SeatSpec::player_spec).collect();
    let mut mirror = create_mirror(settings, &players).unwrap();
    assert_eq!(authority.kind(), mirror.kind());
    for _ in 0..frames {
        if authority.status().is_finished() {
            break;
        }
        authority.run_bots();
        let frame = authority.seal_frame();
        mirror.apply_frame(&frame).unwrap();
    }
    assert_eq!(authority.next_tick(), mirror.next_tick());
    assert!(mirror.view(Viewer::Spectator).game.is_some());
}

#[test]
fn every_game_runs_through_the_registry() {
    let human = || SeatSpec::Human;
    run_frames(
        &default_settings::<Snake>(),
        &[
            human(),
            SeatSpec::Bot(Snake::bot_kind_to_wire(Snake::BOT_KINDS[0])),
        ],
        50,
    );
    run_frames(
        &default_settings::<TicTacToe>(),
        &[
            SeatSpec::Bot(TicTacToe::bot_kind_to_wire(TicTacToe::BOT_KINDS[1])),
            SeatSpec::Bot(TicTacToe::bot_kind_to_wire(TicTacToe::BOT_KINDS[1])),
        ],
        300,
    );
    run_frames(&default_settings::<StackAttack>(), &[human(), human()], 50);
    run_frames(&default_settings::<Puzzle2048>(), &[human()], 3);
    run_frames(&default_settings::<NumbersMatch>(), &[human()], 3);
}

#[test]
fn kinds_and_clocks_are_reported() {
    let settings = default_settings::<Snake>();
    assert_eq!(GameKind::of_settings(&settings).unwrap(), GameKind::Snake);
    let authority = create_authority(&settings, &[SeatSpec::Human], 1).unwrap();
    assert!(matches!(authority.clock(), Clock::Realtime { .. }));
    assert!(GameKind::ALL.contains(&GameKind::Snake));
    assert_eq!(GameKind::TicTacToe.info().id, TicTacToe::INFO.id);
}

#[test]
fn bots_are_rejected_for_games_without_bots() {
    let bot = SeatSpec::Bot(Snake::bot_kind_to_wire(Snake::BOT_KINDS[0]));
    let error = create_authority(
        &default_settings::<StackAttack>(),
        &[SeatSpec::Human, bot],
        1,
    )
    .unwrap_err();
    assert!(matches!(error, CreateError::NoBots(_)));
}

#[test]
fn foreign_bot_kind_and_missing_settings_are_rejected() {
    let snake_bot = SeatSpec::Bot(Snake::bot_kind_to_wire(Snake::BOT_KINDS[0]));
    assert!(
        create_authority(
            &default_settings::<TicTacToe>(),
            &[SeatSpec::Human, snake_bot],
            1
        )
        .is_err()
    );
    assert!(create_authority(&proto::GameSettings { game: None }, &[SeatSpec::Human], 1).is_err());
}

#[test]
fn submitted_commands_are_decoded_and_validated() {
    let settings = default_settings::<Puzzle2048>();
    let mut authority = create_authority(&settings, &[SeatSpec::Human], 3).unwrap();
    authority.seal_frame();
    let foreign = TicTacToe::command_to_wire(&mg_tictactoe::Command::Place(mg_tictactoe::Point {
        x: 0,
        y: 0,
    }));
    assert!(authority.submit(Slot::new(0), &foreign).is_err());
}
