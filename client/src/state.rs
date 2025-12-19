use common::{LobbyInfo, LobbyDetails};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum ClientCommand {
    ListLobbies,
    CreateLobby { name: String, max_players: u32 },
    JoinLobby { lobby_id: String },
    LeaveLobby,
    MarkReady { ready: bool },
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
}

pub struct SharedState {
    state: Arc<Mutex<AppState>>,
    error: Arc<Mutex<Option<String>>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::LobbyList { lobbies: vec![] })),
            error: Arc::new(Mutex::new(None)),
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

    pub fn set_error(&self, error: String) {
        *self.error.lock().unwrap() = Some(error);
    }

    pub fn get_error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }

    pub fn clear_error(&self) {
        *self.error.lock().unwrap() = None;
    }
}

impl Clone for SharedState {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            error: Arc::clone(&self.error),
        }
    }
}
