use super::board::get_available_moves;
use super::game_state::{Mark, TicTacToeGameState};
use super::types::Position;
use common::proto::tictactoe::TicTacToeBotType;
use rand::prelude::IndexedRandom;

pub struct BotInput {
    pub board: Vec<Vec<Mark>>,
    pub win_count: usize,
    pub current_mark: Mark,
}

impl BotInput {
    pub fn from_game_state(state: &TicTacToeGameState) -> Self {
        Self {
            board: state.board.clone(),
            win_count: state.win_count,
            current_mark: state.current_mark,
        }
    }
}

pub fn calculate_move(bot_type: TicTacToeBotType, input: BotInput) -> Option<Position> {
    match bot_type {
        TicTacToeBotType::TictactoeBotTypeRandom => calculate_random_move(&input),
        TicTacToeBotType::TictactoeBotTypeMinimax => calculate_minimax_move(&input),
        _ => None,
    }
}

fn calculate_random_move(input: &BotInput) -> Option<Position> {
    let available_moves = get_available_moves(&input.board);
    available_moves.choose(&mut rand::rng()).map(|&(x, y)| Position::new(x, y))
}

fn calculate_minimax_move(input: &BotInput) -> Option<Position> {
    let bot_mark = input.current_mark;
    let available_moves = get_available_moves(&input.board);

    if available_moves.is_empty() {
        return None;
    }

    let depth_limit = calculate_depth_limit(&input.board);
    let mut board = input.board.clone();

    let mut best_move = None;
    let mut best_score = i32::MIN;

    for (x, y) in available_moves {
        board[y][x] = bot_mark;

        let score = minimax(
            &mut board,
            input.win_count,
            0,
            depth_limit,
            false,
            bot_mark,
            i32::MIN,
            i32::MAX,
            x,
            y,
        );

        board[y][x] = Mark::Empty;

        if score > best_score {
            best_score = score;
            best_move = Some(Position::new(x, y));
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

fn check_win_at(board: &[Vec<Mark>], win_count: usize, x: usize, y: usize) -> Option<Mark> {
    let mark = board[y][x];
    if mark == Mark::Empty {
        return None;
    }

    let height = board.len();
    let width = board[0].len();
    let win_count_i = win_count as isize;

    let directions: [(isize, isize); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];

    for (dx, dy) in directions {
        let mut count = 1;

        let mut i = 1isize;
        while i < win_count_i {
            let nx = x as isize + dx * i;
            let ny = y as isize + dy * i;
            if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                break;
            }
            if board[ny as usize][nx as usize] != mark {
                break;
            }
            count += 1;
            i += 1;
        }

        let mut i = 1isize;
        while i < win_count_i {
            let nx = x as isize - dx * i;
            let ny = y as isize - dy * i;
            if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                break;
            }
            if board[ny as usize][nx as usize] != mark {
                break;
            }
            count += 1;
            i += 1;
        }

        if count >= win_count {
            return Some(mark);
        }
    }

    None
}

fn minimax(
    board: &mut [Vec<Mark>],
    win_count: usize,
    depth: usize,
    max_depth: usize,
    is_maximizing: bool,
    bot_mark: Mark,
    mut alpha: i32,
    mut beta: i32,
    last_x: usize,
    last_y: usize,
) -> i32 {
    if let Some(winner) = check_win_at(board, win_count, last_x, last_y) {
        return if winner == bot_mark {
            1000 - depth as i32
        } else {
            -1000 + depth as i32
        };
    }

    if depth >= max_depth {
        return evaluate_board(board, bot_mark, win_count);
    }

    if is_maximizing {
        let mut max_eval = i32::MIN;
        for y in 0..board.len() {
            for x in 0..board[0].len() {
                if board[y][x] != Mark::Empty {
                    continue;
                }

                board[y][x] = bot_mark;
                let eval = minimax(
                    board,
                    win_count,
                    depth + 1,
                    max_depth,
                    false,
                    bot_mark,
                    alpha,
                    beta,
                    x,
                    y,
                );
                board[y][x] = Mark::Empty;

                max_eval = max_eval.max(eval);
                alpha = alpha.max(eval);
                if beta <= alpha {
                    return max_eval;
                }
            }
        }
        if max_eval == i32::MIN { 0 } else { max_eval }
    } else {
        let opponent_mark = bot_mark.opponent().unwrap();
        let mut min_eval = i32::MAX;
        for y in 0..board.len() {
            for x in 0..board[0].len() {
                if board[y][x] != Mark::Empty {
                    continue;
                }

                board[y][x] = opponent_mark;
                let eval = minimax(
                    board,
                    win_count,
                    depth + 1,
                    max_depth,
                    true,
                    bot_mark,
                    alpha,
                    beta,
                    x,
                    y,
                );
                board[y][x] = Mark::Empty;

                min_eval = min_eval.min(eval);
                beta = beta.min(eval);
                if beta <= alpha {
                    return min_eval;
                }
            }
        }
        if min_eval == i32::MAX { 0 } else { min_eval }
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
