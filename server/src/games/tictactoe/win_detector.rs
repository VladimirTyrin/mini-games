use super::game_state::Mark;

pub fn check_win(board: &[Vec<Mark>], win_count: usize) -> Option<Mark> {
    let height = board.len();
    if height == 0 {
        return None;
    }
    let width = board[0].len();

    for y in 0..height {
        for x in 0..width {
            let mark = board[y][x];
            if mark == Mark::Empty {
                continue;
            }

            if check_horizontal(board, x, y, mark, win_count) {
                return Some(mark);
            }
            if check_vertical(board, x, y, mark, win_count) {
                return Some(mark);
            }
            if check_diagonal_down_right(board, x, y, mark, win_count) {
                return Some(mark);
            }
            if check_diagonal_down_left(board, x, y, mark, win_count) {
                return Some(mark);
            }
        }
    }

    None
}

fn check_horizontal(board: &[Vec<Mark>], x: usize, y: usize, mark: Mark, win_count: usize) -> bool {
    let width = board[0].len();
    if x + win_count > width {
        return false;
    }

    for i in 0..win_count {
        if board[y][x + i] != mark {
            return false;
        }
    }
    true
}

fn check_vertical(board: &[Vec<Mark>], x: usize, y: usize, mark: Mark, win_count: usize) -> bool {
    let height = board.len();
    if y + win_count > height {
        return false;
    }

    for i in 0..win_count {
        if board[y + i][x] != mark {
            return false;
        }
    }
    true
}

fn check_diagonal_down_right(board: &[Vec<Mark>], x: usize, y: usize, mark: Mark, win_count: usize) -> bool {
    let height = board.len();
    let width = board[0].len();
    if x + win_count > width || y + win_count > height {
        return false;
    }

    for i in 0..win_count {
        if board[y + i][x + i] != mark {
            return false;
        }
    }
    true
}

fn check_diagonal_down_left(board: &[Vec<Mark>], x: usize, y: usize, mark: Mark, win_count: usize) -> bool {
    let height = board.len();
    if x + 1 < win_count || y + win_count > height {
        return false;
    }

    for i in 0..win_count {
        if board[y + i][x - i] != mark {
            return false;
        }
    }
    true
}
