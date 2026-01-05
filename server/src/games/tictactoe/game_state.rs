use common::PlayerId;
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Empty,
    X,
    O,
}

impl Mark {
    pub fn to_proto(&self) -> i32 {
        match self {
            Mark::Empty => 1,
            Mark::X => 2,
            Mark::O => 3,
        }
    }

    pub fn opponent(&self) -> Option<Mark> {
        match self {
            Mark::X => Some(Mark::O),
            Mark::O => Some(Mark::X),
            Mark::Empty => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
    InProgress,
    XWon,
    OWon,
    Draw,
}

impl GameStatus {
    pub fn to_proto(&self) -> i32 {
        match self {
            GameStatus::InProgress => 1,
            GameStatus::XWon => 2,
            GameStatus::OWon => 3,
            GameStatus::Draw => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstPlayerMode {
    Random,
    Host,
}

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
    pub last_move: Option<(usize, usize)>,
}

impl TicTacToeGameState {
    pub fn new(
        width: usize,
        height: usize,
        win_count: usize,
        players: Vec<PlayerId>,
        first_player_mode: FirstPlayerMode,
    ) -> Self {
        if players.len() != 2 {
            panic!("TicTacToe requires exactly 2 players");
        }

        let (player_x, player_o) = match first_player_mode {
            FirstPlayerMode::Random => {
                if rand::rng().random() {
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
        self.last_move = Some((x, y));

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
        use super::win_detector::check_win;

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

    pub fn to_proto_state(
        &self,
        player_x_is_bot: bool,
        player_o_is_bot: bool,
        current_player_is_bot: bool,
    ) -> common::proto::tictactoe::TicTacToeGameState {
        let board: Vec<common::proto::tictactoe::CellMark> = self.board
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter().enumerate().filter_map(move |(x, &mark)| {
                    if mark == Mark::Empty {
                        None
                    } else {
                        Some(common::proto::tictactoe::CellMark {
                            x: x as u32,
                            y: y as u32,
                            mark: mark.to_proto(),
                        })
                    }
                })
            })
            .collect();

        common::proto::tictactoe::TicTacToeGameState {
            board,
            field_width: self.width as u32,
            field_height: self.height as u32,
            win_count: self.win_count as u32,
            status: self.status.to_proto(),
            player_x: Some(common::proto::tictactoe::PlayerIdentity {
                player_id: self.player_x.to_string(),
                is_bot: player_x_is_bot,
            }),
            player_o: Some(common::proto::tictactoe::PlayerIdentity {
                player_id: self.player_o.to_string(),
                is_bot: player_o_is_bot,
            }),
            current_player: Some(common::proto::tictactoe::PlayerIdentity {
                player_id: self.current_player.to_string(),
                is_bot: current_player_is_bot,
            }),
            last_move: self.last_move.map(|(x, y)| common::proto::tictactoe::Position {
                x: x as u32,
                y: y as u32,
            }),
        }
    }
}
