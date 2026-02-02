use common::identifiers::ClientId;
use common::{GameOverNotification, GameStateUpdate};
use common::games::GameBroadcaster;
use crate::state::{AppState, GameEndInfo, PlayAgainStatus, SharedState};

#[derive(Clone)]
pub struct LocalBroadcaster {
    shared_state: SharedState,
    player_id: String,
}

impl LocalBroadcaster {
    pub fn new(shared_state: SharedState, player_id: String) -> Self {
        Self { shared_state, player_id }
    }
}

impl GameBroadcaster for LocalBroadcaster {
    async fn broadcast_state(&self, state: GameStateUpdate, _recipients: Vec<ClientId>) {
        self.shared_state.update_game_state(state);
    }

    async fn broadcast_game_over(&self, notification: GameOverNotification, _recipients: Vec<ClientId>) {
        let game_info = match notification.game_info {
            Some(common::game_over_notification::GameInfo::SnakeInfo(info)) => {
                GameEndInfo::Snake(info)
            }
            Some(common::game_over_notification::GameInfo::TictactoeInfo(info)) => {
                GameEndInfo::TicTacToe(info)
            }
            Some(common::game_over_notification::GameInfo::NumbersMatchInfo(info)) => {
                GameEndInfo::NumbersMatch(info)
            }
            Some(common::game_over_notification::GameInfo::StackAttackInfo(info)) => {
                GameEndInfo::StackAttack(info)
            }
            None => return,
        };

        let winner = notification.winner.map(|w| common::PlayerIdentity {
            player_id: w.player_id,
            is_bot: w.is_bot,
        });

        let last_game_state = match self.shared_state.get_state() {
            AppState::InGame { game_state, .. } => game_state,
            _ => None,
        };

        let player = common::PlayerIdentity {
            player_id: self.player_id.clone(),
            is_bot: false,
        };

        self.shared_state.set_state(AppState::GameOver {
            scores: notification.scores,
            winner,
            last_game_state,
            game_info,
            play_again_status: PlayAgainStatus::Available {
                ready_players: vec![],
                pending_players: vec![player],
            },
            is_observer: false,
        });
    }
}
