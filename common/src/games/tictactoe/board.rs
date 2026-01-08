use super::types::Mark;

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
