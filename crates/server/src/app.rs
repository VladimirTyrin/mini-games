use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::extract::{State, WebSocketUpgrade};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, mpsc};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::auth::{BotKeys, GoogleVerifier, HttpGoogleKeySource, SessionTokens};
use crate::config::{Config, ConfigError};
use crate::connection;
use crate::context::ServerContext;
use crate::hub::{Hub, HubCommand, HubHandle};
use crate::limits::Limits;

#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("invalid configuration: {0}")]
    Config(#[source] ConfigError),
    #[error("server I/O failed: {0}")]
    Io(#[source] std::io::Error),
}

const HOUSEKEEPING_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Clone)]
struct AppState {
    context: Arc<ServerContext>,
    connections: Arc<AtomicUsize>,
}

pub(crate) fn build_context(config: &Config, hub: HubHandle) -> Result<ServerContext, ConfigError> {
    let secret = match config.session_secret_bytes()? {
        Some(secret) => secret,
        None => {
            tracing::warn!("MG_SESSION_SECRET not set: sessions will not survive a restart");
            (0..32).map(|_| rand::random::<u8>()).collect()
        }
    };
    let google = (!config.google_client_id.trim().is_empty()).then(|| {
        GoogleVerifier::new(
            config.google_client_id.trim().to_owned(),
            Box::new(HttpGoogleKeySource::new(
                HttpGoogleKeySource::GOOGLE_CERTS_URL,
            )),
        )
    });
    Ok(ServerContext {
        allow_guests: config.allow_guests,
        google,
        session_tokens: SessionTokens::new(secret, config.session_days.saturating_mul(24 * 3600)),
        bot_keys: BotKeys::parse(&config.bot_keys)?,
        hub,
    })
}

/// Runs the server until `shutdown` resolves; then tells clients and drains connections.
pub async fn serve(
    config: Config,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServeError> {
    let (hub_sender, hub_receiver) = mpsc::unbounded_channel();
    let hub_handle = HubHandle::new(hub_sender);
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let bot_cpu = Arc::new(Semaphore::new(cores));
    let context = Arc::new(build_context(&config, hub_handle.clone()).map_err(ServeError::Config)?);
    let hub_task = tokio::spawn(Hub::new(hub_handle.clone(), bot_cpu).run(hub_receiver));

    let housekeeping_hub = hub_handle.clone();
    let housekeeping = tokio::spawn(async move {
        let mut every = tokio::time::interval(HOUSEKEEPING_EVERY);
        loop {
            every.tick().await;
            housekeeping_hub.send(HubCommand::Housekeeping);
        }
    });

    let state = AppState {
        context,
        connections: Arc::new(AtomicUsize::new(0)),
    };
    let mut router = Router::new()
        .route("/ws", get(websocket))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state);
    if let Some(dir) = &config.static_dir {
        let assets = Router::new()
            .nest_service("/assets", ServeDir::new(dir.join("assets")))
            .layer(SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ));
        router = router.merge(assets).fallback_service(
            ServeDir::new(dir)
                .precompressed_gzip()
                .fallback(ServeFile::new(dir.join("index.html"))),
        );
    }

    tracing::info!(listen = %config.listen, "serving");
    let shutdown_hub = hub_handle.clone();
    let result = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            shutdown.await;
            tracing::info!("shutting down");
            shutdown_hub.send(HubCommand::Shutdown);
        })
        .await
        .map_err(ServeError::Io);
    housekeeping.abort();
    if tokio::time::timeout(Limits::SHUTDOWN_GRACE, hub_task)
        .await
        .is_err()
    {
        tracing::warn!("hub did not stop in time");
    }
    result
}

async fn websocket(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
    if state.connections.load(Ordering::Relaxed) >= Limits::MAX_CONNECTIONS {
        return (StatusCode::SERVICE_UNAVAILABLE, "server is full").into_response();
    }
    upgrade
        .max_message_size(Limits::MAX_REPLAY_MESSAGE_BYTES)
        .max_frame_size(Limits::MAX_REPLAY_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            state.connections.fetch_add(1, Ordering::Relaxed);
            connection::run(state.context, socket).await;
            state.connections.fetch_sub(1, Ordering::Relaxed);
        })
}
