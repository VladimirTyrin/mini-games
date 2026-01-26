use std::collections::HashSet;
use super::types::Mark;

pub fn get_available_moves(board: &[Vec<Mark>]) -> Vec<(usize, usize)> {
    let height = board.len();
    if height == 0 {
        return Vec::new();
    }
    let width = board[0].len();

    let mut has_any_mark = false;
    let mut near_moves = HashSet::new();

    for (y, row) in board.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            if cell == Mark::Empty {
                continue;
            }

            has_any_mark = true;

            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0
                        && ny >= 0
                        && (nx as usize) < width
                        && (ny as usize) < height
                        && board[ny as usize][nx as usize] == Mark::Empty
                    {
                        near_moves.insert((nx as usize, ny as usize));
                    }
                }
            }
        }
    }

    if !has_any_mark {
        return vec![(width / 2, height / 2)];
    }

    near_moves.into_iter().collect()
}
