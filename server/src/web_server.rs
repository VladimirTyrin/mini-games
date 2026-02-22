use std::path::PathBuf;
use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use crate::log;

use crate::broadcaster::Broadcaster;
use crate::game_session_manager::GameSessionManager;
use crate::lobby_manager::LobbyManager;
use crate::ws_handler::handle_websocket;

#[derive(Clone)]
pub struct WebServerState {
    pub lobby_manager: LobbyManager,
    pub broadcaster: Broadcaster,
    pub session_manager: GameSessionManager,
}

pub async fn run_web_server(
    lobby_manager: LobbyManager,
    broadcaster: Broadcaster,
    session_manager: GameSessionManager,
    static_files_path: PathBuf,
) {
    let state = WebServerState {
        lobby_manager,
        broadcaster,
        session_manager,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(ws_upgrade_handler))
        .nest_service("/ui", ServeDir::new(&static_files_path))
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:5000";
    log!("Web server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind web server address");

    axum::serve(listener, app)
        .await
        .expect("Web server error");
}

async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    State(state): State<WebServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}
