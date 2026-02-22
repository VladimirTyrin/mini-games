use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{ClientId, LobbyId, PlayerId, log, ServerMessage, server_message, InGameCommand, InReplayCommand, ReplayFileReadyNotification};
use crate::games::{GameResolver, GameSession, GameSessionConfig, ReplayMode};
use crate::replay::{generate_replay_filename, save_replay_to_bytes, REPLAY_VERSION};
use crate::broadcaster::Broadcaster;
use crate::lobby_manager::{LobbyManager, LobbySettings};
use crate::replay_session::{self, ReplaySessionHandle, ReplaySessionCommand};

pub type SessionId = String;

pub struct GameSessionManager {
    sessions: Arc<Mutex<HashMap<SessionId, GameSession>>>,
    client_to_session: Arc<Mutex<HashMap<ClientId, SessionId>>>,
    replay_sessions: Arc<Mutex<HashMap<SessionId, ReplaySessionHandle>>>,
    broadcaster: Broadcaster,
    lobby_manager: LobbyManager,
}

impl std::fmt::Debug for GameSessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSessionManager").finish()
    }
}

impl GameSessionManager {
    pub fn new(broadcaster: Broadcaster, lobby_manager: LobbyManager) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            client_to_session: Arc::new(Mutex::new(HashMap::new())),
            replay_sessions: Arc::new(Mutex::new(HashMap::new())),
            broadcaster,
            lobby_manager,
        }
    }

    pub async fn remove_session(&self, session_id: &SessionId) {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        mapping.retain(|_, sid| sid != session_id);

        log!("Game session removed: {}", session_id);
    }

    pub async fn create_session(
        &self,
        session_id: SessionId,
        _lobby_details: crate::LobbyDetails,
    ) {
        let lobby_id = LobbyId::new(session_id.clone());
        let lobby = match self.lobby_manager.get_lobby(&lobby_id).await {
            Some(l) => l,
            None => {
                log!("Cannot create session: lobby {} not found", session_id);
                return;
            }
        };

        let human_players: Vec<PlayerId> = lobby.players.keys().cloned().collect();

        let players_str: Vec<String> = human_players.iter().map(|p| p.to_string()).collect();
        let bots_str: Vec<String> = lobby.bots.keys().map(|b| format!("{} [BOT]", b)).collect();
        let all_participants: Vec<String> = players_str.into_iter().chain(bots_str).collect();
        log!(
            "Game starting in lobby '{}': [{}]",
            session_id,
            all_participants.join(", ")
        );

        let config = GameSessionConfig {
            session_id: session_id.clone(),
            human_players: human_players.clone(),
            observers: lobby.observers.clone(),
            bots: lobby.bots.clone(),
        };

        let seed: u64 = rand::random();

        let game_session = match &lobby.settings {
            crate::lobby::LobbySettings::Snake(settings) => {
                GameResolver::create_session(&config, settings, seed, ReplayMode::Save)
            }
            crate::lobby::LobbySettings::TicTacToe(settings) => {
                GameResolver::create_session(&config, settings, seed, ReplayMode::Save)
            }
            crate::lobby::LobbySettings::NumbersMatch(settings) => {
                GameResolver::create_session(&config, settings, seed, ReplayMode::Save)
            }
            crate::lobby::LobbySettings::StackAttack(settings) => {
                GameResolver::create_session(&config, settings, seed, ReplayMode::Save)
            }
            crate::lobby::LobbySettings::Puzzle2048(settings) => {
                GameResolver::create_session(&config, settings, seed, ReplayMode::Save)
            }
        };

        match game_session {
            Ok(session) => {
                self.register_session(session_id.clone(), session.clone(), &human_players)
                    .await;

                let manager = self.clone();
                let broadcaster = self.broadcaster.clone();

                tokio::spawn(async move {
                    let notification = GameResolver::run(config.clone(), session, broadcaster).await;
                    manager.handle_game_over(&config, notification).await;
                });
            }
            Err(e) => {
                log!("Failed to create game session: {}", e);
            }
        }
    }

    async fn register_session(
        &self,
        session_id: SessionId,
        session: GameSession,
        human_players: &[PlayerId],
    ) {
        let game_type = session.game_type();

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session);
        drop(sessions);

        let mut mapping = self.client_to_session.lock().await;
        for player_id in human_players {
            mapping.insert(ClientId::new(player_id.to_string()), session_id.clone());
        }

        log!(
            "{:?} game session registered: {} with {} players",
            game_type,
            session_id,
            human_players.len()
        );
    }

    async fn handle_game_over(
        &self,
        config: &GameSessionConfig,
        notification: crate::GameOverNotification,
    ) {
        let winner_str = notification
            .winner
            .as_ref()
            .map(|w| {
                if w.is_bot {
                    format!("{} [BOT]", w.player_id)
                } else {
                    w.player_id.clone()
                }
            })
            .unwrap_or_else(|| "Draw".to_string());

        let scores_str: Vec<String> = notification
            .scores
            .iter()
            .map(|s| {
                let name = s
                    .identity
                    .as_ref()
                    .map(|i| {
                        if i.is_bot {
                            format!("{} [BOT]", i.player_id)
                        } else {
                            i.player_id.clone()
                        }
                    })
                    .unwrap_or_else(|| "Unknown".to_string());
                format!("{}: {}", name, s.score)
            })
            .collect();

        log!(
            "Game over in lobby '{}': winner={}, scores=[{}]",
            config.session_id,
            winner_str,
            scores_str.join(", ")
        );

        let client_ids: Vec<ClientId> = config
            .human_players
            .iter()
            .map(|p| ClientId::new(p.to_string()))
            .chain(
                config
                    .observers
                    .iter()
                    .map(|p| ClientId::new(p.to_string())),
            )
            .collect();

        let game_over_msg = ServerMessage {
            message: Some(server_message::Message::GameOver(notification)),
        };

        self.broadcaster
            .broadcast_to_clients(&client_ids, game_over_msg)
            .await;

        if let Some(replay_notification) = self.finalize_replay(&config.session_id).await {
            let replay_msg = ServerMessage {
                message: Some(server_message::Message::ReplayFile(replay_notification)),
            };
            self.broadcaster
                .broadcast_to_clients(&client_ids, replay_msg)
                .await;
        }

        let lobby_id = LobbyId::new(config.session_id.clone());
        match self.lobby_manager.end_game(&lobby_id).await {
            Ok(_player_ids) => {
                log!(
                    "Game ended for lobby {}, {} players in lobby",
                    config.session_id,
                    _player_ids.len()
                );

                if let Some(_lobby_details) = self.lobby_manager.get_lobby_details(&lobby_id).await
                {
                    match self.lobby_manager.get_play_again_status(&lobby_id).await {
                        Ok(status) => {
                            let (ready_players, pending_players, available) = match status {
                                crate::lobby_manager::PlayAgainStatus::NotAvailable => {
                                    (vec![], vec![], false)
                                }
                                crate::lobby_manager::PlayAgainStatus::Available {
                                    ready_player_ids,
                                    pending_player_ids,
                                } => {
                                    let ready = ready_player_ids
                                        .iter()
                                        .map(|id| crate::PlayerIdentity {
                                            player_id: id.clone(),
                                            is_bot: false,
                                        })
                                        .collect();
                                    let pending = pending_player_ids
                                        .iter()
                                        .map(|id| crate::PlayerIdentity {
                                            player_id: id.clone(),
                                            is_bot: false,
                                        })
                                        .collect();
                                    (ready, pending, true)
                                }
                            };

                            let play_again_msg = ServerMessage {
                                message: Some(server_message::Message::PlayAgainStatus(
                                    crate::PlayAgainStatusNotification {
                                        ready_players,
                                        pending_players,
                                        available,
                                    },
                                )),
                            };

                            self.broadcaster
                                .broadcast_to_clients(&client_ids, play_again_msg)
                                .await;
                        }
                        Err(e) => {
                            log!("Failed to get play again status: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log!("Failed to end game for lobby {}: {}", config.session_id, e);
            }
        }

        self.remove_session(&config.session_id).await;
    }

    async fn finalize_replay(&self, session_id: &SessionId) -> Option<ReplayFileReadyNotification> {
        let sessions = self.sessions.lock().await;
        let session = sessions.get(session_id)?;

        let game_type = session.game_type();
        let replay_recorder = session.replay_recorder()?;
        drop(sessions);

        let replay = {
            let mut recorder = replay_recorder.lock().await;
            recorder.finalize()
        };

        let suggested_file_name = generate_replay_filename(game_type, crate::version::VERSION);
        let content = save_replay_to_bytes(&replay);

        log!(
            "Replay finalized: {} ({} bytes, {} actions)",
            suggested_file_name,
            content.len(),
            replay.actions.len()
        );

        Some(ReplayFileReadyNotification {
            version: REPLAY_VERSION as i32,
            suggested_file_name,
            content,
        })
    }

    pub async fn create_replay_session(
        &self,
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        replay_bytes: Vec<u8>,
        host_id: ClientId,
        host_only_control: bool,
    ) -> Result<(), String> {
        self.create_replay_session_for_group(
            lobby_manager,
            broadcaster,
            replay_bytes,
            host_id.clone(),
            vec![host_id],
            host_only_control,
        )
        .await
    }

    pub async fn create_replay_session_for_group(
        &self,
        lobby_manager: &LobbyManager,
        broadcaster: &Broadcaster,
        replay_bytes: Vec<u8>,
        host_id: ClientId,
        viewer_ids: Vec<ClientId>,
        host_only_control: bool,
    ) -> Result<(), String> {
        let replay = replay_session::parse_replay(replay_bytes)?;
        let game_type = replay_session::replay_game_type(&replay)?;
        let game_name = replay_session::replay_game_type_name(game_type);

        let player = crate::replay::ReplayPlayer::new(replay.clone());
        let lobby_settings = player
            .lobby_settings()
            .map(|s| LobbySettings::from_proto(Some(*s)))
            .transpose()?
            .unwrap_or(LobbySettings::Snake(crate::SnakeLobbySettings {
                field_width: 15,
                field_height: 15,
                wall_collision_mode: 0,
                tick_interval_ms: 200,
                max_food_count: 5,
                food_spawn_probability: 0.5,
                dead_snake_behavior: 0,
            }));

        let lobby_name = format!("Replay: {}", game_name);

        let (lobby_id, lobby_details) = lobby_manager
            .create_replay_lobby(lobby_name, lobby_settings, host_id.clone(), &viewer_ids)
            .await?;

        let session_id = lobby_id.to_string();

        let game_starting_msg = ServerMessage {
            message: Some(server_message::Message::GameStarting(
                crate::GameStartingNotification {
                    session_id: session_id.clone(),
                },
            )),
        };
        broadcaster
            .broadcast_to_clients(&viewer_ids, game_starting_msg)
            .await;

        let lobby_update_msg = ServerMessage {
            message: Some(server_message::Message::LobbyUpdate(
                crate::LobbyUpdateNotification {
                    details: Some(lobby_details),
                },
            )),
        };
        broadcaster
            .broadcast_to_clients(&viewer_ids, lobby_update_msg)
            .await;

        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();

        let handle = ReplaySessionHandle {
            command_tx,
            host_id: host_id.clone(),
            host_only_control,
        };

        {
            let mut replay_sessions = self.replay_sessions.lock().await;
            replay_sessions.insert(session_id.clone(), handle);
        }

        {
            let mut mapping = self.client_to_session.lock().await;
            for viewer_id in &viewer_ids {
                mapping.insert(viewer_id.clone(), session_id.clone());
            }
        }

        let broadcaster_clone = broadcaster.clone();
        let manager_clone = self.clone();
        let viewer_ids_clone = viewer_ids.clone();

        tokio::spawn(async move {
            replay_session::run_replay_session(
                replay,
                command_rx,
                viewer_ids_clone,
                host_only_control,
                broadcaster_clone,
            )
            .await;

            log!("Replay session {} ended", session_id);

            let mut replay_sessions = manager_clone.replay_sessions.lock().await;
            replay_sessions.remove(&session_id);
            drop(replay_sessions);

            let mut mapping = manager_clone.client_to_session.lock().await;
            mapping.retain(|_, sid| *sid != session_id);
        });

        Ok(())
    }

    pub async fn handle_replay_command(&self, client_id: &ClientId, command: InReplayCommand) {
        let mapping = self.client_to_session.lock().await;
        let session_id = match mapping.get(client_id) {
            Some(id) => id.clone(),
            None => return,
        };
        drop(mapping);

        let replay_sessions = self.replay_sessions.lock().await;
        if let Some(handle) = replay_sessions.get(&session_id) {
            if handle.host_only_control && *client_id != handle.host_id {
                return;
            }
            let _ = handle
                .command_tx
                .send(ReplaySessionCommand::ReplayCommand(command));
        }
    }

    pub async fn handle_command(&self, client_id: &ClientId, command: InGameCommand) {
        let mapping = self.client_to_session.lock().await;
        let session_id = match mapping.get(client_id) {
            Some(id) => id.clone(),
            None => return,
        };
        drop(mapping);

        let sessions = self.sessions.lock().await;
        let session = match sessions.get(&session_id) {
            Some(s) => s.clone(),
            None => return,
        };
        drop(sessions);

        GameResolver::handle_command(&session, client_id, command).await;
    }

    pub async fn handle_player_disconnect(&self, client_id: &ClientId) {
        let mapping = self.client_to_session.lock().await;
        let session_id = match mapping.get(client_id) {
            Some(id) => id.clone(),
            None => return,
        };
        drop(mapping);

        let sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get(&session_id) {
            let session = session.clone();
            drop(sessions);
            GameResolver::handle_player_disconnect(&session, client_id).await;
        }
    }
}

impl Clone for GameSessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            client_to_session: self.client_to_session.clone(),
            replay_sessions: self.replay_sessions.clone(),
            broadcaster: self.broadcaster.clone(),
            lobby_manager: self.lobby_manager.clone(),
        }
    }
}
