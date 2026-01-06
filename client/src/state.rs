use common::{LobbyInfo, LobbyDetails, GameStateUpdate, ScoreEntry, proto::snake::{Direction, SnakeGameEndInfo, SnakeBotType}, proto::tictactoe::{TicTacToeGameEndInfo, TicTacToeBotType}, PlayerIdentity};
use crate::config::{SnakeLobbyConfig, TicTacToeLobbyConfig};
use crate::constants::CHAT_BUFFER_SIZE;
use std::sync::{Arc, Mutex};
use ringbuffer::{AllocRingBuffer, RingBuffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    Online,
    TemporaryOffline,
}

#[derive(Debug, Clone, Copy)]
pub enum BotType {
    Snake(SnakeBotType),
    TicTacToe(TicTacToeBotType),
}

#[derive(Debug, Clone)]
pub enum LobbyConfig {
    Snake(SnakeLobbyConfig),
    TicTacToe(TicTacToeLobbyConfig),
}

#[derive(Debug, Clone)]
pub enum MenuCommand {
    ListLobbies,
    CreateLobby { name: String, config: LobbyConfig },
    JoinLobby { lobby_id: String, join_as_observer: bool },
    LeaveLobby,
    MarkReady { ready: bool },
    StartGame,
    PlayAgain,
    AddBot { bot_type: BotType },
    KickFromLobby { player_id: String },
    BecomeObserver,
    BecomePlayer,
    MakePlayerObserver { player_id: String },
    Disconnect,
    InLobbyChatMessage { message: String },
    LobbyListChatMessage { message: String },
}

#[derive(Debug, Clone)]
pub enum SnakeGameCommand {
    SendTurn { direction: Direction },
}

#[derive(Debug, Clone)]
pub enum TicTacToeGameCommand {
    PlaceMark { x: u32, y: u32 },
}

#[derive(Debug, Clone)]
pub enum GameCommand {
    Snake(SnakeGameCommand),
    TicTacToe(TicTacToeGameCommand),
}

#[derive(Debug, Clone)]
pub enum ClientCommand {
    Menu(MenuCommand),
    Game(GameCommand),
}

#[derive(Debug, Clone)]
pub enum GameEndInfo {
    Snake(SnakeGameEndInfo),
    TicTacToe(TicTacToeGameEndInfo),
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
        is_observer: bool,
    },
    GameOver {
        scores: Vec<ScoreEntry>,
        winner: Option<PlayerIdentity>,
        last_game_state: Option<GameStateUpdate>,
        game_info: GameEndInfo,
        play_again_status: PlayAgainStatus,
        is_observer: bool,
    },
}

pub struct InnerState {
    pub state: AppState,
    pub error: Option<String>,
    pub should_close: bool,
    pub connection_mode: ConnectionMode,
    pub retry_server_address: Option<String>,
    pub ping_ms: Option<u64>,
    pub ctx: Option<eframe::egui::Context>,
}

pub struct SharedState {
    inner: Arc<Mutex<InnerState>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerState {
                state: AppState::LobbyList {
                    lobbies: vec![],
                    chat_messages: AllocRingBuffer::new(CHAT_BUFFER_SIZE),
                },
                error: None,
                should_close: false,
                connection_mode: ConnectionMode::Online,
                retry_server_address: None,
                ping_ms: None,
                ctx: None,
            })),
        }
    }

    pub fn set_context(&self, ctx: eframe::egui::Context) {
        self.inner.lock().expect("SharedState lock poisoned").ctx = Some(ctx);
    }

    pub fn has_context(&self) -> bool {
        self.inner.lock().expect("SharedState lock poisoned").ctx.is_some()
    }

    fn request_repaint(&self) {
        if let Some(ctx) = self.inner.lock().expect("SharedState lock poisoned").ctx.as_ref() {
            ctx.request_repaint();
        }
    }

    pub fn set_state(&self, state: AppState) {
        self.inner.lock().expect("SharedState lock poisoned").state = state;
        self.request_repaint();
    }

    pub fn get_state(&self) -> AppState {
        self.inner.lock().expect("SharedState lock poisoned").state.clone()
    }

    pub fn get_state_mut(&self) -> std::sync::MutexGuard<'_, InnerState> {
        self.inner.lock().expect("SharedState lock poisoned")
    }

    pub fn add_event(&self, event: String) {
        let mut inner = self.inner.lock().expect("SharedState lock poisoned");
        if let AppState::InLobby { event_log, .. } = &mut inner.state {
            event_log.enqueue(event);
            drop(inner);
            self.request_repaint();
        }
    }

    pub fn add_event_log(&self, event: String) {
        self.add_event(event);
    }

    pub fn update_game_state(&self, game_state: GameStateUpdate) {
        let mut inner = self.inner.lock().expect("SharedState lock poisoned");
        if let AppState::InGame { game_state: current_state, .. } = &mut inner.state {
            *current_state = Some(game_state);
            drop(inner);
            self.request_repaint();
        }
    }

    pub fn set_error(&self, error: String) {
        self.inner.lock().expect("SharedState lock poisoned").error = Some(error);
        self.request_repaint();
    }

    pub fn get_error(&self) -> Option<String> {
        self.inner.lock().expect("SharedState lock poisoned").error.clone()
    }

    pub fn clear_error(&self) {
        self.inner.lock().expect("SharedState lock poisoned").error = None;
    }

    pub fn set_should_close(&self) {
        self.inner.lock().expect("SharedState lock poisoned").should_close = true;
    }

    pub fn should_close(&self) -> bool {
        self.inner.lock().expect("SharedState lock poisoned").should_close
    }

    pub fn set_connection_failed(&self, failed: bool) {
        let mode = if failed { ConnectionMode::TemporaryOffline } else { ConnectionMode::Online };
        self.inner.lock().expect("SharedState lock poisoned").connection_mode = mode;
    }

    pub fn get_connection_failed(&self) -> bool {
        self.inner.lock().expect("SharedState lock poisoned").connection_mode == ConnectionMode::TemporaryOffline
    }

    pub fn set_connection_mode(&self, mode: ConnectionMode) {
        self.inner.lock().expect("SharedState lock poisoned").connection_mode = mode;
    }

    pub fn get_connection_mode(&self) -> ConnectionMode {
        self.inner.lock().expect("SharedState lock poisoned").connection_mode
    }

    pub fn set_retry_server_address(&self, address: Option<String>) {
        self.inner.lock().expect("SharedState lock poisoned").retry_server_address = address;
    }

    pub fn take_retry_server_address(&self) -> Option<String> {
        self.inner.lock().expect("SharedState lock poisoned").retry_server_address.take()
    }

    pub fn update_play_again_status(&self, play_again_status: PlayAgainStatus) {
        let mut inner = self.inner.lock().expect("SharedState lock poisoned");
        if let AppState::GameOver { play_again_status: current_status, .. } = &mut inner.state {
            *current_status = play_again_status;
            drop(inner);
            self.request_repaint();
        }
    }

    pub fn set_ping(&self, ping_ms: u64) {
        self.inner.lock().expect("SharedState lock poisoned").ping_ms = Some(ping_ms);
        self.request_repaint();
    }

    pub fn get_ping(&self) -> Option<u64> {
        self.inner.lock().expect("SharedState lock poisoned").ping_ms
    }
}

impl Clone for SharedState {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
