use crate::games::SessionRng;
use crate::proto::tictactoe::TicTacToeBotType;
use super::board::get_available_moves;
use super::game_state::TicTacToeGameState;
use super::types::{Mark, Position};

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

pub fn calculate_move(
    bot_type: TicTacToeBotType,
    input: BotInput,
    rng: &mut SessionRng,
) -> Option<Position> {
    match bot_type {
        TicTacToeBotType::TictactoeBotTypeRandom => calculate_random_move(&input, rng),
        TicTacToeBotType::TictactoeBotTypeMinimax => calculate_minimax_move(&input),
        _ => None,
    }
}

fn calculate_random_move(input: &BotInput, rng: &mut SessionRng) -> Option<Position> {
    let available_moves = get_available_moves(&input.board);
    if available_moves.is_empty() {
        return None;
    }
    let idx = rng.random_range(0..available_moves.len());
    let (x, y) = available_moves[idx];
    Some(Position::new(x, y))
}

pub fn calculate_minimax_move(input: &BotInput) -> Option<Position> {
    let bot_mark = input.current_mark;
    let opponent_mark = bot_mark.opponent()?;
    let available_moves = get_available_moves(&input.board);

    if available_moves.is_empty() {
        return None;
    }

    let mut board = input.board.clone();

    if let Some((x, y)) = find_winning_move(&mut board, bot_mark, input.win_count, &available_moves)
    {
        return Some(Position::new(x, y));
    }

    if let Some((x, y)) =
        find_winning_move(&mut board, opponent_mark, input.win_count, &available_moves)
    {
        return Some(Position::new(x, y));
    }

    if let Some((x, y)) =
        find_open_threat_move(&mut board, bot_mark, input.win_count, &available_moves)
    {
        return Some(Position::new(x, y));
    }

    if let Some((x, y)) =
        find_open_threat_move(&mut board, opponent_mark, input.win_count, &available_moves)
    {
        return Some(Position::new(x, y));
    }

    if let Some((x, y)) =
        find_double_block_move(&mut board, opponent_mark, input.win_count, &available_moves)
    {
        return Some(Position::new(x, y));
    }

    let depth_limit = calculate_depth_limit(available_moves.len());
    let initial_score = evaluate_board(&board, bot_mark, input.win_count);

    let mut best_move = None;
    let mut best_score = i32::MIN;

    for (x, y) in &available_moves {
        let (x, y) = (*x, *y);
        let delta = eval_delta_before_move(&board, bot_mark, input.win_count, x, y, bot_mark);
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
            initial_score + delta,
        );

        board[y][x] = Mark::Empty;

        if score > best_score {
            best_score = score;
            best_move = Some(Position::new(x, y));
        }
    }

    best_move
}

fn find_winning_move(
    board: &mut [Vec<Mark>],
    mark: Mark,
    win_count: usize,
    moves: &[(usize, usize)],
) -> Option<(usize, usize)> {
    for &(x, y) in moves {
        board[y][x] = mark;
        let winner = check_win_at(board, win_count, x, y);
        board[y][x] = Mark::Empty;

        if winner == Some(mark) {
            return Some((x, y));
        }
    }
    None
}

fn find_open_threat_move(
    board: &mut [Vec<Mark>],
    mark: Mark,
    win_count: usize,
    moves: &[(usize, usize)],
) -> Option<(usize, usize)> {
    for &(x, y) in moves {
        board[y][x] = mark;
        if has_open_threat(board, mark, win_count, win_count - 1, x, y) {
            board[y][x] = Mark::Empty;
            return Some((x, y));
        }
        board[y][x] = Mark::Empty;
    }
    None
}

fn find_double_block_move(
    board: &mut [Vec<Mark>],
    opponent_mark: Mark,
    win_count: usize,
    moves: &[(usize, usize)],
) -> Option<(usize, usize)> {
    for &(x, y) in moves {
        board[y][x] = opponent_mark;
        let winning_moves = count_winning_moves(board, opponent_mark, win_count, moves, x, y);
        board[y][x] = Mark::Empty;

        if winning_moves >= 2 {
            return Some((x, y));
        }
    }
    None
}

fn count_winning_moves(
    board: &mut [Vec<Mark>],
    mark: Mark,
    win_count: usize,
    moves: &[(usize, usize)],
    exclude_x: usize,
    exclude_y: usize,
) -> usize {
    let mut count = 0;
    for &(x, y) in moves {
        if x == exclude_x && y == exclude_y {
            continue;
        }
        if board[y][x] != Mark::Empty {
            continue;
        }

        board[y][x] = mark;
        if check_win_at(board, win_count, x, y) == Some(mark) {
            count += 1;
        }
        board[y][x] = Mark::Empty;
    }
    count
}

fn has_open_threat(
    board: &[Vec<Mark>],
    mark: Mark,
    win_count: usize,
    required_count: usize,
    last_x: usize,
    last_y: usize,
) -> bool {
    let height = board.len();
    let width = board[0].len();
    let directions: [(isize, isize); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];

    for (dx, dy) in directions {
        let mut count = 1;
        let mut open_ends = 0;

        let mut pos_end = 1isize;
        for i in 1..win_count as isize {
            let nx = last_x as isize + dx * i;
            let ny = last_y as isize + dy * i;
            if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                break;
            }
            if board[ny as usize][nx as usize] != mark {
                break;
            }
            count += 1;
            pos_end = i + 1;
        }

        let check_x = last_x as isize + dx * pos_end;
        let check_y = last_y as isize + dy * pos_end;
        if check_x >= 0
            && check_y >= 0
            && check_x < width as isize
            && check_y < height as isize
            && board[check_y as usize][check_x as usize] == Mark::Empty
        {
            open_ends += 1;
        }

        let mut neg_end = 1isize;
        for i in 1..win_count as isize {
            let nx = last_x as isize - dx * i;
            let ny = last_y as isize - dy * i;
            if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                break;
            }
            if board[ny as usize][nx as usize] != mark {
                break;
            }
            count += 1;
            neg_end = i + 1;
        }

        let check_x = last_x as isize - dx * neg_end;
        let check_y = last_y as isize - dy * neg_end;
        if check_x >= 0
            && check_y >= 0
            && check_x < width as isize
            && check_y < height as isize
            && board[check_y as usize][check_x as usize] == Mark::Empty
        {
            open_ends += 1;
        }

        if count >= required_count && open_ends >= 2 {
            return true;
        }
    }

    false
}

fn calculate_depth_limit(moves_count: usize) -> usize {
    match moves_count {
        0..=4 => moves_count,
        5..=9 => 6,
        10..=16 => 5,
        17..=36 => 4,
        _ => 3,
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
    current_score: i32,
) -> i32 {
    if let Some(winner) = check_win_at(board, win_count, last_x, last_y) {
        return if winner == bot_mark {
            1000 - depth as i32
        } else {
            -1000 + depth as i32
        };
    }

    if depth >= max_depth {
        return current_score;
    }

    let moves = get_available_moves(board);

    if is_maximizing {
        let mut max_eval = i32::MIN;
        for (x, y) in moves {
            let delta = eval_delta_before_move(board, bot_mark, win_count, x, y, bot_mark);
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
                current_score + delta,
            );
            board[y][x] = Mark::Empty;

            max_eval = max_eval.max(eval);
            alpha = alpha.max(eval);
            if beta <= alpha {
                return max_eval;
            }
        }
        if max_eval == i32::MIN { 0 } else { max_eval }
    } else {
        let opponent_mark = bot_mark.opponent().unwrap();
        let mut min_eval = i32::MAX;
        for (x, y) in moves {
            let delta = eval_delta_before_move(board, bot_mark, win_count, x, y, opponent_mark);
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
                current_score + delta,
            );
            board[y][x] = Mark::Empty;

            min_eval = min_eval.min(eval);
            beta = beta.min(eval);
            if beta <= alpha {
                return min_eval;
            }
        }
        if min_eval == i32::MAX { 0 } else { min_eval }
    }
}

fn eval_delta_before_move(
    board: &[Vec<Mark>],
    bot_mark: Mark,
    win_count: usize,
    x: usize,
    y: usize,
    move_mark: Mark,
) -> i32 {
    let height = board.len();
    let width = board[0].len();
    let directions: [(isize, isize); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];

    let mut delta = 0i32;

    for (dx, dy) in directions {
        for offset in 0..win_count as isize {
            let start_x = x as isize - dx * offset;
            let start_y = y as isize - dy * offset;
            let end_x = start_x + dx * (win_count as isize - 1);
            let end_y = start_y + dy * (win_count as isize - 1);

            if start_x < 0
                || start_y < 0
                || end_x < 0
                || end_y < 0
                || start_x >= width as isize
                || start_y >= height as isize
                || end_x >= width as isize
                || end_y >= height as isize
            {
                continue;
            }

            let mut bot_count = 0;
            let mut opp_count = 0;

            for i in 0..win_count as isize {
                let cx = (start_x + dx * i) as usize;
                let cy = (start_y + dy * i) as usize;
                match board[cy][cx] {
                    Mark::Empty => {}
                    m if m == bot_mark => bot_count += 1,
                    _ => opp_count += 1,
                }
            }

            let old_score = if opp_count == 0 {
                bot_count * bot_count
            } else if bot_count == 0 {
                -(opp_count * opp_count)
            } else {
                0
            };

            let new_score = if move_mark == bot_mark {
                if opp_count == 0 {
                    (bot_count + 1) * (bot_count + 1)
                } else {
                    0
                }
            } else if bot_count == 0 {
                -((opp_count + 1) * (opp_count + 1))
            } else {
                0
            };

            delta += new_score - old_score;
        }
    }

    delta
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

#[inline(always)]
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
    let last = (win_count - 1) as isize;

    let end_x = start_x as isize + dx * last;
    let end_y = start_y as isize + dy * last;
    if end_x < 0 || end_y < 0 || end_x >= width as isize || end_y >= height as isize {
        return 0;
    }

    let mut count = 0;

    for i in 0..win_count {
        let cx = start_x.wrapping_add_signed(dx * i as isize);
        let cy = start_y.wrapping_add_signed(dy * i as isize);
        let cell = board[cy][cx];
        if cell == mark {
            count += 1;
        } else if cell != Mark::Empty {
            return 0;
        }
    }

    if count == 0 {
        return 0;
    }

    let mut open_ends = 0;
    let before_x = start_x as isize - dx;
    let before_y = start_y as isize - dy;
    if before_x >= 0
        && before_y >= 0
        && before_x < width as isize
        && before_y < height as isize
        && board[before_y as usize][before_x as usize] == Mark::Empty
    {
        open_ends += 1;
    }

    let after_x = end_x + dx;
    let after_y = end_y + dy;
    if after_x >= 0
        && after_y >= 0
        && after_x < width as isize
        && after_y < height as isize
        && board[after_y as usize][after_x as usize] == Mark::Empty
    {
        open_ends += 1;
    }

    let mut score = 1 << (count * 2);

    if count == win_count - 1 {
        score *= if open_ends == 2 { 16 } else { 4 };
    } else if count == win_count - 2 && open_ends == 2 {
        score *= 8;
    } else if open_ends == 2 {
        score *= 2;
    }

    score
}
