use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, mpsc, watch};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::auth::{BotKeys, GoogleVerifier, HttpGoogleKeySource, PlayerIdKey, SessionTokens};
use crate::config::{Config, ConfigError};
use crate::connection;
use crate::connection_limiter::{ConnectionLimiter, ConnectionRefused};
use crate::context::{ServerContext, server_version};
use crate::hub::{Hub, HubCommand, HubHandle};
use crate::limits::Limits;
use crate::push::{PushQueue, PushSender};
use crate::social::{Social, SocialCommand, SocialHandle, Store, StoreError};

#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("invalid configuration: {0}")]
    Config(#[source] ConfigError),
    #[error("server I/O failed: {0}")]
    Io(#[source] std::io::Error),
    #[error("database: {0}")]
    Database(#[source] StoreError),
}

#[derive(Clone)]
struct AppState {
    context: Arc<ServerContext>,
    connections: Arc<ConnectionLimiter>,
}

fn build_context(
    config: &Config,
    hub: HubHandle,
    shutting_down: watch::Receiver<bool>,
) -> Result<ServerContext, ConfigError> {
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
        google_verifications: Semaphore::new(Limits::GOOGLE_VERIFICATIONS),
        // Same secret: account ids stay stable exactly as long as session tokens stay valid.
        player_ids: PlayerIdKey::new(secret.clone()),
        session_tokens: SessionTokens::new(secret, config.session_days.saturating_mul(24 * 3600)),
        bot_keys: BotKeys::parse(&config.bot_keys)?,
        hub,
        push: Arc::new(
            PushSender::configure(
                &config.fcm_service_account,
                &config.vapid_private_key,
                &config.vapid_subject,
            )
            .map_err(ConfigError::Push)?,
        ),
        push_queue: PushQueue::start(),
        timing: config.timing,
        shutting_down,
    })
}

fn open_store(config: &Config) -> Result<Store, StoreError> {
    match &config.database {
        Some(path) => Store::open(path),
        None => {
            tracing::warn!("MG_DATABASE not set: friends are kept in memory until a restart");
            Store::in_memory()
        }
    }
}

/// Runs the server until `shutdown` resolves; then tells clients and waits (bounded) until
/// their connections have flushed that and closed.
pub async fn serve(
    config: Config,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServeError> {
    let (hub_sender, hub_receiver) = mpsc::unbounded_channel();
    let hub_handle = HubHandle::new(hub_sender);
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let bot_cpu = Arc::new(Semaphore::new(cores));
    let (stopping, shutting_down) = watch::channel(false);
    let context = Arc::new(
        build_context(&config, hub_handle.clone(), shutting_down).map_err(ServeError::Config)?,
    );
    let store = Arc::new(open_store(&config).map_err(ServeError::Database)?);
    let (social_sender, social_receiver) = mpsc::unbounded_channel();
    let social_handle = SocialHandle::new(social_sender);
    // Not awaited on shutdown: it holds nothing that needs flushing (every write commits).
    tokio::spawn(
        Social::new(
            store,
            context.push.clone(),
            context.push_queue.clone(),
            hub_handle.clone(),
            social_handle.clone(),
            config.timing,
        )
        .run(social_receiver),
    );
    let hub_task = tokio::spawn(
        Hub::new(
            hub_handle.clone(),
            social_handle.clone(),
            bot_cpu,
            config.timing,
        )
        .run(hub_receiver),
    );

    let housekeeping_hub = hub_handle.clone();
    let housekeeping_every = config.timing.housekeeping_every;
    let housekeeping = tokio::spawn(async move {
        let mut every = tokio::time::interval(housekeeping_every);
        loop {
            every.tick().await;
            housekeeping_hub.send(HubCommand::Housekeeping);
            social_handle.send(SocialCommand::Housekeeping);
        }
    });

    let connections = Arc::new(ConnectionLimiter::new(
        Limits::MAX_CONNECTIONS,
        config.max_connections_per_ip,
    ));
    let state = AppState {
        context,
        connections: connections.clone(),
    };
    let mut router = Router::new()
        .route("/ws", get(websocket))
        .route("/healthz", get(|| async { "ok" }))
        // Lets deploy.sh check which build is running without a WebSocket.
        .route("/version", get(|| async { server_version() }))
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
    let result = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown.await;
        tracing::info!("shutting down");
        stopping.send_replace(true);
        shutdown_hub.send(HubCommand::Shutdown);
    })
    .await
    .map_err(ServeError::Io);
    housekeeping.abort();
    // WebSocket connections are detached from axum's graceful shutdown; wait for them here so
    // `ServerShuttingDown` reaches clients before the process exits.
    let deadline = tokio::time::Instant::now() + Limits::SHUTDOWN_GRACE;
    if tokio::time::timeout_at(deadline, hub_task).await.is_err() {
        tracing::warn!("hub did not stop in time");
    }
    if tokio::time::timeout_at(deadline, connections.all_closed())
        .await
        .is_err()
    {
        tracing::warn!("connections did not close in time");
    }
    result
}

/// Behind the reverse proxy on the same host every peer is loopback; only then is the
/// proxy's `X-Real-IP` trusted.
fn client_ip(peer: SocketAddr, headers: &HeaderMap) -> IpAddr {
    let forwarded = || {
        headers
            .get("x-real-ip")?
            .to_str()
            .ok()?
            .trim()
            .parse::<IpAddr>()
            .ok()
    };
    match peer.ip() {
        ip if ip.is_loopback() => forwarded().unwrap_or(ip),
        ip => ip,
    }
}

async fn websocket(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    let slot = match state.connections.acquire(client_ip(peer, &headers)) {
        Ok(slot) => slot,
        Err(refused @ ConnectionRefused::ServerFull) => {
            return (StatusCode::SERVICE_UNAVAILABLE, refused.to_string()).into_response();
        }
        Err(refused @ ConnectionRefused::TooManyFromAddress) => {
            return (StatusCode::TOO_MANY_REQUESTS, refused.to_string()).into_response();
        }
    };
    upgrade
        .max_message_size(Limits::MAX_REPLAY_MESSAGE_BYTES)
        .max_frame_size(Limits::MAX_REPLAY_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            connection::run(state.context, socket).await;
            drop(slot);
        })
}
