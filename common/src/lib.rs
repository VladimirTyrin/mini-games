pub mod proto {
    tonic::include_proto!("snake_game");
}

pub mod id_generator;
pub mod logger;
pub mod identifiers;
pub mod config;

pub use proto::*;
pub use identifiers::*;