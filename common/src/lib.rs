pub mod proto {
    tonic::include_proto!("snake_game");
}

pub mod id_generator;
pub mod logger;

pub use proto::*;
