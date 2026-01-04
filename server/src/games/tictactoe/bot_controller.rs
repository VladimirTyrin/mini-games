use super::board::get_available_moves;
use super::game_state::{Mark, TicTacToeGameState};
use super::win_detector::check_win;
use common::proto::tictactoe::TicTacToeBotType;
use rand::prelude::IndexedRandom;

pub fn calculate_move(
    bot_type: TicTacToeBotType,
    state: &TicTacToeGameState,
) -> Option<(usize, usize)> {
    let available = get_available_moves(&state.board);
    common::log!("Bot calculating move. Board: {}x{}, Available moves: {}",
        state.width, state.height, available.len());

    let result = match bot_type {
        TicTacToeBotType::TictactoeBotTypeRandom => calculate_random_move(state),
        TicTacToeBotType::TictactoeBotTypeWinBlock => calculate_winblock_move(state),
        TicTacToeBotType::TictactoeBotTypeMinimax => calculate_minimax_move(state),
        _ => None,
    };

    common::log!("Bot move result: {:?}", result);
    result
}

fn calculate_random_move(state: &TicTacToeGameState) -> Option<(usize, usize)> {
    let available_moves = get_available_moves(&state.board);
    available_moves.choose(&mut rand::rng()).copied()
}

fn calculate_winblock_move(state: &TicTacToeGameState) -> Option<(usize, usize)> {
    let bot_mark = state.current_mark;
    let opponent_mark = bot_mark.opponent()?;

    if let Some(winning_move) = find_winning_move(state, bot_mark) {
        return Some(winning_move);
    }

    if let Some(blocking_move) = find_winning_move(state, opponent_mark) {
        return Some(blocking_move);
    }

    calculate_random_move(state)
}

fn find_winning_move(state: &TicTacToeGameState, mark: Mark) -> Option<(usize, usize)> {
    let available_moves = get_available_moves(&state.board);

    for (x, y) in available_moves {
        let mut test_board = state.board.clone();
        test_board[y][x] = mark;

        if check_win(&test_board, state.win_count).is_some() {
            return Some((x, y));
        }
    }

    None
}

fn calculate_minimax_move(state: &TicTacToeGameState) -> Option<(usize, usize)> {
    let bot_mark = state.current_mark;
    let available_moves = get_available_moves(&state.board);

    common::log!("Minimax: bot_mark={:?}, available_moves={}", bot_mark, available_moves.len());

    if available_moves.is_empty() {
        common::log!("Minimax: no available moves!");
        return None;
    }

    let depth_limit = calculate_depth_limit(&state.board);
    common::log!("Minimax: depth_limit={}", depth_limit);

    let mut best_move = None;
    let mut best_score = i32::MIN;

    for (x, y) in available_moves {
        let mut test_board = state.board.clone();
        test_board[y][x] = bot_mark;

        let score = minimax(
            &test_board,
            state.win_count,
            0,
            depth_limit,
            false,
            bot_mark,
            i32::MIN,
            i32::MAX,
        );

        if score > best_score {
            best_score = score;
            best_move = Some((x, y));
        }
    }

    best_move
}

fn calculate_depth_limit(board: &[Vec<Mark>]) -> usize {
    let empty_cells = board
        .iter()
        .flat_map(|row| row.iter())
        .filter(|&&cell| cell == Mark::Empty)
        .count();

    if empty_cells <= 4 {
        empty_cells
    } else if empty_cells <= 9 {
        6
    } else if empty_cells <= 16 {
        4
    } else if empty_cells <= 49 {
        2
    } else {
        1
    }
}

fn minimax(
    board: &[Vec<Mark>],
    win_count: usize,
    depth: usize,
    max_depth: usize,
    is_maximizing: bool,
    bot_mark: Mark,
    mut alpha: i32,
    mut beta: i32,
) -> i32 {
    if let Some(winner) = check_win(board, win_count) {
        return if winner == bot_mark {
            1000 - depth as i32
        } else {
            -1000 + depth as i32
        };
    }

    let available_moves = get_available_moves(board);
    if available_moves.is_empty() {
        return 0;
    }

    if depth >= max_depth {
        return evaluate_board(board, bot_mark, win_count);
    }

    if is_maximizing {
        let mut max_eval = i32::MIN;
        for (x, y) in available_moves {
            let mut test_board = board.to_vec();
            test_board[y][x] = bot_mark;

            let eval = minimax(
                &test_board,
                win_count,
                depth + 1,
                max_depth,
                false,
                bot_mark,
                alpha,
                beta,
            );

            max_eval = max_eval.max(eval);
            alpha = alpha.max(eval);
            if beta <= alpha {
                break;
            }
        }
        max_eval
    } else {
        let opponent_mark = bot_mark.opponent().unwrap();
        let mut min_eval = i32::MAX;
        for (x, y) in available_moves {
            let mut test_board = board.to_vec();
            test_board[y][x] = opponent_mark;

            let eval = minimax(
                &test_board,
                win_count,
                depth + 1,
                max_depth,
                true,
                bot_mark,
                alpha,
                beta,
            );

            min_eval = min_eval.min(eval);
            beta = beta.min(eval);
            if beta <= alpha {
                break;
            }
        }
        min_eval
    }
}

fn evaluate_board(board: &[Vec<Mark>], bot_mark: Mark, win_count: usize) -> i32 {
    let opponent_mark = bot_mark.opponent().unwrap();
    let bot_score = count_threats(board, bot_mark, win_count);
    let opponent_score = count_threats(board, opponent_mark, win_count);
    bot_score - opponent_score
}

fn count_threats(board: &[Vec<Mark>], mark: Mark, win_count: usize) -> i32 {
    let height = board.len();
    if height == 0 {
        return 0;
    }
    let width = board[0].len();

    let mut score = 0;

    for y in 0..height {
        for x in 0..width {
            score += check_line_threat(board, x, y, 1, 0, mark, win_count);
            score += check_line_threat(board, x, y, 0, 1, mark, win_count);
            score += check_line_threat(board, x, y, 1, 1, mark, win_count);
            score += check_line_threat(board, x, y, 1, -1, mark, win_count);
        }
    }

    score
}

fn check_line_threat(
    board: &[Vec<Mark>],
    start_x: usize,
    start_y: usize,
    dx: isize,
    dy: isize,
    mark: Mark,
    win_count: usize,
) -> i32 {
    let height = board.len();
    let width = board[0].len();

    let mut count = 0;
    let mut empty_count = 0;

    for i in 0..win_count {
        let x = start_x as isize + dx * i as isize;
        let y = start_y as isize + dy * i as isize;

        if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
            return 0;
        }

        let cell = board[y as usize][x as usize];
        if cell == mark {
            count += 1;
        } else if cell == Mark::Empty {
            empty_count += 1;
        } else {
            return 0;
        }
    }

    if count + empty_count == win_count {
        (count * count) as i32
    } else {
        0
    }
}
