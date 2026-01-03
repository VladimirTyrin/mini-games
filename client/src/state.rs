use common::{LobbyInfo, LobbyDetails, GameStateUpdate, ScoreEntry, Direction, BotType, PlayerIdentity};
use crate::config::LobbyConfig;
use std::sync::{Arc, Mutex};
use ringbuffer::{AllocRingBuffer, RingBuffer};

#[derive(Debug, Clone)]
pub enum MenuCommand {
    ListLobbies,
    CreateLobby { name: String, config: LobbyConfig },
    JoinLobby { lobby_id: String },
    LeaveLobby,
    MarkReady { ready: bool },
    StartGame,
    PlayAgain,
    AddBot { bot_type: BotType },
    KickFromLobby { player_id: String },
    Disconnect,
    InLobbyChatMessage { message: String },
    LobbyListChatMessage { message: String },
}

#[derive(Debug, Clone)]
pub enum GameCommand {
    SendTurn { direction: Direction },
}

#[derive(Debug, Clone)]
pub enum ClientCommand {
    Menu(MenuCommand),
    Game(GameCommand),
}

#[derive(Debug, Clone)]
pub enum PlayAgainStatus {
    NotAvailable,
    Available {
        ready_players: Vec<PlayerIdentity>,
        pending_players: Vec<PlayerIdentity>,
    },
}

#[derive(Debug, Clone)]
pub enum AppState {
    LobbyList {
        lobbies: Vec<LobbyInfo>,
        chat_messages: AllocRingBuffer<String>
    },
    InLobby {
        details: LobbyDetails,
        event_log: AllocRingBuffer<String>,
    },
    InGame {
        session_id: String,
        game_state: Option<GameStateUpdate>,
    },
    GameOver {
        scores: Vec<ScoreEntry>,
        winner: Option<PlayerIdentity>,
        last_game_state: Option<GameStateUpdate>,
        reason: common::GameEndReason,
        play_again_status: PlayAgainStatus,
    },
}

pub struct SharedState {
    state: Arc<Mutex<AppState>>,
    error: Arc<Mutex<Option<String>>>,
    should_close: Arc<Mutex<bool>>,
    connection_failed: Arc<Mutex<bool>>,
    retry_server_address: Arc<Mutex<Option<String>>>,
    ping_ms: Arc<Mutex<Option<u64>>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::LobbyList { lobbies: vec![], chat_messages: AllocRingBuffer::new(20) })),
            error: Arc::new(Mutex::new(None)),
            should_close: Arc::new(Mutex::new(false)),
            connection_failed: Arc::new(Mutex::new(false)),
            retry_server_address: Arc::new(Mutex::new(None)),
            ping_ms: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_state(&self, state: AppState) {
        *self.state.lock().unwrap() = state;
    }

    pub fn get_state(&self) -> AppState {
        self.state.lock().unwrap().clone()
    }

    pub fn get_state_mut(&self) -> std::sync::MutexGuard<'_, AppState> {
        self.state.lock().unwrap()
    }

    pub fn add_event(&self, event: String) {
        let mut state = self.state.lock().unwrap();
        if let AppState::InLobby { event_log, .. } = &mut *state {
            event_log.enqueue(event);
        }
    }

    pub fn add_event_log(&self, event: String) {
        self.add_event(event);
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

    pub fn set_connection_failed(&self, failed: bool) {
        *self.connection_failed.lock().unwrap() = failed;
    }

    pub fn get_connection_failed(&self) -> bool {
        *self.connection_failed.lock().unwrap()
    }

    pub fn set_retry_server_address(&self, address: Option<String>) {
        *self.retry_server_address.lock().unwrap() = address;
    }

    pub fn take_retry_server_address(&self) -> Option<String> {
        self.retry_server_address.lock().unwrap().take()
    }

    pub fn update_play_again_status(&self, play_again_status: PlayAgainStatus) {
        let mut state = self.state.lock().unwrap();
        if let AppState::GameOver { play_again_status: current_status, .. } = &mut *state {
            *current_status = play_again_status;
        }
    }

    pub fn set_ping(&self, ping_ms: u64) {
        *self.ping_ms.lock().unwrap() = Some(ping_ms);
    }

    pub fn get_ping(&self) -> Option<u64> {
        *self.ping_ms.lock().unwrap()
    }
}

impl Clone for SharedState {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            error: Arc::clone(&self.error),
            should_close: Arc::clone(&self.should_close),
            connection_failed: Arc::clone(&self.connection_failed),
            retry_server_address: Arc::clone(&self.retry_server_address),
            ping_ms: Arc::clone(&self.ping_ms),
        }
    }
}
