use criterion::{criterion_group, criterion_main, Criterion, SamplingMode};
use std::time::Duration;
use common::games::SessionRng;
use common::games::tictactoe::{calculate_move, BotInput, Mark};
use common::proto::tictactoe::TicTacToeBotType;

fn create_empty_board(width: usize, height: usize) -> Vec<Vec<Mark>> {
    vec![vec![Mark::Empty; width]; height]
}

fn bench_minimax_10_moves() {
    let mut board = create_empty_board(15, 15);
    let win_count = 5;
    let mut current_mark = Mark::X;

    let mut session_rng = SessionRng::from_random();
    for _ in 0..10 {
        let input = BotInput {
            board: board.clone(),
            win_count,
            current_mark,
        };

        if let Some(pos) = calculate_move(TicTacToeBotType::TictactoeBotTypeMinimax, input, &mut session_rng) {
            board[pos.y][pos.x] = current_mark;
            current_mark = current_mark.opponent().unwrap();
        } else {
            break;
        }
    }
}

fn bench_minimax_single_move_empty_board() {
    let board = create_empty_board(15, 15);
    let input = BotInput {
        board,
        win_count: 5,
        current_mark: Mark::X,
    };
    let mut session_rng = SessionRng::from_random();
    calculate_move(TicTacToeBotType::TictactoeBotTypeMinimax, input, &mut session_rng);
}

fn bench_minimax_single_move_mid_game() {
    let mut board = create_empty_board(15, 15);
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

    let input = BotInput {
        board: board.clone(),
        win_count: 5,
        current_mark: Mark::X,
    };
    let mut session_rng = SessionRng::from_random();
    calculate_move(TicTacToeBotType::TictactoeBotTypeMinimax, input, &mut session_rng);
}


fn minimax_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimax");

    group
        .sampling_mode(SamplingMode::Flat)
        .sample_size(10)
        .measurement_time(Duration::from_secs(240));

    group.bench_function("10_moves", |b| {
        b.iter(bench_minimax_10_moves)
    });

    group.bench_function("single_move_empty", |b| {
        b.iter(bench_minimax_single_move_empty_board)
    });

    group.bench_function("single_move_mid_game", |b| {
        b.iter(bench_minimax_single_move_mid_game)
    });

    group.finish();
}

criterion_group!(benches, minimax_bench);
criterion_main!(benches);
