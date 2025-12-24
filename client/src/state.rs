use common::{LobbyInfo, LobbyDetails, GameStateUpdate, ScoreEntry, Direction};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum MenuCommand {
    ListLobbies,
    CreateLobby { name: String, max_players: u32 },
    JoinLobby { lobby_id: String },
    LeaveLobby,
    MarkReady { ready: bool },
    StartGame,
    Disconnect,
}

#[derive(Debug, Clone)]
pub enum GameCommand {
    SendTurn { direction: Direction },
    }

#[derive(Debug, Clone)]
pub enum AppState {
    LobbyList {
        lobbies: Vec<LobbyInfo>,
    },
    InLobby {
        details: LobbyDetails,
        event_log: Vec<String>,
    },
    InGame {
        session_id: String,
        game_state: Option<GameStateUpdate>,
    },
    GameOver {
        scores: Vec<ScoreEntry>,
        winner_id: String,
        last_game_state: Option<GameStateUpdate>,
    },
}

pub struct SharedState {
    state: Arc<Mutex<AppState>>,
    error: Arc<Mutex<Option<String>>>,
    should_close: Arc<Mutex<bool>>,
    game_command_tx: Arc<Mutex<Option<mpsc::UnboundedSender<GameCommand>>>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::LobbyList { lobbies: vec![] })),
            error: Arc::new(Mutex::new(None)),
            should_close: Arc::new(Mutex::new(false)),
            game_command_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_state(&self, state: AppState) {
        *self.state.lock().unwrap() = state;
    }

    pub fn get_state(&self) -> AppState {
        self.state.lock().unwrap().clone()
    }

    pub fn add_event(&self, event: String) {
        let mut state = self.state.lock().unwrap();
        if let AppState::InLobby { event_log, .. } = &mut *state {
            event_log.push(event);
        }
    }

    pub fn update_game_state(&self, game_state: GameStateUpdate) {
        let mut state = self.state.lock().unwrap();
        if let AppState::InGame { game_state: current_state, .. } = &mut *state {
            *current_state = Some(game_state);
        }
    }

    pub fn set_error(&self, error: String) {
        *self.error.lock().unwrap() = Some(error);
    }

    pub fn get_error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }

    pub fn clear_error(&self) {
        *self.error.lock().unwrap() = None;
    }

    pub fn set_should_close(&self) {
        *self.should_close.lock().unwrap() = true;
    }

    pub fn should_close(&self) -> bool {
        *self.should_close.lock().unwrap()
    }

    pub fn set_game_command_tx(&self, tx: mpsc::UnboundedSender<GameCommand>) {
        *self.game_command_tx.lock().unwrap() = Some(tx);
    }

    pub fn get_game_command_tx(&self) -> Option<mpsc::UnboundedSender<GameCommand>> {
        self.game_command_tx.lock().unwrap().clone()
    }

    pub fn clear_game_command_tx(&self) {
        *self.game_command_tx.lock().unwrap() = None;
    }
}

impl Clone for SharedState {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            error: Arc::clone(&self.error),
            should_close: Arc::clone(&self.should_close),
            game_command_tx: Arc::clone(&self.game_command_tx),
        }
    }
}
