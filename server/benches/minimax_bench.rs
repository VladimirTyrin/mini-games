use criterion::{criterion_group, criterion_main, Criterion};
use mini_games_server::games::tictactoe::bot_controller::{calculate_move, BotInput};
use mini_games_server::games::tictactoe::game_state::Mark;
use common::proto::tictactoe::TicTacToeBotType;

fn create_empty_board(width: usize, height: usize) -> Vec<Vec<Mark>> {
    vec![vec![Mark::Empty; width]; height]
}

fn bench_minimax_10_moves(c: &mut Criterion) {
    c.bench_function("minimax_15x15_10_moves", |b| {
        b.iter(|| {
            let mut board = create_empty_board(15, 15);
            let win_count = 5;
            let mut current_mark = Mark::X;

            for _ in 0..10 {
                let input = BotInput {
                    board: board.clone(),
                    win_count,
                    current_mark,
                };

                if let Some(pos) = calculate_move(TicTacToeBotType::TictactoeBotTypeMinimax, input) {
                    board[pos.y][pos.x] = current_mark;
                    current_mark = current_mark.opponent().unwrap();
                } else {
                    break;
                }
            }
        });
    });
}

fn bench_minimax_single_move_empty_board(c: &mut Criterion) {
    c.bench_function("minimax_15x15_single_move_empty", |b| {
        b.iter(|| {
            let board = create_empty_board(15, 15);
            let input = BotInput {
                board,
                win_count: 5,
                current_mark: Mark::X,
            };
            calculate_move(TicTacToeBotType::TictactoeBotTypeMinimax, input)
        });
    });
}

fn bench_minimax_single_move_mid_game(c: &mut Criterion) {
    c.bench_function("minimax_15x15_single_move_midgame", |b| {
        let mut board = create_empty_board(15, 15);
        // Set up a mid-game position with ~20 moves played
        let moves = [
            (7, 7, Mark::X), (8, 7, Mark::O), (7, 8, Mark::X), (8, 8, Mark::O),
            (6, 6, Mark::X), (9, 9, Mark::O), (5, 5, Mark::X), (10, 10, Mark::O),
            (6, 8, Mark::X), (8, 6, Mark::O), (9, 7, Mark::X), (7, 9, Mark::O),
            (10, 6, Mark::X), (6, 10, Mark::O), (5, 7, Mark::X), (7, 5, Mark::O),
            (4, 8, Mark::X), (8, 4, Mark::O), (3, 9, Mark::X), (9, 3, Mark::O),
        ];
        for (x, y, mark) in moves {
            board[y][x] = mark;
        }

        b.iter(|| {
            let input = BotInput {
                board: board.clone(),
                win_count: 5,
                current_mark: Mark::X,
            };
            calculate_move(TicTacToeBotType::TictactoeBotTypeMinimax, input)
        });
    });
}

criterion_group!(benches, bench_minimax_10_moves, bench_minimax_single_move_empty_board, bench_minimax_single_move_mid_game);
criterion_main!(benches);
