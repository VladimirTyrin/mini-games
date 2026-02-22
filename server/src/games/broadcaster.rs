use std::future::Future;

use crate::{ClientId, GameOverNotification, GameStateUpdate};

pub trait GameBroadcaster: Send + Sync + Clone + 'static {
    fn broadcast_state(
        &self,
        state: GameStateUpdate,
        recipients: Vec<ClientId>,
    ) -> impl Future<Output = ()> + Send;

    fn broadcast_game_over(
        &self,
        notification: GameOverNotification,
        recipients: Vec<ClientId>,
    ) -> impl Future<Output = ()> + Send;
}
