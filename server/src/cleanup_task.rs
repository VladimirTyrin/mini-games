use std::time::Duration;

use crate::{log, server_message, ClientId, KickReason, KickedFromLobbyNotification, ServerMessage};

use crate::broadcaster::Broadcaster;
use crate::lobby_manager::LobbyManager;

pub struct CleanupTask {
    lobby_manager: LobbyManager,
    broadcaster: Broadcaster,
    check_interval: Duration,
    inactivity_timeout: Duration,
}

impl CleanupTask {
    pub fn new(
        lobby_manager: LobbyManager,
        broadcaster: Broadcaster,
        check_interval: Duration,
        inactivity_timeout: Duration,
    ) -> Self {
        Self {
            lobby_manager,
            broadcaster,
            check_interval,
            inactivity_timeout,
        }
    }

    pub async fn run(&self) {
        let mut interval = tokio::time::interval(self.check_interval);

        loop {
            interval.tick().await;
            self.cleanup_inactive().await;
        }
    }

    async fn cleanup_inactive(&self) {
        self.cleanup_inactive_lobbies().await;
        self.cleanup_inactive_clients().await;
    }

    async fn cleanup_inactive_lobbies(&self) {
        let inactive_lobbies = self
            .lobby_manager
            .get_inactive_lobbies(self.inactivity_timeout)
            .await;

        for lobby_id in inactive_lobbies {
            log!("Cleaning up inactive lobby: {}", lobby_id);

            let players = self.lobby_manager.get_lobby_players(&lobby_id).await;

            let kick_message = ServerMessage {
                message: Some(server_message::Message::Kicked(KickedFromLobbyNotification {
                    reason: "Lobby inactive for too long".to_string(),
                    kick_reason: KickReason::LobbyInactivity.into(),
                })),
            };

            self.broadcaster
                .broadcast_to_clients(&players, kick_message)
                .await;

            for client_id in &players {
                self.disconnect_client(client_id).await;
            }
        }
    }

    async fn cleanup_inactive_clients(&self) {
        let inactive_clients = self
            .lobby_manager
            .get_inactive_clients(self.inactivity_timeout)
            .await;

        for client_id in inactive_clients {
            log!("Cleaning up inactive client: {}", client_id);

            let kick_message = ServerMessage {
                message: Some(server_message::Message::Kicked(KickedFromLobbyNotification {
                    reason: "Inactive for too long".to_string(),
                    kick_reason: KickReason::PlayerInactivity.into(),
                })),
            };

            self.broadcaster.send_to_client(&client_id, kick_message).await;

            self.disconnect_client(&client_id).await;
        }
    }

    async fn disconnect_client(&self, client_id: &ClientId) {
        let _ = self.lobby_manager.leave_lobby(client_id).await;
        self.lobby_manager.remove_client(client_id).await;
        self.broadcaster.unregister(client_id).await;
    }
}
