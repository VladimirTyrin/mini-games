use super::game_state::Mark;

pub fn get_available_moves(board: &[Vec<Mark>]) -> Vec<(usize, usize)> {
    let mut moves = Vec::new();
    for (y, row) in board.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            if cell == Mark::Empty {
                moves.push((x, y));
            }
        }
    }
    moves
}

pub fn is_valid_move(board: &[Vec<Mark>], x: usize, y: usize) -> bool {
    if y >= board.len() {
        return false;
    }
    if x >= board[0].len() {
        return false;
    }
    board[y][x] == Mark::Empty
}
