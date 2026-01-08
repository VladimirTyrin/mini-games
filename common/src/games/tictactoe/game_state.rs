use crate::PlayerId;
use crate::games::SessionRng;
use super::types::{FirstPlayerMode, GameStatus, Mark, Position};
use super::win_detector::check_win;

#[derive(Debug)]
pub struct TicTacToeGameState {
    pub board: Vec<Vec<Mark>>,
    pub width: usize,
    pub height: usize,
    pub win_count: usize,
    pub player_x: PlayerId,
    pub player_o: PlayerId,
    pub current_player: PlayerId,
    pub current_mark: Mark,
    pub status: GameStatus,
    pub last_move: Option<Position>,
}

impl TicTacToeGameState {
    pub fn new(
        width: usize,
        height: usize,
        win_count: usize,
        players: Vec<PlayerId>,
        first_player_mode: FirstPlayerMode,
        rng: &mut SessionRng,
    ) -> Self {
        if players.len() != 2 {
            panic!("TicTacToe requires exactly 2 players");
        }

        let (player_x, player_o) = match first_player_mode {
            FirstPlayerMode::Random => {
                if rng.random_bool() {
                    (players[0].clone(), players[1].clone())
                } else {
                    (players[1].clone(), players[0].clone())
                }
            }
            FirstPlayerMode::Host => (players[0].clone(), players[1].clone()),
        };

        let current_player = player_x.clone();
        let board = vec![vec![Mark::Empty; width]; height];

        Self {
            board,
            width,
            height,
            win_count,
            player_x,
            player_o,
            current_player,
            current_mark: Mark::X,
            status: GameStatus::InProgress,
            last_move: None,
        }
    }

    pub fn place_mark(&mut self, player_id: &PlayerId, x: usize, y: usize) -> Result<(), String> {
        if self.status != GameStatus::InProgress {
            return Err("Game is already over".to_string());
        }

        if player_id != &self.current_player {
            return Err("Not your turn".to_string());
        }

        if x >= self.width || y >= self.height {
            return Err("Position out of bounds".to_string());
        }

        if self.board[y][x] != Mark::Empty {
            return Err("Cell is already marked".to_string());
        }

        self.board[y][x] = self.current_mark;
        self.last_move = Some(Position::new(x, y));

        self.check_game_over();

        if self.status == GameStatus::InProgress {
            self.switch_turn();
        }

        Ok(())
    }

    fn switch_turn(&mut self) {
        if self.current_mark == Mark::X {
            self.current_mark = Mark::O;
            self.current_player = self.player_o.clone();
        } else {
            self.current_mark = Mark::X;
            self.current_player = self.player_x.clone();
        }
    }

    fn check_game_over(&mut self) {
        if let Some(winner_mark) = check_win(&self.board, self.win_count) {
            self.status = match winner_mark {
                Mark::X => GameStatus::XWon,
                Mark::O => GameStatus::OWon,
                Mark::Empty => unreachable!(),
            };
            return;
        }

        if self.is_board_full() {
            self.status = GameStatus::Draw;
        }
    }

    fn is_board_full(&self) -> bool {
        self.board
            .iter()
            .all(|row| row.iter().all(|&cell| cell != Mark::Empty))
    }

    pub fn get_winner(&self) -> Option<PlayerId> {
        match self.status {
            GameStatus::XWon => Some(self.player_x.clone()),
            GameStatus::OWon => Some(self.player_o.clone()),
            _ => None,
        }
    }

    pub fn forfeit(&mut self, player_id: &PlayerId) -> Result<(), String> {
        if self.status != GameStatus::InProgress {
            return Err("Game is already over".to_string());
        }
        if player_id == &self.player_x {
            self.status = GameStatus::OWon;
            Ok(())
        } else if player_id == &self.player_o {
            self.status = GameStatus::XWon;
            Ok(())
        } else {
            Err(format!("Player {} is not in this game", player_id))
        }
    }

    pub fn to_proto_state(
        &self,
        player_x_is_bot: bool,
        player_o_is_bot: bool,
        current_player_is_bot: bool,
    ) -> crate::proto::tictactoe::TicTacToeGameState {
        let board: Vec<crate::proto::tictactoe::CellMark> = self
            .board
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter().enumerate().filter_map(move |(x, &mark)| {
                    if mark == Mark::Empty {
                        None
                    } else {
                        Some(crate::proto::tictactoe::CellMark {
                            x: x as u32,
                            y: y as u32,
                            mark: mark.to_proto(),
                        })
                    }
                })
            })
            .collect();

        crate::proto::tictactoe::TicTacToeGameState {
            board,
            field_width: self.width as u32,
            field_height: self.height as u32,
            win_count: self.win_count as u32,
            status: self.status.to_proto(),
            player_x: Some(crate::proto::tictactoe::PlayerIdentity {
                player_id: self.player_x.to_string(),
                is_bot: player_x_is_bot,
            }),
            player_o: Some(crate::proto::tictactoe::PlayerIdentity {
                player_id: self.player_o.to_string(),
                is_bot: player_o_is_bot,
            }),
            current_player: Some(crate::proto::tictactoe::PlayerIdentity {
                player_id: self.current_player.to_string(),
                is_bot: current_player_is_bot,
            }),
            last_move: self.last_move.map(|pos| pos.to_proto()),
        }
    }
}
