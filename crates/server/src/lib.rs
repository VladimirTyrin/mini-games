//! Mini games server: WebSocket (protobuf JSON) front end, lobbies, and lockstep match actors
//! built on the shared game core. See `docs/architecture.md`.

mod app;
mod auth;
mod authenticated;
mod clock;
mod codec;
mod config;
mod connection;
mod context;
mod hub;
mod ids;
mod limits;
mod matches;
mod outbound;
mod token_bucket;

pub use app::{ServeError, serve};
pub use config::Config;
