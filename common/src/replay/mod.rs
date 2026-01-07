pub mod recorder;
pub mod file_io;
pub mod player;

pub use recorder::ReplayRecorder;
pub use file_io::{save_replay, load_replay, load_replay_metadata, save_replay_to_bytes, generate_replay_filename};
pub use player::ReplayPlayer;

pub const REPLAY_FILE_EXTENSION: &str = "minigamesreplay";
pub const REPLAY_VERSION: u8 = 1;

#[cfg(test)]
mod integration_tests {
    use crate::{
        PlayerId, ReplayGame, PlayerIdentity, InGameCommand, in_game_command,
        proto::snake::{SnakeInGameCommand, snake_in_game_command, TurnCommand, Direction as ProtoDirection},
        proto::tictactoe::{TicTacToeInGameCommand, tic_tac_toe_in_game_command, PlaceMarkCommand},
        lobby_settings, SnakeBotType, TicTacToeBotType,
    };
    use crate::engine::snake::{GameState as SnakeGameState, FieldSize, WallCollisionMode, DeadSnakeBehavior, Direction, Point, BotController};
    use crate::engine::tictactoe::{TicTacToeGameState, FirstPlayerMode, GameStatus, calculate_move, BotInput};
    use crate::engine::session::SessionRng;
    use super::{ReplayRecorder, ReplayPlayer};

    fn direction_to_proto(dir: Direction) -> i32 {
        match dir {
            Direction::Up => ProtoDirection::Up as i32,
            Direction::Down => ProtoDirection::Down as i32,
            Direction::Left => ProtoDirection::Left as i32,
            Direction::Right => ProtoDirection::Right as i32,
        }
    }

    fn proto_to_direction(proto: i32) -> Direction {
        match ProtoDirection::try_from(proto) {
            Ok(ProtoDirection::Up) => Direction::Up,
            Ok(ProtoDirection::Down) => Direction::Down,
            Ok(ProtoDirection::Left) => Direction::Left,
            Ok(ProtoDirection::Right) => Direction::Right,
            _ => Direction::Up,
        }
    }

    fn create_snake_command(direction: Direction) -> InGameCommand {
        InGameCommand {
            command: Some(in_game_command::Command::Snake(SnakeInGameCommand {
                command: Some(snake_in_game_command::Command::Turn(TurnCommand {
                    direction: direction_to_proto(direction),
                })),
            })),
        }
    }

    fn create_tictactoe_command(x: u32, y: u32) -> InGameCommand {
        InGameCommand {
            command: Some(in_game_command::Command::Tictactoe(TicTacToeInGameCommand {
                command: Some(tic_tac_toe_in_game_command::Command::Place(PlaceMarkCommand { x, y })),
            })),
        }
    }

    #[test]
    fn test_snake_replay_determinism() {
        let game_seed = 12345u64;
        let bot_seed = 99999u64;
        let player1 = PlayerId::new("bot1".to_string());
        let player2 = PlayerId::new("bot2".to_string());

        let field_width = 10;
        let field_height = 10;

        let mut game_state = SnakeGameState::new(
            FieldSize { width: field_width, height: field_height },
            WallCollisionMode::WrapAround,
            DeadSnakeBehavior::Disappear,
            3,
            1.0,
        );

        game_state.add_snake(player1.clone(), Point::new(3, 5), Direction::Up);
        game_state.add_snake(player2.clone(), Point::new(7, 5), Direction::Up);

        let mut game_rng = SessionRng::new(game_seed);
        let mut bot_rng = SessionRng::new(bot_seed);

        let settings = lobby_settings::Settings::Snake(crate::proto::snake::SnakeLobbySettings {
            field_width: field_width as u32,
            field_height: field_height as u32,
            wall_collision_mode: crate::proto::snake::WallCollisionMode::WrapAround as i32,
            dead_snake_behavior: crate::proto::snake::DeadSnakeBehavior::Disappear as i32,
            max_food_count: 3,
            food_spawn_probability: 1.0,
            tick_interval_ms: 100,
        });

        let mut recorder = ReplayRecorder::new(
            "test".to_string(),
            ReplayGame::Snake,
            game_seed,
            Some(settings),
            vec![
                PlayerIdentity { player_id: player1.to_string(), is_bot: true },
                PlayerIdentity { player_id: player2.to_string(), is_bot: true },
            ],
        );

        let max_ticks = 50;
        let mut tick = 0i64;

        while tick < max_ticks {
            let alive_count = game_state.snakes.values().filter(|s| s.is_alive()).count();
            if alive_count <= 1 {
                break;
            }

            if let Some(dir) = BotController::calculate_move(SnakeBotType::Efficient, &player1, &game_state, &mut bot_rng) {
                game_state.set_snake_direction(&player1, dir);
                recorder.record_command(tick, 0, create_snake_command(dir));
            }

            if let Some(dir) = BotController::calculate_move(SnakeBotType::Efficient, &player2, &game_state, &mut bot_rng) {
                game_state.set_snake_direction(&player2, dir);
                recorder.record_command(tick, 1, create_snake_command(dir));
            }

            game_state.update(&mut game_rng);
            tick += 1;
        }

        let final_score_p1_original = game_state.snakes.get(&player1).map(|s| s.score).unwrap_or(0);
        let final_score_p2_original = game_state.snakes.get(&player2).map(|s| s.score).unwrap_or(0);
        let final_alive_p1_original = game_state.snakes.get(&player1).map(|s| s.is_alive()).unwrap_or(false);
        let final_alive_p2_original = game_state.snakes.get(&player2).map(|s| s.is_alive()).unwrap_or(false);

        let replay = recorder.finalize();
        let player = ReplayPlayer::new(replay);

        let mut replay_game_state = SnakeGameState::new(
            FieldSize { width: field_width, height: field_height },
            WallCollisionMode::WrapAround,
            DeadSnakeBehavior::Disappear,
            3,
            1.0,
        );

        replay_game_state.add_snake(player1.clone(), Point::new(3, 5), Direction::Up);
        replay_game_state.add_snake(player2.clone(), Point::new(7, 5), Direction::Up);

        let mut replay_rng = SessionRng::new(player.seed());

        let player_map = vec![player1.clone(), player2.clone()];

        let mut replay_tick = 0i64;
        let mut action_idx = 0;
        let actions = player.replay_ref().actions.clone();

        while replay_tick < tick {
            while action_idx < actions.len() && actions[action_idx].tick == replay_tick {
                let action = &actions[action_idx];
                if let Some(content) = &action.content {
                    if let Some(crate::player_action_content::Content::Command(cmd)) = &content.content {
                        if let Some(in_game_command::Command::Snake(snake_cmd)) = &cmd.command {
                            if let Some(snake_in_game_command::Command::Turn(turn)) = &snake_cmd.command {
                                let dir = proto_to_direction(turn.direction);
                                let player_id = &player_map[action.player_index as usize];
                                replay_game_state.set_snake_direction(player_id, dir);
                            }
                        }
                    }
                }
                action_idx += 1;
            }

            replay_game_state.update(&mut replay_rng);
            replay_tick += 1;
        }

        let final_score_p1_replay = replay_game_state.snakes.get(&player1).map(|s| s.score).unwrap_or(0);
        let final_score_p2_replay = replay_game_state.snakes.get(&player2).map(|s| s.score).unwrap_or(0);
        let final_alive_p1_replay = replay_game_state.snakes.get(&player1).map(|s| s.is_alive()).unwrap_or(false);
        let final_alive_p2_replay = replay_game_state.snakes.get(&player2).map(|s| s.is_alive()).unwrap_or(false);

        assert_eq!(final_score_p1_original, final_score_p1_replay, "Player 1 score should match after replay");
        assert_eq!(final_score_p2_original, final_score_p2_replay, "Player 2 score should match after replay");
        assert_eq!(final_alive_p1_original, final_alive_p1_replay, "Player 1 alive status should match after replay");
        assert_eq!(final_alive_p2_original, final_alive_p2_replay, "Player 2 alive status should match after replay");
    }

    #[test]
    fn test_tictactoe_replay_determinism() {
        let seed = 67890u64;
        let player1 = PlayerId::new("bot1".to_string());
        let player2 = PlayerId::new("bot2".to_string());

        let mut rng = SessionRng::new(seed);
        let mut game_state = TicTacToeGameState::new(
            3, 3, 3,
            vec![player1.clone(), player2.clone()],
            FirstPlayerMode::Random,
            &mut rng,
        );

        let settings = lobby_settings::Settings::Tictactoe(crate::proto::tictactoe::TicTacToeLobbySettings {
            field_width: 3,
            field_height: 3,
            win_count: 3,
            first_player: crate::proto::tictactoe::FirstPlayerMode::Random as i32,
        });

        let mut recorder = ReplayRecorder::new(
            "test".to_string(),
            ReplayGame::Tictactoe,
            seed,
            Some(settings),
            vec![
                PlayerIdentity { player_id: player1.to_string(), is_bot: true },
                PlayerIdentity { player_id: player2.to_string(), is_bot: true },
            ],
        );

        let mut turn = 0i64;

        while game_state.status == GameStatus::InProgress {
            let current = game_state.current_player.clone();
            let player_index = if current == player1 { 0 } else { 1 };

            let bot_input = BotInput::from_game_state(&game_state);
            if let Some(pos) = calculate_move(TicTacToeBotType::TictactoeBotTypeRandom, bot_input, &mut rng) {
                let cmd = create_tictactoe_command(pos.x as u32, pos.y as u32);
                recorder.record_command(turn, player_index, cmd);
                let _ = game_state.place_mark(&current, pos.x, pos.y);
            }

            turn += 1;
            if turn > 20 {
                break;
            }
        }

        let original_status = game_state.status.clone();
        let original_winner = game_state.get_winner();

        let replay = recorder.finalize();
        let player = ReplayPlayer::new(replay);

        let mut replay_rng = SessionRng::new(player.seed());
        let mut replay_game_state = TicTacToeGameState::new(
            3, 3, 3,
            vec![player1.clone(), player2.clone()],
            FirstPlayerMode::Random,
            &mut replay_rng,
        );

        let player_map = vec![player1.clone(), player2.clone()];
        let actions = player.replay_ref().actions.clone();

        for action in &actions {
            if let Some(content) = &action.content {
                if let Some(crate::player_action_content::Content::Command(cmd)) = &content.content {
                    if let Some(in_game_command::Command::Tictactoe(ttt_cmd)) = &cmd.command {
                        if let Some(tic_tac_toe_in_game_command::Command::Place(place)) = &ttt_cmd.command {
                            let player_id = &player_map[action.player_index as usize];
                            let _ = replay_game_state.place_mark(player_id, place.x as usize, place.y as usize);
                        }
                    }
                }
            }
        }

        assert_eq!(original_status, replay_game_state.status, "Game status should match after replay");
        assert_eq!(original_winner, replay_game_state.get_winner(), "Winner should match after replay");
    }
}
